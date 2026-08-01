import {
  DndContext,
  DragEndEvent,
  PointerSensor,
  useSensor,
} from "@dnd-kit/core";
import type { ReactNode } from "react";
import { useCreatePlan, useUpdateTask } from "../hooks";
import { api } from "./api";

export const SNAP_MINUTES = 5;

/** Shared DnD context provider for DayTimeline + BacklogView. */
export function DndProvider({ children }: { children: ReactNode }) {
  const upd = useUpdateTask();
  const createPlan = useCreatePlan();

  const pointerSensor = useSensor(PointerSensor, {
    activationConstraint: { distance: 8 },
  });

  async function handleDragEnd(event: DragEndEvent) {
    const { active, over } = event;
    if (!over) return;
    const data = active.data.current;
    if (!data) return;
    const overData = over.data.current as
      | { kind: string; date?: string; pxPerMin?: number; dayStartMin?: number }
      | undefined;
    if (!overData || overData.kind !== "timeline-slot") return;

    const dropMinute = computeDropMinute(event, overData);

    if (data.kind === "backlog") {
      // Backlog → timeline: materialise if virtual, schedule on drop zone.
      let realId = (data.task as { id: string }).id;
      if (realId.startsWith("virtual:")) {
        realId = await api.materializeIfVirtual(realId);
      }
      upd.mutate({
        id: realId,
        date: overData.date as string,
        startMinute: dropMinute,
        durationMinute: 30,
      });
    } else if (data.kind === "block") {
      // Block move.
      const item = data.item as { id: string };
      let realId = item.id;
      if (realId.startsWith("virtual:")) {
        realId = await api.materializeIfVirtual(realId);
      }
      upd.mutate({
        id: realId,
        startMinute: dropMinute,
      });
    } else if (data.kind === "activity") {
      // Library card → timetable: create a one-shot plan at the drop minute.
      createPlan.mutate({
        date: overData.date as string,
        start_minute: dropMinute,
        duration_minute: 60,
        weekday_mask: 0,
        title: null,
        activity_ids: [(data as { activityId: string }).activityId],
      });
    }
  }

  return (
    <DndContext sensors={[pointerSensor]} onDragEnd={handleDragEnd}>
      {children}
    </DndContext>
  );
}

function computeDropMinute(
  event: DragEndEvent,
  overData: { pxPerMin?: number; dayStartMin?: number },
): number {
  const overRect = event.over?.rect;
  const activeRect =
    event.active.rect.current.translated ?? event.active.rect.current.initial;
  if (!overRect || !activeRect) return 0;
  const dropY = activeRect.top - overRect.top;
  const minute = Math.round(
    dropY / (overData.pxPerMin ?? 1) + (overData.dayStartMin ?? 0),
  );
  return snapMinute(Math.max(0, Math.min(1439, minute)));
}

export function snapMinute(m: number): number {
  return Math.round(m / SNAP_MINUTES) * SNAP_MINUTES;
}
