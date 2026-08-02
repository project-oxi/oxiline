import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Search, CornerDownLeft, Plus, Play, Square } from "lucide-react";
import {
  useActivities,
  useCategories,
  useCreateActivity,
  useCreatePlan,
  useRecordState,
  useStartRecord,
  useStopRecord,
} from "../hooks";
import { useUi, todayStr } from "../lib/store";
import { categoryById, categoryColor } from "../lib/colors";
import { Modal } from "./Modal";
import type { Activity } from "../types";

// Lightweight @time hint parser. "@HH:MM" → today at that time (schedule a
// plan); "@내일 HH:MM"/"@tomorrow HH:MM" → tomorrow at that time. No time hint
// means "record now".
interface Parsed {
  title: string;
  date: string | null;
  startMinute: number | null;
}

function parseInput(raw: string): Parsed {
  let title = raw;
  let date: string | null = null;
  let startMinute: number | null = null;

  const m = raw.match(/@(\S+)/);
  if (m) {
    const token = m[1];
    if (token === "내일" || token === "tomorrow" || token === "today" || token === "오늘") {
      const tomorrow = token === "내일" || token === "tomorrow";
      date = tomorrow ? shiftDay(todayStr(), 1) : todayStr();
      const after = raw.slice(raw.indexOf(m[0]) + m[0].length).trim();
      const tm = after.match(/(\d{1,2}):(\d{2})/);
      if (tm) startMinute = Number(tm[1]) * 60 + Number(tm[2]);
      title = (raw.slice(0, raw.indexOf(m[0])) + " " + after.replace(tm ? tm[0] : "", "")).trim();
    } else {
      const tm = token.match(/(\d{1,2}):(\d{2})/);
      if (tm) {
        startMinute = Number(tm[1]) * 60 + Number(tm[2]);
        date = todayStr();
        title = raw.slice(0, raw.indexOf(m[0])).trim();
      }
    }
  }
  return { title: title.trim(), date, startMinute };
}

function shiftDay(dateStr: string, n: number): string {
  const [y, m, d] = dateStr.split("-").map(Number);
  const dt = new Date(y, m - 1, d);
  dt.setDate(dt.getDate() + n);
  return `${dt.getFullYear()}-${String(dt.getMonth() + 1).padStart(2, "0")}-${String(dt.getDate()).padStart(2, "0")}`;
}

type Row =
  | { kind: "activity"; activity: Activity }
  | { kind: "new"; title: string };

