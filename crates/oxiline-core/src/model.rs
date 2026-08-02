//! Domain types shared by GUI and CLI.
//!
//! `specta::Type` is derived on every public struct/enum so `tauri-specta` can
//! emit matching TypeScript bindings (`03-data-model.md` §3.11).

use serde::{Deserialize, Serialize};
use specta::Type;

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

/// "What is happening now + next" derived from the recording layer (active
/// record + plan slots). `current` is the in-progress thing — an active record
/// (priority) or the plan slot containing now; `next` is the closest future
/// plan slot. Shared by the notifier, tray, and `oxiline now`.
#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct NowSummary {
    pub current: Option<NowEntry>,
    pub next: Option<NowEntry>,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct NowEntry {
    pub id: String,
    pub title: String,
    /// Slot start (local minute-of-day). `None` for an open-ended record.
    pub start_minute: Option<u16>,
    /// Minutes until the entry starts (only for `next`).
    pub starts_in_minute: Option<i64>,
    /// Minutes until the slot ends (only for a `current` plan slot; a record
    /// is open-ended so this is `None`).
    pub remaining_minute: Option<i64>,
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