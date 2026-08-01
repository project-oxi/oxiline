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

export interface NowItem {
  id: string;
  is_virtual: boolean;
  title: string;
  start_minute: number | null;
  duration_minute: number | null;
  category_id: string | null;
  remaining_minute: number | null;
  starts_in_minute: number | null;
}

export interface NowContext {
  current: NowItem | null;
  next: NowItem | null;
  generated_at_minute: number;
  generated_at: string;
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
