//! OpenRouter streaming chat with function-calling loop.
//!
//! Each user message can take up to MAX_TOOL_ROUNDS turns with the model:
//! we stream one completion; if the model returned tool_calls, we execute
//! them, append the tool results as messages, and stream again. The loop
//! ends when the model returns text without tool calls.
//!
//! Events emitted to the frontend (unchanged from M2):
//!   "llm-chunk"  → { request_id, delta }
//!   "llm-done"   → { request_id, full_text, prompt_tokens, completion_tokens }
//!   "llm-error"  → { request_id, message }
//!
//! Tool execution side effects (e.g. `lcd-scene`) go out via their own
//! event channels — see `llm::tools`.

use std::time::Duration;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use super::tools::{self, ToolCall, ToolFnCall};
use super::{ChatMessage, LlmError};
use crate::config::Config;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_TOOL_ROUNDS: usize = 3;

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

#[derive(Default, Deserialize, Clone, Debug)]
struct Usage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
}

enum RoundResult {
    Final {
        full_text_appended: String,
        usage: Usage,
    },
    Tools {
        tool_calls: Vec<ToolCall>,
        text_before_tools: Option<String>,
        /// DeepSeek thinking-mode models (and similar) require the reasoning
        /// content from the assistant's turn be passed back verbatim in the
        /// next request. None for non-thinking models.
        reasoning_content: Option<String>,
    },
}

#[derive(Default)]
struct AccumTool {
    id: String,
    name: String,
    arguments: String,
}

pub async fn stream_chat(
    app: AppHandle,
    request_id: String,
    cfg: Config,
    initial_messages: Vec<ChatMessage>,
) {
    if !cfg.is_usable() {
        emit_error(&app, &request_id, &LlmError::NoApiKey.to_string());
        return;
    }

    // Internal message stack as serde_json so we can shape assistant
    // messages with tool_calls and tool result messages with tool_call_id.
    let mut messages: Vec<serde_json::Value> = initial_messages
        .into_iter()
        .map(|m| serde_json::json!({ "role": m.role, "content": m.content }))
        .collect();

    let mut full_text = String::new();
    let mut total_prompt_tokens: u32 = 0;
    let mut total_completion_tokens: u32 = 0;

    for round in 0..MAX_TOOL_ROUNDS {
        match run_one_round(&app, &request_id, &cfg, &messages).await {
            Err(err) => {
                tracing::error!(?err, "stream failed");
                emit_error(&app, &request_id, &err.to_string());
                return;
            }
            Ok(RoundResult::Final {
                full_text_appended,
                usage,
            }) => {
                full_text.push_str(&full_text_appended);
                total_prompt_tokens += usage.prompt_tokens;
                total_completion_tokens += usage.completion_tokens;
                tracing::info!(
                    model = %cfg.model,
                    rounds = round + 1,
                    prompt_tokens = total_prompt_tokens,
                    completion_tokens = total_completion_tokens,
                    chars = full_text.chars().count(),
                    "llm chat done"
                );
                emit_done(
                    &app,
                    &request_id,
                    full_text,
                    total_prompt_tokens,
                    total_completion_tokens,
                );
                return;
            }
            Ok(RoundResult::Tools {
                tool_calls,
                text_before_tools,
                reasoning_content,
            }) => {
                if let Some(t) = &text_before_tools {
                    full_text.push_str(t);
                }
                // Append the assistant message with tool_calls. If the model
                // emitted reasoning_content (thinking mode), pass it back —
                // DeepSeek V4 etc. require this.
                let mut asst_msg = serde_json::json!({
                    "role": "assistant",
                    "content": text_before_tools,
                    "tool_calls": tool_calls,
                });
                if let Some(rc) = reasoning_content {
                    if let Some(obj) = asst_msg.as_object_mut() {
                        obj.insert("reasoning_content".into(), serde_json::Value::String(rc));
                    }
                }
                messages.push(asst_msg);
                // Execute each and append the result.
                for tc in &tool_calls {
                    let result = tools::execute(&app, &tc.function.name, &tc.function.arguments);
                    messages.push(serde_json::json!({
                        "role": "tool",
                        "tool_call_id": tc.id,
                        "content": result,
                    }));
                }
                // Loop into next round.
            }
        }
    }

    emit_error(
        &app,
        &request_id,
        &format!("model exceeded {MAX_TOOL_ROUNDS} tool rounds without finalizing"),
    );
}

