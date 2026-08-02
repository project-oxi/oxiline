#![allow(clippy::too_many_arguments)]

//! Tauri commands — thin typed wrappers over `oxiline-core`.
//!
//! Every command derives `#[specta::specta]` so `tauri-specta` emits matching
//! TypeScript bindings; the frontend never hand-writes `invoke()` strings
//! (`04-architecture.md` §4.6). Errors are surfaced as `Result<_, String>`.

use oxiline_core::model::{
    Activity, ActivityInput, CardSuggestion, Category, Compliance, Plan, PlanInput,
    PlanOption, PlanSlot, RangeReport, Record, RecordState, RoutineBlock, RoutineStreak, Scope, Task,
    TimelineItem, WeekReport,
};
use oxiline_core::{
    activities, cards, categories, plan, record, reports, routines, settings, tasks, timeline, util,
};
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

// ---- routines ----

#[tauri::command]
#[specta::specta]
pub fn list_routine_groups(
    state: State<AppState>,
) -> Result<Vec<oxiline_core::model::RoutineGroup>, String> {
    oxiline_core::routine_groups::list(&state.conn()).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn create_routine_group(
    state: State<AppState>,
    name: String,
    icon: Option<String>,
) -> Result<oxiline_core::model::RoutineGroup, String> {
    oxiline_core::routine_groups::create(
        &state.conn(),
        oxiline_core::routine_groups::NewRoutineGroup { name, icon },
    )
    .map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn update_routine_group(
    state: State<AppState>,
    id: String,
    name: Option<String>,
    icon: Option<Option<String>>,
    sort_order: Option<i64>,
) -> Result<oxiline_core::model::RoutineGroup, String> {
    oxiline_core::routine_groups::update(
        &state.conn(),
        &id,
        oxiline_core::routine_groups::RoutineGroupUpdate {
            name,
            icon,
            sort_order,
        },
    )
    .map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn delete_routine_group(state: State<AppState>, id: String) -> Result<(), String> {
    oxiline_core::routine_groups::delete(&state.conn(), &id).map_err(map_err)?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn set_routine_group_active(
    state: State<AppState>,
    id: String,
    active: bool,
) -> Result<oxiline_core::model::RoutineGroup, String> {
    oxiline_core::routine_groups::set_active(&state.conn(), &id, active).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn list_routines(
    state: State<AppState>,
    active_only: bool,
) -> Result<Vec<RoutineBlock>, String> {
    routines::list(&state.conn(), active_only).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn create_routine(
    state: State<AppState>,
    title: String,
    start_minute: u16,
    duration_minute: u16,
    weekday_mask: u8,
    category_id: Option<String>,
    effective_from: Option<String>,
    effective_until: Option<String>,
    notes: Option<String>,
) -> Result<RoutineBlock, String> {
    routines::create(
        &state.conn(),
        routines::NewRoutineBlock {
            title,
            start_minute,
            duration_minute,
            weekday_mask,
            category_id,
            effective_from,
            effective_until,
            notes,
        },
    )
    .map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn update_routine(
    state: State<AppState>,
    id: String,
    title: Option<String>,
    start_minute: Option<u16>,
    duration_minute: Option<u16>,
    weekday_mask: Option<u8>,
    category_id: Option<Option<String>>,
    notes: Option<Option<String>>,
) -> Result<RoutineBlock, String> {
    routines::update(
        &state.conn(),
        &id,
        routines::RoutineUpdate {
            title,
            start_minute,
            duration_minute,
            weekday_mask,
            category_id,
            notes,
        },
    )
    .map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn set_routine_active(
    state: State<AppState>,
    id: String,
    active: bool,
) -> Result<RoutineBlock, String> {
    routines::set_active(&state.conn(), &id, active).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn delete_routine(state: State<AppState>, id: String) -> Result<(), String> {
    routines::delete(&state.conn(), &id).map_err(map_err)
}

// ---- timeline ----

#[tauri::command]
#[specta::specta]
pub fn get_timeline(state: State<AppState>, date: String) -> Result<Vec<TimelineItem>, String> {
    timeline::get_timeline_for_date(&state.conn(), &date).map_err(map_err)
}

// ---- tasks ----

#[tauri::command]
#[specta::specta]
pub fn list_backlog(state: State<AppState>) -> Result<Vec<Task>, String> {
    tasks::list_backlog(&state.conn()).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn create_task(
    state: State<AppState>,
    date: Option<String>,
    title: String,
    category_id: Option<String>,
    start_minute: Option<u16>,
    duration_minute: Option<u16>,
    notes: Option<String>,
) -> Result<Task, String> {
    tasks::create(
        &state.conn(),
        tasks::NewTask {
            date,
            title,
            category_id,
            start_minute,
            duration_minute,
            notes,
        },
    )
    .map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn update_task(
    state: State<AppState>,
    id: String,
    title: Option<String>,
    date: Option<Option<String>>,
    start_minute: Option<Option<u16>>,
    duration_minute: Option<Option<u16>>,
    category_id: Option<Option<String>>,
    notes: Option<Option<String>>,
) -> Result<Task, String> {
    tasks::update(
        &state.conn(),
        &id,
        tasks::TaskUpdate {
            title,
            date,
            start_minute,
            duration_minute,
            category_id,
            notes,
        },
    )
    .map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn set_task_done(state: State<AppState>, id: String, done: bool) -> Result<Task, String> {
    let real = tasks::materialize_if_virtual(&state.conn(), &id).map_err(map_err)?;
    tasks::set_done(&state.conn(), &real, done).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn set_task_skipped(state: State<AppState>, id: String, skipped: bool) -> Result<Task, String> {
    let real = tasks::materialize_if_virtual(&state.conn(), &id).map_err(map_err)?;
    tasks::set_skipped(&state.conn(), &real, skipped).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn delete_task(state: State<AppState>, id: String) -> Result<(), String> {
    use oxiline_core::model::TaskSource;
    let real = tasks::materialize_if_virtual(&state.conn(), &id).map_err(map_err)?;
    let t = tasks::get(&state.conn(), &real).map_err(map_err)?;
    match t.source {
        // Routine occurrence → skip (hide) so the virtual does not reappear
        // (03-data-model.md §3.7). Manual task → physical delete.
        TaskSource::Routine => {
            tasks::set_skipped(&state.conn(), &real, true).map_err(map_err)?;
        }
        TaskSource::Manual => {
            tasks::delete(&state.conn(), &real).map_err(map_err)?;
        }
    }
    Ok(())
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

// ---- drag-and-drop ----

/// Materialize a virtual task (turn a routine occurrence into a real DB row)
/// if the given id is virtual, returning the real task id.
#[tauri::command]
#[specta::specta]
pub fn materialize_if_virtual(state: State<AppState>, id: String) -> Result<String, String> {
    tasks::materialize_if_virtual(&state.conn(), &id).map_err(map_err)
}

// ---- quick-add suggestions (autocomplete from templates + history) --------

/// Ranked card signatures for the quick-add palette: on-demand templates
/// (mask=0 routines) first, then distinct historical task/routine titles.
/// Selecting one prefills a new task (`07-ui-screens-and-flows.md` §7.5).
#[tauri::command]
#[specta::specta]
pub fn suggest_cards(
    state: State<AppState>,
    limit: Option<usize>,
) -> Result<Vec<CardSuggestion>, String> {
    cards::suggest(&state.conn(), limit.unwrap_or(0)).map_err(map_err)
}

// ---- reports (habit streak / weekly report) -------------------------------

#[tauri::command]
#[specta::specta]
pub fn get_week_report(state: State<AppState>) -> Result<WeekReport, String> {
    let conn = state.conn();
    reports::week_report(&conn, &util::today_local(), util::now_minute_local()).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn get_range_report(
    state: State<AppState>,
    from: String,
    to: String,
) -> Result<RangeReport, String> {
    let conn = state.conn();
    reports::range_report(
        &conn,
        &from,
        &to,
        &util::today_local(),
        util::now_minute_local(),
    )
    .map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn get_routine_streaks(state: State<AppState>) -> Result<Vec<RoutineStreak>, String> {
    let conn = state.conn();
    reports::routine_streaks(&conn, &util::today_local()).map_err(map_err)
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
pub fn update_activity(state: State<AppState>, id: String, input: ActivityInput) -> Result<Activity, String> {
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
    record::start(&conn, &activity_id, chrono::Utc::now(), &util::today_local()).map_err(map_err)
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