//! Menu-bar tray icon + menu (`07-ui-screens-and-flows.md` §7.7, §4.3).

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, MenuEvent};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

use image::{ImageBuffer, Rgba};

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
    let icon = render_progress_icon(0.0);
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
    // Update tray icon with current day progress.
    if let Some(tray) = app.tray_by_id("main-tray") {
        let state = app.state::<AppState>();
        let conn = state.conn();
        let now_min = oxiline_core::util::now_minute_local() as f32;
        let day_start = oxiline_core::settings::get_i64(&conn, "day_start_hour", 5) as f32 * 60.0;
        let day_end = oxiline_core::settings::get_i64(&conn, "day_end_hour", 26) as f32 * 60.0;
        let progress = ((now_min - day_start) / (day_end - day_start)).clamp(0.0, 1.0);
        let _ = tray.set_icon(Some(render_progress_icon(progress)));

        // Rebuild the menu (dynamic now-label + autostart state).
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

/// render_progress_icon: 22×22 RGBA progress bar.
/// `progress` = 0.0 (start of day) → 1.0 (end of day).
fn render_progress_icon(progress: f32) -> tauri::image::Image<'static> {
    const SIZE: u32 = 22;
    const BAR_PAD: u32 = 4;
    const OXIDE_R: u8 = 0x2b;
    const OXIDE_G: u8 = 0xb3;
    const OXIDE_B: u8 = 0xa0;

    let p = progress.clamp(0.0, 1.0);
    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(SIZE, SIZE, Rgba([0, 0, 0, 0]));

    // Bar background: subtle outline
    for y in BAR_PAD..(SIZE - BAR_PAD) {
        for x in 2..(SIZE - 2) {
            img.put_pixel(x, y, Rgba([128, 128, 128, 60]));
        }
    }
    // Bar fill
    let fill_w = ((SIZE - 4) as f32 * p) as u32;
    for y in BAR_PAD..(SIZE - BAR_PAD) {
        for x in 2..(2 + fill_w) {
            img.put_pixel(x, y, Rgba([OXIDE_R, OXIDE_G, OXIDE_B, 255]));
        }
    }
    // Leading white dot at the fill edge
    if p > 0.0 && fill_w > 0 {
        let cx = (2 + fill_w - 1) as i32;
        let cy = (SIZE / 2) as i32;
        for dy in -2..=2 {
            for dx in -2..=2 {
                if dx * dx + dy * dy <= 4 {
                    let px = (cx + dx) as u32;
                    let py = (cy + dy) as u32;
                    if px < SIZE && py < SIZE {
                        img.put_pixel(px, py, Rgba([255, 255, 255, 255]));
                    }
                }
            }
        }
    }

    let rgba = img.into_raw();
    tauri::image::Image::new_owned(rgba, SIZE, SIZE)
}