async fn run_one_round(
    app: &AppHandle,
    request_id: &str,
    cfg: &Config,
    messages: &[serde_json::Value],
) -> Result<RoundResult, LlmError> {
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()?;

    // max_tokens needs to cover BOTH the assistant's text reply AND any
    // tool call JSON arguments (which can be lengthy for set_scene with
    // overlays/cells), AND for thinking-mode models (DeepSeek V4 Pro etc)
    // the reasoning_content too. Earlier 220 truncated tool args mid-JSON.
    let body = serde_json::json!({
        "model": cfg.model,
        "messages": messages,
        "stream": true,
        "max_tokens": 1500,
        "temperature": 0.85,
        "tools": tools::all_tools(),
        "tool_choice": "auto",
    });

    tracing::debug!(
        model = %cfg.model,
        msg_count = messages.len(),
        body_bytes = serde_json::to_string(&body).map(|s| s.len()).unwrap_or(0),
        "llm round: sending request (with tools)"
    );

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
    let mut full_content = String::new();
    let mut reasoning_content = String::new();
    let mut tools_by_idx: std::collections::BTreeMap<u32, AccumTool> = Default::default();
    let mut usage = Usage::default();
    let mut finish_reason: Option<String> = None;

    while let Some(chunk) = stream.next().await {
        let bytes = chunk?;
        buf.push_str(&String::from_utf8_lossy(&bytes));
        while let Some(idx) = buf.find("\n\n") {
            let event = buf[..idx].to_string();
            buf.drain(..idx + 2);
            for line in event.lines() {
                let line = line.trim();
                let Some(data) = line.strip_prefix("data:") else {
                    continue;
                };
                let data = data.trim();
                if data == "[DONE]" || data.is_empty() {
                    continue;
                }
                let parsed: serde_json::Value = match serde_json::from_str(data) {
                    Ok(v) => v,
                    Err(err) => {
                        tracing::debug!(?err, %data, "skip non-chunk line");
                        continue;
                    }
                };
                if let Some(u) = parsed.get("usage") {
                    if let Ok(u) = serde_json::from_value::<Usage>(u.clone()) {
                        usage = u;
                    }
                }
                let Some(choices) = parsed.get("choices").and_then(|v| v.as_array()) else {
                    continue;
                };
                let Some(choice) = choices.first() else {
                    continue;
                };
                if let Some(fr) = choice.get("finish_reason").and_then(|v| v.as_str()) {
                    finish_reason = Some(fr.to_string());
                }
                let Some(delta) = choice.get("delta") else {
                    continue;
                };

                if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                    if !content.is_empty() {
                        full_content.push_str(content);
                        let _ = app.emit(
                            "llm-chunk",
                            ChunkEvent {
                                request_id: request_id.to_string(),
                                delta: content.to_string(),
                            },
                        );
                    }
                }
                // Thinking-mode reasoning content (DeepSeek V4 etc). Not sent
                // to the UI — accumulated so we can pass it back next round.
                for key in ["reasoning_content", "reasoning"] {
                    if let Some(r) = delta.get(key).and_then(|v| v.as_str()) {
                        reasoning_content.push_str(r);
                    }
                }
                if let Some(tcs) = delta.get("tool_calls").and_then(|v| v.as_array()) {
                    for tc in tcs {
                        let idx = tc.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                        let entry = tools_by_idx.entry(idx).or_default();
                        if let Some(id) = tc.get("id").and_then(|v| v.as_str()) {
                            if entry.id.is_empty() {
                                entry.id = id.to_string();
                            }
                        }
                        if let Some(func) = tc.get("function") {
                            if let Some(name) = func.get("name").and_then(|v| v.as_str()) {
                                if entry.name.is_empty() {
                                    entry.name = name.to_string();
                                }
                            }
                            if let Some(args) = func.get("arguments").and_then(|v| v.as_str()) {
                                entry.arguments.push_str(args);
                            }
                        }
                    }
                }
            }
        }
    }

    let collected_tools: Vec<ToolCall> = tools_by_idx
        .into_iter()
        .map(|(idx, t)| ToolCall {
            id: if t.id.is_empty() {
                format!("call_local_{idx}")
            } else {
                t.id
            },
            kind: "function".into(),
            function: ToolFnCall {
                name: t.name,
                arguments: if t.arguments.is_empty() {
                    "{}".into()
                } else {
                    t.arguments
                },
            },
        })
        .filter(|t| !t.function.name.is_empty())
        .collect();

    tracing::debug!(
        finish_reason = ?finish_reason,
        content_len = full_content.chars().count(),
        reasoning_len = reasoning_content.chars().count(),
        tool_calls_n = collected_tools.len(),
        "llm round: stream complete"
    );

    // If the model hit the max_tokens limit mid-tool-call, the arguments JSON
    // will be truncated and unparseable. We'd rather degrade gracefully than
    // try to feed a broken tool call back: skip tool execution this round
    // and emit whatever text we have.
    let any_truncated_tool = collected_tools.iter().any(|t| {
        serde_json::from_str::<serde_json::Value>(&t.function.arguments).is_err()
    });
    if any_truncated_tool && finish_reason.as_deref() == Some("length") {
        tracing::warn!(
            tools_n = collected_tools.len(),
            "tool arguments truncated by max_tokens; emitting whatever text we have"
        );
        return Ok(RoundResult::Final {
            full_text_appended: full_content,
            usage,
        });
    }

    let is_tool_round = !collected_tools.is_empty()
        && (finish_reason.as_deref() == Some("tool_calls") || full_content.trim().is_empty());

    if is_tool_round {
        return Ok(RoundResult::Tools {
            tool_calls: collected_tools,
            text_before_tools: if full_content.is_empty() {
                None
            } else {
                Some(full_content)
            },
            reasoning_content: if reasoning_content.is_empty() {
                None
            } else {
                Some(reasoning_content)
            },
        });
    }

    Ok(RoundResult::Final {
        full_text_appended: full_content,
        usage,
    })
}

fn emit_error(app: &AppHandle, request_id: &str, message: &str) {
    let _ = app.emit(
        "llm-error",
        ErrorEvent {
            request_id: request_id.to_string(),
            message: message.to_string(),
        },
    );
}

fn emit_done(
    app: &AppHandle,
    request_id: &str,
    full_text: String,
    prompt_tokens: u32,
    completion_tokens: u32,
) {
    let _ = app.emit(
        "llm-done",
        DoneEvent {
            request_id: request_id.to_string(),
            full_text,
            prompt_tokens,
            completion_tokens,
        },
    );
}
