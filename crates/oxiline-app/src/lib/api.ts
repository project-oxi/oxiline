// Typed wrappers over the Rust Tauri commands. Command names match the
// `#[tauri::command]` fn names exactly; args are camelCased by Tauri's JS
// binding convention (snake_case Rust → camelCase JS) so we pass camelCase keys.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  Category,
  NowContext,
  RoutineBlock,
  RoutineGroup,
  Settings,
  Task,
  TimelineItem,
} from "../types";

export const api = {
  // categories
  listCategories: () => invoke<Category[]>("list_categories"),
  createCategory: (name: string, colorHue: number, icon: string | null) =>
    invoke<Category>("create_category", { name, colorHue, icon }),
  deleteCategory: (id: string) => invoke<void>("delete_category", { id }),

  // routines
  listRoutines: (activeOnly: boolean) =>
    invoke<RoutineBlock[]>("list_routines", { activeOnly }),
  createRoutine: (r: {
    title: string;
    startMinute: number;
    durationMinute: number;
    weekdayMask: number;
    categoryId: string | null;
    effectiveFrom: string | null;
    effectiveUntil: string | null;
    notes: string | null;
  }) => invoke<RoutineBlock>("create_routine", r),
  updateRoutine: (
    id: string,
    patch: Partial<{
      title: string;
      startMinute: number;
      durationMinute: number;
      weekdayMask: number;
      categoryId: string | null;
      notes: string | null;
    }>,
  ) =>
    invoke<RoutineBlock>("update_routine", {
      id,
      ...patch,
      // explicit Option<Option> semantics: pass through as-is
    }),
  setRoutineActive: (id: string, active: boolean) =>
    invoke<RoutineBlock>("set_routine_active", { id, active }),
  deleteRoutine: (id: string) => invoke<void>("delete_routine", { id }),

  // timeline
  getTimeline: (date: string) =>
    invoke<TimelineItem[]>("get_timeline", { date }),
  getNowContext: () => invoke<NowContext>("get_now_context"),

  // tasks
  listBacklog: () => invoke<Task[]>("list_backlog"),
  createTask: (t: {
    date: string | null;
    title: string;
    categoryId: string | null;
    startMinute: number | null;
    durationMinute: number | null;
    notes: string | null;
  }) => invoke<Task>("create_task", t),
  updateTask: (
    id: string,
    patch: Partial<{
      title: string;
      date: string | null;
      startMinute: number | null;
      durationMinute: number | null;
      categoryId: string | null;
      notes: string | null;
    }>,
  ) => invoke<Task>("update_task", { id, ...patch }),
  setTaskDone: (id: string, done: boolean) =>
    invoke<Task>("set_task_done", { id, done }),
  setTaskSkipped: (id: string, skipped: boolean) =>
    invoke<Task>("set_task_skipped", { id, skipped }),
  deleteTask: (id: string) => invoke<void>("delete_task", { id }),

  // settings
  getSettings: () => invoke<Settings>("get_settings"),
  setSetting: (key: string, value: string) =>
    invoke<unknown>("set_setting", { key, value }),
  getDbPath: () => invoke<string>("get_db_path"),

  // onboarding
  setOnboardingDone: () => invoke<void>("set_onboarding_done"),
  isOnboardingDone: () => invoke<boolean>("is_onboarding_done"),

  // notifications
  requestNotificationPermission: () => invoke<boolean>("request_notification_permission"),
  isNotificationPermissionGranted: () => invoke<boolean>("is_notification_permission_granted"),
  openNotificationSettings: () => invoke<void>("open_notification_settings"),

  // drag-and-drop
  materializeIfVirtual: (id: string) => invoke<string>("materialize_if_virtual", { id }),

  // routine groups
  listRoutineGroups: () => invoke<RoutineGroup[]>("list_routine_groups"),
  createRoutineGroup: (name: string, icon: string | null) =>
    invoke<RoutineGroup>("create_routine_group", { name, icon }),
  updateRoutineGroup: (
    id: string,
    patch: { name?: string; icon?: string | null; sortOrder?: number },
  ) => invoke<RoutineGroup>("update_routine_group", { id, ...patch }),
  deleteRoutineGroup: (id: string) => invoke<void>("delete_routine_group", { id }),
  setRoutineGroupActive: (id: string, active: boolean) =>
    invoke<RoutineGroup>("set_routine_group_active", { id, active }),
};

/** Subscribe to the cross-process DB-changed event. Returns an unlisten fn. */
export function onDbChanged(cb: () => void): Promise<UnlistenFn> {
  return listen("oxiline://db-changed", () => cb());
}
export function onOpenPreferences(cb: () => void): Promise<UnlistenFn> {
  return listen("oxiline://open-preferences", () => cb());
}
export function onOpenQuickAdd(cb: () => void): Promise<UnlistenFn> {
  return listen("oxiline://open-quick-add", () => cb());
}
export function onNowUpdate(cb: (ctx: NowContext) => void): Promise<UnlistenFn> {
  return listen<NowContext>("oxiline://now", (e) => cb(e.payload));
}
