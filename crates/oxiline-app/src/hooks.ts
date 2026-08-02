import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo } from "react";
import { api } from "./lib/api";
import { useUi } from "./lib/store";
import type { Scope } from "./types";

export const qk = {
  timeline: (date: string) => ["timeline", date] as const,
  timelineRange: (from: string, to: string) => ["timeline-range", from, to] as const,
  categories: ["categories"] as const,
  routines: (activeOnly: boolean) => ["routines", activeOnly] as const,
  backlog: ["backlog"] as const,
  cardSuggestions: ["cardSuggestions"] as const,
  settings: ["settings"] as const,
  weekReport: ["weekReport"] as const,
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

export function useWeekReport() {
  return useQuery({ queryKey: qk.weekReport, queryFn: api.getWeekReport });
}

export function useTimeline(date: string) {
  return useQuery({ queryKey: qk.timeline(date), queryFn: () => api.getTimeline(date) });
}

export function useRoutines(activeOnly = false) {
  return useQuery({
    queryKey: qk.routines(activeOnly),
    queryFn: () => api.listRoutines(activeOnly),
  });
}

export function useBacklog() {
  return useQuery({ queryKey: qk.backlog, queryFn: api.listBacklog });
}

export function useSuggestCards(enabled = true) {
  return useQuery({
    queryKey: qk.cardSuggestions,
    queryFn: () => api.suggestCards(),
    enabled,
  });
}

export function useTimelineRange(from: string, to: string) {
  const dates = useMemo(() => {
    const out: string[] = [];
    const [y, m, d] = from.split("-").map(Number);
    let dt = new Date(y, m - 1, d);
    const [y2, m2, d2] = to.split("-").map(Number);
    const end = new Date(y2, m2 - 1, d2);
    while (dt <= end) {
      out.push(
        `${dt.getFullYear()}-${String(dt.getMonth() + 1).padStart(2, "0")}-${String(dt.getDate()).padStart(2, "0")}`,
      );
      dt = new Date(dt.getFullYear(), dt.getMonth(), dt.getDate() + 1);
    }
    return out;
  }, [from, to]);

  return useQuery({
    queryKey: qk.timelineRange(from, to),
    queryFn: async () => {
      const results = await Promise.all(
        dates.map(async (date) => ({ date, items: await api.getTimeline(date) })),
      );
      return results;
    },
  });
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

export function useCreateTask() {
  const inv = useInvalidate();
  return useMutation({
    mutationFn: (vars: Parameters<typeof api.createTask>[0]) => api.createTask(vars),
    onSuccess: () => inv(),
  });
}

export function useSetTaskDone() {
  const inv = useInvalidate();
  return useMutation({
    mutationFn: ({ id, done }: { id: string; done: boolean }) =>
      api.setTaskDone(id, done),
    onSuccess: () => inv(),
  });
}

export function useDeleteTask() {
  const inv = useInvalidate();
  return useMutation({
    mutationFn: (id: string) => api.deleteTask(id),
    onSuccess: () => inv(),
  });
}

export function useSetTaskSkipped() {
  const inv = useInvalidate();
  return useMutation({
    mutationFn: ({ id, skipped }: { id: string; skipped: boolean }) =>
      api.setTaskSkipped(id, skipped),
    onSuccess: () => inv(),
  });
}

export function useUpdateTask() {
  const inv = useInvalidate();
  return useMutation({
    mutationFn: (vars: { id: string } & Parameters<typeof api.updateTask>[1]) =>
      api.updateTask(vars.id, vars),
    onSuccess: () => inv(),
  });
}

export function useCreateRoutine() {
  const inv = useInvalidate();
  return useMutation({
    mutationFn: (vars: Parameters<typeof api.createRoutine>[0]) => api.createRoutine(vars),
    onSuccess: () => inv(),
  });
}

export function useUpdateRoutine() {
  const inv = useInvalidate();
  return useMutation({
    mutationFn: (vars: { id: string } & Parameters<typeof api.updateRoutine>[1]) =>
      api.updateRoutine(vars.id, vars),
    onSuccess: () => inv(),
  });
}

export function useSetRoutineActive() {
  const inv = useInvalidate();
  return useMutation({
    mutationFn: ({ id, active }: { id: string; active: boolean }) =>
      api.setRoutineActive(id, active),
    onSuccess: () => inv(),
  });
}

export function useDeleteRoutine() {
  const inv = useInvalidate();
  return useMutation({
    mutationFn: (id: string) => api.deleteRoutine(id),
    onSuccess: () => inv(),
  });
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
  const { date } = useUi();
  return useMutation({
    mutationFn: ({ key, value }: { key: string; value: string }) =>
      api.setSetting(key, value),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: qk.settings });
      qc.invalidateQueries({ queryKey: qk.timeline(date) });
    },
  });
}

export function useRoutineGroups() {
  return useQuery({
    queryKey: ["routine-groups"],
    queryFn: api.listRoutineGroups,
  });
}

export function useCreateRoutineGroup() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: { name: string; icon: string | null }) =>
      api.createRoutineGroup(input.name, input.icon),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["routine-groups"] }),
  });
}

export function useUpdateRoutineGroup() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: {
      id: string;
      name?: string;
      icon?: string | null;
      sortOrder?: number;
    }) => api.updateRoutineGroup(input.id, input),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["routine-groups"] }),
  });
}

export function useDeleteRoutineGroup() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.deleteRoutineGroup(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["routine-groups"] }),
  });
}

export function useSetRoutineGroupActive() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: { id: string; active: boolean }) =>
      api.setRoutineGroupActive(input.id, input.active),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["routine-groups"] });
      qc.invalidateQueries({ queryKey: ["routines"] });
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
