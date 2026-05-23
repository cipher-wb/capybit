//! Proactive opener generation (PRD §4.3).
//!
//! When the state machine decides to speak (urge > 70 + !in_flow + cooldown
//! elapsed), the scheduler calls this to produce one short opening line based
//! on what the user is doing right now. Returned text is also persisted to
//! conversations.sqlite as a capybit turn so the next user reply has natural
//! context.

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::llm::ChatMessage;
use crate::perception::PerceptionSnapshot;
use crate::profile::Profile;
use crate::scheduler::InnerState;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

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

pub async fn generate(
    cfg: &Config,
    profile: &Profile,
    state: &InnerState,
    perception: &PerceptionSnapshot,
) -> Result<String, String> {
    if !cfg.is_usable() {
        return Ok(fallback(state, perception));
    }
    let prompt = render_prompt(profile, state, perception);
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
        temperature: 0.9,
        max_tokens: 80,
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
        return Ok(fallback(state, perception));
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
        return Ok(fallback(state, perception));
    }
    Ok(cleaned)
}

fn render_prompt(profile: &Profile, state: &InnerState, perception: &PerceptionSnapshot) -> String {
    let template = read_template("proactive_opener.md");
    let user_name = profile
        .user_name_for_capybit
        .clone()
        .unwrap_or_else(|| "你".into());
    template
        .replace("{name}", &profile.name)
        .replace("{user_name}", &user_name)
        .replace("{energy}", &format!("{:.0}", state.energy))
        .replace("{mood}", &format!("{:.0}", state.mood))
        .replace("{curiosity}", &format!("{:.0}", state.curiosity))
        .replace("{urge}", &format!("{:.0}", state.urge))
        .replace(
            "{active_app}",
            perception.active_app.as_deref().unwrap_or("（不知道）"),
        )
        .replace(
            "{window_title}",
            perception.window_title.as_deref().unwrap_or(""),
        )
        .replace("{idle_seconds}", &perception.idle_seconds.to_string())
}

fn read_template(name: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("docs")
        .join("prompts")
        .join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|err| {
        tracing::error!(?err, ?p, "missing opener template");
        String::new()
    })
}

/// No-API-key / network-error fallback. Picks a soft observational line.
fn fallback(state: &InnerState, perception: &PerceptionSnapshot) -> String {
    if perception.idle_seconds > 30 * 60 {
        return "你回来啦？".into();
    }
    if state.curiosity > 80.0 {
        return "你在干嘛呀？看起来挺专注的。".into();
    }
    if state.mood < 40.0 {
        return "我今天有点闷闷的，你也是吗？".into();
    }
    "嘿，跟你打个招呼。".into()
}
