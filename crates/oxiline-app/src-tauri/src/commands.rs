#![allow(clippy::too_many_arguments)]

//! Tauri commands — thin typed wrappers over `oxiline-core`.
//!
//! Every command derives `#[specta::specta]` so `tauri-specta` emits matching
//! TypeScript bindings; the frontend never hand-writes `invoke()` strings
//! (`04-architecture.md` §4.6). Errors are surfaced as `Result<_, String>`.

use oxiline_core::model::{
    Activity, ActivityInput, Category, Compliance, Plan, PlanInput, PlanOption, PlanSlot, Record,
    RecordState, Scope,
};
use oxiline_core::{activities, categories, plan, record, settings, util};
use serde_json::Value;
use tauri::State;
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

use crate::state::AppState;

/// Run a core operation against a pooled connection, mapping the error to a
/// plain string (carrying the stable error code as a prefix for the frontend).
fn map_err(e: oxiline_core::CoreError) -> String {
    format!("{}: {}", e.code().as_str(), e)
}

// ---- categories ----

#[tauri::command]
#[specta::specta]
pub fn list_categories(state: State<AppState>) -> Result<Vec<Category>, String> {
    categories::list(&state.conn()).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn create_category(
    state: State<AppState>,
    name: String,
    color_hue: f64,
    icon: Option<String>,
) -> Result<Category, String> {
    categories::create(
        &state.conn(),
        categories::NewCategory {
            name,
            color_hue,
            icon,
        },
    )
    .map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn delete_category(state: State<AppState>, id: String) -> Result<(), String> {
    categories::delete(&state.conn(), &id).map_err(map_err)
}

// ---- settings ----

#[tauri::command]
#[specta::specta]
pub fn get_settings(
    state: State<AppState>,
) -> Result<oxiline_core::model::SettingsSnapshot, String> {
    Ok(settings::snapshot(&state.conn()))
}

#[tauri::command]
#[specta::specta]
pub fn set_setting(
    state: State<AppState>,
    key: String,
    value: String,
) -> Result<oxiline_core::model::SettingsSnapshot, String> {
    settings::set_from_str(&state.conn(), &key, &value).map_err(map_err)?;
    Ok(settings::snapshot(&state.conn()))
}

#[tauri::command]
#[specta::specta]
pub fn get_db_path(state: State<AppState>) -> Result<String, String> {
    Ok(state.db_path.display().to_string())
}

// ---- onboarding ----

#[tauri::command]
#[specta::specta]
pub fn set_onboarding_done(state: State<AppState>) -> Result<(), String> {
    settings::set(&state.conn(), "onboarding_done", &Value::Bool(true)).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn is_onboarding_done(state: State<AppState>) -> Result<bool, String> {
    Ok(settings::get_bool(&state.conn(), "onboarding_done", false))
}

// ---- notifications ----

/// Request macOS notification permission. Returns true if granted.
#[tauri::command]
#[specta::specta]
pub async fn request_notification_permission(app: tauri::AppHandle) -> Result<bool, String> {
    use tauri_plugin_notification::PermissionState;
    match app.notification().request_permission() {
        Ok(PermissionState::Granted) => Ok(true),
        Ok(PermissionState::Denied) => Ok(false),
        Ok(_) => Ok(false),
        Err(e) => Err(format!("notification:request_permission: {e}")),
    }
}

/// Check whether notification permission has been granted.
#[tauri::command]
#[specta::specta]
pub fn is_notification_permission_granted(app: tauri::AppHandle) -> Result<bool, String> {
    use tauri_plugin_notification::PermissionState;
    match app.notification().permission_state() {
        Ok(PermissionState::Granted) => Ok(true),
        _ => Ok(false),
    }
}

/// Open macOS System Settings → Notifications (for when permission is denied).
#[tauri::command]
#[specta::specta]
pub fn open_notification_settings(app: tauri::AppHandle) -> Result<(), String> {
    app.opener()
        .open_path(
            "x-apple.systempreferences:com.apple.preference.notifications",
            None::<&str>,
        )
        .map_err(|e| format!("opener: {e}"))
}

// ---- recording layer: activities / plans / records (Plan 2 Task 2) --------

#[tauri::command]
#[specta::specta]
pub fn list_activities(state: State<AppState>, active_only: bool) -> Result<Vec<Activity>, String> {
    activities::list_activities(&state.conn(), active_only).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn create_activity(state: State<AppState>, input: ActivityInput) -> Result<Activity, String> {
    activities::create_activity(&state.conn(), input).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn resolve_activity(state: State<AppState>, query: String) -> Result<Activity, String> {
    activities::resolve_activity(&state.conn(), &query).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn update_activity(
    state: State<AppState>,
    id: String,
    input: ActivityInput,
) -> Result<Activity, String> {
    activities::update_activity(&state.conn(), &id, input).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn delete_activity(state: State<AppState>, id: String, force: bool) -> Result<(), String> {
    activities::delete_activity(&state.conn(), &id, force).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn start_record(state: State<AppState>, activity_id: String) -> Result<RecordState, String> {
    let conn = state.conn();
    record::start(
        &conn,
        &activity_id,
        chrono::Utc::now(),
        &util::today_local(),
    )
    .map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn stop_record(state: State<AppState>) -> Result<RecordState, String> {
    let conn = state.conn();
    record::stop(&conn, chrono::Utc::now(), &util::today_local()).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn current_record_state(state: State<AppState>) -> Result<RecordState, String> {
    let conn = state.conn();
    record::current(&conn, chrono::Utc::now(), &util::today_local()).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn list_records(
    state: State<AppState>,
    activity_id: Option<String>,
    from: String,
    to: String,
) -> Result<Vec<Record>, String> {
    let conn = state.conn();
    record::list_records(&conn, activity_id.as_deref(), &from, &to).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn get_compliance(state: State<AppState>, scope: Scope) -> Result<Vec<Compliance>, String> {
    let conn = state.conn();
    record::compliance(&conn, scope, chrono::Utc::now(), &util::today_local()).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn list_plans(state: State<AppState>, recurring_only: bool) -> Result<Vec<Plan>, String> {
    plan::list_plans(&state.conn(), recurring_only).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn create_plan(state: State<AppState>, input: PlanInput) -> Result<Plan, String> {
    plan::create_plan(&state.conn(), input).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn get_slots_for_date(state: State<AppState>, date: String) -> Result<Vec<PlanSlot>, String> {
    plan::slots_for_date(&state.conn(), &date).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn update_plan(state: State<AppState>, id: String, input: PlanInput) -> Result<Plan, String> {
    plan::update_plan(&state.conn(), &id, input).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn delete_plan(state: State<AppState>, id: String) -> Result<(), String> {
    plan::delete_plan(&state.conn(), &id).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn add_plan_options(
    state: State<AppState>,
    plan_id: String,
    activity_ids: Vec<String>,
) -> Result<Vec<PlanOption>, String> {
    plan::add_options(&state.conn(), &plan_id, &activity_ids).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn resize_plan(
    state: State<AppState>,
    plan_id: String,
    duration_minute: u16,
) -> Result<Plan, String> {
    plan::resize_plan(&state.conn(), &plan_id, duration_minute).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn edit_record(
    state: State<AppState>,
    id: String,
    started_at: Option<String>,
    ended_at: Option<String>,
) -> Result<Record, String> {
    record::edit_record(&state.conn(), &id, started_at, ended_at).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn delete_record(state: State<AppState>, id: String) -> Result<(), String> {
    record::delete_record(&state.conn(), &id).map_err(map_err)
}
