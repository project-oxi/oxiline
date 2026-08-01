/**
 * Inspector (Plan 2 Task 1 scaffolding) — right pane of the 3-pane shell.
 *
 * Regions from the converged mockup:
 *   1. 충족도 overview — neutral weekly/today compliance (Task 5).
 *   2. Total line — week recorded vs goal (Task 5).
 *   3. 최근 세션 — recent records (Task 5).
 *
 * Scaffolding renders intentional empty states; Task 5 wires `compliance`
 * + `list_records` (Task 2 api) into these regions. Neutral copy only.
 */
export function Inspector() {
  return (
    <aside className="flex w-[300px] shrink-0 flex-col gap-4 overflow-y-auto p-3">
      <section>
        <h2 className="mb-2 text-[11px] font-semibold uppercase tracking-wide text-text-subtle">
          충족도
        </h2>
        <div className="mb-3 inline-flex rounded-md bg-surface-sunken p-0.5 text-[12px]">
          <button className="rounded-[5px] bg-surface-raised px-3 py-1 font-medium text-text">
            주간
          </button>
          <button className="rounded-[5px] px-3 py-1 text-text-subtle">오늘</button>
        </div>
        <div className="flex items-center justify-center rounded-lg border border-dashed border-border py-8 text-[12px] text-text-subtle">
          기록된 활동이 없어요
        </div>
      </section>

      <section>
        <h2 className="mb-2 text-[11px] font-semibold uppercase tracking-wide text-text-subtle">
          최근 세션
        </h2>
        <div className="text-[12px] text-text-subtle">기록이 없어요</div>
      </section>
    </aside>
  );
}
