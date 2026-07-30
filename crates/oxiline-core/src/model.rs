//! Domain types shared by GUI and CLI.
//!
//! `specta::Type` is derived on every public struct/enum so `tauri-specta` can
//! emit matching TypeScript bindings (`03-data-model.md` §3.11).

use serde::{Deserialize, Serialize};
use specta::Type;

/// Where a `Task` row came from (`03-data-model.md` §3.5).
#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskSource {
    Manual,
    Routine,
}

impl TaskSource {
    pub fn as_db_str(&self) -> &'static str {
        match self {
            TaskSource::Manual => "manual",
            TaskSource::Routine => "routine",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "routine" => TaskSource::Routine,
            _ => TaskSource::Manual,
        }
    }
}

/// A recurring block of the day — the skeleton of a routine
/// (`03-data-model.md` §3.3).
#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct RoutineBlock {
    pub id: String,
    pub group_id: Option<String>,
    pub title: String,
    pub category_id: Option<String>,
    pub start_minute: u16,
    pub duration_minute: u16,
    /// bit0=Mon … bit6=Sun. 0b1111111 = daily.
    pub weekday_mask: u8,
    pub effective_from: Option<String>,
    pub effective_until: Option<String>,
    pub is_active: bool,
    pub color_override: Option<String>,
    pub notes: Option<String>,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// A bundle of routine blocks (`03-data-model.md` §3.4). UI is Phase 2 but the
/// schema exists from v1.
#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct RoutineGroup {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub is_active: bool,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// A task row — a concrete item for a date (or backlog when `date` is None)
/// (`03-data-model.md` §3.5). May be a materialized routine occurrence.
#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct Task {
    pub id: String,
    pub date: Option<String>,
    pub title: String,
    pub category_id: Option<String>,
    pub start_minute: Option<u16>,
    pub duration_minute: Option<u16>,
    pub is_done: bool,
    pub done_at: Option<String>,
    pub is_skipped: bool,
    pub source: TaskSource,
    pub source_routine_block_id: Option<String>,
    pub notes: Option<String>,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Color-tag category (`03-data-model.md` §3.6).
#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub color_hue: f64,
    pub icon: Option<String>,
    pub sort_order: i64,
    pub is_builtin: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Unified view model returned by `get_timeline_for_date`
/// (`03-data-model.md` §3.11). Virtual occurrences and materialized tasks share
/// one shape so the frontend need not distinguish.
#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct TimelineItem {
    /// Real `task.id` or `"virtual:{block_id}:{date}"`.
    pub id: String,
    pub is_virtual: bool,
    pub title: String,
    pub start_minute: Option<u16>,
    pub duration_minute: Option<u16>,
    pub category_id: Option<String>,
    pub is_done: bool,
    pub is_skipped: bool,
    pub origin_routine_block_id: Option<String>,
}

/// "What is happening now" context shared by the HUD and `oxiline now`
/// (`05-cli-spec.md` §5.2). `current` is the in-progress item (if any); `next`
/// is the closest future item today.
#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct NowItem {
    pub id: String,
    pub is_virtual: bool,
    pub title: String,
    pub start_minute: Option<u16>,
    pub duration_minute: Option<u16>,
    pub category_id: Option<String>,
    /// Minutes remaining in the current item (only for `current`).
    pub remaining_minute: Option<i64>,
    /// Minutes until the item starts (only for `next`).
    pub starts_in_minute: Option<i64>,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct NowContext {
    pub current: Option<NowItem>,
    pub next: Option<NowItem>,
    pub generated_at_minute: u16,
    pub generated_at: String,
}

/// Typed snapshot of all known settings (avoids `serde_json::Value` which does
/// not implement `specta::Type`).
#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct SettingsSnapshot {
    pub locale: String,
    pub theme: String,
    pub global_hotkey: String,
    pub hud_duration_ms: i64,
    pub day_start_hour: i64,
    pub day_end_hour: i64,
    pub week_starts_on: String,
    pub launch_at_login: bool,
    pub workload_warning_minutes: i64,
    pub onboarding_done: bool,
}
