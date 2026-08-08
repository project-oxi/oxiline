//! Menu-bar tray icon + menu (`07-ui-screens-and-flows.md` §7.7, §4.3).
//!
//! Multi-slot layout:
//! - one always-on menu tray (`MENU_TRAY_ID`) that owns the dropdown menu;
//! - one `TrayIcon` per enabled data slot (`tray-slot-{kind_id}`) tracked in
//!   [`BUILT_SLOTS`]. Each data slot left-clicks to [`show_main`] and does
//!   NOT show a menu on left-click.

use std::collections::HashMap;
use std::sync::Mutex;
use tauri::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIcon;
use tauri::{AppHandle, Emitter, Manager};

use tauri_plugin_autostart::ManagerExt;

use crate::tray_render::{label_for, render_menu_dot, render_slot, LabelCtx};
use crate::{hud, state::AppState};
use oxiline_core::model::TraySlotKind;
use oxiline_core::tray_slots;

const EVENT_OPEN_PREFERENCES: &str = "oxiline://open-preferences";
const EVENT_OPEN_QUICK_ADD: &str = "oxiline://open-quick-add";

const MENU_TRAY_ID: &str = "tray-menu";

fn slot_tray_id(kind: TraySlotKind) -> String {
    format!("tray-slot-{}", tray_slots::slot_kind_to_id(kind))
}

static BUILT_SLOTS: once_cell::sync::Lazy<Mutex<HashMap<TraySlotKind, TrayIcon>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

const FG_COLOR: (u8, u8, u8, u8) = (60, 60, 60, 255);
const MENU_DOT_COLOR: (u8, u8, u8, u8) = (130, 130, 130, 255);
const STATE_DOT_RECORDING: (u8, u8, u8, u8) = (43, 179, 160, 255);
const STATE_DOT_NEXT_SOON: (u8, u8, u8, u8) = (220, 160, 40, 255);
const STATE_DOT_IDLE: (u8, u8, u8, u8) = (130, 130, 130, 255);

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let sep0 = PredefinedMenuItem::separator(app)?;
    let now_label = now_summary(app);
    let info = MenuItem::with_id(app, "info", &now_label, false, None::<&str>)?;
    let hud_item = MenuItem::with_id(app, "hud", "지금 보기 (HUD)", true, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "OxiLine 열기", true, None::<&str>)?;
    let quick = MenuItem::with_id(app, "quick", "빠른 추가…", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let prefs = MenuItem::with_id(app, "prefs", "환경설정…", true, None::<&str>)?;
    let autostart = CheckMenuItem::with_id(
        app,
        "autostart",
        "로그인 시 자동 실행",
        true,
        autostart_enabled(app),
        None::<&str>,
    )?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "OxiLine 종료", true, None::<&str>)?;

    Menu::with_items(
        app,
        &[
            &info, &sep0, &hud_item, &open, &quick, &sep1, &prefs, &autostart, &sep2, &quit,
        ],
    )
}

/// Build the always-on menu tray and one `TrayIcon` per enabled data slot.
pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app)?;
    let menu_dot = render_menu_dot(MENU_DOT_COLOR);
    let menu_tray = tauri::tray::TrayIconBuilder::with_id(MENU_TRAY_ID)
        .icon(menu_dot)
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(on_menu_event)
        .build(app)?;
    let _ = menu_tray.set_icon_as_template(true);

    let conn = app.state::<AppState>().conn();
    let resolved = tray_slots::resolve(&conn);
    for pref in &resolved.enabled {
        build_slot(app, pref.kind)?;
    }
    refresh(app);
    Ok(())
}

fn build_slot(app: &AppHandle, kind: TraySlotKind) -> tauri::Result<()> {
    let label = slot_label(app, kind);
    let img = render_slot(&label, FG_COLOR);
    let app_for_event = app.clone();
    let tray = tauri::tray::TrayIconBuilder::with_id(slot_tray_id(kind))
        .icon(img)
        .icon_as_template(true)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(move |_tray, event| {
            if let tauri::tray::TrayIconEvent::Click { button, .. } = event
                && matches!(button, tauri::tray::MouseButton::Left)
            {
                show_main(&app_for_event);
            }
        })
        .build(app)?;
    let _ = tray.set_icon_as_template(true);
    BUILT_SLOTS.lock().unwrap().insert(kind, tray);
    Ok(())
}

