//! Perception layer (PRD §3.3).
//!
//! M4 scope: active window app/title + user idle duration.
//! Out of scope (later): screen capture, vision, lock-screen detection.

pub mod active_window;
pub mod activity;

use serde::{Deserialize, Serialize};

/// One snapshot of "what the user is doing right now". Cheap to assemble
/// (single OS call each for active-win and idle). Read every 5s by the tick
/// scheduler; also frozen into messages.context_snapshot if M3 needs it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerceptionSnapshot {
    pub active_app: Option<String>,
    pub window_title: Option<String>,
    /// Seconds since last keyboard/mouse input. 0 = currently active.
    pub idle_seconds: u32,
    pub captured_at: String,
}

/// Apps whose window title must never enter LLM context.
/// CLAUDE.md §8.1 blacklist. Returns true when the active app should be
/// considered "private" — perception still records the timing facts but
/// scrubs the title.
pub fn is_blacklisted_app(app: &str) -> bool {
    const BLACKLIST: &[&str] = &[
        "1Password",
        "Bitwarden",
        "KeePass",
        "LastPass",
        "支付宝",
        "Alipay",
        "微信",
        "WeChat",
        "网银",
    ];
    BLACKLIST.iter().any(|b| app.contains(b))
}

pub fn capture() -> PerceptionSnapshot {
    let (app, title) = active_window::current().unwrap_or((None, None));
    let scrub = app.as_deref().map(is_blacklisted_app).unwrap_or(false);
    PerceptionSnapshot {
        active_app: app,
        window_title: if scrub {
            Some("（敏感，已隐藏）".into())
        } else {
            title
        },
        idle_seconds: activity::idle_seconds(),
        captured_at: time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unknown".into()),
    }
}
