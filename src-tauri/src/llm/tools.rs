//! LLM tools (function calling).
//!
//! Models can call these mid-conversation to take side effects. Right now
//! the only one is `set_scene`, which drives the 32×32 LCD compositor in
//! `src/renderer/placeholder_capybara.js`. Each tool sends events to the
//! frontend; the call's textual "result" (what we feed back to the model)
//! is a short status string.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub function: ToolFnCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFnCall {
    pub name: String,
    pub arguments: String,
}

/// JSON tool schemas — sent on every chat request.
///
/// The descriptions are intentionally specific about coordinate ranges,
/// available sprite/icon names, and font limits, because the model is the
/// thing deciding what to draw and needs accurate context.
pub fn all_tools() -> serde_json::Value {
    serde_json::json!([
        {
            "type": "function",
            "function": {
                "name": "set_scene",
                "description": "REQUIRED for emotional expression: display a custom scene on your 32x32 LCD screen for a few seconds. You SHOULD call this in most replies — to express what you feel (heart icon when happy, dots when thoughtful, a small drawing for visual reply), to show info (digits for time/temperature), or just to paint a small picture for the user. After duration_ms the screen reverts to default. Always reply with a short text alongside calling this.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "sprite": {
                            "type": "string",
                            "description": "Your base body pose (16x16 bitmap placed at cols 8-23, rows 10-25). Omit this field to keep your current body and only overlay things.",
                            "enum": ["idle", "idle_blink", "happy", "sad", "sleep", "surprised", "curious", "stretch", "walk_0", "walk_1", "talk_0", "talk_1", "sneeze_windup", "sneeze_burst"]
                        },
                        "overlays": {
                            "type": "array",
                            "description": "Optional list of icons, text, or raw pixel bitmaps overlaid on the LCD. Put them in the top zone (rows 0-9) or bottom zone (rows 26-31) so they don't overlap your body.",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "type": { "type": "string", "enum": ["icon", "text", "cells"] },
                                    "name": {
                                        "type": "string",
                                        "description": "Icon name when type=icon. One of: heart (7x6), heart_small (5x4), star (5x5), spark (3x3), z (3x5), exclaim (1x6), question (3x6), music (3x5), sun (5x5), moon (5x5), cloud (6x3), arrow_up (5x5), arrow_down (5x5), dots3 (5x1), capy_face (7x6)."
                                    },
                                    "text": {
                                        "type": "string",
                                        "description": "Text to render with the 5x7 pixel font when type=text. ASCII only: digits, uppercase A-Z (lowercase auto-uppercased), and these symbols: : . - / ? ! space. Each char is 6 cells wide (5 + 1 gap)."
                                    },
                                    "cells": {
                                        "type": "array",
                                        "items": { "type": "string" },
                                        "description": "Raw bitmap as array of strings; '#' = lit pixel, '.' = off. Use when type=cells. This is your most expressive tool — draw small custom pictures (a snowflake, a face, a swirl, anything you want to express)."
                                    },
                                    "x": { "type": "integer", "minimum": 0, "maximum": 31, "description": "Top-left column (0-31)." },
                                    "y": { "type": "integer", "minimum": 0, "maximum": 31, "description": "Top-left row (0-31)." },
                                    "blink": { "type": "boolean", "description": "If true, the overlay blinks at ~1Hz." },
                                    "animate": { "type": "string", "enum": ["bob"], "description": "Optional animation. 'bob' = gentle vertical bounce." }
                                },
                                "required": ["type", "x", "y"]
                            }
                        },
                        "duration_ms": {
                            "type": "integer",
                            "minimum": 1500,
                            "maximum": 30000,
                            "description": "How long to show this scene before reverting. 4000-8000 is typical."
                        }
                    }
                }
            }
        }
    ])
}

/// Execute a tool call. Returns a short status string that goes into the
/// follow-up "tool" message so the model knows whether it worked.
pub fn execute(app: &AppHandle, name: &str, arguments_json: &str) -> String {
    match name {
        "set_scene" => execute_set_scene(app, arguments_json),
        _ => format!("error: unknown tool {name}"),
    }
}

fn execute_set_scene(app: &AppHandle, args_json: &str) -> String {
    let args: serde_json::Value = match serde_json::from_str(args_json) {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!(?err, args_json, "set_scene args parse failed");
            return format!("error: invalid arguments: {err}");
        }
    };

    let duration_ms = args
        .get("duration_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(6000)
        .clamp(1500, 30000) as u32;

    // Strip transport-only field out of the scene payload.
    let mut scene = args.clone();
    if let Some(obj) = scene.as_object_mut() {
        obj.remove("duration_ms");
    }

    let payload = serde_json::json!({
        "scene": scene,
        "duration_ms": duration_ms,
    });

    match app.emit("lcd-scene", &payload) {
        Ok(_) => {
            tracing::info!(duration_ms, scene = %scene, "set_scene from LLM");
            format!("ok, scene shown for {duration_ms}ms")
        }
        Err(err) => {
            tracing::warn!(?err, "lcd-scene emit failed");
            format!("error: {err}")
        }
    }
}
