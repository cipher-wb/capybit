//! 60-second tick loop. Holds the canonical `InnerState` behind a Mutex
//! managed by Tauri so commands (chat, hover) can read or mutate it.

use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use super::{state_machine, InnerState};
use crate::perception::{self, PerceptionSnapshot};

/// Minimum gap between two proactive-opener firings. PRD doesn't pin a
/// number; 30 min is the polite default. "等会儿" 按钮 manually resets the
/// clock too (so user can mute for the cooldown window).
const PROACTIVE_COOLDOWN_SECS: i64 = 30 * 60;

pub const TICK_INTERVAL: Duration = Duration::from_secs(60);
const PERSIST_EVERY_N_TICKS: u32 = 5;

/// State holder owned by Tauri. Wraps last perception too so the next tick
/// knows what app the user was on previously (for curiosity bump detection).
pub struct InnerStateStore {
    pub state: Mutex<InnerState>,
    pub last_perception: Mutex<Option<PerceptionSnapshot>>,
}

impl InnerStateStore {
    pub fn new(initial: InnerState) -> Self {
        Self {
            state: Mutex::new(initial),
            last_perception: Mutex::new(None),
        }
    }
}

#[derive(Serialize, Clone)]
struct PetStateEvent {
    energy: f32,
    mood: f32,
    curiosity: f32,
    urge: f32,
    current_action: String,
    in_flow: bool,
    active_app: Option<String>,
}

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut tick_count: u32 = 0;
        loop {
            tokio::time::sleep(TICK_INTERVAL).await;
            tick_count = tick_count.wrapping_add(1);
            run_one_tick(&app, tick_count).await;
        }
    });
}

async fn run_one_tick(app: &AppHandle, tick_count: u32) {
    let perception = perception::capture();

    let store = app.state::<InnerStateStore>();
    let (event, action_changed, want_proactive) = {
        let mut s = store.state.lock().unwrap();
        let mut last = store.last_perception.lock().unwrap();
        let prev_app = last.as_ref().and_then(|p| p.active_app.clone());
        let prev_action = s.current_action.clone();
        state_machine::step(&mut s, &perception, prev_app.as_deref());
        *last = Some(perception.clone());
        let changed = prev_action != s.current_action;

        // Proactive trigger: urge crossed threshold + not in flow + cooldown
        // expired. We mark `last_proactive_at` here (inside the lock) so a
        // slow LLM opener call doesn't cause a second trigger on the next tick.
        let trigger = should_fire_proactive(&s);
        if trigger {
            s.last_proactive_at = OffsetDateTime::now_utc().format(&Rfc3339).ok();
        }
        (snapshot(&s, &perception), changed, trigger)
    };

    let _ = app.emit("pet-state", event.clone());

    if action_changed {
        tracing::debug!(
            action = %event.current_action,
            energy = event.energy,
            mood = event.mood,
            curiosity = event.curiosity,
            urge = event.urge,
            "action changed"
        );
    }

    if want_proactive {
        spawn_proactive_opener(app.clone(), perception.clone());
    }

    if tick_count.is_multiple_of(PERSIST_EVERY_N_TICKS) {
        persist_inner_state(app);
    }
}

fn should_fire_proactive(state: &InnerState) -> bool {
    if state.urge <= 70.0 {
        return false;
    }
    if state.in_flow {
        return false;
    }
    let now = OffsetDateTime::now_utc();
    if let Some(last) = state
        .last_proactive_at
        .as_deref()
        .and_then(|s| OffsetDateTime::parse(s, &Rfc3339).ok())
    {
        let elapsed = (now - last).whole_seconds();
        if elapsed < PROACTIVE_COOLDOWN_SECS {
            return false;
        }
    }
    true
}

/// Generate the opener line off the tick thread, persist it as a capybit
/// turn so user replies have context, then emit `proactive-opener` for the
/// frontend to surface the bubble.
fn spawn_proactive_opener(app: AppHandle, perception: PerceptionSnapshot) {
    tauri::async_runtime::spawn(async move {
        let cfg = crate::config::load(&app);
        let prof = crate::profile::load(&app);
        let state_snapshot = {
            let store = app.state::<InnerStateStore>();
            let s = store.state.lock().unwrap();
            s.clone()
        };

        let opener =
            match crate::llm::opener::generate(&cfg, &prof, &state_snapshot, &perception).await {
                Ok(text) => text,
                Err(err) => {
                    tracing::warn!(?err, "proactive opener generation failed");
                    return;
                }
            };

        tracing::info!(opener = %opener, "proactive opener firing");

        // Persist as a capybit turn so when the user replies, history is natural.
        let db = app
            .state::<std::sync::Arc<crate::memory::Db>>()
            .inner()
            .clone();
        if let Err(err) = crate::memory::write_message(&db, "capybit", &opener, None) {
            tracing::warn!(?err, "failed to persist proactive opener");
        }

        let _ = app.emit("proactive-opener", ProactiveEvent { text: opener });
    });
}

#[derive(Serialize, Clone)]
struct ProactiveEvent {
    text: String,
}

fn snapshot(s: &InnerState, p: &PerceptionSnapshot) -> PetStateEvent {
    PetStateEvent {
        energy: s.energy,
        mood: s.mood,
        curiosity: s.curiosity,
        urge: s.urge,
        current_action: s.current_action.clone(),
        in_flow: s.in_flow,
        active_app: p.active_app.clone(),
    }
}

fn persist_inner_state(app: &AppHandle) {
    let inner = app.state::<InnerStateStore>().state.lock().unwrap().clone();
    let window_state = app.state::<crate::state::AppState>();
    let snapshot = {
        let mut s = window_state.0.lock().unwrap();
        s.inner = Some(inner);
        s.clone()
    };
    crate::state::save(app, &snapshot);
}
