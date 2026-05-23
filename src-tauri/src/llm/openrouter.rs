//! OpenRouter streaming chat client.
//!
//! Emits chunks via Tauri events:
//!   "llm-chunk"  → { request_id, delta }
//!   "llm-done"   → { request_id, full_text, prompt_tokens, completion_tokens }
//!   "llm-error"  → { request_id, message }
//!
//! Cost monitoring per CLAUDE.md §9.3: every completed request logs token
//! counts via `tracing::info!` so they can be tallied offline.

use std::time::Duration;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use super::{ChatMessage, LlmError};
use crate::config::Config;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    stream: bool,
    /// PRD §3.2.2 — output should feel "温柔语速", so we keep responses short.
    /// Real cap; not just a stylistic hint. ~150 tokens ≈ 2-3 short sentences.
    max_tokens: u32,
    temperature: f32,
}

#[derive(Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct StreamChoice {
    delta: Delta,
}

#[derive(Deserialize, Default)]
struct Delta {
    #[serde(default)]
    content: Option<String>,
}

#[derive(Deserialize, Default, Clone)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Serialize, Clone)]
struct ChunkEvent {
    request_id: String,
    delta: String,
}

#[derive(Serialize, Clone)]
struct DoneEvent {
    request_id: String,
    full_text: String,
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Serialize, Clone)]
struct ErrorEvent {
    request_id: String,
    message: String,
}

pub async fn stream_chat(
    app: AppHandle,
    request_id: String,
    cfg: Config,
    messages: Vec<ChatMessage>,
) {
    if !cfg.is_usable() {
        let _ = app.emit(
            "llm-error",
            ErrorEvent {
                request_id,
                message: LlmError::NoApiKey.to_string(),
            },
        );
        return;
    }

    let request_id_for_err = request_id.clone();
    if let Err(err) = run(app.clone(), request_id, cfg, messages).await {
        tracing::error!(?err, "llm stream failed");
        let _ = app.emit(
            "llm-error",
            ErrorEvent {
                request_id: request_id_for_err,
                message: err.to_string(),
            },
        );
    }
}

async fn run(
    app: AppHandle,
    request_id: String,
    cfg: Config,
    messages: Vec<ChatMessage>,
) -> Result<(), LlmError> {
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()?;

    let body = ChatRequest {
        model: &cfg.model,
        messages: &messages,
        stream: true,
        max_tokens: 200,
        temperature: 0.85,
    };

    let url = format!("{}/chat/completions", cfg.base_url.trim_end_matches('/'));
    let resp = client
        .post(&url)
        .bearer_auth(&cfg.api_key)
        .header("HTTP-Referer", "https://github.com/cipher/capybit")
        .header("X-Title", "Capybit")
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Err(LlmError::Server(status, text));
    }

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    let mut full = String::new();
    let mut usage: Option<Usage> = None;

    while let Some(chunk) = stream.next().await {
        let bytes = chunk?;
        buf.push_str(&String::from_utf8_lossy(&bytes));

        // Process complete SSE events (separated by "\n\n").
        while let Some(idx) = buf.find("\n\n") {
            let event = buf[..idx].to_string();
            buf.drain(..idx + 2);

            for line in event.lines() {
                let line = line.trim();
                let Some(data) = line.strip_prefix("data:") else {
                    continue;
                };
                let data = data.trim();
                if data == "[DONE]" {
                    continue;
                }
                if data.is_empty() {
                    continue;
                }
                let parsed: StreamChunk = match serde_json::from_str(data) {
                    Ok(p) => p,
                    Err(err) => {
                        tracing::debug!(?err, %data, "skip non-chunk line");
                        continue;
                    }
                };
                if let Some(u) = parsed.usage {
                    usage = Some(u);
                }
                if let Some(choice) = parsed.choices.into_iter().next() {
                    if let Some(delta) = choice.delta.content {
                        if !delta.is_empty() {
                            full.push_str(&delta);
                            let _ = app.emit(
                                "llm-chunk",
                                ChunkEvent {
                                    request_id: request_id.clone(),
                                    delta,
                                },
                            );
                        }
                    }
                }
            }
        }
    }

    let usage = usage.unwrap_or_default();
    tracing::info!(
        model = %cfg.model,
        prompt_tokens = usage.prompt_tokens,
        completion_tokens = usage.completion_tokens,
        chars = full.chars().count(),
        "llm request done"
    );

    let _ = app.emit(
        "llm-done",
        DoneEvent {
            request_id,
            full_text: full,
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
        },
    );
    Ok(())
}
