import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Pause,
  Play,
  Plus,
  Trash2,
  X,
  GripVertical,
  ChevronRight,
} from "lucide-react";
import {
  useCategories,
  useCreateRoutine,
  useDeleteRoutine,
  useRoutines,
  useSetRoutineActive,
  useRoutineGroups,
  useCreateRoutineGroup,
  useUpdateRoutineGroup,
  useDeleteRoutineGroup,
  useSetRoutineGroupActive,
  useUpdateRoutine,
} from "../hooks";
import { useUi } from "../lib/store";
import {
  categoryById,
  categoryColor,
  minuteToHHMM,
  WEEKDAY_KEYS,
  MASK_DAILY,
  MASK_WEEKDAYS,
  MASK_WEEKENDS,
} from "../lib/colors";

function daysFromMask(mask: number): string {
  return WEEKDAY_KEYS.map((k, i) => ((mask >> i) & 1) ? k : null)
    .filter(Boolean)
    .join(",");
}

export function RoutineManager() {
  const { t } = useTranslation();
  const { routineManagerOpen: open, setRoutineManagerOpen } = useUi();
  const routinesQ = useRoutines(false);
  const catsQ = useCategories();
  const groupsQ = useRoutineGroups();
  const toggle = useSetRoutineActive();
  const del = useDeleteRoutine();
  const create = useCreateRoutine();
  const deleteGroup = useDeleteRoutineGroup();
  const createGroup = useCreateRoutineGroup();
  const updateGroup = useUpdateRoutineGroup();
  const setGroupActive = useSetRoutineGroupActive();

  const [selectedGroup, setSelectedGroup] = useState<string | null>(null);
  const [adding, setAdding] = useState(false);
  const [newGroupName, setNewGroupName] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [title, setTitle] = useState("");
  const [at, setAt] = useState("09:00");
  const [dur, setDur] = useState(30);
  const [mask, setMask] = useState(MASK_DAILY);
  const [catId, setCatId] = useState<string>("");

  if (!open) return null;

  const toggleDay = (i: number) => setMask((m) => m ^ (1 << i));

  const resetForm = () => {
    setAdding(false);
    setTitle("");
    setAt("09:00");
    setDur(30);
    setMask(MASK_DAILY);
    setCatId("");
  };

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
      { onSuccess: () => resetForm() },
    );
  };

  const groups = groupsQ.data ?? [];
  const allRoutines = routinesQ.data ?? [];
  const filtered = selectedGroup
    ? allRoutines.filter((r) => r.group_id === selectedGroup)
    : allRoutines;

  return (
    <div
      className="fixed inset-0 z-40 flex justify-end"
      style={{ background: "oklch(0 0 0 / 0.25)" }}
      onClick={() => setRoutineManagerOpen(false)}
    >
      <div
        className="flex w-[640px] max-w-full flex-col overflow-hidden border-l"
        style={{
          background: "var(--surface-canvas)",
          borderColor: "var(--border-default)",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div
          className="flex items-center justify-between border-b px-4 py-3"
          style={{ borderColor: "var(--border-subtle)" }}
        >
          <span className="text-[14px] font-semibold" style={{ color: "var(--text-primary)" }}>
            {t("routine.title")}
          </span>
          <button
            aria-label={t("common.close")}
            onClick={() => setRoutineManagerOpen(false)}
            className="rounded p-1 hover:bg-border-subtle"
          >
            <X size={16} style={{ color: "var(--text-secondary)" }} />
          </button>
        </div>

        <div className="flex flex-1 overflow-hidden">
          {/* Group sidebar */}
          <div
            className="flex w-36 shrink-0 flex-col border-r"
            style={{ borderColor: "var(--border-subtle)" }}
          >
            <div className="flex-1 overflow-y-auto p-2">
              <button
                onClick={() => setSelectedGroup(null)}
                className="flex w-full items-center gap-1 rounded px-2 py-1.5 text-left text-[12px]"
                style={{
                  background: selectedGroup === null ? "var(--accent-oxide-subtle)" : "transparent",
                  color: "var(--text-primary)",
                }}
                aria-label={t("routineGroup.noGroup")}
              >
                <ChevronRight size={12} />
                {t("routineGroup.unGrouped")}
              </button>
              {groups.map((g) => (
                <button
                  key={g.id}
                  onClick={() => setSelectedGroup(g.id)}
                  className="flex w-full items-center gap-1 rounded px-2 py-1.5 text-left text-[12px]"
                  style={{
                    background: selectedGroup === g.id ? "var(--accent-oxide-subtle)" : "transparent",
                    color: "var(--text-primary)",
                    opacity: g.is_active ? 1 : 0.5,
                  }}
                  aria-label={g.name}
                >
                  <ChevronRight size={12} />
                  <span className="flex-1 truncate">{g.name}</span>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      setGroupActive.mutate({ id: g.id, active: !g.is_active });
                    }}
                    className="rounded p-0.5 hover:bg-border-subtle"
                    aria-label={g.is_active ? t("routine.pause") : t("routine.resume")}
                  >
                    {g.is_active ? <Pause size={10} /> : <Play size={10} />}
                  </button>
                </button>
              ))}
            </div>
            <div className="border-t p-2" style={{ borderColor: "var(--border-subtle)" }}>
              <div className="flex gap-1">
                <input
                  value={newGroupName}
                  onChange={(e) => setNewGroupName(e.target.value)}
                  placeholder={t("routineGroup.addGroup")}
                  className="min-w-0 flex-1 rounded border bg-transparent px-2 py-1 text-[11px]"
                  style={{ borderColor: "var(--border-default)" }}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && newGroupName.trim()) {
                      createGroup.mutate({ name: newGroupName.trim(), icon: null });
                      setNewGroupName("");
                    }
                  }}
                  aria-label={t("routineGroup.addGroup")}
                />
                <button
                  onClick={() => {
                    if (newGroupName.trim()) {
                      createGroup.mutate({ name: newGroupName.trim(), icon: null });
                      setNewGroupName("");
                    }
                  }}
                  className="rounded p-1 hover:bg-border-subtle"
                  aria-label={t("routineGroup.addGroup")}
                >
                  <Plus size={14} style={{ color: "var(--accent-oxide)" }} />
                </button>
              </div>
            </div>
          </div>

          {/* Block list */}
          <div className="flex flex-1 flex-col overflow-hidden">
            <div className="flex-1 overflow-y-auto p-3">
              {/* Add new button */}
              {!adding && (
                <button
                  onClick={() => setAdding(true)}
                  className="mb-3 flex items-center gap-1.5 rounded px-2 py-1.5 text-[12px]"
                  style={{ color: "var(--accent-oxide)" }}
                  aria-label={t("routine.add")}
                >
                  <Plus size={14} />
                  {t("routine.add")}
                </button>
              )}

              {/* Inline add form */}
              {adding && <RoutineFormInline
                title={title} setTitle={setTitle}
                at={at} setAt={setAt}
                dur={dur} setDur={setDur}
                mask={mask} toggleDay={toggleDay}
                catId={catId} setCatId={setCatId}
                cats={catsQ.data ?? []}
                onSubmit={submit}
                onCancel={resetForm}
                t={t}
              />}

              {/* Routine list */}
              {filtered.map((r) =>
                editingId === r.id ? (
                  <RoutineEditInline
                    key={r.id}
                    routine={r}
                    cats={catsQ.data ?? []}
                    onCancel={() => setEditingId(null)}
                    onSaved={() => setEditingId(null)}
                    t={t}
                  />
                ) : (
                  <div
                    key={r.id}
                    className="group mb-1 flex items-center gap-2 rounded-md px-2 py-2 hover:bg-sunken"
                    onClick={() => {
                      setEditingId(r.id);
                      setAdding(false);
                    }}
                  >
                    <GripVertical size={14} className="shrink-0 opacity-0 group-hover:opacity-40" style={{ color: "var(--text-tertiary)" }} />
                    <span
                      className="inline-block h-2.5 w-2.5 shrink-0 rounded-full"
                      style={{
                        background: categoryColor(categoryById(catsQ.data ?? [], r.category_id)?.color_hue ?? null),
                      }}
                    />
                    <span className="flex-1 truncate text-[13px]" style={{ color: "var(--text-primary)" }}>
                      {r.title}
                    </span>
                    <span className="font-mono text-[11px]" style={{ color: "var(--text-tertiary)" }}>
                      {minuteToHHMM(r.start_minute)}
                    </span>
                    <span className="font-mono text-[11px]" style={{ color: "var(--text-tertiary)" }}>
                      {r.duration_minute}분
                    </span>
                    <span className="hidden text-[11px] group-hover:inline" style={{ color: "var(--text-tertiary)" }}>
                      {daysFromMask(r.weekday_mask)}
                    </span>
                    <button
                      onClick={(e) => { e.stopPropagation(); del.mutate(r.id); }}
                      className="rounded p-1 opacity-0 transition group-hover:opacity-100 hover:bg-border-subtle"
                      aria-label={t("routine.delete")}
                    >
                      <Trash2 size={14} style={{ color: "var(--signal-rust)" }} />
                    </button>
                  </div>
                ),
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

function RoutineFormInline({
  title, setTitle, at, setAt, dur, setDur, mask, toggleDay,
  catId, setCatId, cats, onSubmit, onCancel, t,
}: {
  title: string; setTitle: (v: string) => void;
  at: string; setAt: (v: string) => void;
  dur: number; setDur: (v: number) => void;
  mask: number; toggleDay: (i: number) => void;
  catId: string; setCatId: (v: string) => void;
  cats: { id: string; name: string }[];
  onSubmit: () => void;
  onCancel: () => void;
  t: (k: string) => string;
}) {
  return (
    <div className="mb-3 rounded-md border p-3" style={{ borderColor: "var(--accent-oxide)", background: "var(--surface-raised)" }}>
      <input
        autoFocus
        value={title}
        onChange={(e) => setTitle(e.target.value)}
        placeholder={t("routine.name")}
        className="mb-2 w-full bg-transparent text-[13px] outline-none"
        onKeyDown={(e) => { if (e.key === "Enter") onSubmit(); if (e.key === "Escape") onCancel(); }}
        aria-label={t("routine.name")}
      />
      <div className="mb-2 flex gap-2">
        <label className="text-[11px]" style={{ color: "var(--text-tertiary)" }}>
          {t("routine.at")}
          <input type="time" value={at} onChange={(e) => setAt(e.target.value)} className="ml-1 rounded border bg-transparent px-1 text-[11px]" style={{ borderColor: "var(--border-default)" }} />
        </label>
        <label className="text-[11px]" style={{ color: "var(--text-tertiary)" }}>
          {t("routine.duration")}
          <input type="number" value={dur} onChange={(e) => setDur(Number(e.target.value))} min={5} step={5} className="ml-1 w-16 rounded border bg-transparent px-1 text-[11px]" style={{ borderColor: "var(--border-default)" }} />
        </label>
      </div>
      <div className="mb-2 flex gap-1 text-[11px]" style={{ color: "var(--text-tertiary)" }}>
        {t("routine.days")}:
        {WEEKDAY_KEYS.map((k, i) => (
          <button key={k}
            onClick={() => toggleDay(i)}
            className="rounded px-1 py-0.5"
            style={{
              background: (mask >> i) & 1 ? "var(--accent-oxide)" : "var(--surface-subtle)",
              color: (mask >> i) & 1 ? "var(--text-on-accent)" : "var(--text-secondary)",
            }}
          >
            {t(`weekdays.${k}` as keyof typeof t)}
          </button>
        ))}
      </div>
      <div className="flex items-center gap-2">
        <select value={catId} onChange={(e) => setCatId(e.target.value)}
          className="rounded border bg-transparent px-2 py-1 text-[11px]" style={{ borderColor: "var(--border-default)" }}
          aria-label={t("routine.category")}
        >
          <option value="">{t("routine.category")}</option>
          {cats.map((c) => (<option key={c.id} value={c.id}>{c.name}</option>))}
        </select>
        <button onClick={onSubmit} className="ml-auto rounded px-3 py-1 text-[12px]" style={{ background: "var(--accent-oxide)", color: "var(--text-on-accent)" }}>{t("task.save")}</button>
        <button onClick={onCancel} className="rounded px-3 py-1 text-[12px]" style={{ color: "var(--text-secondary)" }}>{t("task.cancel")}</button>
      </div>
    </div>
  );
}

function RoutineEditInline({
  routine, cats, onCancel, onSaved, t,
}: {
  routine: {
    id: string; title: string;
    start_minute: number; duration_minute: number;
    weekday_mask: number; category_id: string | null;
    notes: string | null;
  };
  cats: { id: string; name: string }[];
  onCancel: () => void;
  onSaved: () => void;
  t: (k: string) => string;
}) {
  const upd = useUpdateRoutine();
  const [title, setTitle] = useState(routine.title);
  const [at, setAt] = useState(minuteToHHMM(routine.start_minute));
  const [dur, setDur] = useState(routine.duration_minute);
  const [mask, setMask] = useState(routine.weekday_mask);
  const [catId, setCatId] = useState(routine.category_id ?? "");

  const toggleDay = (i: number) => setMask((m) => m ^ (1 << i));

  const save = () => {
    const [h, m] = at.split(":").map(Number);
    upd.mutate(
      {
        id: routine.id,
        title: title.trim(),
        startMinute: h * 60 + m,
        durationMinute: dur,
        weekdayMask: mask,
        categoryId: catId || null,
        notes: routine.notes,
      },
      { onSuccess: () => onSaved() },
    );
  };

  return (
    <div className="mb-1 rounded-md border p-3" style={{ borderColor: "var(--accent-oxide)", background: "var(--surface-raised)" }}>
      <input
        autoFocus
        value={title}
        onChange={(e) => setTitle(e.target.value)}
        className="mb-2 w-full bg-transparent text-[13px] outline-none"
        onKeyDown={(e) => { if (e.key === "Enter") save(); if (e.key === "Escape") onCancel(); }}
        aria-label={t("routine.name")}
      />
      <div className="mb-2 flex gap-2">
        <label className="text-[11px]" style={{ color: "var(--text-tertiary)" }}>
          {t("routine.at")}
          <input type="time" value={at} onChange={(e) => setAt(e.target.value)} className="ml-1 rounded border bg-transparent px-1 text-[11px]" style={{ borderColor: "var(--border-default)" }} />
        </label>
        <label className="text-[11px]" style={{ color: "var(--text-tertiary)" }}>
          {t("routine.duration")}
          <input type="number" value={dur} onChange={(e) => setDur(Number(e.target.value))} min={5} step={5} className="ml-1 w-16 rounded border bg-transparent px-1 text-[11px]" style={{ borderColor: "var(--border-default)" }} />
        </label>
      </div>
      <div className="mb-2 flex gap-1 text-[11px]" style={{ color: "var(--text-tertiary)" }}>
        {t("routine.days")}:
        {WEEKDAY_KEYS.map((k, i) => (
          <button key={k}
            onClick={() => toggleDay(i)}
            className="rounded px-1 py-0.5"
            style={{
              background: (mask >> i) & 1 ? "var(--accent-oxide)" : "var(--surface-subtle)",
              color: (mask >> i) & 1 ? "var(--text-on-accent)" : "var(--text-secondary)",
            }}
          >
            {t(`weekdays.${k}` as keyof typeof t)}
          </button>
        ))}
      </div>
      <div className="flex items-center gap-2">
        <select value={catId} onChange={(e) => setCatId(e.target.value)}
          className="rounded border bg-transparent px-2 py-1 text-[11px]" style={{ borderColor: "var(--border-default)" }}
          aria-label={t("routine.category")}
        >
          <option value="">{t("routine.category")}</option>
          {cats.map((c) => (<option key={c.id} value={c.id}>{c.name}</option>))}
        </select>
        <button onClick={save} className="ml-auto rounded px-3 py-1 text-[12px]" style={{ background: "var(--accent-oxide)", color: "var(--text-on-accent)" }}>{t("task.save")}</button>
        <button onClick={onCancel} className="rounded px-3 py-1 text-[12px]" style={{ color: "var(--text-secondary)" }}>{t("task.cancel")}</button>
      </div>
    </div>
  );
}
