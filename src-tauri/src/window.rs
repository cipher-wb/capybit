//! Window setup + click-through cursor poller + move-event persistence.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, LogicalPosition, Manager, PhysicalPosition, WindowEvent};

use crate::state::{self, PersistedState, Position};

/// Half-side of the LCD device's solid area, in physical pixels. The
/// renderer (placeholder_capybara.js) now draws only an LCD panel + thin
/// border occupying 168×168 px centered in the window; outside that the
/// window is truly transparent and should pass clicks through to apps
/// behind. See docs/ADR/0010-lcd-scene-system.md.
///
/// Naming kept as `PET_HITBOX_RADIUS_PX` for diff churn; semantically it's
/// now half-side of a square, not a radius.
const PET_HITBOX_RADIUS_PX: f64 = 84.0;

/// Poll interval for the click-through cursor watcher. 33ms ≈ 30Hz, plenty
/// responsive for "mouse approaching the pet" without burning CPU.
const POLL_INTERVAL: Duration = Duration::from_millis(33);

pub fn apply_initial_state(app: &AppHandle, st: &PersistedState) -> tauri::Result<()> {
    let win = app
        .get_webview_window("main")
        .expect("main window should exist");
    win.set_position(LogicalPosition::new(
        st.position.x as f64,
        st.position.y as f64,
    ))?;
    if st.hidden {
        win.hide()?;
    }
    // Start fully click-through; the poller will turn it off when the cursor
    // approaches the pet.
    win.set_ignore_cursor_events(true)?;
    Ok(())
}

/// Spawn a background task that toggles `set_ignore_cursor_events` based on
/// the global cursor distance to the window center. Necessary because
/// transparent click-through windows do not receive mouse events at the OS
/// level — we cannot rely on JS `mousemove` to detect when the cursor enters
/// the pet area.
pub fn spawn_clickthrough_poller(app: AppHandle) {
    let currently_ignoring = Arc::new(AtomicBool::new(true));
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(POLL_INTERVAL).await;

            let Some(win) = app.get_webview_window("main") else {
                continue;
            };

            // Cursor position is global, in physical pixels.
            let cursor: PhysicalPosition<f64> = match app.cursor_position() {
                Ok(p) => p,
                Err(_) => continue,
            };

            let outer = match win.outer_position() {
                Ok(p) => p,
                Err(_) => continue,
            };
            let size = match win.outer_size() {
                Ok(s) => s,
                Err(_) => continue,
            };

            let cx = outer.x as f64 + size.width as f64 / 2.0;
            let cy = outer.y as f64 + size.height as f64 / 2.0;
            let dx = (cursor.x - cx).abs();
            let dy = (cursor.y - cy).abs();
            // Square hit area matching the LCD device border.
            let inside = dx < PET_HITBOX_RADIUS_PX && dy < PET_HITBOX_RADIUS_PX;

            let want_ignore = !inside;
            let now_ignoring = currently_ignoring.load(Ordering::Relaxed);
            if want_ignore != now_ignoring {
                if let Err(err) = win.set_ignore_cursor_events(want_ignore) {
                    tracing::warn!(?err, "set_ignore_cursor_events failed");
                    continue;
                }
                currently_ignoring.store(want_ignore, Ordering::Relaxed);
            }
        }
    });
}

/// Persist window position on every move. Uses Tauri's window event stream;
/// since move events fire many times during a drag, we throttle via a simple
/// debounce timer.
pub fn watch_window_moves(app: AppHandle) {
    let win = app
        .get_webview_window("main")
        .expect("main window should exist");
    let handle = app.clone();

    // Debounce: collect rapid moves, write 250ms after the last one.
    let pending: Arc<tokio::sync::Mutex<Option<Position>>> =
        Arc::new(tokio::sync::Mutex::new(None));
    let writer_pending = pending.clone();
    let writer_handle = handle.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(250)).await;
            let mut guard = writer_pending.lock().await;
            if let Some(pos) = guard.take() {
                let state = writer_handle.state::<state::AppState>();
                let mut s = state.0.lock().unwrap();
                s.position = pos;
                let snapshot = s.clone();
                drop(s);
                state::save(&writer_handle, &snapshot);
            }
        }
    });

    win.on_window_event(move |event| {
        if let WindowEvent::Moved(pos) = event {
            let pos = Position { x: pos.x, y: pos.y };
            let pending = pending.clone();
            tauri::async_runtime::spawn(async move {
                *pending.lock().await = Some(pos);
            });
        }
    });
}
