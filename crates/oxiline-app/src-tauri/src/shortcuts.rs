//! Global shortcut registration (`04-architecture.md` §4.4).
//!
//! Two shortcuts: the HUD hotkey (`global_hotkey`, default ⌘⇧O) reveals the
//! floating "now" panel; the quick-record hotkey (`quick_record_hotkey`,
//! default ⌘⇧R) toggles recording without focusing the window — the frontend
//! decides start-vs-stop from the live record state.

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent};

use crate::{hud, state::AppState};

/// Read the configured hotkeys from settings and register both.
pub fn register_default(app: &AppHandle) {
    let state = app.state::<AppState>();
    let hud_hk =
        oxiline_core::settings::get_string(&state.conn(), "global_hotkey", "CmdOrCtrl+Shift+O");
    let qr_hk = oxiline_core::settings::get_string(
        &state.conn(),
        "quick_record_hotkey",
        "CmdOrCtrl+Shift+R",
    );
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    register_hud(app, &hud_hk);
    register_quick_record(app, &qr_hk);
}

fn register_hud(app: &AppHandle, hotkey: &str) {
    let Ok(sc) = hotkey.parse::<Shortcut>() else {
        eprintln!("oxiline: invalid hud hotkey '{hotkey}'");
        return;
    };
    let app_handle = app.clone();
    let handler = move |_app: &AppHandle, _shortcut: &Shortcut, _event: ShortcutEvent| {
        hud::show(&app_handle);
    };
    if let Err(e) = app.global_shortcut().on_shortcut(sc, handler) {
        eprintln!("oxiline: failed to register hud hotkey '{hotkey}': {e}");
    }
}

fn register_quick_record(app: &AppHandle, hotkey: &str) {
    let Ok(sc) = hotkey.parse::<Shortcut>() else {
        eprintln!("oxiline: invalid quick-record hotkey '{hotkey}'");
        return;
    };
    let app_handle = app.clone();
    let handler = move |_app: &AppHandle, _shortcut: &Shortcut, _event: ShortcutEvent| {
        let _ = app_handle.emit("oxiline://quick-record", ());
    };
    if let Err(e) = app.global_shortcut().on_shortcut(sc, handler) {
        eprintln!("oxiline: failed to register quick-record hotkey '{hotkey}': {e}");
    }
}
