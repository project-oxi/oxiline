// Domain types — mirror `oxiline-core::model` (kept in sync by hand; the Rust
// side also exports tauri-specta bindings as a cross-check).

export type TaskSource = "manual" | "routine";

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
