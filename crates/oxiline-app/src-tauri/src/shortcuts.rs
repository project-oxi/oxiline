//! Global shortcut registration → HUD (`04-architecture.md` §4.4).

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent};

use crate::{hud, state::AppState};

/// Read the configured hotkey from settings and register it.
pub fn register_default(app: &AppHandle) {
    let state = app.state::<AppState>();
    let hotkey =
        oxiline_core::settings::get_string(&state.conn(), "global_hotkey", "CmdOrCtrl+Shift+O");
    register_str(app, &hotkey);
}

/// Register a hotkey string, unregistering any previously registered one first.
pub fn register_str(app: &AppHandle, hotkey: &str) {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();

    let Ok(sc) = hotkey.parse::<Shortcut>() else {
        eprintln!("oxiline: invalid hotkey '{hotkey}'");
        return;
    };
    let app_handle = app.clone();
    let handler = move |_app: &AppHandle, _shortcut: &Shortcut, _event: ShortcutEvent| {
        hud::show(&app_handle);
    };
    if let Err(e) = gs.on_shortcut(sc, handler) {
        eprintln!("oxiline: failed to register hotkey '{hotkey}': {e}");
    }
}
