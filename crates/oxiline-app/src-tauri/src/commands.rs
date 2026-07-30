//! Tauri commands — thin typed wrappers over `oxiline-core`.
//!
//! Every command derives `#[specta::specta]` so `tauri-specta` emits matching
//! TypeScript bindings; the frontend never hand-writes `invoke()` strings
//! (`04-architecture.md` §4.6). Errors are surfaced as `Result<_, String>`.

use oxiline_core::model::{
    Category, NowContext, RoutineBlock, Task, TimelineItem,
};
use oxiline_core::{categories, routines, settings, tasks, timeline, util};
use serde_json::Value;
use tauri::State;

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

#[tauri::command]
#[specta::specta]
pub fn get_now_context(state: State<AppState>) -> Result<NowContext, String> {
    timeline::get_now_context(&state.conn(), util::now_minute_local()).map_err(map_err)
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
pub fn set_task_done(
    state: State<AppState>,
    id: String,
    done: bool,
) -> Result<Task, String> {
    let real = tasks::materialize_if_virtual(&state.conn(), &id).map_err(map_err)?;
    tasks::set_done(&state.conn(), &real, done).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn set_task_skipped(
    state: State<AppState>,
    id: String,
    skipped: bool,
) -> Result<Task, String> {
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
pub fn get_settings(state: State<AppState>) -> Result<oxiline_core::model::SettingsSnapshot, String> {
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
