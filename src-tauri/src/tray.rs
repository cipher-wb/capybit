//! System tray with: 显示/隐身 toggle + 退出.

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};

use crate::state::{self, AppState};

pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(app, "toggle", "暂时隐身 / 显示", true, None::<&str>)?;
    let devtools = MenuItem::with_id(
        app,
        "devtools",
        "打开调试控制台 (DevTools)",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "退出 Capybit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle, &devtools, &quit])?;

    TrayIconBuilder::with_id("capybit-tray")
        .tooltip("Capybit")
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "toggle" => {
                let Some(win) = app.get_webview_window("main") else {
                    return;
                };
                let visible = win.is_visible().unwrap_or(true);
                let _ = if visible { win.hide() } else { win.show() };
                let state = app.state::<AppState>();
                let snapshot = {
                    let mut s = state.0.lock().unwrap();
                    s.hidden = visible; // we just toggled, so new hidden = old visible
                    s.clone()
                };
                state::save(app, &snapshot);
            }
            "devtools" => {
                // DevTools is only compiled in debug builds (Tauri default).
                #[cfg(debug_assertions)]
                if let Some(win) = app.get_webview_window("main") {
                    win.open_devtools();
                }
                #[cfg(not(debug_assertions))]
                tracing::warn!("devtools not available in release build");
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
