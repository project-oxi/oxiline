//! Menu-bar tray icon + menu (`07-ui-screens-and-flows.md` §7.7, §4.3).

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, MenuEvent};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

use crate::{hud, state::AppState};

const EVENT_OPEN_PREFERENCES: &str = "oxiline://open-preferences";
const EVENT_OPEN_QUICK_ADD: &str = "oxiline://open-quick-add";

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let sep0 = PredefinedMenuItem::separator(app)?;
    let now_label = now_summary(app);
    let info = MenuItem::with_id(app, "info", &now_label, false, None::<&str>)?;
    let hud_item = MenuItem::with_id(app, "hud", "지금 보기 (HUD)", true, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "OxiLine 열기", true, None::<&str>)?;
    let quick = MenuItem::with_id(app, "quick", "빠른 추가…", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let prefs = MenuItem::with_id(app, "prefs", "환경설정…", true, None::<&str>)?;
    let autostart =
        CheckMenuItem::with_id(app, "autostart", "로그인 시 자동 실행", true, autostart_enabled(app), None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "OxiLine 종료", true, None::<&str>)?;

    Menu::with_items(
        app,
        &[
            &info, &sep0, &hud_item, &open, &quick, &sep1, &prefs, &autostart, &sep2, &quit,
        ],
    )
}

/// Build the tray icon and its menu.
pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app)?;
    let icon = load_tray_icon();
    tauri::tray::TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .tooltip("OxiLine")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(on_menu_event)
        .build(app)?;
    Ok(())
}

/// Refresh the dynamic menu (now-label + autostart) by rebuilding it.
pub fn refresh(app: &AppHandle) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        if let Ok(menu) = build_menu(app) {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        "open" => show_main(app),
        "quick" => {
            show_main(app);
            let _ = app.emit(EVENT_OPEN_QUICK_ADD, ());
        }
        "prefs" => {
            show_main(app);
            let _ = app.emit(EVENT_OPEN_PREFERENCES, ());
        }
        "hud" => hud::show(app),
        "autostart" => toggle_autostart(app),
        "quit" => app.exit(0),
        _ => {}
    }
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn toggle_autostart(app: &AppHandle) {
    let mgr = app.autolaunch();
    let enabled = mgr.is_enabled().unwrap_or(false);
    let _ = if enabled { mgr.disable() } else { mgr.enable() };
    let state = app.state::<AppState>();
    let _ = oxiline_core::settings::set(
        &state.conn(),
        "launch_at_login",
        &serde_json::Value::Bool(!enabled),
    );
    // reflect the change immediately
    refresh(app);
}

fn autostart_enabled(app: &AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

fn now_summary(app: &AppHandle) -> String {
    let state = app.state::<AppState>();
    match oxiline_core::timeline::get_now_context(
        &state.conn(),
        oxiline_core::util::now_minute_local(),
    ) {
        Ok(ctx) => {
            if let Some(c) = &ctx.current {
                let rem = c
                    .remaining_minute
                    .map(|r| format!(" · {r}분 남음"))
                    .unwrap_or_default();
                format!("지금: {}{}", c.title, rem)
            } else if let Some(n) = &ctx.next {
                let in_min = n
                    .starts_in_minute
                    .map(|s| format!(" · {}분 후", s))
                    .unwrap_or_default();
                format!("다음: {}{}", n.title, in_min)
            } else {
                "오늘 예정된 일이 모두 끝났어요".into()
            }
        }
        Err(_) => "OxiLine".into(),
    }
}

/// A small verdigris rounded-square tray icon rendered procedurally (no asset).
fn load_tray_icon() -> tauri::image::Image<'static> {
    let (w, h) = (22u32, 22u32);
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    let (r, g, b, a) = (0x2b_u8, 0xb3_u8, 0xa0_u8, 0xff_u8);
    for y in 0..h {
        for x in 0..w {
            let corner = (x < 4 && y < 4 && (x + y) < 5)
                || (x >= w - 4 && y < 4 && ((w - 1 - x) + y) < 5)
                || (x < 4 && y >= h - 4 && (x + (h - 1 - y)) < 5)
                || (x >= w - 4 && y >= h - 4 && ((w - 1 - x) + (h - 1 - y)) < 5);
            if corner {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                rgba.extend_from_slice(&[r, g, b, a]);
            }
        }
    }
    tauri::image::Image::new_owned(rgba, w, h)
}
