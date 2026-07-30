import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo } from "react";
import { api } from "./lib/api";
import { useUi } from "./lib/store";

export const qk = {
  timeline: (date: string) => ["timeline", date] as const,
  timelineRange: (from: string, to: string) => ["timeline-range", from, to] as const,
  categories: ["categories"] as const,
  routines: (activeOnly: boolean) => ["routines", activeOnly] as const,
  backlog: ["backlog"] as const,
  now: ["now"] as const,
  settings: ["settings"] as const,
};

export function useCategories() {
  return useQuery({ queryKey: qk.categories, queryFn: api.listCategories });
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

export function useNowContext() {
  return useQuery({ queryKey: qk.now, queryFn: api.getNowContext });
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
