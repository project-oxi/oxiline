//! Background notification scheduler (Phase 2 spec §3).
//!
//! Polls every 60 s. When a block is about to start, posts a one-shot
//! macOS notification. Keeps a memory-only `last_notified` set so the
//! same item is only notified once per scheduler lifetime.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::state::AppState;

const POLL_INTERVAL: Duration = Duration::from_secs(60);
const SLEEP_GAP_MINUTES: i64 = 5;

pub fn spawn_scheduler(app: AppHandle) {
    std::thread::Builder::new()
        .name("oxiline-notifier".into())
        .spawn(move || {
            let last_notified: Arc<Mutex<HashSet<String>>> =
                Arc::new(Mutex::new(HashSet::new()));
            let mut last_now_minute: i64 = -1;

            loop {
                std::thread::sleep(POLL_INTERVAL);

                let state = app.state::<AppState>();
                let conn = state.conn();
                let enabled = oxiline_core::settings::get_bool(
                    &conn,
                    "notifications_enabled",
                    false,
                );
                if !enabled {
                    continue;
                }

                let now_minute = oxiline_core::util::now_minute_local() as i64;

                // Sleep/wake detection: if wall time jumped significantly,
                // drop the dedup set so stale items can be re-notified.
                if last_now_minute >= 0
                    && (now_minute - last_now_minute).abs() > SLEEP_GAP_MINUTES
                {
                    last_notified.lock().clear();
                }
                last_now_minute = now_minute;

                let lead = oxiline_core::settings::get_i64(
                    &conn,
                    "notification_lead_minutes",
                    5,
                );
                let Ok(ctx) =
                    oxiline_core::timeline::get_now_context(&conn, now_minute as u16)
                else {
                    continue;
                };
                let Some(next) = ctx.next else {
                    continue;
                };
                let Some(starts_in) = next.starts_in_minute else {
                    continue;
                };
                if (starts_in as i64) > lead {
                    continue;
                }
                if last_notified.lock().contains(&next.id) {
                    continue;
                }

                let body = {
                    let locale = oxiline_core::settings::get_string(
                        &conn,
                        "locale",
                        "system",
                    );
                    if locale.starts_with("en") {
                        format!("Starts in {} min", starts_in)
                    } else {
                        format!("{}분 후 시작돼요", starts_in)
                    }
                };
                let _ = app
                    .notification()
                    .builder()
                    .title(&next.title)
                    .body(&body)
                    .show();
                last_notified.lock().insert(next.id);
            }
        })
        .expect("failed to spawn notifier thread");
}
