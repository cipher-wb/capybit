//! Capybit — M0 scaffold.
//!
//! Owns: window setup, click-through poller, tray, position persistence.
//! Layers below (perception/memory/llm/scheduler) come online in later milestones.

mod birth;
mod commands;
mod config;
mod llm;
mod memory;
mod perception;
mod profile;
mod scheduler;
mod state;
mod tray;
mod window;

use tauri::Manager;
use tracing_subscriber::EnvFilter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,capybit=debug")),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let handle = app.handle().clone();

            // Load persisted state and apply to the window.
            let loaded = state::load(&handle);
            window::apply_initial_state(&handle, &loaded)?;
            let initial_inner = loaded.inner.clone().unwrap_or_default();
            app.manage(state::AppState::new(loaded));
            app.manage(scheduler::tick::InnerStateStore::new(initial_inner));
            app.manage(llm::monologue::MonologueCache::default());

            // First-launch gate: if the profile has no birth record yet, hide
            // the pet and show the birth window centered for the ritual.
            // See docs/ADR/0003-m5-birth-ritual.md.
            if birth::is_uninitiated(&profile::load(&handle)) {
                tracing::info!("first launch detected — running birth ritual");
                if let Some(main) = app.get_webview_window("main") {
                    let _ = main.hide();
                }
                if let Some(birth_win) = app.get_webview_window("birth") {
                    let _ = birth_win.show();
                    let _ = birth_win.set_focus();
                }
            }

            // Open conversations DB and hand to Tauri state.
            let db = std::sync::Arc::new(memory::Db::open(&handle).expect("open db"));
            app.manage(db.clone());

            // Catch-up: summarize any past days that have messages but no
            // summary yet. See docs/ADR/0004-m3-memory.md for cadence design.
            spawn_summary_catchup(handle.clone());
            spawn_daily_summary_ticker(handle.clone());

            // Vector index backfill — embed any diaries that aren't in
            // vec_memories yet. See docs/ADR/0005-m3-vector-recall.md.
            spawn_vector_backfill(handle.clone());

            // Tick scheduler: 60s state machine driving energy/mood/curiosity/
            // urge + sprite action selection. See docs/ADR/0006-m4-perception.md.
            scheduler::tick::spawn(handle.clone());

            // Global hotkey: Ctrl+Shift+\ opens the bubble (PRD §3.2.1).
            if let Err(err) = register_global_hotkey(&handle) {
                tracing::warn!(
                    ?err,
                    "global hotkey registration failed; chat hotkey disabled"
                );
            }

            // System tray.
            tray::install(&handle)?;

            // Click-through poller: toggles ignore_cursor_events based on cursor proximity
            // to the pet center. See docs/ADR/0001-m0-scaffold.md for rationale.
            window::spawn_clickthrough_poller(handle.clone());

            // Persist window position whenever it moves (debounced inside).
            window::watch_window_moves(handle.clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::save_state,
            commands::set_pet_visible,
            commands::quit_app,
            commands::open_bubble,
            commands::close_bubble,
            commands::send_message,
            commands::is_first_launch,
            commands::pick_birth_folder,
            commands::scan_for_excerpt,
            commands::confirm_birth,
            commands::run_pending_summaries,
            commands::force_summary_for_date,
            commands::backfill_vector_index,
            commands::get_perception,
            commands::get_inner_state,
            commands::get_inner_monologue,
            commands::dismiss_proactive,
            commands::set_scene,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Capybit");
}