/// Refresh the dynamic content of every data slot, then refresh the menu's
/// "지금" row. Called from the 60-second timer and from `db-changed`.
pub fn refresh(app: &AppHandle) {
    let slots = BUILT_SLOTS.lock().unwrap();
    for (kind, tray) in slots.iter() {
        let img = match kind {
            TraySlotKind::StateDot => Some(render_state_dot(app)),
            _ => {
                let label = slot_label(app, *kind);
                if label.is_empty() { None } else { Some(render_slot(&label, FG_COLOR)) }
            }
        };
        if let Some(img) = img {
            let _ = tray.set_icon(Some(img));
        }
    }
    drop(slots);
    // Also rebuild the menu so the dynamic "지금" row stays fresh.
    if let Ok(menu) = build_menu(app)
        && let Some(tray) = app.tray_by_id(MENU_TRAY_ID)
    {
        let _ = tray.set_menu(Some(menu));
    }
}

fn render_state_dot(app: &AppHandle) -> tauri::image::Image<'static> {
    let conn = app.state::<AppState>().conn();
    let summary = oxiline_core::plan::now_summary(&conn, oxiline_core::util::now_minute_local()).ok();
    let color = match (summary.as_ref().and_then(|s| s.current.as_ref()), summary.as_ref().and_then(|s| s.next.as_ref())) {
        (Some(_), _) => STATE_DOT_RECORDING,
        (None, Some(n)) if n.starts_in_minute.unwrap_or(i64::MAX) <= 5 => STATE_DOT_NEXT_SOON,
        _ => STATE_DOT_IDLE,
    };
    render_menu_dot(color)
}

fn slot_label(app: &AppHandle, kind: TraySlotKind) -> String {
    let conn = app.state::<AppState>().conn();
    let locale_raw = oxiline_core::settings::get_string(&conn, "locale", "system");
    let locale = if locale_raw == "en" { "en" } else { "ko" };
    let now_minute = oxiline_core::util::now_minute_local();
    let summary = oxiline_core::plan::now_summary(&conn, now_minute).ok();
    let summary = match summary { Some(s) => s, None => return String::new() };
    let ctx = LabelCtx {
        now_minute,
        rounding_minutes: oxiline_core::settings::get_i64(&conn, "record_rounding_minutes", 5),
        now_summary: &summary,
    };
    label_for(kind, locale, &ctx)
}

/// Tear down every data-slot tray icon and rebuild from the persisted prefs.
/// Used after `update_tray_slots` and on the `oxiline://tray-changed` event.
pub fn rebuild(app: &AppHandle) {
    let kinds: Vec<TraySlotKind> = BUILT_SLOTS.lock().unwrap().keys().copied().collect();
    for kind in kinds {
        let id = slot_tray_id(kind);
        let _ = app.remove_tray_by_id(&id);
    }
    BUILT_SLOTS.lock().unwrap().clear();
    if let Err(e) = build(app) {
        eprintln!("tray::rebuild failed: {e}");
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

/// Bring the main window to the front — the **reopen** half of the
/// close/reopen pair. The close button only hides the window (`lib.rs`
/// `CloseRequested` → `prevent_close` + `hide`), and this brings it back,
/// reached from the tray menu, the single-instance callback, and the HUD.
///
/// `unminimize()` is load-bearing: tao's `set_focus()` only fires
/// `activateIgnoringOtherApps` when the window is visible **and not
/// miniaturized**, so a window the user minimized (yellow dot) would stay
/// buried behind the active app even after `show()`. `unminimize` clears that
/// so `set_focus` activates for real.
pub(crate) fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
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
    match oxiline_core::plan::now_summary(&state.conn(), oxiline_core::util::now_minute_local()) {
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