export function CommandPalette() {
  const { t, i18n } = useTranslation();
  const { paletteOpen: open, setPaletteOpen, paletteDate } = useUi();
  const inputRef = useRef<HTMLInputElement>(null);
  const [q, setQ] = useState("");
  const [sel, setSel] = useState(0);

  const startRec = useStartRecord();
  const stopRec = useStopRecord();
  const createPlan = useCreatePlan();
  const createActivity = useCreateActivity();
  const actsQ = useActivities(true);
  const catsQ = useCategories();
  const recQ = useRecordState();
  const lang = i18n.language.startsWith("en") ? "en" : "ko";

  useEffect(() => {
    if (open) {
      setQ("");
      setSel(0);
    }
  }, [open]);

  const parsed = parseInput(q);
  const lower = parsed.title.toLowerCase();

  const activities = actsQ.data ?? [];
  const matched = useMemo(
    () => (lower === "" ? activities : activities.filter((a) => a.name.toLowerCase().includes(lower))),
    [activities, lower],
  );
  const exactExists = activities.some((a) => a.name.trim().toLowerCase() === lower);

  const rows = useMemo<Row[]>(() => {
    const list: Row[] = matched.slice(0, 7).map((activity) => ({ kind: "activity", activity }));
    if (parsed.title.length > 0 && !exactExists) {
      list.push({ kind: "new", title: parsed.title });
    }
    return list;
  }, [matched, parsed.title, exactExists]);

  useEffect(() => {
    setSel((s) => Math.min(s, Math.max(0, rows.length - 1)));
  }, [rows.length]);

  const recording = recQ.data?.active != null;
  const scheduling = parsed.startMinute != null;

  // Act on a resolved activity id: schedule a plan (@time) or record now.
  function commit(activityId: string) {
    if (scheduling) {
      createPlan.mutate({
        date: parsed.date ?? paletteDate ?? todayStr(),
        start_minute: parsed.startMinute!,
        duration_minute: 30,
        weekday_mask: 0,
        title: null,
        activity_ids: [activityId],
      });
    } else {
      startRec.mutate(activityId);
    }
    setPaletteOpen(false);
  }

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSel((s) => Math.min(rows.length - 1, s + 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSel((s) => Math.max(0, s - 1));
    } else if (e.key === "Enter") {
      e.preventDefault();
      const row = rows[sel];
      if (row?.kind === "activity") {
        commit(row.activity.id);
      } else if (row?.kind === "new") {
        // Free-text: create the activity on the fly, then record/schedule it.
        createActivity.mutate({ name: row.title }, { onSuccess: (a) => commit(a.id) });
        setPaletteOpen(false);
      } else if (parsed.title === "" && recording) {
        // Empty input while recording → stop.
        stopRec.mutate();
        setPaletteOpen(false);
      }
    }
  };

  const cats = catsQ.data ?? [];

  return (
    <Modal
      open={open}
      onClose={() => setPaletteOpen(false)}
      variant="top"
      ariaLabel={t("palette.title")}
      panelClassName="w-full max-w-sm overflow-hidden rounded-lg border border-border"
      panelStyle={{ background: "var(--color-surface-raised)", boxShadow: "var(--shadow-lg)" }}
    >
      <div className="flex items-center gap-2 px-3 py-2.5">
        <Search size={15} style={{ color: "var(--color-text-subtle)" }} />
        <input
          ref={inputRef}
          role="combobox"
          aria-expanded={rows.length > 0}
          aria-controls="palette-list"
          aria-autocomplete="list"
          aria-activedescendant={rows.length > 0 ? `palette-opt-${sel}` : undefined}
          value={q}
          onChange={(e) => {
            setQ(e.target.value);
            setSel(0);
          }}
          onKeyDown={onKeyDown}
          placeholder={t("palette.placeholder")}
          className="w-full bg-transparent text-[14px] outline-none"
        />
        {recording && parsed.title === "" && (
          <Square size={13} className="shrink-0" style={{ color: "var(--color-accent)" }} aria-hidden />
        )}
      </div>

      {rows.length > 0 && (
        <ul id="palette-list" role="listbox" className="max-h-[280px] overflow-y-auto border-t border-border">
          {rows.map((r, i) => {
            const active = i === sel;
            const isNew = r.kind === "new";
            const act = r.kind === "activity" ? r.activity : null;
            const cat = act ? categoryById(cats, act.category_id) : undefined;
            return (
              <li
                key={r.kind === "activity" ? r.activity.id : `new:${r.title}`}
                role="option"
                id={`palette-opt-${i}`}
                aria-selected={active}
                onMouseEnter={() => setSel(i)}
                className={`flex items-center gap-2 px-3 py-2 text-[13px]${
                  active ? " bg-surface-sunken" : ""
                }`}
              >
                {isNew ? (
                  <Plus size={13} className="shrink-0" style={{ color: "var(--color-interactive-primary)" }} />
                ) : (
                  <Play size={13} className="shrink-0" style={{ color: "var(--color-text-subtle)" }} aria-hidden />
                )}
                <span
                  className="inline-block h-2 w-2 shrink-0 rounded-full"
                  style={{ background: cat ? categoryColor(cat.color_hue) : "transparent" }}
                />
                <span className="flex-1 truncate">{r.kind === "activity" ? r.activity.name : r.title}</span>
                {active && (
                  <span className="shrink-0 font-mono text-[11px]" style={{ color: "var(--color-text-subtle)" }}>
                    {scheduling ? hhmm(parsed.startMinute!) : lang === "en" ? "record" : "녹화"}
                  </span>
                )}
                {active && <CornerDownLeft size={13} style={{ color: "var(--color-text-subtle)" }} />}
              </li>
            );
          })}
        </ul>
      )}

      <div
        className="flex items-center justify-between border-t border-border px-3 py-1.5 text-[11px]"
        style={{ color: "var(--color-text-subtle)" }}
      >
        <span>{t("palette.hint")}</span>
        {scheduling && (
          <span style={{ color: "var(--color-interactive-primary)" }}>
            {parsed.date ? `${parsed.date} ` : ""}
            {hhmm(parsed.startMinute!)}
          </span>
        )}
      </div>
    </Modal>
  );
}

function hhmm(minute: number): string {
  return `${String(Math.floor(minute / 60)).padStart(2, "0")}:${String(minute % 60).padStart(2, "0")}`;
}
