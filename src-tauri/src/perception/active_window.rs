//! Active window read via active-win-pos-rs (cross-platform).
//! macOS first call needs Accessibility permission — see CLAUDE.md §10.

/// Returns (app_name, window_title). None on error / no foreground.
pub fn current() -> Option<(Option<String>, Option<String>)> {
    match active_win_pos_rs::get_active_window() {
        Ok(win) => Some((Some(win.app_name), Some(win.title))),
        Err(()) => {
            tracing::trace!("active window query failed");
            None
        }
    }
}
