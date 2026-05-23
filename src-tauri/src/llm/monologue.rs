//! Inner-monologue LLM call (PRD §3.5).
//!
//! "用户悬停查看'他在做什么'时，基于 4 个数值 + 时间 + 上次动作，由 LLM
//! （或缓存）生成一段一句话的内心独白。"
//!
//! Strategy: cache per (action × hour). Hovering 50 times in an hour
//! still results in at most 1 LLM call per action change.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::config::Config;
use crate::llm::ChatMessage;
use crate::scheduler::InnerState;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
/// Reuse a generated monologue for this many seconds even if the action
/// hasn't changed. 5 min keeps things feeling alive without burning tokens.
const CACHE_TTL_SECS: i64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedMonologue {
    pub text: String,
    pub action_at_capture: String,
    pub captured_at: String,
}

pub struct MonologueCache(pub Mutex<Option<CachedMonologue>>);
impl Default for MonologueCache {
    fn default() -> Self {
        Self(Mutex::new(None))
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    temperature: f32,
    max_tokens: u32,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}
#[derive(Deserialize)]
struct Choice {
    message: AssistantMessage,
}
#[derive(Deserialize)]
struct AssistantMessage {
    content: String,
}

pub async fn get_or_fetch(
    cfg: &Config,
    cache: &MonologueCache,
    state: &InnerState,
    active_app: Option<&str>,
    capybara_name: &str,
) -> Result<String, String> {
    if let Some(cached) = cache.0.lock().unwrap().as_ref() {
        if is_fresh(cached, &state.current_action) {
            return Ok(cached.text.clone());
        }
    }
    let text = fetch(cfg, state, active_app, capybara_name).await?;
    {
        let mut guard = cache.0.lock().unwrap();
        *guard = Some(CachedMonologue {
            text: text.clone(),
            action_at_capture: state.current_action.clone(),
            captured_at: OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .unwrap_or_default(),
        });
    }
    Ok(text)
}

fn is_fresh(cached: &CachedMonologue, current_action: &str) -> bool {
    if cached.action_at_capture != current_action {
        return false;
    }
    let captured = match OffsetDateTime::parse(&cached.captured_at, &Rfc3339) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let age = (OffsetDateTime::now_utc() - captured).whole_seconds();
    age < CACHE_TTL_SECS
}

async fn fetch(
    cfg: &Config,
    state: &InnerState,
    active_app: Option<&str>,
    capybara_name: &str,
) -> Result<String, String> {
    if !cfg.is_usable() {
        return Ok(fallback_monologue(state));
    }

    let prompt = render_prompt(state, active_app, capybara_name);
    let messages = vec![ChatMessage {
        role: "user".into(),
        content: prompt,
    }];

    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;

    let body = ChatRequest {
        model: &cfg.model,
        messages: &messages,
        temperature: 0.95,
        max_tokens: 60,
    };

    let url = format!("{}/chat/completions", cfg.base_url.trim_end_matches('/'));
    let resp = client
        .post(&url)
        .bearer_auth(&cfg.api_key)
        .header("HTTP-Referer", "https://github.com/cipher/capybit")
        .header("X-Title", "Capybit")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Ok(fallback_monologue(state));
    }

    let parsed: ChatResponse = resp.json().await.map_err(|e| e.to_string())?;
    let raw = parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .unwrap_or_default();
    let cleaned = raw
        .trim()
        .trim_matches('"')
        .trim_matches('「')
        .trim_matches('」')
        .trim()
        .to_string();
    if cleaned.is_empty() {
        tracing::warn!(%raw, "monologue LLM returned empty content, using fallback");
        return Ok(fallback_monologue(state));
    }
    tracing::debug!(text = %cleaned, "monologue fetched");
    Ok(cleaned)
}

fn render_prompt(state: &InnerState, active_app: Option<&str>, capybara_name: &str) -> String {
    let template = read_template("inner_monologue.md");
    template
        .replace("{name}", capybara_name)
        .replace("{energy}", &format!("{:.0}", state.energy))
        .replace("{mood}", &format!("{:.0}", state.mood))
        .replace("{curiosity}", &format!("{:.0}", state.curiosity))
        .replace("{urge}", &format!("{:.0}", state.urge))
        .replace("{current_action}", &state.current_action)
        .replace("{active_app}", active_app.unwrap_or("（不知道）"))
        .replace("{in_flow}", if state.in_flow { "是" } else { "不在" })
}

fn read_template(name: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("docs")
        .join("prompts")
        .join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|err| {
        tracing::error!(?err, ?p, "missing monologue template");
        String::new()
    })
}

/// Used when no api_key is configured or the network is dead. Short, mood-
/// aware fallbacks so hover still feels alive.
fn fallback_monologue(state: &InnerState) -> String {
    match state.current_action.as_str() {
        "sleep" => "在打盹儿。",
        "sad" => "今天心情有点低。",
        "curious" => "好像有点想跟你说话。",
        "walk" => "走两步，活动活动。",
        "stretch" => "伸了个懒腰。",
        _ if state.mood > 70.0 => "今天心情还不错。",
        _ => "在发呆。",
    }
    .to_string()
}
