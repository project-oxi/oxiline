# Actual-Record Editing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let the user drag-to-move and delete *actual-record* blocks on the timeline (the `ActualLane`), mirroring the move/delete UX already shipped for plan blocks (`PlanCard`).

**Architecture:** The core already has `record::edit_record` and `record::delete_record` — they are only missing Tauri command exposure. Task 1 wires them as commands; Task 2 adds the typed API + React Query hooks; Tasks 3–4 port the PlanCard pointer-drag + hover-delete pattern onto `ActualLane`; Task 5 guards the live (open) session and runs the full gate.

**Tech Stack:** Rust + Tauri v2 + specta (`oxiline-core`, `oxiline-app/src-tauri`); React 19 + TypeScript + TanStack Query + Tailwind v4 (`oxiline-app/src`). Time is stored as UTC ISO instants; the timeline renders LOCAL minute-of-day via `lib/record-time.ts`'s `isoLocal`.

## Global Constraints

- **Build gates:** after every task — `cargo build --workspace` (and `cargo test --workspace` for backend tasks) + `bun run build` + `bun run test` (vitest) must be green. Clippy is `-D warnings`.
- **Naming:** Rust command args are `snake_case`; the TS binding is `camelCase` (Tauri convention). Domain structs stay `snake_case` (serde). Mirrored by hand in `src/types.ts`.
- **Neutral copy only:** never failure language (no 실패/깜빡/놓침). UI copy is Korean-first; English via `i18n.language`.
- **Time invariant:** `ended_at > started_at` (enforced by core `edit_record`). Records are UTC ISO; convert via `lib/record-time.ts`.
- **No `any`:** use `unknown` + type guards; schema-validated boundaries.
- **Token system:** consume `--color-*` semantic tokens (see `doc/06-design-system.md`); no raw hex in components.

## Context (what the previous session shipped)

The immediately preceding session landed, on `main`:
- Single-row header + oxide strip, weekday chips with micro oxide bars, record hero pill, calendar body-portal (`Header.tsx`); inline draft editor (`RecordTimeline.tsx` DraftBlock).
- **Plan-block move + delete** on `PlanCard` (pointer-drag → `update_plan`; hover × → `delete_plan`), plus `PlanSlot.weekday_mask` carried through core so moves preserve recurring masks.
- `doc/09-ui-redesign.md` rewritten as the canonical UI/interaction spec.

This plan is the symmetric completion for *actual records*. Read `doc/09-ui-redesign.md` §9.4 (timeline) and §9.4.2 (PlanCard move/delete) for the pattern to mirror. The reference implementation of the drag/delete gesture lives in `RecordTimeline.tsx` `PlanCard`.

**Key evidence already in tree:**
- Core: `crates/oxiline-core/src/record.rs` — `edit_record(conn, id, started_at: Option<String>, ended_at: Option<String>) -> Result<Record>` (None preserves; validates `ended_at > started_at`), `delete_record(conn, id) -> Result<()>`.
- Command registry: `crates/oxiline-app/src-tauri/src/lib.rs` lines ~40-46 — the specta builder list (`commands::update_plan, delete_plan, resize_plan, …`) is where new commands are registered; `.invoke_handler(specta.invoke_handler())` auto-binds them.
- Existing record commands: `start_record`/`stop_record`/`current_record_state`/`list_records` in `commands.rs` (~176-204) — copy their shape.
- Frontend mirror of the move/delete pattern: `RecordTimeline.tsx` `PlanCard` (pointer capture, `snapMinute`, `useMovePlan`/`useDeletePlan` in `hooks.ts`).

---

### Task 1: Expose `edit_record` and `delete_record` as Tauri commands

**Files:**
- Modify: `crates/oxiline-app/src-tauri/src/commands.rs` (add two fns next to `resize_plan`)
- Modify: `crates/oxiline-app/src-tauri/src/lib.rs` (~line 45, the specta builder list)
- Test: `cargo test --workspace` (existing core tests cover the logic; this task only verifies wiring compiles + commands are registered)

