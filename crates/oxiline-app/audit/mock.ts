// AUDIT-ONLY scaffolding. Patches window.__TAURI_INTERNALS__ so the real app
// boots in a plain browser without Tauri/DB. Removed before shipping (Task 7).
import type { TimelineItem, Category, Task } from "../src/types";

const DAY = "2026-07-31";

const categories: Category[] = [
  { id: "c-work", name: "업무", color_hue: 250, icon: "briefcase", sort_order: 0, is_builtin: true, created_at: "", updated_at: "" },
  { id: "c-study", name: "학습", color_hue: 300, icon: "book-open", sort_order: 1, is_builtin: true, created_at: "", updated_at: "" },
];

let timeline: TimelineItem[] = [
  { id: "t1", is_virtual: false, title: "아침 회의", start_minute: 540, duration_minute: 30, category_id: "c-work", is_done: false, is_skipped: false, origin_routine_block_id: null },
  { id: "t2", is_virtual: false, title: "코딩 세션", start_minute: 555, duration_minute: 60, category_id: "c-work", is_done: false, is_skipped: false, origin_routine_block_id: null },
  // 4-overlap cluster at 14:00 (for Task 6 verification)
  { id: "t3", is_virtual: false, title: "페어 프로그래밍", start_minute: 840, duration_minute: 60, category_id: "c-work", is_done: false, is_skipped: false, origin_routine_block_id: null },
  { id: "t4", is_virtual: false, title: "기사 읽기", start_minute: 840, duration_minute: 45, category_id: "c-study", is_done: false, is_skipped: false, origin_routine_block_id: null },
  { id: "t5", is_virtual: false, title: "블로그 초안", start_minute: 840, duration_minute: 60, category_id: "c-study", is_done: false, is_skipped: false, origin_routine_block_id: null },
  { id: "t6", is_virtual: false, title: "코드 리뷰", start_minute: 840, duration_minute: 30, category_id: "c-work", is_done: false, is_skipped: false, origin_routine_block_id: null },
];

const settings = { day_start_hour: 5, day_end_hour: 22, workload_warning_minutes: 600, locale: "ko", theme: "light" };

declare global {
  interface Window { __mockLog: { cmd: string; args: unknown }[]; }
}
window.__mockLog = [];

let nextId = 100;
const handlers: Record<string, (args: any) => unknown> = {
  get_settings: () => settings,
  is_onboarding_done: () => true,
  get_timeline: () => timeline,
  list_categories: () => categories,
  list_backlog: () => [],
  list_routines: () => [],
  get_now_context: () => ({ now: null, current: null, next: null }),
  get_week_report: () => ({}),
  create_task: (a) => {
    const t: Task = { id: `t${nextId++}`, date: a.date, title: a.title, category_id: a.categoryId, start_minute: a.startMinute, duration_minute: a.durationMinute, is_done: false, done_at: null, is_skipped: false, source: "manual", source_routine_block_id: null, notes: a.notes, sort_order: 0 };
    window.__mockLog.push({ cmd: "create_task", args: a });
    timeline.push({ id: t.id, is_virtual: false, title: t.title, start_minute: t.start_minute, duration_minute: t.duration_minute, category_id: t.category_id, is_done: false, is_skipped: false, origin_routine_block_id: null });
    return t;
  },
  update_task: (a) => {
    window.__mockLog.push({ cmd: "update_task", args: a });
    const ti = timeline.find((x) => x.id === a.id);
    if (ti) {
      if (a.startMinute != null) ti.start_minute = a.startMinute;
      if (a.durationMinute != null) ti.duration_minute = a.durationMinute;
    }
    return null;
  },
  materialize_if_virtual: (a) => a.id,
  set_task_done: (a) => { window.__mockLog.push({ cmd: "set_task_done", args: a }); return null; },
  set_task_skipped: () => null,
  delete_task: () => null,
};

let cbId = 0;
(window as any).__TAURI_INTERNALS__ = {
  invoke: (cmd: string, args?: Record<string, unknown>) => {
    if (cmd === "plugin:event|listen") return Promise.resolve(cbId++);
    const h = handlers[cmd];
    if (!h) return Promise.reject(new Error(`audit mock: unhandled command ${cmd}`));
    return Promise.resolve(h(args));
  },
  transformCallback: () => 0,
};
