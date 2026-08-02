import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo } from "react";
import { api } from "./lib/api";
import { useUi } from "./lib/store";
import type { Scope } from "./types";

export const qk = {
  categories: ["categories"] as const,
  settings: ["settings"] as const,
  slots: (date: string) => ["slots", date] as const,
  dayRecords: (date: string) => ["day-records", date] as const,
  compliance: (scope: Scope) => ["compliance", scope] as const,
  recordState: ["recordState"] as const,
  activities: (activeOnly: boolean) => ["activities", activeOnly] as const,
  plans: (recurringOnly: boolean) => ["plans", recurringOnly] as const,
};

export function useCategories() {
  return useQuery({ queryKey: qk.categories, queryFn: api.listCategories });
}

export function useSettings() {
  return useQuery({ queryKey: qk.settings, queryFn: api.getSettings });
}

function useInvalidate() {
  const qc = useQueryClient();
  return () => {
    qc.invalidateQueries();
  };
}

export function useCreateCategory() {
  const inv = useInvalidate();
  return useMutation({
    mutationFn: (vars: { name: string; hue: number; icon: string | null }) =>
      api.createCategory(vars.name, vars.hue, vars.icon),
    onSuccess: () => inv(),
  });
}

export function useDeleteCategory() {
  const inv = useInvalidate();
  return useMutation({
    mutationFn: (id: string) => api.deleteCategory(id),
    onSuccess: () => inv(),
  });
}

export function useSetSetting() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ key, value }: { key: string; value: string }) =>
      api.setSetting(key, value),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: qk.settings });
    },
  });
}

// ---- recording layer (Plan 2) --------------------------------------------

export function useSlots(date: string) {
  return useQuery({ queryKey: qk.slots(date), queryFn: () => api.getSlotsForDate(date) });
}

/** Records near `date` (±1 day window) so the component can filter by LOCAL
 * date — records store UTC instants but the timetable is local wall-clock. */
export function useDayRecords(date: string) {
  const win = windowFor(date);
  return useQuery({
    queryKey: qk.dayRecords(date),
 queryFn: () => api.listRecords(null, win.from, win.to),
  });
}

/** Records across a date range — one query covering [start of `from`, end of
 * `to`]; callers group by local date for per-day markers. */
export function useRecordsRange(from: string, to: string) {
  const fromWin = windowFor(from);
  const toWin = windowFor(to);
  return useQuery({
    queryKey: ["records-range", from, to],
    queryFn: () => api.listRecords(null, fromWin.from, toWin.to),
  });
}

export function useCompliance(scope: Scope) {
  return useQuery({ queryKey: qk.compliance(scope), queryFn: () => api.getCompliance(scope) });
}

export function useRecordState() {
  return useQuery({ queryKey: qk.recordState, queryFn: api.currentRecordState });
}

export function useActivities(activeOnly = true) {
  return useQuery({ queryKey: qk.activities(activeOnly), queryFn: () => api.listActivities(activeOnly) });
}

export function useStartRecord() {
  const inv = useInvalidate();
  return useMutation({
    mutationFn: (activityId: string) => api.startRecord(activityId),
    onSuccess: () => inv(),
  });
}

export function useStopRecord() {
  const inv = useInvalidate();
  return useMutation({ mutationFn: () => api.stopRecord(), onSuccess: () => inv() });
}

export function useCreateActivity() {
  const inv = useInvalidate();
  return useMutation({
    mutationFn: (input: Parameters<typeof api.createActivity>[0]) => api.createActivity(input),
    onSuccess: () => inv(),
  });
}

export function useCreatePlan() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: Parameters<typeof api.createPlan>[0]) => api.createPlan(input),
    onSuccess: (_d, _vars) => {
      qc.invalidateQueries({ queryKey: ["slots"] });
      qc.invalidateQueries({ queryKey: ["plans"] });
    },
  });
}

export function useAddPlanOptions() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (args: { planId: string; activityIds: string[] }) =>
      api.addPlanOptions(args.planId, args.activityIds),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["slots"] });
      qc.invalidateQueries({ queryKey: ["plans"] });
    },
  });
}

export function useResizePlan() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (args: { planId: string; durationMinute: number }) =>
      api.resizePlan(args.planId, args.durationMinute),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["slots"] });
      qc.invalidateQueries({ queryKey: ["plans"] });
    },
  });
}

/** ISO window [date-1 00:00Z, date+1 23:59Z] for a local `date` (YYYY-MM-DD). */
function windowFor(date: string): { from: string; to: string } {
  const [y, m, d] = date.split("-").map(Number);

  const prev = new Date(y, m - 1, d - 1);
  const next = new Date(y, m - 1, d + 1);
  const f = (x: Date) =>
    `${x.getFullYear()}-${String(x.getMonth() + 1).padStart(2, "0")}-${String(x.getDate()).padStart(2, "0")}`;

  return { from: `${f(prev)}T00:00:00Z`, to: `${f(next)}T23:59:59Z` };
}
