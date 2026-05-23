//! Local config (api key, model). Stored at app_data_dir/config.local.json.
//! Gitignored. Never logged.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const DEFAULT_MODEL: &str = "anthropic/claude-haiku-4-5";
const DEFAULT_BASE: &str = "https://openrouter.ai/api/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_base")]
    pub base_url: String,
    /// Soft fallback when the primary model errors. PRD §5.2 calls out GLM-4.6
    /// as the cost-equivalent alternative.
    #[serde(default)]
    pub fallback_model: Option<String>,
}

fn default_model() -> String {
    DEFAULT_MODEL.into()
}
fn default_base() -> String {
    DEFAULT_BASE.into()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: default_model(),
            base_url: default_base(),
            fallback_model: Some("z-ai/glm-4.6".into()),
        }
    }
}

fn path(app: &AppHandle) -> PathBuf {
    let dir = app.path().app_data_dir().expect("app_data_dir");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("config.local.json")
}

/// Load config from disk, creating a template on first run so the user has
/// a place to drop their api_key.
pub fn load(app: &AppHandle) -> Config {
    let p = path(app);
    if !p.exists() {
        let template = Config::default();
        if let Ok(json) = serde_json::to_string_pretty(&template) {
            let _ = std::fs::write(&p, json);
        }
        tracing::warn!(
            ?p,
            "config.local.json created with empty api_key — fill it in"
        );
        return template;
    }
    match std::fs::read_to_string(&p) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|err| {
            tracing::error!(?err, "config.local.json malformed, using defaults");
            Config::default()
        }),
        Err(err) => {
            tracing::error!(?err, "failed to read config.local.json");
            Config::default()
        }
    }
}

impl Config {
    pub fn is_usable(&self) -> bool {
        !self.api_key.trim().is_empty()
    }
}
