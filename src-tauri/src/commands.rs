//! Tauri commands exposed to the WebView.

use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::birth::{self, ExcerptResult};
use crate::config;
use crate::llm::{self, embedder, monologue::MonologueCache, openrouter, summarizer};
use crate::memory::{self, Db};
use crate::perception::{self, PerceptionSnapshot};
use crate::profile;
use crate::scheduler::{state_machine, tick::InnerStateStore, InnerState};
use crate::state::{self, AppState, PersistedState};

#[tauri::command]
pub fn get_state(state: State<'_, AppState>) -> PersistedState {
    state.0.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_state(app: AppHandle, new_state: PersistedState, state: State<'_, AppState>) {
    {
        let mut s = state.0.lock().unwrap();
        *s = new_state.clone();
    }
    state::save(&app, &new_state);
}

#[tauri::command]
pub fn set_pet_visible(
    app: AppHandle,
    visible: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let win = app
        .get_webview_window("main")
        .ok_or_else(|| "main window missing".to_string())?;
    if visible {
        win.show().map_err(|e| e.to_string())?;
    } else {
        win.hide().map_err(|e| e.to_string())?;
    }
    let snapshot = {
        let mut s = state.0.lock().unwrap();
        s.hidden = !visible;
        s.clone()
    };
    state::save(&app, &snapshot);
    Ok(())
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

/// Open the bubble window next to the pet, focus it.
#[tauri::command]
pub fn open_bubble(app: AppHandle) -> Result<(), String> {
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "main window missing".to_string())?;
    let bubble = app
        .get_webview_window("bubble")
        .ok_or_else(|| "bubble window missing".to_string())?;

    // Anchor bubble to the upper-left of the pet window.
    if let (Ok(pos), Ok(size)) = (main.outer_position(), bubble.outer_size()) {
        let target = tauri::PhysicalPosition::new(
            (pos.x - size.width as i32 - 8).max(0),
            (pos.y - 40).max(0),
        );
        let _ = bubble.set_position(target);
    }
    bubble.show().map_err(|e| e.to_string())?;
    bubble.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn close_bubble(app: AppHandle) -> Result<(), String> {
    if let Some(bubble) = app.get_webview_window("bubble") {
        bubble.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

// -- Birth ritual ----------------------------------------------------------

#[tauri::command]
pub fn is_first_launch(app: AppHandle) -> bool {
    birth::is_uninitiated(&profile::load(&app))
}

/// Open the native folder picker. Returns the selected path or None if the
/// user cancelled.
#[tauri::command]
pub async fn pick_birth_folder(app: AppHandle) -> Option<String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("选一个写字的地方…（Documents / Obsidian Vault / …）")
        .pick_folder(move |path| {
            let _ = tx.send(path.map(|p| p.to_string()));
        });
    rx.await.ok().flatten()
}

#[tauri::command]
pub fn scan_for_excerpt(folder: String) -> Result<ExcerptResult, String> {
    birth::scan(&PathBuf::from(folder)).map_err(|e| e.to_string())
}

/// Persist the birth record, then close the birth window and show the pet.
#[tauri::command]
pub fn confirm_birth(
    app: AppHandle,
    capybara_name: String,
    source_file: String,
    source_excerpt: String,
    user_name_for_capybit: Option<String>,
) -> Result<(), String> {
    let mut prof = profile::load(&app);
    let birth = birth::build_birth_record(capybara_name.clone(), source_file, source_excerpt);
    prof.name = capybara_name;
    prof.birth = Some(birth);
    if let Some(uname) = user_name_for_capybit {
        let uname = uname.trim();
        if !uname.is_empty() {
            prof.user_name_for_capybit = Some(uname.to_string());
        }
    }
    profile::save(&app, &prof).map_err(|e| e.to_string())?;

    if let Some(birth_win) = app.get_webview_window("birth") {
        let _ = birth_win.hide();
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
    }
    Ok(())
}

/// Send a user message to the LLM. Replies stream back via Tauri events
/// (see openrouter.rs for event names and payloads).
///
/// `request_id` lets the frontend match chunks to the bubble that opened.
///
/// Side effects (M3): both turns persisted to conversations.sqlite. Recent
/// 5 turns of history + recent daily diaries injected into the prompt.
#[tauri::command]
pub fn send_message(app: AppHandle, request_id: String, message: String) {
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        // Apply state machine effect (mood +5, urge reset, etc).
        {
            let store = app_clone.state::<InnerStateStore>();
            let mut s = store.state.lock().unwrap();
            state_machine::on_user_chat(&mut s);
        }

        // Persist user turn first so we don't lose it if the LLM call fails.
        let db = app_clone.state::<Arc<Db>>();
        if let Err(err) = memory::write_message(&db, "user", &message, None) {
            tracing::error!(?err, "failed to write user message");
        }

        let cfg = config::load(&app_clone);
        let profile = profile::load(&app_clone);

        let history = memory::recent_history(&db, 10)
            .unwrap_or_default()
            .into_iter()
            // Drop the user turn we just wrote — it'll be added back as the
            // current "user" message by build_chat_messages.
            .filter(|m| !(m.role == "user" && m.content == message))
            .map(|m| llm::ChatMessage {
                role: if m.role == "capybit" {
                    "assistant".into()
                } else {
                    m.role
                },
                content: m.content,
            })
            .collect();

        let recent_summaries_block = render_recent_summaries(&db);

        // Semantic recall: only embed + query when the user message contains
        // a 召唤词 ("还记得", "上次", etc). Saves an embedding call per turn.
        let recalled_block = if has_recall_trigger(&message) {
            recall_for_query(&app_clone, &db, &cfg, &message).await
        } else {
            "（这次对话没有触发追溯。）".into()
        };

        let messages = llm::prompts::build_chat_messages(
            &profile,
            history,
            &recent_summaries_block,
            &recalled_block,
            &message,
        );

        // Subscribe to llm-done for THIS request to persist the assistant turn.
        // We use a one-shot channel: openrouter emits events to the frontend;
        // for persistence we tap the same full_text via a parallel listener.
        spawn_persist_listener(app_clone.clone(), request_id.clone());

        openrouter::stream_chat(app_clone, request_id, cfg, messages).await;
    });
}

/// 召唤词列表 (PRD §3.4 Layer 3). Cheap substring scan.
fn has_recall_trigger(msg: &str) -> bool {
    const TRIGGERS: &[&str] = &[
        "还记得",
        "记得吗",
        "记不记得",
        "上次",
        "之前",
        "前几天",
        "上周",
        "你说过",
        "我跟你说过",
        "我说过",
        "你提过",
        "之前那次",
        "那时候",
    ];
    TRIGGERS.iter().any(|t| msg.contains(t))
}

async fn recall_for_query(app: &AppHandle, db: &Db, cfg: &config::Config, query: &str) -> String {
    if !cfg.is_usable() {
        return "（暂无足够上下文。）".into();
    }
    let emb = match embedder::embed(cfg, query).await {
        Ok(e) => e,
        Err(err) => {
            tracing::warn!(?err, "recall embed failed");
            return "（追溯失败，没找到。）".into();
        }
    };
    let hits = match memory::query_similar(db, &emb, 3) {
        Ok(h) => h,
        Err(err) => {
            tracing::warn!(?err, "vector search failed");
            return "（追溯失败，没找到。）".into();
        }
    };
    let _ = app; // reserved for future telemetry events
    if hits.is_empty() {
        return "（没有找到相关的更早记忆。）".into();
    }
    hits.iter()
        .map(|m| {
            format!(
                "- [{}] {} (相似度 {:.2})",
                m.ref_key,
                m.text,
                1.0 - m.distance
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_recent_summaries(db: &Db) -> String {
    match memory::summaries::recent_diaries(db, 5) {
        Ok(rows) if !rows.is_empty() => rows
            .into_iter()
            .map(|(d, diary)| format!("[{d}] {diary}"))
            .collect::<Vec<_>>()
            .join("\n\n"),
        _ => "（暂无每日总结。）".into(),
    }
}

/// Listen for `llm-done` matching `request_id`, then write the assistant
/// turn to conversations.sqlite. Self-unsubscribes after firing.
fn spawn_persist_listener(app: AppHandle, request_id: String) {
    use tauri::Listener;
    let app_for_handler = app.clone();
    let id_for_handler = request_id.clone();
    // listen_any so we don't depend on which window emits.
    let handler_id = app.listen("llm-done", move |event| {
        let Ok(payload): Result<serde_json::Value, _> = serde_json::from_str(event.payload())
        else {
            return;
        };
        if payload.get("request_id").and_then(|v| v.as_str()) != Some(id_for_handler.as_str()) {
            return;
        }
        let text = payload
            .get("full_text")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if text.is_empty() {
            return;
        }
        let db = app_for_handler.state::<Arc<Db>>();
        if let Err(err) = memory::write_message(&db, "capybit", text, None) {
            tracing::error!(?err, "failed to persist assistant turn");
        }
    });
    // Unsubscribe after a short delay — the event will have fired by then.
    // 30s is way longer than any reasonable response, and stale listeners
    // would just no-op the request_id filter above.
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        app.unlisten(handler_id);
    });
}

// -- Daily summary ---------------------------------------------------------

/// Local-date today, "YYYY-MM-DD" in local timezone.
fn today_local() -> String {
    let offset = time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC);
    let now = time::OffsetDateTime::now_utc().to_offset(offset);
    format!(
        "{:04}-{:02}-{:02}",
        now.year(),
        now.month() as u8,
        now.day()
    )
}

/// Manually trigger summarization of all unsummarized past days (excluding
/// today). Useful for testing without waiting for 23:59.
#[tauri::command]
pub async fn run_pending_summaries(app: AppHandle) -> Result<usize, String> {
    let db = app.state::<Arc<Db>>().inner().clone();
    let today = today_local();
    let pending = memory::unsummarized_dates(&db, &today).map_err(|e| e.to_string())?;
    let count = pending.len();
    for date in pending {
        summarizer::run_for_date(app.clone(), db.clone(), date).await?;
    }
    Ok(count)
}

/// Force-summarize a specific date (mainly for dev/testing). Date format:
/// "YYYY-MM-DD" local.
#[tauri::command]
pub async fn force_summary_for_date(app: AppHandle, date: String) -> Result<(), String> {
    let db = app.state::<Arc<Db>>().inner().clone();
    summarizer::run_for_date(app, db, date).await
}

/// Push an arbitrary scene to the LCD. Front-end exposes
/// `window.__capybit_pushScene(scene, duration_ms)` and listens for the
/// `lcd-scene` event we emit here. Scene schema is documented in
/// `src/renderer/placeholder_capybara.js`.
#[tauri::command]
pub fn set_scene(
    app: AppHandle,
    scene: serde_json::Value,
    duration_ms: Option<u32>,
) -> Result<(), String> {
    use tauri::Emitter;
    let payload = serde_json::json!({
        "scene": scene,
        "duration_ms": duration_ms.unwrap_or(5000),
    });
    app.emit("lcd-scene", payload).map_err(|e| e.to_string())
}

/// User pressed "等会儿" on a proactive bubble. Drops urge below threshold
/// so the same condition doesn't immediately re-fire, but keeps it warmer
/// than a full reset (the capybara still has things to say later).
#[tauri::command]
pub fn dismiss_proactive(app: AppHandle) {
    use time::{format_description::well_known::Rfc3339, OffsetDateTime};
    let store = app.state::<InnerStateStore>();
    let mut s = store.state.lock().unwrap();
    s.urge = 30.0;
    s.last_proactive_at = OffsetDateTime::now_utc().format(&Rfc3339).ok();
}

// -- Perception / Inner state ---------------------------------------------

#[tauri::command]
pub fn get_perception() -> PerceptionSnapshot {
    perception::capture()
}

#[tauri::command]
pub fn get_inner_state(app: AppHandle) -> InnerState {
    app.state::<InnerStateStore>().state.lock().unwrap().clone()
}

/// Inner monologue for hover tooltip. Cached for 5min per action; only hits
/// the LLM when the action has changed or the cache is stale.
#[tauri::command]
pub async fn get_inner_monologue(app: AppHandle) -> Result<String, String> {
    let cfg = config::load(&app);
    let prof = profile::load(&app);
    let (inner, active_app) = {
        let store = app.state::<InnerStateStore>();
        let s = store.state.lock().unwrap().clone();
        let last = store.last_perception.lock().unwrap();
        let app_name = last.as_ref().and_then(|p| p.active_app.clone());
        (s, app_name)
    };
    let cache = app.state::<MonologueCache>();
    llm::monologue::get_or_fetch(&cfg, &cache, &inner, active_app.as_deref(), &prof.name).await
}

/// Index any daily summaries that don't yet have a vector embedding. Runs on
/// startup; can be re-triggered manually for testing. Returns the number
/// of new embeddings written.
#[tauri::command]
pub async fn backfill_vector_index(app: AppHandle) -> Result<usize, String> {
    let db = app.state::<Arc<Db>>().inner().clone();
    let cfg = config::load(&app);
    if !cfg.is_usable() {
        return Err("no api_key for embedding backfill".into());
    }
    let pending = memory::unindexed_diaries(&db).map_err(|e| e.to_string())?;
    let total = pending.len();
    let mut indexed = 0usize;
    for (date, diary) in pending {
        match embedder::embed(&cfg, &diary).await {
            Ok(emb) => {
                if let Err(err) = memory::upsert_memory(&db, "daily_diary", &date, &diary, &emb) {
                    tracing::warn!(?err, %date, "backfill insert failed");
                } else {
                    indexed += 1;
                }
            }
            Err(err) => {
                tracing::warn!(?err, %date, "backfill embed failed");
                // Don't bail on the whole loop — next startup will retry the rest.
            }
        }
    }
    tracing::info!(indexed, total, "vector index backfill done");
    Ok(indexed)
}