**Interfaces:**
- Consumes: `record::edit_record`, `record::delete_record`, `state::AppState`, `map_err` (all already imported in `commands.rs`).
- Produces (for downstream tasks — exact signatures):
  - `pub fn edit_record(state: State<AppState>, id: String, started_at: Option<String>, ended_at: Option<String>) -> Result<Record, String>`
  - `pub fn delete_record(state: State<AppState>, id: String) -> Result<(), String>`
  - Both annotated `#[tauri::command]` + `#[specta::specta]`, registered in the specta builder so `.invoke_handler(specta.invoke_handler())` binds them.

- [ ] **Step 1: Add the two command functions**

In `commands.rs`, after `resize_plan`, add (mirroring `update_plan`'s shape):

```rust
#[tauri::command]
#[specta::specta]
pub fn edit_record(
    state: State<AppState>,
    id: String,
    started_at: Option<String>,
    ended_at: Option<String>,
) -> Result<Record, String> {
    record::edit_record(&state.conn(), &id, started_at, ended_at).map_err(map_err)
}

#[tauri::command]
#[specta::specta]
pub fn delete_record(state: State<AppState>, id: String) -> Result<(), String> {
    record::delete_record(&state.conn(), &id).map_err(map_err)
}
```

Confirm `Record` is the model type already imported (it is — used by `list_records`).

- [ ] **Step 2: Register both in the specta builder**

In `lib.rs` (~line 45, inside the `...generate_handler!`-equivalent builder list that currently holds `commands::resize_plan,`), append:

```rust
        commands::edit_record,
        commands::delete_record,
```

right after `commands::resize_plan,`.

- [ ] **Step 3: Build the workspace**

Run: `cargo build --workspace`
Expected: green (the core fns already exist; this is pure wiring). If `Record` import is missing in commands.rs, add it to the existing `use oxiline_core::model::{...}`.

- [ ] **Step 4: Run workspace tests**

Run: `cargo test --workspace`
Expected: all green (no new test here — core `edit_record`/`delete_record` are already covered by `record.rs` tests; this task is exposure only).

- [ ] **Step 5: Commit**

```bash
git add crates/oxiline-app/src-tauri/src/commands.rs crates/oxiline-app/src-tauri/src/lib.rs
git commit -m "feat(record): expose edit_record and delete_record as tauri commands"
```

---

### Task 2: Typed API + React Query hooks

**Files:**
- Modify: `crates/oxiline-app/src/lib/api.ts` (add two methods)
- Modify: `crates/oxiline-app/src/hooks.ts` (add `useEditRecord`, `useDeleteRecord`)

**Interfaces:**
- Consumes: `invoke` from `@tauri-apps/api/core`; `ActivityRecord` from `../types`; the invalidation key convention (`["day-records"]`, `["records"]`, `["compliance"]`, `["recordState"]`, `["slots"]`).
- Produces (for Tasks 3–4):
  - `api.editRecord(id: string, startedAt?: string | null, endedAt?: string | null)` → `invoke<ActivityRecord>("edit_record", { id, startedAt, endedAt })`. NOTE: `undefined` must serialize to absent/`null`; core treats `None` as preserve. Pass `null` explicitly when you mean "preserve", and a real ISO string when you mean "set".
  - `api.deleteRecord(id: string)` → `invoke<void>("delete_record", { id })`.
  - `useEditRecord()` → `useMutation({ mutationFn: (args: { recordId: string; startedAt?: string | null; endedAt?: string | null }) => api.editRecord(args.recordId, args.startedAt, args.endedAt), onSuccess: invalidateAllRecordDerived })`.
  - `useDeleteRecord()` → `useMutation({ mutationFn: (recordId: string) => api.deleteRecord(recordId), onSuccess: invalidateAllRecordDerived })`.
  - `invalidateAllRecordDerived`: invalidate `["day-records"]`, `["records"]`, `["compliance"]`, `["recordState"]`, AND `["slots"]` (a moved/deleted record can change plan resolution — `resolved_by`).

- [ ] **Step 1: Add the API methods**

In `api.ts`, in the `// recording — records` block after `listRecords`:

```ts
  editRecord: (id: string, startedAt?: string | null, endedAt?: string | null) =>
    invoke<ActivityRecord>("edit_record", { id, startedAt, endedAt }),
  deleteRecord: (id: string) => invoke<void>("delete_record", { id }),
```

- [ ] **Step 2: Add the hooks**

In `hooks.ts` after `useDeletePlan`:

```ts
/** Move or resize an actual record. `null` for a timestamp preserves it. */
export function useEditRecord() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (args: {
      recordId: string;
      startedAt?: string | null;
      endedAt?: string | null;
    }) => api.editRecord(args.recordId, args.startedAt, args.endedAt),
    onSuccess: () => invalidateRecordDerived(qc),
  });
}

export function useDeleteRecord() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (recordId: string) => api.deleteRecord(recordId),
    onSuccess: () => invalidateRecordDerived(qc),
  });
}

/** A record change ripples into day records, compliance, the live state, AND
 *  plan resolution (a moved/deleted record may resolve a different plan). */
function invalidateRecordDerived(qc: ReturnType<typeof useQueryClient>) {
  qc.invalidateQueries({ queryKey: ["day-records"] });
  qc.invalidateQueries({ queryKey: ["records"] });
  qc.invalidateQueries({ queryKey: ["compliance"] });
  qc.invalidateQueries({ queryKey: ["recordState"] });
  qc.invalidateQueries({ queryKey: ["slots"] });
}
```

- [ ] **Step 3: Build + typecheck**

Run: `cd crates/oxiline-app && bun run build`
Expected: green (tsc + vite). If `ReturnType<typeof useQueryClient>` is awkward, inline the five `qc.invalidateQueries` calls in each hook instead.

- [ ] **Step 4: Commit**

```bash
git add crates/oxiline-app/src/lib/api.ts crates/oxiline-app/src/hooks.ts
git commit -m "feat(record): add editRecord/deleteRecord api + hooks"
```

---

### Task 3: Drag-to-move actual-record blocks

**Files:**
- Modify: `crates/oxiline-app/src/components/RecordTimeline.tsx` — `ActualLane` (currently ~lines 393-446) and its render call site in `RecordTimeline` (passes `records`, `hueById`, `nameById`, `dayStartMin`, `divider`).

**Interfaces:**
- Consumes: `useEditRecord` (Task 2); `snapMinute`, `PX_PER_MIN` (module-local const = `64/60`); `isoLocal` (module-local), the record's `started_at`/`ended_at` (UTC ISO strings); `ActivityRecord` type.
- Produces: a movable actual block. Live (open) records (`ended_at === null`) are **not** movable — skip them (Task 5 may add an affordance, but for now render them non-interactive for move).

**Time math (critical):** the timeline shows LOCAL minute-of-day; records are UTC ISO. To move by `deltaMin`:
1. Parse `r.started_at` and `r.ended_at` to `Date` (`new Date(iso)`).
2. `new Date(start.getTime() + deltaMin * 60000).toISOString()` for both — the whole block shifts by the same delta, preserving duration and the UTC offset. This is correct because the delta is in absolute time, not wall-clock.
3. Snap `deltaMin` to `SNAP_MINUTES` increments (5 min) before computing — snap the *delta*, not the absolute, so the block keeps its exact length.
4. Clamp so the block stays inside `[dayStartMin, dayStartMin + totalMin]`: clamp the *start* local-minute (`isoLocal(newStartedAt).minute`) into that window, recompute delta from the clamped start.

- [ ] **Step 1: Pass `totalMin` to `ActualLane`**

At the render call site, add `totalMin={totalMin}` (RecordTimeline already computes `totalMin`). Update `ActualLane`'s props destructure + type to include `totalMin: number`.

- [ ] **Step 2: Add drag state + handler in `ActualLane`**

In the block render (`records.map(({ r, start }) => ...)`), for non-live records add:
- a `useState<number | null>(null)` for `dragDeltaMin` (the live delta during drag) — since `ActualLane` maps many records, hoist the drag state to `ActualLane` as a single `dragId` + `dragDeltaMin` pair (one block drags at a time), OR render each block via a small `ActualBlock` subcomponent that owns its own state. **Prefer a subcomponent** (`ActualBlock`) to keep state local — mirrors how `PlanCard` owns its own `dragStart`.

Sketch the subcomponent (full code in Step 3):

```tsx
function ActualBlock({
  r, start, end, live, name, hue, dayStartMin, totalMin,
}: { r: ActivityRecord; start: number; end: number; live: boolean;
     name: string; hue: string; dayStartMin: number; totalMin: number }) {
  const edit = useEditRecord();
  const [dragDelta, setDragDelta] = useState(0);
  const dragging = dragDelta !== 0;
  const liveTop = (start - dayStartMin + dragDelta) * PX_PER_MIN;
  const height = Math.max(16, (end - start) * PX_PER_MIN);
  return (
    <div
      className={`absolute left-1 right-1 overflow-hidden rounded-md p-1.5 text-[11px] transition-shadow ${
        live ? "" : dragging ? "z-30 cursor-grabbing shadow-[var(--shadow-lg)]" : "cursor-grab hover:shadow-[var(--shadow-md)]"
      }`}
      style={{
        top: liveTop,
        height,
        background: `color-mix(in oklch, ${hue} 22%, var(--color-surface-raised))`,
        borderLeft: `3px solid ${hue}`,
      }}
      onPointerDown={(e) => {
        if (live || e.button !== 0) return;
        const startY = e.clientY;
        const ref = { current: 0 };
        (e.currentTarget as Element).setPointerCapture(e.pointerId);
        const onMove = (ev: PointerEvent) => {
          const raw = (ev.clientY - startY) / PX_PER_MIN;
          const snapped = snapMinute(raw);
          // clamp: keep start inside the day window
          const newStartMin = start + snapped;
          const clamped = Math.min(dayStartMin + totalMin - (end - start),
                                    Math.max(dayStartMin, newStartMin));
          ref.current = clamped - start;
          setDragDelta(ref.current);
        };
        const onUp = () => {
          window.removeEventListener("pointermove", onMove);
          window.removeEventListener("pointerup", onUp);
          window.removeEventListener("pointercancel", onUp);
          setDragDelta(0);
          if (ref.current !== 0) {
            const ns = new Date(new Date(r.started_at).getTime() + ref.current * 60000).toISOString();
            const ne = r.ended_at
              ? new Date(new Date(r.ended_at).getTime() + ref.current * 60000).toISOString()
              : null;
            edit.mutate({ recordId: r.id, startedAt: ns, endedAt: ne });
          }
        };
        window.addEventListener("pointermove", onMove);
        window.addEventListener("pointerup", onUp);
        window.addEventListener("pointercancel", onUp);
      }}
    >
      <div className="flex items-center gap-1 font-medium">
        {live && <span className="inline-block h-1.5 w-1.5 animate-pulse rounded-full bg-status-error" />}
        <span className="truncate">{name}</span>
      </div>
      <div className="text-text-subtle">{hhmm(start + dragDelta)}–{live ? "" : hhmm(end + dragDelta)}</div>
    </div>
  );
}
```

Then `ActualLane`'s `.map` becomes `<ActualBlock key={r.id} r={r} start={start} end={end} live={live} name={nameById.get(r.activity_id) ?? r.activity_id} hue={hueVar(hueById.get(r.activity_id) ?? null)} dayStartMin={dayStartMin} totalMin={totalMin} />`.

- [ ] **Step 3: Write the implementation** (apply the sketch above; import `useEditRecord` + `useState` if not already).

- [ ] **Step 4: Build + vitest**

Run: `cd crates/oxiline-app && bun run build && bun run test`
Expected: green. (The existing `now-next.test.ts` etc. don't touch `ActualLane`; if a snapshot test does, update it.)

- [ ] **Step 5: Commit**

```bash
git add crates/oxiline-app/src/components/RecordTimeline.tsx
git commit -m "feat(record): drag-to-move actual-record blocks on the timeline"
```

---

### Task 4: Hover-delete actual-record blocks

**Files:**
- Modify: `crates/oxiline-app/src/components/RecordTimeline.tsx` — `ActualBlock` (from Task 3).

**Interfaces:**
- Consumes: `useDeleteRecord` (Task 2); `X` from `lucide-react` (already imported).
- Produces: a hover `×` button top-right of each non-live block; click → `deleteRecord(r.id)`.

- [ ] **Step 1: Add the delete affordance**

In `ActualBlock`, give the outer `<div>` the `group` class, and add a delete button in the header row (next to the name), mirroring `PlanCard`:

```tsx
<div className="flex items-center gap-1 font-medium">
  {live && <span className="inline-block h-1.5 w-1.5 animate-pulse rounded-full bg-status-error" />}
  <span className="truncate">{name}</span>
  {!live && (
    <button
      onPointerDown={(e) => e.stopPropagation()}
      onClick={(e) => { e.stopPropagation(); del.mutate(r.id); }}
      className="ml-auto rounded p-0.5 text-text-subtle opacity-0 transition hover:bg-status-error-subtle hover:text-status-error group-hover:opacity-100"
      aria-label="삭제"
      title="삭제"
    >
      <X size={11} />
    </button>
  )}
</div>
```

Add `const del = useDeleteRecord();` to `ActualBlock`. `ml-auto` pushes it right; `group-hover` reveals it. `stopPropagation` on pointerdown keeps the move gesture from starting on the ×.

- [ ] **Step 2: Build + vitest**

Run: `cd crates/oxiline-app && bun run build && bun run test`
Expected: green.

- [ ] **Step 3: Commit**

```bash
git add crates/oxiline-app/src/components/RecordTimeline.tsx
git commit -m "feat(record): hover-delete actual-record blocks"
```

---

### Task 5: Live-session guard, full gate, doc + spec update

**Files:**
- Modify: `crates/oxiline-app/src/components/RecordTimeline.tsx` (already guarded — live blocks skip move in Task 3 and hide × in Task 4; this task just verifies + adds a `title` tooltip on live blocks explaining why they're fixed).
- Modify: `doc/09-ui-redesign.md` §9.4.4 (ActualLane) — add move/delete bullets; §9.11 checklist — check the new items.
- Modify: `HANDOFF.md` — note the feature shipped.

**Interfaces:** none new.

- [ ] **Step 1: Add a tooltip on live blocks**

On `ActualBlock`'s outer div, when `live`, add `title="녹화 중 — 멈춤 후 편집"` so users understand why it won't drag/delete. (Open record: editing `ended_at` is moot since it's null; deleting would lose the running session — guard it.)

- [ ] **Step 2: Run the full gate**

Run:
```bash
cargo test --workspace
cd crates/oxiline-app && bun run build && bun run test
cargo clippy --workspace --all-targets -- -D warnings
```
Expected: all green, 0 warnings.

- [ ] **Step 3: Update doc/09-ui-redesign.md**

In §9.4.4 (ActualLane), add: blocks are **drag-to-move** (`edit_record`, UTC-delta), **hover-×-to-delete** (`delete_record`); live (open) blocks are fixed (stop first). In §9.11 checklist, check: "[x] 실제 기록 블록을 드래그로 이동, 호버 ×로 삭제(라이브 제외)."

- [ ] **Step 4: Update HANDOFF.md**

Add a dated entry under the UI section: "Actual-record editing — drag-to-move + hover-delete on ActualLane (edit_record/delete_record commands exposed). Commits: <hashes>."

- [ ] **Step 5: Commit**

```bash
git add doc/09-ui-redesign.md HANDOFF.md crates/oxiline-app/src/components/RecordTimeline.tsx
git commit -m "feat(record): actual-record move/delete + spec/handoff update"
```

---

## Self-Review (run before handing to execution)

- **Spec coverage:** move ✓ (Task 3), delete ✓ (Task 4), live guard ✓ (Task 3+4+5), backend wiring ✓ (Task 1), API/hooks ✓ (Task 2), docs ✓ (Task 5).
- **Type consistency:** `useEditRecord`/`useDeleteRecord` arg names match between Task 2 (definition) and Tasks 3/4 (callers) — `recordId`/`startedAt`/`endedAt` for edit, plain `id` string for delete. `ActivityRecord` is the shared return type.
- **Time correctness:** Task 3 moves by an absolute UTC delta (ms), preserving duration and DST/offset behavior — correct. Snapping applies to the delta, not the absolute.
- **Placeholder scan:** none — each step has real code or a real command.

## Execution Handoff

**Plan complete and saved to `docs/superpowers/plans/2026-08-05-actual-record-editing.md`. Two execution options:**

**1. Subagent-Driven (recommended)** — fresh subagent per task, review between tasks, fast iteration (superpowers:subagent-driven-development).

**2. Inline Execution** — batch tasks in-session with checkpoints (superpowers:executing-plans).
