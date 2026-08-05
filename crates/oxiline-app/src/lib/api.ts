// Typed wrappers over the Rust Tauri commands. Command names match the
// `#[tauri::command]` fn names exactly; args are camelCased by Tauri's JS
// binding convention (snake_case Rust → camelCase JS) so we pass camelCase keys.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  Activity,
  ActivityInput,
  ActivityRecord,
  Category,
  Compliance,
  Plan,
  PlanInput,
  PlanOption,
  PlanSlot,
  RecordState,
  Scope,
  Settings,
} from "../types";

export const api = {
  // categories
  listCategories: () => invoke<Category[]>("list_categories"),
  createCategory: (name: string, colorHue: number, icon: string | null) =>
    invoke<Category>("create_category", { name, colorHue, icon }),
  deleteCategory: (id: string) => invoke<void>("delete_category", { id }),




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



  // recording — activities
  listActivities: (activeOnly: boolean) =>
    invoke<Activity[]>("list_activities", { activeOnly }),
  createActivity: (input: ActivityInput) =>
    invoke<Activity>("create_activity", { input }),
  resolveActivity: (query: string) =>
    invoke<Activity>("resolve_activity", { query }),
  updateActivity: (id: string, input: ActivityInput) =>
    invoke<Activity>("update_activity", { id, input }),
  deleteActivity: (id: string, force: boolean) =>
    invoke<void>("delete_activity", { id, force }),
  // recording — records
  startRecord: (activityId: string) =>
    invoke<RecordState>("start_record", { activityId }),
  stopRecord: () => invoke<RecordState>("stop_record"),
  currentRecordState: () => invoke<RecordState>("current_record_state"),
  listRecords: (activityId: string | null, from: string, to: string) =>
    invoke<ActivityRecord[]>("list_records", { activityId, from, to }),
  getCompliance: (scope: Scope) => invoke<Compliance[]>("get_compliance", { scope }),
  // recording — plans
  listPlans: (recurringOnly: boolean) =>
    invoke<Plan[]>("list_plans", { recurringOnly }),
  createPlan: (input: PlanInput) => invoke<Plan>("create_plan", { input }),
  getSlotsForDate: (date: string) =>
    invoke<PlanSlot[]>("get_slots_for_date", { date }),
  updatePlan: (id: string, input: PlanInput) =>
    invoke<Plan>("update_plan", { id, input }),
  deletePlan: (id: string) => invoke<void>("delete_plan", { id }),
  addPlanOptions: (planId: string, activityIds: string[]) =>
    invoke<PlanOption[]>("add_plan_options", { planId, activityIds }),
  resizePlan: (planId: string, durationMinute: number) =>
    invoke<Plan>("resize_plan", { planId, durationMinute }),

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
export function onQuickRecord(cb: () => void): Promise<UnlistenFn> {
  return listen("oxiline://quick-record", () => cb());
}
