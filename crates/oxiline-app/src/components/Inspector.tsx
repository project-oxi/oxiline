/**
 * Inspector (Plan 2 Task 5) — right pane.
 *   1. 충족도 overview — neutral weekly/today compliance per activity.
 *   2. Total line — recorded vs goal for the scope.
 *   3. 최근 세션 — today's records.
 *
 * Neutral copy only: 미달/달성/초과 +Xm/목표 없음 (never failure language).
 */
import { useState } from "react";
import { useCompliance, useDayRecords } from "../hooks";
import { todayStr } from "../lib/store";
import { complianceLabel, hmm, hueVar } from "../lib/record-format";
import type { Scope } from "../types";

export function Inspector() {
  const [scope, setScope] = useState<Scope>("week");
  return (
    <aside className="flex w-[300px] shrink-0 flex-col gap-4 overflow-y-auto border-l border-border bg-surface-raised p-3">
      <ComplianceOverview scope={scope} onScope={setScope} />
      <RecentSessions />
    </aside>
  );
}

function ComplianceOverview({ scope, onScope }: { scope: Scope; onScope: (s: Scope) => void }) {
  const q = useCompliance(scope);
  const rows = q.data ?? [];
  const totalRecorded = rows.reduce((s, c) => s + c.recorded_seconds, 0);
  const totalTarget = rows.reduce((s, c) => s + (c.target_seconds ?? 0), 0);

  return (
    <section>
      <h2 className="mb-2 text-[11px] font-semibold uppercase tracking-wide text-text-subtle">충족도</h2>
      <div className="mb-3 inline-flex rounded-md bg-surface-sunken p-0.5 text-[12px]">
        {(["week", "today"] as Scope[]).map((s) => (
          <button
            key={s}
            onClick={() => onScope(s)}
            className={`rounded-[5px] px-3 py-1 transition ${
              scope === s ? "bg-surface-raised font-medium text-text" : "text-text-subtle"
            }`}
          >
            {s === "week" ? "주간" : "오늘"}
          </button>
        ))}
      </div>

      {rows.length === 0 ? (
        <div className="flex items-center justify-center rounded-lg border border-dashed border-border py-8 text-[12px] text-text-subtle">
          기록된 활동이 없어요
        </div>
      ) : (
        <div className="flex flex-col gap-2.5">
          {rows.map((c) => {
            const pct = c.ratio != null ? Math.min(100, Math.round(c.ratio * 100)) : 0;
            const surplus = Math.max(0, c.recorded_seconds - (c.target_seconds ?? 0));
            const sub =
              c.target_seconds == null
                ? hmm(c.recorded_seconds)
                : c.state === "met"
                  ? `${hmm(c.recorded_seconds)} / ${hmm(c.target_seconds)} · 달성`
                  : c.state === "over"
                    ? `${hmm(c.recorded_seconds)} / ${hmm(c.target_seconds)} · ${complianceLabel("over", surplus)}`
                    : `${hmm(c.recorded_seconds)} / ${hmm(c.target_seconds)} · 남음 ${hmm(c.target_seconds - c.recorded_seconds)}`;
            return (
              <div key={c.activity.id}>
                <div className="flex items-center justify-between text-[12px]">
                  <span className="flex items-center gap-1.5">
                    <span className="inline-block h-2 w-2 rounded-full" style={{ background: hueVar(c.activity.hue_label) }} />
                    {c.activity.name}
                  </span>
                  <span className="text-text-subtle">{c.ratio != null ? `${pct}%` : "—"}</span>
                </div>
                <div className="relative mt-1 h-1.5 rounded-full bg-surface-sunken">
                  <div className="h-full rounded-full" style={{ width: `${pct}%`, background: hueVar(c.activity.hue_label) }} />
                  {c.target_seconds != null && (
                    <span className="absolute top-1/2 h-2.5 w-0.5 -translate-y-1/2 bg-text-subtle" style={{ left: "100%" }} />
                  )}
                </div>
                <div className="mt-0.5 text-[10px] text-text-subtle">{sub}</div>
              </div>
            );
          })}
          <div className="mt-1 border-t border-border pt-2 text-[11px] text-text-subtle">
            {scope === "week" ? "이번 주" : "오늘"} 기록 <b className="text-text">{hmm(totalRecorded)}</b>
            {totalTarget > 0 && <> · 목표 <b className="text-text">{hmm(totalTarget)}</b></>}
          </div>
        </div>
      )}
    </section>
  );
}

function RecentSessions() {
  const q = useDayRecords(todayStr());
  const records = (q.data ?? [])
    .filter((r) => localDate(r.started_at) === todayStr())
    .sort((a, b) => (a.started_at < b.started_at ? 1 : -1))
    .slice(0, 8);

  return (
    <section>
      <h2 className="mb-2 text-[11px] font-semibold uppercase tracking-wide text-text-subtle">최근 세션</h2>
      {records.length === 0 ? (
        <div className="text-[12px] text-text-subtle">기록이 없어요</div>
      ) : (
        <div className="flex flex-col gap-1">
          {records.map((r) => {
            const start = new Date(r.started_at);
            const end = r.ended_at ? new Date(r.ended_at) : new Date();
            const live = r.ended_at === null;
            return (
              <div key={r.id} className="flex items-center justify-between text-[12px]">
                <span className="flex items-center gap-1.5 truncate">
                  <span className="inline-block h-1.5 w-1.5 rounded-full bg-text-subtle" />
                  <span className="truncate text-text-muted">{r.activity_id}</span>
                </span>
                <span className="shrink-0 text-text-subtle">
                  {pad(start.getHours())}:{pad(start.getMinutes())} · {hmm((end.getTime() - start.getTime()) / 1000)}
                  {live && " ▶"}
                </span>
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}

function localDate(iso: string): string {
  const d = new Date(iso);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

function pad(n: number): string {
  return String(n).padStart(2, "0");
}
