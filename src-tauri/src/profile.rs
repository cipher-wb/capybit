//! profile.json — long-lived facts about the user. Read on startup, written
//! after daily_summary (M3+). For M2 we only need the read path + a stub
//! default so prompt assembly works before any facts exist.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Profile {
    #[serde(default)]
    pub birth: Option<Birth>,
    #[serde(default)]
    pub user_facts: Vec<Fact>,
    #[serde(default)]
    pub milestones: Vec<Milestone>,
    #[serde(default)]
    pub name_evolution: Vec<String>,
    #[serde(default)]
    pub user_name_for_capybit: Option<String>,
    /// Current name of the capybara. Defaults to "Capy" until the birth ritual
    /// (M5) lets the user override.
    #[serde(default = "default_name")]
    pub name: String,
}

fn default_name() -> String {
    "Capy".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Birth {
    pub named_by_user: String,
    pub named_at: String,
    pub source_file: String,
    pub source_excerpt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    pub fact: String,
    pub extracted_at: String,
    pub confidence: f32,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub date: String,
    pub summary: String,
    pub full_excerpt: String,
}

fn path(app: &AppHandle) -> PathBuf {
    let dir = app.path().app_data_dir().expect("app_data_dir");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("profile.json")
}

pub fn load(app: &AppHandle) -> Profile {
    let p = path(app);
    match std::fs::read_to_string(&p) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|err| {
            tracing::warn!(?err, "profile.json malformed, using empty profile");
            Profile::default_with_name()
        }),
        Err(_) => Profile::default_with_name(),
    }
}

pub fn save(app: &AppHandle, profile: &Profile) -> std::io::Result<()> {
    let p = path(app);
    let json = serde_json::to_string_pretty(profile).map_err(std::io::Error::other)?;
    std::fs::write(&p, json)
}

impl Profile {
    fn default_with_name() -> Self {
        Self {
            name: default_name(),
            ..Default::default()
        }
    }
}
