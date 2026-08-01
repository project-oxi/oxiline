/**
 * ActivitySwitcher (Plan 2 Task 6) — the quick record-switcher (⌘⇧A).
 *
 * A top-anchored modal: type to filter activities, ↑/↓ to move, Enter to
 * start recording that activity (single-active: it closes any prior open
 * record). Space/Enter on the stop row halts the live session. This is the
 * primary "manual time recording" surface — like screen-time, but you flip
 * the active activity yourself and it lands on the clock.
 */
import { useEffect, useMemo, useRef, useState } from "react";
import { useActivities, useRecordState, useStartRecord, useStopRecord } from "../hooks";
import { useUi } from "../lib/store";
import { hueVar } from "../lib/record-format";
import { Modal } from "./Modal";

export function ActivitySwitcher() {
  const open = useUi((s) => s.switcherOpen);
  const setOpen = useUi((s) => s.setSwitcherOpen);
  const activitiesQ = useActivities(true);
  const stateQ = useRecordState();
  const start = useStartRecord();
  const stop = useStopRecord();

  const [q, setQ] = useState("");
  const [sel, setSel] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const activeId = stateQ.data?.active?.activity.id ?? null;
  const activities = useMemo(() => {
    const all = activitiesQ.data ?? [];
    const needle = q.trim().toLowerCase();
    return needle ? all.filter((a) => a.name.toLowerCase().includes(needle)) : all;
  }, [activitiesQ.data, q]);

  // reset query/selection each open; focus the field.
  useEffect(() => {
    if (open) {
      setQ("");
      setSel(0);
      requestAnimationFrame(() => inputRef.current?.focus());
    }
  }, [open]);

  // keep selection in range as the filter shrinks
  useEffect(() => {
    if (sel > activities.length) setSel(0);
  }, [activities.length, sel]);

  function choose(activityId: string) {
    start.mutate(activityId, { onSuccess: () => setOpen(false) });
  }

  function onKey(e: React.KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSel((s) => Math.min(activities.length, s + 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSel((s) => Math.max(0, s - 1));
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (sel === activities.length && activeId) {
        stop.mutate(undefined, { onSuccess: () => setOpen(false) });
      } else if (activities[sel]) {
        choose(activities[sel].id);
      }
    }
  }

  return (
    <Modal open={open} onClose={() => setOpen(false)} variant="top" labelledBy="switcher-title">
      <div
        className="w-[420px] overflow-hidden rounded-lg border border-border bg-surface-raised shadow-lg"
        onKeyDown={onKey}
      >
        <div className="border-b border-border px-3 py-2">
          <h2 id="switcher-title" className="text-[12px] font-semibold text-text-subtle">
            활동 전환 <span className="font-normal">· ⌘⇧A</span>
          </h2>
          <input
            ref={inputRef}
            value={q}
            onChange={(e) => {
              setQ(e.target.value);
              setSel(0);
            }}
            placeholder="지금 무엇을 하고 있나요?"
            className="mt-1 w-full bg-transparent text-[15px] text-text outline-none placeholder:text-text-subtle"
          />
        </div>
        <div className="max-h-[320px] overflow-y-auto py-1">
          {activities.map((a, i) => {
            const isActive = a.id === activeId;
            return (
              <button
                key={a.id}
                onMouseEnter={() => setSel(i)}
                onClick={() => choose(a.id)}
                className={`flex w-full items-center gap-2 px-3 py-2 text-left text-[13px] ${
                  sel === i ? "bg-surface-sunken" : ""
                }`}
              >
                <span className="inline-block h-2 w-2 rounded-full" style={{ background: hueVar(a.hue_label) }} />
                <span className={isActive ? "font-medium text-text" : "text-text"}>{a.name}</span>
                {isActive && <span className="text-[10px] text-status-error">녹화 중</span>}
              </button>
            );
          })}
          {activeId && (
            <button
              onMouseEnter={() => setSel(activities.length)}
              onClick={() => stop.mutate(undefined, { onSuccess: () => setOpen(false) })}
              className={`flex w-full items-center gap-2 border-t border-border px-3 py-2 text-left text-[13px] text-text-muted ${
                sel === activities.length ? "bg-surface-sunken" : ""
              }`}
            >
              <span className="inline-block h-2 w-2 rounded-full bg-text-subtle" />
              멈춤 (현재 세션 종료)
            </button>
          )}
          {activities.length === 0 && !activeId && (
            <div className="px-3 py-4 text-center text-[12px] text-text-subtle">활동을 먼저 추가하세요</div>
          )}
        </div>
      </div>
    </Modal>
  );
}
