import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "./lib/api";
import { useUi } from "./lib/store";

export const qk = {
  timeline: (date: string) => ["timeline", date] as const,
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
