# Task 5 Report — OR-plan multi-select drag + drop-to-merge

## Status
Complete. Sidebar multi-select model, custom nested-droppable collision detection, and per-card `useDroppable` PlanCard extracted and wired. Typecheck + production build both pass clean.
## Commits
- `9655c7f feat(app): OR-plan multi-select drag + drop-to-merge (Task 5)`

## Build
`cd crates/oxiline-app && npx tsc -b && npx vite build` → **PASS** (tsc clean, vite built 1684 modules, dist/hud.html 1.35 kB, dist/index.html 1.81 kB, main bundle 167 kB).

## Collision-resolution reasoning
Two droppables overlap when the pointer is over a plan card:
1. The card itself → droppable id `plan-<plan_id>`, `data.kind = "plan-slot"`.
2. The timeline lane enclosing it → droppable id `record-timeline`, `data.kind = "timeline-slot"`.

`rectIntersection` ranks collisions by **intersection area**, descending. The timeline lane is the full day (typically ~1200×960px), the card is a small slice (~220×64px). Both are intersected. By area, timeline always wins. That made the brief's plain `rectIntersection` choice a silent bug: `over` would resolve to the timeline, `overData.kind === "timeline-slot"`, and the merge branch was unreachable.

The custom `nestedCollision` overrides this: after `rectIntersection`, it `find`s the first entry whose `droppableContainer.data.current.kind === "plan-slot"` (verified against `@dnd-kit/core`'s `CollisionDescriptor` shape) and returns `[planSlot]` if present, otherwise the original list. So when the pointer is over a card, the card is the sole collision → `over` resolves to the plan-slot droppable → `overData.kind === "plan-slot"` → the new merge branch fires.

Additional guard fix: `handleDragEnd`'s early-return was `overData.kind !== "timeline-slot"` only. That guard would have returned before the activity branch even reached the new `plan-slot` sub-branch. Relaxed to `!(kind === "timeline-slot" || kind === "plan-slot")` so plan-slot overData is admitted. `computeDropMinute` then runs against plan-slot's missing `pxPerMin`/`dayStartMin` and falls back via the `??` defaults — harmless because the merge branch never reads `dropMinute`.

Drop-merge outcome: `overData.kind === "plan-slot"` → for each selected activityId, `addOption.mutate({ planId, activityId })`. Each option is added as a new OR alternative on the existing plan (the picked → executed rule still picks whichever option is the resolved_by activity_id; multiple un-picked options coexist as OR alternatives).

## Multi-select payload
`DraggableActivity` computes `ids = isSelected && selectedSet.size > 0 ? [...selectedSet] : [activity.id]` and stores it as `data.activityIds` (replacing the prior single `activityId`). `ActivityLibrary` owns the `Set<string>` state; `onClick={onSelect(activity.id, e.metaKey || e.ctrlKey)}` toggles additively under meta/ctrl, otherwise sets a single-id set. The list container's `onPointerDown` clears the set when the user clicks empty padding. The selection ring (`ring-2 ring-interactive-primary`) and a small count badge when `selectedSet.size > 1` give visual feedback. Drag payload carries the full set regardless of which card was picked up.

## Files changed
- `crates/oxiline-app/src/lib/dnd.tsx` (+54 −15): `rectIntersection` + `CollisionDetection` import, `useAddPlanOption` import, `nestedCollision` (scoped to activity draggables), `acceptsPlanSlot` kind-branched guard, activity-branch merge sub-branch.
- `crates/oxiline-app/src/styles.css`: not modified — `ring-interactive-primary` is already a working utility class (used by `Header.tsx` for date-popover cells).

## Concerns
- **Backlog/block drops on plan cards are unaffected**. `nestedCollision` is now scoped to activity draggables only via `args.active.data.current?.kind === "activity"` short-circuit. Backlog and block draggables fall through to plain `rectIntersection`, so they continue resolving to the timeline droppable — their `useUpdateTask` calls still receive a valid `date`. The guard in `handleDragEnd` admits both kinds but only for activity draggables via the `acceptsPlanSlot` predicate; backlog/block exit at the original `timeline-slot`-only check.
- **`overData` widening cast**: added `planId?: string` to the `overData` type union in `handleDragEnd`. The `c.data` access inside `nestedCollision` still uses an inline cast because `@dnd-kit/core` types `CollisionDescriptor.data` as `Record<string, unknown>` (verified) — runtime check via nested `?.` chain, not silently trusting.
- **DnD is not unit-tested** (vitest is pure-logic). Verification was limited to typecheck + production build; functional smoke (select 2 activities → drag → OR plan; drop 1 onto card → option added) was reasoned through collision-detection flow rather than executed in a browser because no dev server was started. A human smoke run is recommended before merge.
- **Drop on plan card without selection set** still works: `ids = [activity.id]` → single `addOption.mutate`. Same flow for createPlan.
- **`computeDropMinute` on plan-slot overData**: when merging onto a card, `dropMinute` is computed against plan-slot's missing `pxPerMin`/`dayStartMin` and falls back via the `?? 1` / `?? 0` defaults. The result is unused (merge branch only reads `planId`), so this is harmless but wasteful.


## Fix — clear activity multi-select after drop

Lifted the activity selection from `ActivityLibrary` component state into the shared zustand `useUi` store. The store now owns `selectedActivityIds`, `toggleActivitySelect(id, additive)`, and `clearActivitySelection()`. `ActivityLibrary` uses those actions for normal/additive selection and empty-area clearing, while each `DraggableActivity` reads the shared ids for its ring, count badge, and drag payload.

`DndProvider.handleDragEnd` now clears the shared selection only at the end of the successfully routed `data.kind === "activity"` branch, after either all `addPlanOption` mutations have been started or the `createPlan` mutation has been started. It does not clear at drag-start, on cancellation, or on an invalid drop, so the payload is preserved until a valid activity drop is handled.

### Fix files
- `crates/oxiline-app/src/lib/store.ts`: shared activity-selection state and actions.
- `crates/oxiline-app/src/components/Sidebar.tsx`: store-backed selection, empty-area clear, and store-backed drag payload/visual state.
- `crates/oxiline-app/src/lib/dnd.tsx`: post-drop activity-selection clear.
- `crates/oxiline-app/src/lib/__tests__/store.test.ts`: selection toggle, replacement, and clear regression coverage.

### Behavioral reasoning
- **After an activity drop:** the activity branch invokes `clearActivitySelection()` after its create-or-merge mutation path, removing the rings/count badges immediately and preventing the completed group from remaining armed.
- **Subsequent unselected single drag:** with `selectedActivityIds` empty, `isSelected` is false and `DraggableActivity` computes `activityIds` as `[activity.id]`, so only the card being dragged is carried.
- **Backlog/block drops and single createPlan:** backlog and block branches are unchanged and never clear activity selection. A single activity still produces a one-element `activity_ids` array and follows the same `createPlan.mutate` path; the only new behavior is clearing selection afterward.
