// Domain types — mirror `oxiline-core::model` (kept in sync by hand; the Rust
// side also exports tauri-specta bindings as a cross-check).

export interface Category {
  id: string;
  name: string;
  color_hue: number;
  icon: string | null;
  sort_order: number;
  is_builtin: boolean;
}

export type Settings = Record<string, unknown>;

// ---- recording layer (Plan 1 core; mirrors oxiline-core::model) -----------
// Struct fields are snake_case (serde default); command *args* are camelCase.

export interface Activity {
  id: string;
  name: string;
  hue_label: string | null;
  icon: string | null;
  category_id: string | null;
  target_minutes_daily: number | null;
  target_minutes_weekly: number | null;
  is_active: boolean;
  sort_order: number;
}

/** Tri-state budgets: a number sets it; null/omitted leaves it unset on create. */
export interface ActivityInput {
  name?: string | null;
  hue_label?: string | null;
  icon?: string | null;
  category_id?: string | null;
  target_minutes_daily?: number | null;
  target_minutes_weekly?: number | null;
  is_active?: boolean | null;
  sort_order?: number | null;
}

export interface Plan {
  id: string;
  date: string | null;
  start_minute: number;
  duration_minute: number;
  weekday_mask: number;
  title: string | null;
  sort_order: number;
}

/** One alternative within a plan (mirrors `oxiline_core::model::PlanOption`). */
export interface PlanOption {
  id: string;
  plan_id: string;
  activity_id: string;
  sort_order: number;
}

export interface PlanInput {
  date?: string | null;
  start_minute: number;
  duration_minute: number;
  weekday_mask: number;
  title?: string | null;
  activity_ids: string[];
}

export interface PlanSlot {
  plan_id: string;
  date: string;
  start_minute: number;
  duration_minute: number;
  weekday_mask: number;
  options: Activity[];
  is_resolved: boolean;
  resolved_by: ActivityRecord | null;
}

export interface ActivityRecord {
  id: string;
  activity_id: string;
  started_at: string;
  ended_at: string | null;
  note: string | null;
}

export interface ActiveSession {
  record: ActivityRecord;
  activity: Activity;
  elapsed_seconds: number;
}

export type ComplianceState = "under" | "met" | "over" | "unbudgeted";

export interface Compliance {
  activity: Activity;
  recorded_seconds: number;
  target_seconds: number | null;
  ratio: number | null;
  remaining_seconds: number | null;
  state: ComplianceState;
}

export interface RecordState {
  active: ActiveSession | null;
  today: Compliance[];
  generated_at: string;
}

export type Scope = "today" | "week";

/** Menu-bar slot kind (`oxiline_core::model::TraySlotKind`).
 *  Serde `rename_all = "snake_case"` so the wire values are snake_case. */
export type TraySlotKind = "now_recording" | "now_next" | "state_dot";

/** Per-slot preference persisted in the `tray_slots` setting
 *  (`oxiline_core::model::TraySlotPref`). */
export interface TraySlotPref {
  kind: TraySlotKind;
  on: boolean;
  order: number;
}

/** Whether the bundled `oxiline` CLI is exposed on $PATH. Mirrors
 * `oxiline_app_lib::cli::CliState` (serde + specta rename_all =
 * "lowercase"). */
export type CliState = "installed" | "not-installed" | "stale";
