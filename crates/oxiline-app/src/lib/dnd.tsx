import {
  DndContext,
  DragEndEvent,
  PointerSensor,
  rectIntersection,
  useSensor,
  type CollisionDetection,
} from "@dnd-kit/core";
import type { ReactNode } from "react";
import { useAddPlanOptions, useCreatePlan } from "../hooks";
import { useUi } from "./store";

export const SNAP_MINUTES = 5;

/** Shared DnD context provider for the recording timeline. */
export function DndProvider({ children }: { children: ReactNode }) {
  const createPlan = useCreatePlan();
  const addOptions = useAddPlanOptions();
  const clearActivitySelection = useUi((state) => state.clearActivitySelection);

  const pointerSensor = useSensor(PointerSensor, {
    activationConstraint: { distance: 8 },
  });

  // Plan cards are droppables nested INSIDE the timeline droppable.
  // plain rectIntersection ranks by intersection area, so the large timeline
  // would always win over a small card — making drop-to-merge impossible.
  // Prefer the plan-slot when present so the card "captures" the pointer.
  // Scoped to activity draggables only: backlog/block must keep resolving to
  // the timeline droppable so their update path keeps a valid `date`.
  const nestedCollision: CollisionDetection = (args) => {
    if (args.active.data.current?.kind !== "activity") return rectIntersection(args);
    const collisions = rectIntersection(args);
    const planSlot = collisions.find(
      (c) =>
        (c.data as { droppableContainer?: { data?: { current?: { kind?: string } } } })
          ?.droppableContainer?.data?.current?.kind === "plan-slot",
    );
    return planSlot ? [planSlot] : collisions;
  };

  async function handleDragEnd(event: DragEndEvent) {
    const { active, over } = event;
    if (!over) return;
    const data = active.data.current;
    if (!data) return;
    const overData = over.data.current as
      | { kind: string; date?: string; pxPerMin?: number; dayStartMin?: number; planId?: string }
      | undefined;
    if (!overData) return;
    const acceptsPlanSlot = data.kind === "activity";
    if (overData.kind !== "timeline-slot" && !(acceptsPlanSlot && overData.kind === "plan-slot")) return;
    const dropMinute = computeDropMinute(event, overData);

    if (data.kind === "activity") {
      const activityIds = (data as { activityIds: string[] }).activityIds;
      if (overData.kind === "plan-slot") {
        const planId = (overData as { planId: string }).planId;
        addOptions.mutate({ planId, activityIds });
      } else {
        createPlan.mutate({
          date: overData.date as string,
          start_minute: dropMinute,
          duration_minute: 60,
          weekday_mask: 0,
          title: null,
          activity_ids: activityIds,
        });
      }
      clearActivitySelection();
    }
  }

  return (
    <DndContext sensors={[pointerSensor]} collisionDetection={nestedCollision} onDragEnd={handleDragEnd}>
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
