//! Tick scheduler + state machine (PRD §3.5).
//!
//! Tick = 60s (PRD §8.2 — do not shorten). Each tick:
//!   1. read perception snapshot
//!   2. update flow-mode flag (same-app dwell + activity)
//!   3. update energy/mood/curiosity/urge
//!   4. choose next action
//!   5. emit "pet-state" event to frontend
//!   6. every 5 minutes, persist to state.json

pub mod state_machine;
pub mod tick;

use serde::{Deserialize, Serialize};

/// Mutable per-tick inner state of the capybara. Persisted into the same
/// state.json as window position (see state::PersistedState).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct InnerState {
    pub energy: f32,
    pub mood: f32,
    pub curiosity: f32,
    pub urge: f32,
    pub current_action: String,
    pub in_flow: bool,
    /// RFC3339 of last user-facing interaction (chat send).
    pub last_user_interaction: Option<String>,
    /// App that started the current "same app" dwell, used for flow detection.
    #[serde(default)]
    pub flow_app: Option<String>,
    /// RFC3339 of when current `flow_app` started being foreground.
    #[serde(default)]
    pub flow_app_since: Option<String>,
    /// RFC3339 of the last proactive opener firing. Used for cooldown so the
    /// capybara doesn't pop bubbles at the user repeatedly.
    #[serde(default)]
    pub last_proactive_at: Option<String>,
}

impl Default for InnerState {
    fn default() -> Self {
        Self {
            energy: 75.0,
            mood: 60.0,
            curiosity: 30.0,
            urge: 10.0,
            current_action: "idle".into(),
            in_flow: false,
            last_user_interaction: None,
            flow_app: None,
            flow_app_since: None,
            last_proactive_at: None,
        }
    }
}
