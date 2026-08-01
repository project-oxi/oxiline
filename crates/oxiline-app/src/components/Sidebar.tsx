/**
 * Sidebar (Plan 2 Task 1 scaffolding) — left pane of the 3-pane shell.
 *
 * Two regions matching the converged mockup (`2026-08-01-final-mockup.html`):
 *   1. Now-card  — the active recording session (filled in Task 4).
 *   2. Activity library — drag-to-place cards with weekly bars (Task 4 + 7).
 *
 * This scaffolding renders intentional empty states so the shell is visually
 * complete before the data layer (Task 2) lands.
 */
export function Sidebar() {
  return (
    <aside className="flex w-[260px] shrink-0 flex-col gap-4 overflow-y-auto p-3">
      {/* Now-card */}
      <section className="rounded-lg border border-border bg-surface-raised p-3">
        <div className="flex items-center gap-2 text-[11px] text-text-subtle">
          <span className="inline-block h-1.5 w-1.5 rounded-full bg-text-subtle" />
          지금 녹화 중
        </div>
        <div className="mt-2 text-[13px] text-text-muted">녹화 중인 활동이 없어요</div>
        <div className="mt-1 text-[11px] text-text-subtle">⌘⇧A 로 빠르게 전환</div>
      </section>

      {/* Activity library */}
      <section className="flex min-h-0 flex-1 flex-col">
        <div className="mb-2 flex items-center justify-between">
          <h2 className="text-[11px] font-semibold uppercase tracking-wide text-text-subtle">
            활동 · 카드
          </h2>
          <span className="text-[10px] text-text-subtle">드래그 → 배치</span>
        </div>
        <div className="flex flex-1 items-center justify-center rounded-lg border border-dashed border-border text-[12px] text-text-subtle">
          활동을 추가하세요
        </div>
      </section>
    </aside>
  );
}
