import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Pause, Play, Plus, Trash2, X } from "lucide-react";
import {
  useCategories,
  useCreateRoutine,
  useDeleteRoutine,
  useRoutines,
  useSetRoutineActive,
} from "../hooks";
import { useUi } from "../lib/store";
import { categoryById, categoryColor, minuteToHHMM, WEEKDAY_KEYS, MASK_DAILY, MASK_WEEKDAYS, MASK_WEEKENDS } from "../lib/colors";

function daysFromMask(mask: number): string {
  return WEEKDAY_KEYS.map((k, i) => ((mask >> i) & 1) ? k : null)
    .filter(Boolean)
    .join(",");
}

export function RoutineManager() {
  const { t } = useTranslation();
  const { routineManagerOpen: open, setRoutineManagerOpen } = useUi();
  const q = useRoutines(false);
  const catsQ = useCategories();
  const toggle = useSetRoutineActive();
  const del = useDeleteRoutine();
  const create = useCreateRoutine();

  const [adding, setAdding] = useState(false);
  const [title, setTitle] = useState("");
  const [at, setAt] = useState("09:00");
  const [dur, setDur] = useState(30);
  const [mask, setMask] = useState(MASK_DAILY);
  const [catId, setCatId] = useState<string>("");

  if (!open) return null;

  const toggleDay = (i: number) => setMask((m) => m ^ (1 << i));

  const submit = () => {
    if (!title.trim()) return;
    const [h, m] = at.split(":").map(Number);
    create.mutate(
      {
        title: title.trim(),
        startMinute: h * 60 + m,
        durationMinute: dur,
        weekdayMask: mask,
        categoryId: catId || null,
        effectiveFrom: null,
        effectiveUntil: null,
        notes: null,
      },
      {
        onSuccess: () => {
          setTitle("");
          setAdding(false);
        },
      },
    );
  };

  return (
    <div className="fixed inset-0 z-40 flex justify-end" style={{ background: "oklch(0 0 0 / 0.25)" }} onClick={() => setRoutineManagerOpen(false)}>
      <div
        className="h-full w-full max-w-md overflow-y-auto border-l border-border-subtle p-4"
        style={{ background: "var(--surface-canvas)" }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="mb-4 flex items-center justify-between">
          <h2 className="text-[16px] font-semibold">{t("routine.title")}</h2>
          <div className="flex items-center gap-1">
            <button
              className="rounded-md px-2 py-1 text-[12px] font-medium hover:bg-sunken"
              style={{ background: "var(--accent-oxide-subtle)", color: "var(--accent-oxide-strong)" }}
              onClick={() => setAdding((a) => !a)}
            >
              <Plus size={14} className="mr-1 inline" />
              {t("routine.add")}
            </button>
            <button className="rounded p-1 hover:bg-sunken" onClick={() => setRoutineManagerOpen(false)} aria-label={t("common.close")}>
              <X size={16} />
            </button>
          </div>
        </div>

        {adding && (
          <div className="mb-4 space-y-2 rounded-md border border-border-subtle p-3" style={{ background: "var(--surface-raised)" }}>
            <input
              autoFocus
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder={t("routine.name")}
              className="w-full rounded border border-border-subtle bg-transparent px-2 py-1.5 text-[13px] outline-none"
            />
            <div className="flex gap-2">
              <input type="time" value={at} onChange={(e) => setAt(e.target.value)} className="rounded border border-border-subtle bg-transparent px-2 py-1 font-mono text-[12px]" />
              <input
                type="number"
                value={dur}
                onChange={(e) => setDur(Number(e.target.value))}
                className="w-20 rounded border border-border-subtle bg-transparent px-2 py-1 text-[12px]"
                aria-label={t("routine.duration")}
              />
            </div>
            <div className="flex gap-1">
              {WEEKDAY_KEYS.map((k, i) => (
                <button
                  key={k}
                  onClick={() => toggleDay(i)}
                  className="h-7 w-7 rounded text-[11px] font-medium"
                  style={{
                    background: (mask >> i) & 1 ? "var(--accent-oxide)" : "var(--surface-sunken)",
                    color: (mask >> i) & 1 ? "white" : "var(--text-secondary)",
                  }}
                >
                  {t(`weekdays.${k}`)}
                </button>
              ))}
            </div>
            <div className="flex gap-1">
              {[
                { v: MASK_DAILY, k: "daily" },
                { v: MASK_WEEKDAYS, k: "weekdays" },
                { v: MASK_WEEKENDS, k: "weekends" },
              ].map((p) => (
                <button key={p.k} onClick={() => setMask(p.v)} className="rounded-full border border-border-subtle px-2 py-0.5 text-[11px]" style={{ color: "var(--text-secondary)" }}>
                  {t(`dayPresets.${p.k}`)}
                </button>
              ))}
            </div>
            <select value={catId} onChange={(e) => setCatId(e.target.value)} className="w-full rounded border border-border-subtle bg-transparent px-2 py-1 text-[12px]">
              <option value="">{t("routine.category")}</option>
              {(catsQ.data ?? []).map((c) => (
                <option key={c.id} value={c.id}>{c.name}</option>
              ))}
            </select>
            <div className="flex justify-end gap-2 pt-1">
              <button className="rounded px-2 py-1 text-[12px] hover:bg-sunken" onClick={() => setAdding(false)}>
                {t("task.cancel")}
              </button>
              <button className="rounded px-3 py-1 text-[12px] font-medium text-white" style={{ background: "var(--accent-oxide)" }} onClick={submit}>
                {t("task.save")}
              </button>
            </div>
          </div>
        )}

        {(q.data ?? []).length === 0 && (
          <p className="px-1 text-[12px]" style={{ color: "var(--text-tertiary)" }}>
            {t("routine.empty")}
          </p>
        )}

        <ul className="space-y-1">
          {(q.data ?? []).map((r) => {
            const cat = categoryById(catsQ.data ?? [], r.category_id);
            const days = daysFromMask(r.weekday_mask);
            return (
              <li key={r.id} className="group flex items-center gap-2 rounded-md px-2 py-2 hover:bg-sunken">
                <span className="h-2.5 w-2.5 shrink-0 rounded-full" style={{ background: categoryColor(cat?.color_hue ?? null) }} />
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <span className="truncate text-[13px] font-medium" style={{ opacity: r.is_active ? 1 : 0.5 }}>
                      {r.title}
                    </span>
                  </div>
                  <div className="mt-0.5 flex items-center gap-1 text-[11px]" style={{ color: "var(--text-tertiary)" }}>
                    <span className="font-mono">{minuteToHHMM(r.start_minute)}</span>
                    <span>·</span>
                    <span>{r.duration_minute}{t("common.minutes")}</span>
                    <span>·</span>
                    {days.split(",").map((d) => (
                      <span key={d} style={{ color: "var(--text-secondary)" }}>{t(`weekdays.${d}`)}</span>
                    ))}
                  </div>
                </div>
                <button
                  className="rounded p-1 opacity-60 hover:opacity-100"
                  onClick={() => toggle.mutate({ id: r.id, active: !r.is_active })}
                  aria-label={r.is_active ? t("routine.pause") : t("routine.resume")}
                >
                  {r.is_active ? <Pause size={14} /> : <Play size={14} />}
                </button>
                <button
                  className="rounded p-1 opacity-0 transition group-hover:opacity-100 hover:bg-border-subtle"
                  onClick={() => del.mutate(r.id)}
                  aria-label={t("routine.delete")}
                >
                  <Trash2 size={14} style={{ color: "var(--text-tertiary)" }} />
                </button>
              </li>
            );
          })}
        </ul>
      </div>
    </div>
  );
}