/// On startup, summarize any past days that have messages but no summary row.
/// Capped at 7 days back to avoid burning api budget on first-ever launch
/// after long absence.
fn spawn_summary_catchup(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        // Brief delay so the rest of setup (DB, network) is settled.
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        let db = app.state::<std::sync::Arc<memory::Db>>().inner().clone();
        let today = today_local();
        let pending = match memory::unsummarized_dates(&db, &today) {
            Ok(v) => v,
            Err(err) => {
                tracing::error!(?err, "catchup: query failed");
                return;
            }
        };
        let take = pending.len().min(7);
        if take == 0 {
            tracing::debug!("catchup: nothing to summarize");
            return;
        }
        tracing::info!(count = take, "catchup: running daily summaries");
        for date in pending.into_iter().take(take) {
            if let Err(err) =
                llm::summarizer::run_for_date(app.clone(), db.clone(), date.clone()).await
            {
                tracing::warn!(?err, %date, "catchup summary failed");
            }
        }
    });
}

/// Wake at 23:59 local each day; summarize today (which will then be
/// "yesterday" from the catchup query's POV on next start, but doing it now
/// captures the day before midnight wraps).
fn spawn_daily_summary_ticker(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            let secs_until_2359 = seconds_until_local(23, 59);
            tokio::time::sleep(std::time::Duration::from_secs(secs_until_2359)).await;
            let db = app.state::<std::sync::Arc<memory::Db>>().inner().clone();
            let date = today_local();
            tracing::info!(%date, "23:59 tick — summarizing today");
            if let Err(err) =
                llm::summarizer::run_for_date(app.clone(), db.clone(), date.clone()).await
            {
                tracing::warn!(?err, %date, "23:59 summary failed");
            }
            // Sleep past midnight so the next iteration computes a fresh wait.
            tokio::time::sleep(std::time::Duration::from_secs(120)).await;
        }
    });
}

fn register_global_hotkey(app: &tauri::AppHandle) -> anyhow::Result<()> {
    use tauri_plugin_global_shortcut::{
        Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
    };

    let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Backslash);
    let app_for_handler = app.clone();
    app.global_shortcut()
        .on_shortcut(shortcut, move |_app, _sc, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            let app = app_for_handler.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(err) = commands::open_bubble(app) {
                    tracing::warn!(?err, "hotkey open_bubble failed");
                }
            });
        })
        .map_err(|e| anyhow::anyhow!("global shortcut: {e}"))?;
    Ok(())
}

fn spawn_vector_backfill(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        // Wait until after the summary catchup has had a chance to write new
        // diaries — otherwise we'd just re-index them immediately.
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        let db = app.state::<std::sync::Arc<memory::Db>>().inner().clone();
        let cfg = config::load(&app);
        if !cfg.is_usable() {
            tracing::debug!("vector backfill: skipping (no api_key)");
            return;
        }
        let pending = match memory::unindexed_diaries(&db) {
            Ok(v) => v,
            Err(err) => {
                tracing::error!(?err, "vector backfill: query failed");
                return;
            }
        };
        if pending.is_empty() {
            tracing::debug!("vector backfill: nothing to index");
            return;
        }
        tracing::info!(count = pending.len(), "vector backfill: starting");
        let mut indexed = 0usize;
        for (date, diary) in pending {
            match llm::embedder::embed(&cfg, &diary).await {
                Ok(emb) => {
                    if let Err(err) = memory::upsert_memory(&db, "daily_diary", &date, &diary, &emb)
                    {
                        tracing::warn!(?err, %date, "backfill insert failed");
                    } else {
                        indexed += 1;
                    }
                }
                Err(err) => tracing::warn!(?err, %date, "backfill embed failed"),
            }
        }
        tracing::info!(indexed, "vector backfill: done");
    });
}

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

fn seconds_until_local(target_hour: u8, target_minute: u8) -> u64 {
    let offset = time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC);
    let now = time::OffsetDateTime::now_utc().to_offset(offset);
    let target_today = now
        .replace_hour(target_hour)
        .and_then(|t| t.replace_minute(target_minute))
        .and_then(|t| t.replace_second(0))
        .unwrap_or(now);
    let target = if target_today <= now {
        target_today + time::Duration::days(1)
    } else {
        target_today
    };
    let delta = target - now;
    delta.whole_seconds().max(0) as u64
}
