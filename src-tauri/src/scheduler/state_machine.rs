//! 4-dim state machine (PRD §3.5). Pure-ish — given previous state +
//! perception, produce next state and the chosen action.

use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use super::InnerState;
use crate::perception::PerceptionSnapshot;

/// Seconds the user must dwell in one app with continuous activity before
/// flow mode kicks in. PRD §3.1.2 says 20 minutes.
const FLOW_DWELL_SECS: i64 = 20 * 60;

/// Idle threshold (seconds) above which we count the user as "absent" —
/// energy slowly regenerates, curiosity/urge stop accumulating.
const ABSENT_IDLE_SECS: u32 = 5 * 60;

/// Idle reset for flow mode (PRD §3.1.2): "连续 5 分钟无键鼠活动" → out of flow.
const FLOW_EXIT_IDLE_SECS: u32 = 5 * 60;

/// Update flow mode based on app dwell + activity. Mutates flow_app / since /
/// in_flow on `state`.
fn update_flow(state: &mut InnerState, perception: &PerceptionSnapshot) {
    let now = OffsetDateTime::now_utc();
    let active_app = perception.active_app.as_deref().unwrap_or("");

    // Long idle → reset flow.
    if perception.idle_seconds >= FLOW_EXIT_IDLE_SECS {
        state.in_flow = false;
        state.flow_app = None;
        state.flow_app_since = None;
        return;
    }

    // App switched → restart dwell timer.
    if state.flow_app.as_deref() != Some(active_app) {
        state.flow_app = Some(active_app.into());
        state.flow_app_since = now.format(&Rfc3339).ok();
        state.in_flow = false;
        return;
    }

    // Same app — check dwell length.
    if let Some(since_str) = state.flow_app_since.as_deref() {
        if let Ok(since) = OffsetDateTime::parse(since_str, &Rfc3339) {
            let dwell = (now - since).whole_seconds();
            state.in_flow = dwell >= FLOW_DWELL_SECS;
        }
    }
}

/// Single tick of the state machine. `prev_active_app` is the app captured
/// last tick — used for the "new app → curiosity bump" rule.
pub fn step(
    state: &mut InnerState,
    perception: &PerceptionSnapshot,
    prev_active_app: Option<&str>,
) {
    update_flow(state, perception);

    let user_present = perception.idle_seconds < ABSENT_IDLE_SECS;

    // energy: gentle restore when user absent, baseline drift otherwise
    if !user_present {
        state.energy = (state.energy + 1.5).min(100.0);
    } else if state.current_action == "walk" {
        state.energy = (state.energy - 0.3).max(0.0);
    } else if state.current_action == "sleep" {
        state.energy = (state.energy + 2.0).min(100.0);
    } else {
        state.energy = (state.energy + 0.2).min(100.0);
    }

    // mood: rises when user is present (passive co-presence is good for him);
    // drops slowly during long ignore. last_user_interaction-driven boost is
    // applied directly by send_message handlers when the user actually chats.
    if user_present {
        state.mood = (state.mood + 0.3).min(100.0);
    } else {
        let neglect = neglect_seconds(state);
        if neglect > 30 * 60 {
            state.mood = (state.mood - 0.5).max(0.0);
        }
    }

    // curiosity: bumps on new app, decays otherwise
    let new_app = match (prev_active_app, perception.active_app.as_deref()) {
        (Some(a), Some(b)) if a != b => true,
        (None, Some(_)) => true,
        _ => false,
    };
    if new_app {
        state.curiosity = (state.curiosity + 20.0).min(100.0);
    } else {
        state.curiosity = (state.curiosity - 1.0).max(0.0);
    }

    // urge: rises when curiosity high or after long silence
    let silence = neglect_seconds(state);
    if state.curiosity > 60.0 {
        state.urge = (state.urge + 2.0).min(100.0);
    }
    if silence > 30 * 60 {
        state.urge = (state.urge + 1.0).min(100.0);
    }
    // Flow protection: cap urge at 50 while user is in flow.
    if state.in_flow {
        state.urge = state.urge.min(50.0);
    }

    state.current_action = choose_action(state);
}

/// Seconds since the last chat. None → fall back to a large number so older
/// installs without `last_user_interaction` still drift gracefully.
fn neglect_seconds(state: &InnerState) -> i64 {
    state
        .last_user_interaction
        .as_deref()
        .and_then(|s| OffsetDateTime::parse(s, &Rfc3339).ok())
        .map(|t| (OffsetDateTime::now_utc() - t).whole_seconds())
        .unwrap_or(86_400)
}

/// PRD §3.5 decision table.
fn choose_action(state: &InnerState) -> String {
    if state.energy < 20.0 {
        return "sleep".into();
    }
    if state.mood < 30.0 {
        return "sad".into();
    }
    if state.curiosity > 70.0 && state.urge > 70.0 && !state.in_flow {
        // The bubble auto-open is M5 work — for now we just signal
        // "wanting to talk" as the `curious` animation.
        return "curious".into();
    }
    if state.curiosity > 70.0 {
        return "curious".into();
    }
    // Mild random oscillation between idle and stretch / walk so he doesn't
    // look frozen. Deterministic from urge to avoid RNG state plumbing.
    if state.urge > 40.0 {
        "stretch".into()
    } else if state.energy > 80.0 && state.curiosity > 40.0 {
        "walk".into()
    } else {
        "idle".into()
    }
}

/// Apply a positive interaction effect: user just chatted with the capybara.
/// PRD §3.5 rules: mood +5, urge reset, last_user_interaction stamp.
pub fn on_user_chat(state: &mut InnerState) {
    state.mood = (state.mood + 5.0).min(100.0);
    state.urge = 0.0;
    state.last_user_interaction = OffsetDateTime::now_utc().format(&Rfc3339).ok();
    // Cost of speaking, per PRD §3.5
    state.energy = (state.energy - 3.0).max(0.0);
}
