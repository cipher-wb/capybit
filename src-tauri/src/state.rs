//! Runtime state for M0 — only window position and visibility.
//! Later milestones extend this with energy/mood/curiosity/urge.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedState {
    pub position: Position,
    pub hidden: bool,
    /// M4: persisted inner state (energy/mood/curiosity/urge etc).
    /// Old state.json files without this field default to None and we'll
    /// initialize the runtime store with `InnerState::default()`.
    #[serde(default)]
    pub inner: Option<crate::scheduler::InnerState>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            position: Position { x: 1600, y: 800 },
            hidden: false,
            inner: None,
        }
    }
}

/// Wrapper held by Tauri's `app.manage()` so commands can read/write the in-memory state.
pub struct AppState(pub Mutex<PersistedState>);

impl AppState {
    pub fn new(initial: PersistedState) -> Self {
        Self(Mutex::new(initial))
    }
}

fn state_path(app: &AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_data_dir()
        .expect("cannot resolve app_data_dir");
    if !dir.exists() {
        let _ = std::fs::create_dir_all(&dir);
    }
    dir.join("state.json")
}

pub fn load(app: &AppHandle) -> PersistedState {
    let path = state_path(app);
    match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|err| {
            tracing::warn!(?err, "state.json malformed, using defaults");
            PersistedState::default()
        }),
        Err(_) => PersistedState::default(),
    }
}

pub fn save(app: &AppHandle, state: &PersistedState) {
    let path = state_path(app);
    match serde_json::to_string_pretty(state) {
        Ok(json) => {
            if let Err(err) = std::fs::write(&path, json) {
                tracing::error!(?err, ?path, "failed to write state.json");
            }
        }
        Err(err) => tracing::error!(?err, "failed to serialize state"),
    }
}
