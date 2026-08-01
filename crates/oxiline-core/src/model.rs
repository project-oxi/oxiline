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

/// A reusable card signature for quick-add autocomplete (`cards::suggest`).
///
/// Merges on-demand templates (`routine_blocks` with `weekday_mask == 0`)
/// with distinct historical titles drawn from past `tasks` and recurring
/// `routine_blocks`. Selecting one prefills a new task
/// (title/category/duration/notes) instead of retyping it (`07-ui-screens-and-flows.md` §7.5).
#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct CardSuggestion {
    pub title: String,
    pub category_id: Option<String>,
    pub duration_minute: Option<u16>,
    pub notes: Option<String>,
    /// `true` when this is a curated on-demand template (a `routine_block`
    /// with `weekday_mask == 0`); `false` for an aggregated history entry.
    pub is_template: bool,
    /// The originating `routine_block.id` when this suggestion is a template
    /// or a recurring routine; `None` for task-only history.
    pub template_id: Option<String>,
    pub last_used_at: Option<String>,
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
    pub notifications_enabled: bool,
    pub notification_lead_minutes: u32,
}

// ---- report types (habit streak / weekly report) -------------------------

/// Per-day completion breakdown for reports (`reports::day_breakdown`).
#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct DayBreakdown {
    pub date: String,
    pub done: u32,
    pub skipped: u32,
    pub not_recorded: u32,
    pub upcoming: u32,
    pub completion_rate: Option<f64>,
    pub categories: Vec<CategoryBreakdown>,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct CategoryBreakdown {
    pub category_id: Option<String>,
    /// Localized at the display layer when empty (no category).
    pub category_name: String,
    pub done: u32,
    pub skipped: u32,
    pub not_recorded: u32,
    pub completion_rate: Option<f64>,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct DayTotals {
    pub done: u32,
    pub skipped: u32,
    pub not_recorded: u32,
    pub upcoming: u32,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct WeekReport {
    pub week_start: String,
    pub week_end: String,
    pub days: Vec<DayBreakdown>,
    pub totals: DayTotals,
    pub completion_rate: Option<f64>,
    pub prev_completion_rate: Option<f64>,
    pub categories: Vec<CategoryBreakdown>,
    pub streaks: Vec<RoutineStreak>,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct RangeReport {
    pub from: String,
    pub to: String,
    pub days: Vec<DayBreakdown>,
    pub totals: DayTotals,
    pub completion_rate: Option<f64>,
    pub categories: Vec<CategoryBreakdown>,
    pub streaks: Vec<RoutineStreak>,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct RoutineStreak {
    pub routine_id: String,
    pub title: String,
    pub current: u32,
    pub last_done_date: Option<String>,
}

// ---- recording layer (docs/superpowers/specs/2026-08-01-record-layer-design.md §5.4) ----

/// A switchable, budgetable unit of work (subsumes legacy task/routine/card-template).
#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct Activity {
    pub id: String,
    pub name: String,
    pub hue_label: Option<String>,
    pub icon: Option<String>,
    pub category_id: Option<String>,
    pub target_minutes_daily: Option<u32>,
    pub target_minutes_weekly: Option<u32>,
    pub is_active: bool,
    pub sort_order: i32,
}

/// A planned time slot holding OR alternatives (replaces routine_blocks).
#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct Plan {
    pub id: String,
    pub date: Option<String>,
    pub start_minute: u16,
    pub duration_minute: u16,
    pub weekday_mask: u8,
    pub title: Option<String>,
    pub sort_order: i32,
}

/// One alternative within a plan (an activity reference).
#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct PlanOption {
    pub id: String,
    pub plan_id: String,
    pub activity_id: String,
    pub sort_order: i32,
}

/// A plan rendered for a specific date, with its option activities and resolution
/// (set by `plan::slots_for_date`; `is_resolved` / `resolved_by` populated when a
/// matching record exists for this date).
#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct PlanSlot {
    pub plan_id: String,
    pub date: String,
    pub start_minute: u16,
    pub duration_minute: u16,
    pub options: Vec<Activity>,
    pub is_resolved: bool,
    pub resolved_by: Option<Record>,
}

/// An actual recorded interval of one activity (`started_at` -> `ended_at`).
/// `ended_at == None` means the record is currently open.
#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct Record {
    pub id: String,
    pub activity_id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub note: Option<String>,
}

/// The currently-recording session (if any). `elapsed_seconds` is computed at
/// query time and rounded through `record_rounding_minutes`.
#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct ActiveSession {
    pub record: Record,
    pub activity: Activity,
    pub elapsed_seconds: u64,
}

/// Per-activity compliance snapshot (neutral ratio, never a verdict).
#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct Compliance {
    pub activity: Activity,
    pub recorded_seconds: u64,
    pub target_seconds: Option<u64>,
    pub ratio: Option<f64>,
    pub remaining_seconds: Option<i64>,
    pub state: ComplianceState,
}

/// Compliance state — neutral labels, never status-red/green.
#[derive(Serialize, Deserialize, Type, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceState {
    Under,
    Met,
    Over,
    Unbudgeted,
}

/// Snapshot of the recording state at a moment: the active session (if any)
/// plus the compliance summary for today.
#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct RecordState {
    pub active: Option<ActiveSession>,
    pub today: Vec<Compliance>,
    pub generated_at: String,
}

/// Compliance scope selector (today vs weekly).
#[derive(Serialize, Deserialize, Type, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Today,
    Week,
}

/// Create/update payload for an activity. Double-Option on target axes:
/// `None` = leave the column unchanged, `Some(None)` = clear the budget,
/// `Some(Some(minutes))` = set the budget (mirrors `oxiline activity edit --daily 0`).
#[derive(Serialize, Deserialize, Type, Clone, Debug, Default)]
pub struct ActivityInput {
    pub name: Option<String>,
    pub hue_label: Option<String>,
    pub icon: Option<String>,
    pub category_id: Option<String>,
    pub target_minutes_daily: Option<Option<u32>>,
    pub target_minutes_weekly: Option<Option<u32>>,
    pub is_active: Option<bool>,
    pub sort_order: Option<i32>,
}

/// Create/update payload for a plan. `activity_ids` defines the OR alternatives;
/// `date` + `weekday_mask` follow the spec constraint (one of the two is set).
#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct PlanInput {
    pub date: Option<String>,
    pub start_minute: u16,
    pub duration_minute: u16,
    pub weekday_mask: u8,
    pub title: Option<String>,
    pub activity_ids: Vec<String>,
}