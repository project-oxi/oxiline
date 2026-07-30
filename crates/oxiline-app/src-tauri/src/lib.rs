//! OxiLine Tauri GUI wiring (`04-architecture.md`).

mod commands;
mod hud;
mod notifier;
mod shortcuts;
mod state;
mod tray;
mod watcher;

use tauri::{Listener, Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_specta::{collect_commands, Builder as SpectaBuilder};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let specta = SpectaBuilder::<tauri::Wry>::new().commands(collect_commands![
        commands::list_categories,
        commands::create_category,
        commands::delete_category,
        commands::list_routines,
        commands::create_routine,
        commands::update_routine,
        commands::set_routine_active,
        commands::delete_routine,
        commands::get_timeline,
        commands::get_now_context,
        commands::list_backlog,
        commands::create_task,
        commands::update_task,
        commands::set_task_done,
        commands::set_task_skipped,
        commands::delete_task,
        commands::get_settings,
        commands::set_setting,
        commands::get_db_path,
        commands::set_onboarding_done,
        commands::is_onboarding_done,
        commands::request_notification_permission,
        commands::is_notification_permission_granted,
        commands::open_notification_settings,
    ]);

    // Emit typed TS bindings for the frontend in dev builds.
    #[cfg(debug_assertions)]
    {
        let _ = specta.export(
            specta_typescript::Typescript::default(),
            "../src/bindings.ts",
        );
    }

    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ));

    #[cfg(target_os = "macos")]
    {
        builder = builder.plugin(tauri_nspanel::init());
    }

    builder
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .manage(state::AppState::new())
        .invoke_handler(specta.invoke_handler())
        .setup(|app| {
            // Dock icon hidden; the app lives in the menu bar (§4.3).
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            tray::build(app.handle())?;
            shortcuts::register_default(app.handle());
            // macOS: convert hud window to non-activating NSPanel.
            #[cfg(target_os = "macos")]
            if let Err(e) = hud::init_panel(app.handle()) {
                eprintln!("oxiline: hud init_panel failed: {e}");
            }
            notifier::spawn_scheduler(app.handle().clone());
            watcher::spawn(app.handle().clone());

            // Refresh the tray's dynamic "지금" row when the DB changes.
            let h = app.handle().clone();
            app.listen("oxiline://db-changed", move |_event| {
                tray::refresh(&h);
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the main window hides it; the process keeps running in the
            // menu bar (§4.3). Only the tray "종료" truly quits.
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running OxiLine");
}
