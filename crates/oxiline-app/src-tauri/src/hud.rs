//! Floating HUD logic (`04-architecture.md` §4.4, `07-ui-screens-and-flows.md`
//! §7.6).
//!
//! MVP uses a standard transparent always-on-top overlay window rather than a
//! non-activating NSPanel: it builds reliably and still shows/auto-dismisses on
//! the global shortcut. Upgrading to `tauri-nspanel` for true focus
//! non-activation is a documented follow-up.

use std::sync::LazyLock;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewWindow};

use crate::state::AppState;

static LATEST_SHOW: LazyLock<Mutex<Option<Instant>>> = LazyLock::new(|| Mutex::new(None));

/// Show the HUD: push fresh "now" data, center it near the top of the active
/// screen, reveal it, then auto-hide after `hud_duration_ms`.
pub fn show(app: &AppHandle) {
    let Some(hud) = app.get_webview_window("hud") else {
        return;
    };

    // Fresh context for the HUD body (same calc as `oxiline now`).
    let state = app.state::<AppState>();
    if let Ok(ctx) = oxiline_core::timeline::get_now_context(
        &state.conn(),
        oxiline_core::util::now_minute_local(),
    ) {
        let _ = hud.emit("oxiline://now", &ctx);
    }

    position_top_center(&hud);
    let _ = hud.show();

    let duration =
        oxiline_core::settings::get_i64(&state.conn(), "hud_duration_ms", 2000).max(500) as u64;

    let now = Instant::now();
    *LATEST_SHOW.lock() = Some(now);

    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(duration));
        // Only hide if no newer show superseded this one (reset on re-trigger).
        let still_latest = matches!(*LATEST_SHOW.lock(), Some(t) if t == now);
        if still_latest {
            if let Some(hud) = app.get_webview_window("hud") {
                let _ = hud.hide();
            }
            *LATEST_SHOW.lock() = None;
        }
    });
}

/// Position the HUD at the horizontal center, near the top, of the screen the
/// HUD window is on.
fn position_top_center(hud: &WebviewWindow) {
    let Ok(Some(monitor)) = hud.current_monitor() else {
        return;
    };
    let size = monitor.size();
    let scale = monitor.scale_factor();
    let win_w = 360.0_f64;
    let win_h = 150.0_f64;
    let pos = monitor.position();
    let x = pos.x as f64 + (size.width as f64 / scale - win_w) / 2.0;
    let y = pos.y as f64 + 48.0;
    let _ = hud.set_position(PhysicalPosition::new(x, y));
    let _ = hud.set_size(LogicalSize::new(win_w, win_h));
}

/// macOS: convert the `hud` window to a non-activating NSPanel so it
/// never steals focus from the foreground app (Phase 2 spec §1).
/// Non-macOS: no-op (the existing transparent overlay already works).
pub fn init_panel(app: &AppHandle) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    {
        use tauri::Manager;
        use tauri_nspanel::{CollectionBehavior, PanelLevel, StyleMask, WebviewWindowExt};

        let Some(win) = app.get_webview_window("hud") else {
            return Ok(());
        };
        let panel = win.to_panel::<HudPanel>()?;

        // HUD window style: borderless + non-activating so the panel
        // never takes focus from the foreground app.
        panel.set_style_mask(StyleMask::empty().borderless().nonactivating_panel().into());
        // Floating level — above normal windows, below modal.
        panel.set_level(PanelLevel::Floating.value());
        // Don't hide when the app deactivates.
        panel.set_hides_on_deactivate(false);
        // Show on every Space + above fullscreen apps.
        panel.set_collection_behavior(
            CollectionBehavior::new()
                .can_join_all_spaces()
                .full_screen_auxiliary()
                .value(),
        );
        // Start hidden.
        panel.hide();
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(())
    }
}

// Custom NSPanel subclass for the HUD.
// Config: never becomes key (no focus steal), floats above other windows,
// stays visible when the app deactivates.
#[cfg(target_os = "macos")]
tauri_nspanel::tauri_panel! {
    panel!(HudPanel {
        config: {
            can_become_key_window: false,
            is_floating_panel: true,
            hides_on_deactivate: false
        }
    })
}
