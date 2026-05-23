//! Daily summary LLM call.
//!
//! Non-streaming. Asks the model to return JSON `{diary, new_facts}` —
//! parses it, writes summary to SQLite, merges high-confidence facts back
//! into profile.json.

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::config::Config;
use crate::llm::ChatMessage;
use crate::memory::{self, conversations, MessageRow};
use crate::profile::{self, Fact};

const AUTO_MERGE_CONFIDENCE: f32 = 0.8;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Deserialize, Debug)]
struct SummaryJson {
    diary: String,
    #[serde(default)]
    new_facts: Vec<FactJson>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct FactJson {
    fact: String,
    confidence: f32,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    temperature: f32,
    max_tokens: u32,
    /// Hint to provider that we want JSON. Many providers honor this; if not,
    /// our parser tolerates ```json fences.
    response_format: ResponseFormat<'a>,
}

#[derive(Serialize)]
struct ResponseFormat<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
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

/// Run the daily summary for one local date. Reads messages, calls LLM,
/// writes summary row, merges facts into profile.
pub async fn run_for_date(
    app: AppHandle,
    db: std::sync::Arc<memory::Db>,
    date_local: String,
) -> Result<(), String> {
    let messages = conversations::messages_for_date(&db, &date_local).map_err(|e| e.to_string())?;
    if messages.is_empty() {
        tracing::debug!(date = %date_local, "no messages, skipping summary");
        return Ok(());
    }

    let cfg = crate::config::load(&app);
    if !cfg.is_usable() {
        return Err("no api_key for daily summary".into());
    }
    let prof = profile::load(&app);

    let prompt = render_prompt(&prof.name, &messages);
    let parsed = call_llm(&cfg, &prompt).await?;

    // Write summary row.
    let facts_json = serde_json::to_string(&parsed.new_facts).ok();
    memory::write_summary(&db, &date_local, &parsed.diary, facts_json.as_deref())
        .map_err(|e| e.to_string())?;

    // Embed and index the diary for semantic recall. Best-effort — if the
    // embedding call fails, the diary still made it to SQL and the backfill
    // on next startup will retry. Don't fail the whole summary on this.
    match crate::llm::embedder::embed(&cfg, &parsed.diary).await {
        Ok(emb) => {
            if let Err(err) =
                memory::upsert_memory(&db, "daily_diary", &date_local, &parsed.diary, &emb)
            {
                tracing::warn!(?err, %date_local, "diary embed insert failed");
            }
        }
        Err(err) => {
            tracing::warn!(?err, %date_local, "diary embedding failed; backfill will retry")
        }
    }

    // Auto-merge high-confidence facts.
    let mut merged = 0usize;
    let mut dropped = 0usize;
    let now = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "unknown".into());
    let source = format!("daily_summary_{date_local}");

    let mut prof_mut = prof;
    for f in parsed.new_facts.iter() {
        if f.confidence >= AUTO_MERGE_CONFIDENCE {
            prof_mut.user_facts.push(Fact {
                fact: f.fact.clone(),
                extracted_at: now.clone(),
                confidence: f.confidence,
                source: source.clone(),
            });
            merged += 1;
        } else {
            dropped += 1;
        }
    }
    if merged > 0 {
        profile::save(&app, &prof_mut).map_err(|e| e.to_string())?;
    }
    tracing::info!(
        date = %date_local,
        merged,
        dropped,
        diary_chars = parsed.diary.chars().count(),
        "daily summary done"
    );
    Ok(())
}

fn render_prompt(capybara_name: &str, messages: &[MessageRow]) -> String {
    let template = read_template("daily_summary.md");

    let transcript = messages
        .iter()
        .map(|m| {
            let speaker = if m.role == "user" {
                "用户"
            } else {
                capybara_name
            };
            format!("{speaker}：{}", m.content)
        })
        .collect::<Vec<_>>()
        .join("\n");

    template
        .replace("{name}", capybara_name)
        .replace("{messages}", &transcript)
        .replace("{context_log}", "（M4 接入感知后填充）")
}

fn read_template(name: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("docs")
        .join("prompts")
        .join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|err| {
        tracing::error!(?err, ?p, "missing prompt template");
        String::new()
    })
}

async fn call_llm(cfg: &Config, prompt: &str) -> Result<SummaryJson, String> {
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;

    let messages = vec![ChatMessage {
        role: "user".into(),
        content: prompt.into(),
    }];
    let body = ChatRequest {
        model: &cfg.model,
        messages: &messages,
        temperature: 0.4,
        max_tokens: 600,
        response_format: ResponseFormat {
            kind: "json_object",
        },
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
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("server {status}: {text}"));
    }

    let parsed: ChatResponse = resp.json().await.map_err(|e| e.to_string())?;
    let raw = parsed
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| "no choices in response".to_string())?
        .message
        .content;

    let json_slice = extract_json(&raw);
    serde_json::from_str(json_slice).map_err(|e| format!("parse JSON failed: {e}\nraw: {raw}"))
}

/// Tolerate ```json fenced wrappers and leading prose.
fn extract_json(raw: &str) -> &str {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_prefix("```json") {
        if let Some(end) = rest.rfind("```") {
            return rest[..end].trim();
        }
    }
    if let Some(rest) = trimmed.strip_prefix("```") {
        if let Some(end) = rest.rfind("```") {
            return rest[..end].trim();
        }
    }
    // Fall back to substring between first { and last }.
    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        if end > start {
            return &trimmed[start..=end];
        }
    }
    trimmed
}
