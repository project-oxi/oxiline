import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Search, CornerDownLeft } from "lucide-react";
import { useBacklog, useCreateTask, useSetTaskDone, useTimeline } from "../hooks";
import { useUi, todayStr } from "../lib/store";
import { Modal } from "./Modal";

// Lightweight @time hint parser (§7.5): "@HH:MM" → today at that time;
// "@내일 HH:MM"/"@tomorrow HH:MM" → tomorrow.
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
    let rest = token;
    if (token === "내일" || token === "tomorrow" || token === "today" || token === "오늘") {
      const tomorrow = token === "내일" || token === "tomorrow";
      date = tomorrow ? shiftDay(todayStr(), 1) : todayStr();
      // optional HH:MM after the keyword
      const after = raw.slice(raw.indexOf(m[0]) + m[0].length).trim();
      const tm = after.match(/(\d{1,2}):(\d{2})/);
      if (tm) startMinute = Number(tm[1]) * 60 + Number(tm[2]);
      title = (raw.slice(0, raw.indexOf(m[0])) + " " + after.replace(tm ? tm[0] : "", "")).trim();
    } else {
      const tm = token.match(/(\d{1,2}):(\d{2})/);
      if (tm) {
        startMinute = Number(tm[1]) * 60 + Number(tm[2]);
        date = todayStr();
        rest = "";
      }
      title = (raw.slice(0, raw.indexOf(m[0])) + (rest ? "" : "")).trim();
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

export function CommandPalette() {
  const { t } = useTranslation();
  const { paletteOpen: open, setPaletteOpen, paletteDate } = useUi();
  const inputRef = useRef<HTMLInputElement>(null);
  const [q, setQ] = useState("");
  const [sel, setSel] = useState(0);
  const create = useCreateTask();
  const done = useSetTaskDone();
  const backlogQ = useBacklog();
  const tlQ = useTimeline(todayStr());

  useEffect(() => {
    if (open) {
      setQ("");
      setSel(0);
    }
  }, [open]);

  const matches = useMemo(() => {
    if (!q.trim()) return [];
    const lower = q.toLowerCase();
    const pool: { id: string; title: string; is_done: boolean }[] = [
      ...(backlogQ.data ?? []).map((t) => ({ id: t.id, title: t.title, is_done: t.is_done })),
      ...(tlQ.data ?? []).map((i) => ({ id: i.id, title: i.title, is_done: i.is_done })),
    ];
    return pool.filter((x) => x.title.toLowerCase().includes(lower)).slice(0, 6);
  }, [q, backlogQ.data, tlQ.data]);


  const parsed = parseInput(q);
  const willAdd = parsed.title.length > 0;

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSel((s) => Math.min(matches.length, s + 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSel((s) => Math.max(0, s - 1));
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (sel < matches.length && matches[sel]) {
        done.mutate({ id: matches[sel].id, done: !matches[sel].is_done });
      } else if (willAdd) {
        create.mutate({
          date: parsed.date ?? paletteDate,
          title: parsed.title,
          categoryId: null,
          startMinute: parsed.startMinute,
          durationMinute: parsed.startMinute != null ? 30 : null,
          notes: null,
        });
      }
      setPaletteOpen(false);
    }
  };

  return (
    <Modal
      open={open}
      onClose={() => setPaletteOpen(false)}
      variant="top"
      ariaLabel={t("palette.title")}
      panelClassName="w-full max-w-sm overflow-hidden rounded-lg border border-border-subtle"
      panelStyle={{ background: "var(--surface-raised)", boxShadow: "var(--elevation-panel)" }}
    >
        <div className="flex items-center gap-2 px-3 py-2.5">
          <Search size={15} style={{ color: "var(--text-tertiary)" }} />
          <input
            ref={inputRef}
            role="combobox"
            aria-expanded={matches.length > 0}
            aria-controls="palette-list"
            aria-autocomplete="list"
            aria-activedescendant={matches.length > 0 ? `palette-opt-${sel}` : undefined}
            value={q}
            onChange={(e) => { setQ(e.target.value); setSel(0); }}
            onKeyDown={onKeyDown}
            placeholder={t("palette.placeholder")}
            className="w-full bg-transparent text-[14px] outline-none"
          />
        </div>

        {matches.length > 0 && (
          <ul id="palette-list" role="listbox" className="border-t border-border-subtle">
            {matches.map((m, i) => (
              <li
                key={m.id}
                role="option"
                id={`palette-opt-${i}`}
                aria-selected={i === sel}
                onMouseEnter={() => setSel(i)}
                className="flex items-center justify-between px-3 py-2 text-[13px]"
                style={{ background: i === sel ? "var(--surface-sunken)" : "transparent" }}
              >
                <span className="truncate" style={{ textDecoration: m.is_done ? "line-through" : "none", opacity: m.is_done ? 0.6 : 1 }}>
                  {m.title}
                </span>
                {i === sel && <CornerDownLeft size={13} style={{ color: "var(--text-tertiary)" }} />}
              </li>
            ))}
          </ul>
        )}

        <div className="flex items-center justify-between border-t border-border-subtle px-3 py-1.5 text-[11px]" style={{ color: "var(--text-tertiary)" }}>
          <span>{t("palette.hint")}</span>
          {willAdd && (
            <span style={{ color: "var(--accent-oxide-strong)" }}>
              {parsed.date ? `${parsed.date}${parsed.startMinute != null ? ` ${String(Math.floor(parsed.startMinute / 60)).padStart(2, "0")}:${String(parsed.startMinute % 60).padStart(2, "0")}` : ""}` : t("palette.addToList")}
            </span>
          )}
        </div>
    </Modal>
  );
}
