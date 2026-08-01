import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Search, CornerDownLeft, Plus, Check } from "lucide-react";
import {
  useBacklog,
  useCategories,
  useCreateTask,
  useSetTaskDone,
  useSuggestCards,
  useTimeline,
} from "../hooks";
import { useUi, todayStr } from "../lib/store";
import { categoryById, categoryColor, formatDuration } from "../lib/colors";
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

// A unified quick-add row: either a *create* source (template/history
// suggestion that prefills title+category+duration+notes) or a *toggle*
// source (an existing today/backlog item whose completion flips on Enter).
type Row =
  | {
      kind: "create";
      title: string;
      categoryId: string | null;
      durationMinute: number | null;
      notes: string | null;
      isTemplate: boolean;
    }
  | { kind: "toggle"; id: string; title: string; isDone: boolean };

export function CommandPalette() {
  const { t, i18n } = useTranslation();
  const { paletteOpen: open, setPaletteOpen, paletteDate } = useUi();
  const inputRef = useRef<HTMLInputElement>(null);
  const [q, setQ] = useState("");
  const [sel, setSel] = useState(0);
  const create = useCreateTask();
  const done = useSetTaskDone();
  const backlogQ = useBacklog();
  // Suppression/toggle pool follows the palette's target date so a card
  // already on that date isn't offered as a fresh create (and is toggleable).
  const tlQ = useTimeline(paletteDate ?? todayStr());
  const catsQ = useCategories();
  const suggestQ = useSuggestCards(open);
  const lang = i18n.language.startsWith("en") ? "en" : "ko";

  useEffect(() => {
    if (open) {
      setQ("");
      setSel(0);
    }
  }, [open]);

  // Existing today/backlog items — the *toggle* pool (current behavior).
  const existingPool = useMemo(
    () => [
      ...(backlogQ.data ?? []).map((x) => ({ id: x.id, title: x.title, isDone: x.is_done })),
      ...(tlQ.data ?? []).map((x) => ({ id: x.id, title: x.title, isDone: x.is_done })),
    ],
    [backlogQ.data, tlQ.data],
  );

  const rows = useMemo<Row[]>(() => {
    const lower = q.trim().toLowerCase();
    const present = new Set(existingPool.map((e) => e.title.trim().toLowerCase()));
    // Create sources: templates + history, minus any title already on the
    // plate today (those are reachable as toggle rows → no duplicate create).
    const creates: Row[] = (suggestQ.data ?? [])
      .filter((s) => !present.has(s.title.trim().toLowerCase()))
      .filter((s) => (lower === "" ? true : s.title.toLowerCase().includes(lower)))
      .slice(0, 6)
      .map((s) => ({
        kind: "create" as const,
        title: s.title,
        categoryId: s.category_id,
        durationMinute: s.duration_minute,
        notes: s.notes,
        isTemplate: s.is_template,
      }));
    // Toggle sources: only while typing (partial title match).
    const toggles: Row[] =
      lower === ""
        ? []
        : existingPool
            .filter((e) => e.title.toLowerCase().includes(lower))
            .slice(0, 4)
            .map((e) => ({ kind: "toggle" as const, id: e.id, title: e.title, isDone: e.isDone }));
    return [...creates, ...toggles];
  }, [q, suggestQ.data, existingPool]);

  const parsed = parseInput(q);
  const willAdd = parsed.title.length > 0;

  // Keep selection in range as the list shrinks/grows.
  useEffect(() => {
    setSel((s) => Math.min(s, rows.length));
  }, [rows.length]);

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSel((s) => Math.min(rows.length, s + 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSel((s) => Math.max(0, s - 1));
    } else if (e.key === "Enter") {
      e.preventDefault();
      const row = rows[sel];
      if (row?.kind === "toggle") {
        done.mutate({ id: row.id, done: !row.isDone });
      } else if (row?.kind === "create") {
        // Prefilled create from a template/history card.
        create.mutate({
          date: parsed.date ?? paletteDate,
          title: row.title,
          categoryId: row.categoryId,
          startMinute: parsed.startMinute,
          durationMinute: row.durationMinute ?? (parsed.startMinute != null ? 30 : null),
          notes: row.notes,
        });
      } else if (willAdd) {
        // Bare create — what the user typed, no prefill.
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

  const cats = catsQ.data ?? [];
  const offList = sel >= rows.length;

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
      </div>

      {rows.length > 0 && (
        <ul id="palette-list" role="listbox" className="max-h-[280px] overflow-y-auto border-t border-border">
          {rows.map((r, i) => {
            const active = i === sel;
            const isCreate = r.kind === "create";
            const cat = isCreate ? categoryById(cats, r.categoryId) : undefined;
            return (
              <li
                key={r.kind === "toggle" ? r.id : `c:${r.title}`}
                role="option"
                id={`palette-opt-${i}`}
                aria-selected={active}
                onMouseEnter={() => setSel(i)}
                className="flex items-center gap-2 px-3 py-2 text-[13px]"
                style={{ background: active ? "var(--color-surface-sunken)" : "transparent" }}
              >
                {isCreate ? (
                  <Plus size={13} className="shrink-0" style={{ color: "var(--color-interactive-primary)" }} />
                ) : (
                  <Check
                    size={13}
                    className="shrink-0"
                    style={{ color: "var(--color-text-subtle)", opacity: r.isDone ? 1 : 0.3 }}
                  />
                )}
                <span
                  className="inline-block h-2 w-2 shrink-0 rounded-full"
                  style={{ background: cat ? categoryColor(cat.color_hue) : "transparent" }}
                />
                <span
                  className="flex-1 truncate"
                  style={{
                    textDecoration: r.kind === "toggle" && r.isDone ? "line-through" : "none",
                    opacity: r.kind === "toggle" && r.isDone ? 0.6 : 1,
                  }}
                >
                  {r.title}
                </span>
                {r.kind === "create" && r.isTemplate && (
                  <span
                    className="shrink-0 rounded px-1 text-[10px]"
                    style={{
                      background: "var(--color-interactive-primary-subtle)",
                      color: "var(--color-interactive-primary)",
                    }}
                  >
                    {t("palette.template")}
                  </span>
                )}
                {r.kind === "create" && r.durationMinute != null && (
                  <span className="shrink-0 font-mono text-[11px]" style={{ color: "var(--color-text-subtle)" }}>
                    {formatDuration(r.durationMinute, lang)}
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
        {offList && willAdd && (
          <span style={{ color: "var(--color-interactive-primary)" }}>
            {parsed.date
              ? `${parsed.date}${
                  parsed.startMinute != null
                    ? ` ${String(Math.floor(parsed.startMinute / 60)).padStart(2, "0")}:${String(
                        parsed.startMinute % 60,
                      ).padStart(2, "0")}`
                    : ""
                }`
              : t("palette.addToList")}
          </span>
        )}
      </div>
    </Modal>
  );
}
