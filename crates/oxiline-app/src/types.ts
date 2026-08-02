// Domain types — mirror `oxiline-core::model` (kept in sync by hand; the Rust
// side also exports tauri-specta bindings as a cross-check).

export type TaskSource = "manual" | "routine";

export interface RoutineGroup {
  id: string;
  name: string;
  icon: string | null;
  is_active: boolean;
  sort_order: number;
  created_at: string;
  updated_at: string;
}

export interface Category {
  id: string;
  name: string;
  color_hue: number;
  icon: string | null;
  sort_order: number;
  is_builtin: boolean;
}

export interface Task {
  id: string;
  date: string | null;
  title: string;
  category_id: string | null;
  start_minute: number | null;
  duration_minute: number | null;
  is_done: boolean;
  done_at: string | null;
  is_skipped: boolean;
  source: TaskSource;
  source_routine_block_id: string | null;
  notes: string | null;
  sort_order: number;
}

export interface RoutineBlock {
  id: string;
  group_id: string | null;
  title: string;
  category_id: string | null;
  start_minute: number;
  duration_minute: number;
  weekday_mask: number;
  effective_from: string | null;
  effective_until: string | null;
  is_active: boolean;
  color_override: string | null;
  notes: string | null;
  sort_order: number;
}

export interface TimelineItem {
  id: string;
  is_virtual: boolean;
  title: string;
  start_minute: number | null;
  duration_minute: number | null;
  category_id: string | null;
  is_done: boolean;
  is_skipped: boolean;
  origin_routine_block_id: string | null;
}

/** Reusable card signature for quick-add autocomplete
 *  (mirrors `oxiline_core::model::CardSuggestion`). */
export interface CardSuggestion {
  title: string;
  category_id: string | null;
  duration_minute: number | null;
  notes: string | null;
  is_template: boolean;
  template_id: string | null;
  last_used_at: string | null;
}

export type Settings = Record<string, unknown>;

// ---- reports (habit streak / weekly report) ----

export interface CategoryBreakdown {
  category_id: string | null;
  category_name: string;
  done: number;
  skipped: number;
  not_recorded: number;
  completion_rate: number | null;
}

export interface DayTotals {
  done: number;
  skipped: number;
  not_recorded: number;
  upcoming: number;
}

export interface RoutineStreak {
  routine_id: string;
  title: string;
  current: number;
  last_done_date: string | null;
}

export interface WeekReport {
  week_start: string;
  week_end: string;
  days: unknown[];
  totals: DayTotals;
  completion_rate: number | null;
  prev_completion_rate: number | null;
  categories: CategoryBreakdown[];
  streaks: RoutineStreak[];
}

export interface RangeReport {
  from: string;
  to: string;
  days: unknown[];
  totals: DayTotals;
  completion_rate: number | null;
  categories: CategoryBreakdown[];
  streaks: RoutineStreak[];
}

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
