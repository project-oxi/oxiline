# Timeline + Header Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redesign the day timeline (hybrid spine, no gridlines, softer blocks, raised card) and header (week strip, big title, segmented tabs) for a modern minimal aesthetic.

**Architecture:** Pure frontend restyling across 4 React components in `crates/oxiline-app/src/components/`. No backend, no new queries, no new dependencies. All colors via existing OKLCH CSS variables. Verification = TypeScript typecheck + Vite build + visual smoke test (no test framework in project).

**Tech Stack:** React 19, TypeScript 5.7, Tailwind CSS v4 (OKLCH tokens), Vite 6, Tauri 2, @dnd-kit/core, zustand, lucide-react, i18next.

## Global Constraints

- **No left accent bars** on blocks — category color via spine-node fill + background tint + checkbox ring only.
- **All colors via CSS variables** (`--surface-*`, `--text-*`, `--accent-oxide*`, `--signal-*`, `--elevation-*`) — dark theme works automatically.
- **Traffic-light safe** — header root keeps `pt-9` + `data-tauri-drag-region`.
- **DnD, keyboard, click-to-add, quick-add composer preserved** — no behavioral regressions.
- **Cross-week navigation visible** — ±1-day chevrons flanking the date title are always-visible (muted, brighten on hover). A fixed ISO week strip can't cross boundaries by clicking, so the chevrons are the primary mouse affordance.
- **Reduced-motion respected** — `prefers-reduced-motion` media query in `styles.css` already disables pulse; do not add unguarded animations.
- **Verification command:** `cd crates/oxiline-app && npx tsc --noEmit` (typecheck) then `npm run build` (build). Visual check via `npm run dev`.

---

## Geometry Reference (all tasks share these)

```
x=0         44          54           66
|─ time ────|── gap ──|─ spine ──|── blocks ────────────|
   labels (44px, right-aligned)  ↑ nodes (14px, centered at 54)
                                  spine line (2px at x=53)
```

- `GUTTER_PX = 44` — time label width
- `SPINE_X = 54` — center x of spine line + nodes
- `LANE_GAP = 12` — gap from spine center to block lane start
- `laneLeft = SPINE_X + LANE_GAP = 66` — block content lane left edge
- `pxPerMin = 64 / 60` — block height density (was `56/60`)

---

### Task 1: BlockView — remove left bar, softer cards

**Files:**
- Modify: `crates/oxiline-app/src/components/BlockView.tsx` (style object lines 37–56, className line 80, checkbox line 98)

**Interfaces:**
- Consumes: `BlockView` props unchanged (`item, categories, left, columns, top, height, past`)
- Produces: restyled `BlockView` (same API) — no interface change for callers

- [ ] **Step 1: Replace the style object (lines 37–56)**

Remove `borderLeft`, `filter`, and the `inset` rust shadow. Add category-tinted background. Move shadow to className-driven hover. Replace:

```ts
  const style: React.CSSProperties = {
    top,
    height: Math.max(height, 22),
    left: `calc(${leftPct}% + ${leftPct > 0 ? 4 : 0}px)`,
    width: `calc(${widthPct}% - 4px)`,
    borderLeft: `4px ${item.is_virtual ? 'dashed' : 'solid'} ${accent}`,
    opacity: isDragging ? 0.5 : item.is_virtual && !item.is_done ? 0.92 : 1,
    boxShadow: past && !item.is_done
      ? "inset 3px 0 0 var(--signal-rust), var(--elevation-card)"
      : "var(--elevation-card)",
    transform: CSS.Translate.toString(transform),
    zIndex: isDragging ? 999 : 2,
    transition: isDragging
      ? undefined
      : `opacity var(--motion-sweep) var(--ease-standard),
          filter var(--motion-sweep) var(--ease-standard),
          margin-top var(--motion-base) var(--ease-standard)`,
    filter: past ? "saturate(0.4)" : undefined,
    cursor: "grab",
  };
```

with:

```ts
  const style: React.CSSProperties = {
    top,
    height: Math.max(height, 22),
    left: `calc(${leftPct}% + ${leftPct > 0 ? 4 : 0}px)`,
    width: `calc(${widthPct}% - 4px)`,
    background: item.is_done
      ? "color-mix(in oklch, var(--signal-success) 6%, var(--surface-raised))"
      : `color-mix(in oklch, ${accent} 8%, var(--surface-raised))`,
    opacity: isDragging
      ? 0.5
      : item.is_virtual && !item.is_done
        ? 0.92
        : past && !item.is_done
          ? 0.55
          : 1,
    transform: CSS.Translate.toString(transform),
    zIndex: isDragging ? 999 : 2,
    transition: isDragging
      ? undefined
      : `transform var(--motion-base) var(--ease-standard),
          box-shadow var(--motion-base) var(--ease-standard)`,
    cursor: "grab",
  };
```

- [ ] **Step 2: Replace the outer div className (line 80)**

Remove `border-border-subtle`, `bg-raised`, `hover:-mt-0.5`. Add shadow utilities (hover lifts card→panel). Replace:

```tsx
      className={`absolute overflow-hidden rounded-lg border border-border-subtle bg-raised${past ? '' : ' hover:-mt-0.5'}`}
```

with:

```tsx
      className={`absolute overflow-hidden rounded-lg ${
        isDragging
          ? "shadow-[var(--elevation-panel)]"
          : "shadow-[var(--elevation-card)] hover:shadow-[var(--elevation-panel)]"
      }`}
```

- [ ] **Step 3: Update checkbox border to use category color (line 98)**

The checkbox border currently uses `var(--border-default)` when not done. Change it to the category accent so the ring carries category identity. Replace:

```tsx
            borderColor: item.is_done ? "var(--signal-success)" : "var(--border-default)",
```

with:

```tsx
            borderColor: item.is_done ? "var(--signal-success)" : accent,
```

- [ ] **Step 4: Typecheck**

Run: `cd crates/oxiline-app && npx tsc --noEmit`
Expected: PASS (no type errors — only style/className changes)

- [ ] **Step 5: Commit**

```bash
git add crates/oxiline-app/src/components/BlockView.tsx
git commit -m "feat(app): block restyle — remove left bar, category-tinted bg, shadow hover"
```

---

### Task 2: NowLine — spine node treatment

**Files:**
- Modify: `crates/oxiline-app/src/components/NowLine.tsx` (Props interface lines 4–7, destructure line 16, render lines 46–78)

**Interfaces:**
- Consumes: nothing new from other tasks
- Produces: `NowLine` now requires `spineX: number` prop. DayTimeline (Task 3) must pass it.

- [ ] **Step 1: Add `spineX` to Props (lines 4–7)**

Replace:

```ts
interface Props {
  pxPerMin: number;
  dayStartMin: number;
}
```

with:

```ts
interface Props {
  pxPerMin: number;
  dayStartMin: number;
  spineX: number;
}
```

- [ ] **Step 2: Destructure `spineX` (line 16)**

Replace:

```ts
export function NowLine({ pxPerMin, dayStartMin }: Props) {
```

with:

```ts
export function NowLine({ pxPerMin, dayStartMin, spineX }: Props) {
```

- [ ] **Step 3: Replace the render (lines 46–78)**

Anchor the container at the spine; render a right-fading gradient line instead of full-width solid; center the dot on the spine. Replace:

```tsx
  return (
    <div
      ref={lineRef}
      className="pointer-events-none absolute left-0 right-0 z-20"
      style={{ top: 0, willChange: "transform" }}
    >
      <div className="relative h-0">
        <div
          className="absolute left-0 right-0"
          style={{ height: 2, background: "var(--accent-oxide-strong)" }}
        />
        <div
          ref={dotRef}
          className="absolute"
          style={{
            left: 0,
            top: -5,
            width: 10,
            height: 10,
            borderRadius: 999,
            background: "var(--accent-oxide)",
            boxShadow: "0 0 0 4px var(--accent-oxide-subtle)",
            animation: "oxiline-pulse 2s var(--ease-standard) infinite",
          }}
        />
        <span
          ref={labelRef}
          className="absolute font-mono text-[11px]"
          style={{ left: 14, top: -8, color: "var(--accent-oxide-strong)" }}
        />
      </div>
      <style>{`@keyframes oxiline-pulse { 0%,100%{opacity:.85;transform:scale(1)} 50%{opacity:1;transform:scale(1.06)} }`}</style>
    </div>
  );
```

with:

```tsx
  return (
    <div
      ref={lineRef}
      className="pointer-events-none absolute z-20"
      style={{ top: 0, left: spineX, right: 0, willChange: "transform" }}
    >
      <div className="relative h-0">
        <div
          className="absolute"
          style={{
            left: 6,
            right: 0,
            height: 1.5,
            background:
              "linear-gradient(90deg, var(--accent-oxide-strong), transparent)",
          }}
        />
        <div
          ref={dotRef}
          className="absolute"
          style={{
            left: -5,
            top: -5,
            width: 10,
            height: 10,
            borderRadius: 999,
            background: "var(--accent-oxide)",
            boxShadow: "0 0 0 4px var(--accent-oxide-subtle)",
            animation: "oxiline-pulse 2s var(--ease-standard) infinite",
          }}
        />
        <span
          ref={labelRef}
          className="absolute font-mono text-[10px]"
          style={{ left: 10, top: -7, color: "var(--accent-oxide-strong)" }}
        />
      </div>
      <style>{`@keyframes oxiline-pulse { 0%,100%{opacity:.85;transform:scale(1)} 50%{opacity:1;transform:scale(1.06)} }`}</style>
    </div>
  );
```

- [ ] **Step 4: Typecheck**

Run: `cd crates/oxiline-app && npx tsc --noEmit`
Expected: FAIL — `NowLine` now requires `spineX` but DayTimeline doesn't pass it yet. This is expected; Task 3 fixes it. Proceed to Task 3 without committing yet.

---

### Task 3: DayTimeline — hybrid spine + card + oxide handle

**Files:**
- Modify: `crates/oxiline-app/src/components/DayTimeline.tsx` (imports, constants lines 67–70, pxPerMin line 87, laneLeft line 106, main render lines 108–256)

**Interfaces:**
- Consumes: `NowLine` with `spineX` prop (from Task 2)
- Produces: redesigned `DayTimeline` — renders OxideBar at card top, passes `spineX` to NowLine

- [ ] **Step 1: Update imports (lines 1–9)**

Add `Check` from lucide-react, `OxideBar`, `categoryById`, `categoryColor`. Replace:

```ts
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useTimeline, useCategories, useCreateTask, useSettings } from "../hooks";
import { useUi } from "../lib/store";
import { useDroppable } from "@dnd-kit/core";
import { BlockView } from "./BlockView";
import { NowLine } from "./NowLine";
import { formatDuration, minuteToHHMM } from "../lib/colors";
import type { TimelineItem } from "../types";
```

with:

```ts
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Check } from "lucide-react";
import { useTimeline, useCategories, useCreateTask, useSettings } from "../hooks";
import { useUi } from "../lib/store";
import { useDroppable } from "@dnd-kit/core";
import { BlockView } from "./BlockView";
import { NowLine } from "./NowLine";
import { OxideBar } from "./OxideBar";
import { formatDuration, minuteToHHMM, categoryById, categoryColor } from "../lib/colors";
import type { TimelineItem } from "../types";
```

- [ ] **Step 2: Update layout constants (lines 67–70)**

Replace:

```ts
const GUTTER_PX = 56;
const LANE_GAP = 10;
```

with:

```ts
const GUTTER_PX = 44;
const SPINE_X = 54;
const LANE_GAP = 12;
```

- [ ] **Step 3: Update pxPerMin (line 87)**

Replace:

```ts
  const pxPerMin = 56 / 60;
```

with:

```ts
  const pxPerMin = 64 / 60;
```

- [ ] **Step 4: Update laneLeft derivation (line 106)**

Replace:

```ts
  const laneLeft = GUTTER_PX + LANE_GAP;
```

with:

```ts
  const laneLeft = SPINE_X + LANE_GAP;
```

(Value stays 66; derivation now references the spine explicitly.)

- [ ] **Step 5: Replace the entire return block (lines 108–256)**

This is the core change: wrap in a raised card, add OxideBar handle, replace gridlines with a spine + quiet labels, add spine nodes. Replace from `return (` (line 108) through the closing `);` (line 256) with:

```tsx
  return (
    <div className="flex h-full flex-col px-3 pb-3">
      <div
        className="flex flex-1 flex-col overflow-hidden rounded-2xl"
        style={{ background: "var(--surface-raised)", boxShadow: "var(--elevation-panel)" }}
      >
        {/* oxide handle — day minimap as card grabber */}
        <div className="px-3 pt-2.5 pb-1.5">
          <OxideBar
            items={items}
            categories={catsQ.data ?? []}
            dayStartMin={dayStartMin}
            totalMin={totalMin}
            compact
            onClickMinute={(m) => setAdding({ minute: snap(m) })}
          />
        </div>

        <div className="relative flex-1 overflow-y-auto px-2 pb-6">
          <div className="relative" style={{ height: heightPx }}>
            {/* spine line */}
            <div
              className="pointer-events-none absolute"
              style={{ left: SPINE_X - 1, top: 0, bottom: 0, width: 2, background: "var(--border-subtle)" }}
            />

            {/* quiet time labels — no gridlines */}
            {hours.map((h) => {
              const top = (h * 60 - dayStartMin) * pxPerMin;
              const label = `${String(h % 24).padStart(2, "0")}:00`;
              return (
                <span
                  key={h}
                  className="pointer-events-none absolute font-mono text-[10px]"
                  style={{
                    left: 0,
                    width: GUTTER_PX,
                    top,
                    textAlign: "right",
                    paddingRight: 8,
                    color: "var(--text-tertiary)",
                    transform: "translateY(-5px)",
                  }}
                >
                  {label}
                </span>
              );
            })}

            {/* spine nodes — one per block, colored by category */}
            {laid.map(({ item }) => {
              const start = item.start_minute!;
              const dur = item.duration_minute!;
              const nodeTop = (start - dayStartMin) * pxPerMin;
              const cat = categoryById(catsQ.data ?? [], item.category_id);
              const nodeColor = categoryColor(cat?.color_hue ?? null);
              const isPastUndone = !item.is_done && start + dur <= nowMin();
              const fill = item.is_done
                ? "var(--signal-success)"
                : isPastUndone
                  ? "var(--surface-raised)"
                  : nodeColor;
              const ring = item.is_done
                ? "var(--signal-success)"
                : isPastUndone
                  ? "var(--signal-rust)"
                  : "var(--surface-raised)";
              return (
                <div
                  key={`node-${item.id}`}
                  className="pointer-events-none absolute z-10 flex items-center justify-center rounded-full"
                  style={{
                    left: SPINE_X - 7,
                    top: nodeTop - 7,
                    width: 14,
                    height: 14,
                    background: fill,
                    border: `2px solid ${ring}`,
                  }}
                >
                  {item.is_done && <Check size={9} color="white" strokeWidth={3} />}
                </div>
              );
            })}

            {/* content lane — right of the spine */}
            <div className="absolute bottom-0 right-0 top-0" style={{ left: laneLeft }}>
              {/* hover slot hint */}
              {hover != null && !adding && (
                <div
                  className="pointer-events-none absolute left-0 right-0 z-[1] flex items-center rounded-md"
                  style={{
                    top: (hover - dayStartMin) * pxPerMin,
                    height: SLOT * pxPerMin,
                    background: "color-mix(in oklch, var(--accent-oxide-subtle) 70%, transparent)",
                  }}
                >
                  <span
                    className="ml-1 font-mono text-[11px] font-medium"
                    style={{ color: "var(--accent-oxide-strong)" }}
                  >
                    + {minuteToHHMM(hover)}
                  </span>
                </div>
              )}

              {/* blocks */}
              {laid.map(({ item, col, columns }) => {
                const start = item.start_minute!;
                const dur = item.duration_minute!;
                const top = (start - dayStartMin) * pxPerMin;
                const height = dur * pxPerMin;
                const past = start + dur <= nowMin();
                return (
                  <BlockView
                    key={item.id}
                    item={item}
                    categories={catsQ.data ?? []}
                    left={col}
                    columns={columns}
                    top={top}
                    height={height}
                    past={past}
                  />
                );
              })}

              {/* quick-add composer */}
              {adding && (
                <div
                  className="absolute left-0 right-0 z-30 flex items-center gap-2 rounded-lg border px-2 py-1.5"
                  style={{
                    top: (adding.minute - dayStartMin) * pxPerMin,
                    borderColor: "var(--accent-oxide)",
                    background: "var(--surface-raised)",
                    boxShadow: "var(--elevation-panel)",
                  }}
                >
                  <span
                    className="shrink-0 rounded-md px-1.5 py-0.5 font-mono text-[11px] font-medium"
                    style={{
                      background: "var(--accent-oxide-subtle)",
                      color: "var(--accent-oxide-strong)",
                    }}
                  >
                    {minuteToHHMM(adding.minute)}
                  </span>
                  <input
                    autoFocus
                    value={draft}
                    onChange={(e) => setDraft(e.target.value)}
                    placeholder={t("palette.placeholder")}
                    className="flex-1 bg-transparent text-[13px] outline-none"
                    onKeyDown={(e) => {
                      if (e.key === "Enter" && draft.trim()) {
                        create.mutate({
                          date,
                          title: draft.trim(),
                          categoryId: null,
                          startMinute: adding.minute,
                          durationMinute: 30,
                          notes: null,
                        });
                        setAdding(null);
                        setDraft("");
                      }
                      if (e.key === "Escape") {
                        setAdding(null);
                        setDraft("");
                      }
                    }}
                    onBlur={() => {
                      setAdding(null);
                      setDraft("");
                    }}
                  />
                </div>
              )}

              <DropZone
                dayStartMin={dayStartMin}
                pxPerMin={pxPerMin}
                date={date}
                heightPx={heightPx}
                onAdd={(minute) => setAdding({ minute })}
                onHover={setHover}
              />

              <NowLine pxPerMin={pxPerMin} dayStartMin={dayStartMin} spineX={SPINE_X} />
            </div>
          </div>
        </div>

        {/* workload footer */}
        <div
          className="flex items-center justify-center gap-1.5 border-t border-border-subtle px-3 py-1.5 text-[12px]"
          style={{ color: tight ? "var(--signal-rust)" : "var(--text-secondary)" }}
        >
          <span>{t("timeline.plannedDur", { dur: formatDuration(workloadMin, lang as "ko" | "en") })}</span>
          <span style={{ color: "var(--text-tertiary)" }}>·</span>
          <span>{tight ? t("timeline.workloadTight") : t("timeline.workloadEasy")}</span>
        </div>
      </div>
    </div>
  );
```

- [ ] **Step 6: Typecheck**

Run: `cd crates/oxiline-app && npx tsc --noEmit`
Expected: PASS (NowLine now receives `spineX`, all imports resolve)

- [ ] **Step 7: Commit (Tasks 2 + 3 together)**

```bash
git add crates/oxiline-app/src/components/NowLine.tsx crates/oxiline-app/src/components/DayTimeline.tsx
git commit -m "feat(app): hybrid spine timeline — gridlines removed, colored nodes, raised card"
```

---

### Task 4: Header — week strip + big title + segmented tabs + chevrons

**Files:**
- Modify: `crates/oxiline-app/src/components/Header.tsx` (full rewrite)

**Interfaces:**
- Consumes: `useTimelineRange`, `useCategories` from hooks; `shift`, `todayStr`, `useUi` from store; `categoryById`, `categoryColor` from colors
- Produces: redesigned `Header` — same default export shape, no OxideBar

- [ ] **Step 1: Replace imports and helpers (lines 1–22)**

Remove `OxideBar`, `useTimeline`, `useSettings`, `num`, `localeDateLabel`. Keep `ChevronLeft`/`ChevronRight`. Replace:

```ts
import { ChevronLeft, ChevronRight, Search, Settings as SettingsIcon, Layers } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useTimeline, useCategories, useSettings } from "../hooks";
import { useUi } from "../lib/store";
import { OxideBar } from "./OxideBar";

function num(v: unknown, d: number): number {
  return typeof v === "number" ? v : d;
}

function localeDateLabel(dateStr: string, lang: string): string {
  const [y, m, d] = dateStr.split("-").map(Number);
  const dt = new Date(y, m - 1, d);
  const wd = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"][dt.getDay()];
  const wdKo = ["일", "월", "화", "수", "목", "금", "토"][dt.getDay()];
  if (lang === "ko") {
    return `${y}년 ${m}월 ${d}일 (${wdKo})`;
  }
  const wdEn = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][dt.getDay()];
  void wd;
  return `${wdEn}, ${dt.toLocaleDateString("en-US", { month: "short" })} ${d}`;
}
```

with:

```ts
import { ChevronLeft, ChevronRight, Search, Settings as SettingsIcon, Layers } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useTimelineRange, useCategories } from "../hooks";
import { useUi, todayStr, shift } from "../lib/store";
import { categoryById, categoryColor } from "../lib/colors";
```

- [ ] **Step 2: Replace the component body (lines 24–125)**

Replace the entire `export function Header()` block with:

```tsx
export function Header() {
  const { t, i18n } = useTranslation();
  const { date, view, setView, setDate, shiftDate, goToToday, setPaletteOpen, setPreferencesOpen, setRoutineManagerOpen } =
    useUi();
  const catsQ = useCategories();
  const lang = i18n.language?.startsWith("en") ? "en" : "ko";

  // Week (Mon–Sun) containing the selected date
  const dow = new Date(date + "T12:00:00").getDay();
  const mondayOffset = dow === 0 ? -6 : 1 - dow;
  const monday = shift(date, mondayOffset);
  const sunday = shift(monday, 6);
  const weekQ = useTimelineRange(monday, sunday);
  const weekCols = weekQ.data ?? [];
  const categories = catsQ.data ?? [];
  const today = todayStr();

  const tabs: { key: typeof view; label: string }[] = [
    { key: "today", label: t("nav.today") },
    { key: "week", label: t("nav.week") },
    { key: "backlog", label: t("nav.backlog") },
    { key: "report", label: t("nav.report") },
  ];

  // Date title parts
  const [yy, mm, dd] = date.split("-").map(Number);
  const titleDt = new Date(yy, mm - 1, dd);
  const wdKo = ["일", "월", "화", "수", "목", "금", "토"][titleDt.getDay()];

  return (
    <div data-tauri-drag-region className="shrink-0 px-4 pt-9 pb-2">
      {/* Row 1: chevrons + big date title + icons */}
      <div className="flex items-center justify-between gap-2 pb-2.5">
        <div className="flex items-center gap-1">
          <button
            className="rounded p-1 opacity-40 transition hover:bg-sunken hover:opacity-100"
            onClick={() => shiftDate(-1)}
            aria-label="prev day"
          >
            <ChevronLeft size={16} />
          </button>
          <button
            onClick={goToToday}
            className="flex items-baseline gap-1.5 rounded px-1 hover:bg-sunken"
            title={t("nav.today")}
          >
            <span className="text-[18px] font-semibold" style={{ color: "var(--accent-oxide-strong)" }}>
              {yy}
            </span>
            <span className="text-[21px] font-bold tracking-tight" style={{ color: "var(--text-primary)" }}>
              {lang === "ko"
                ? `${mm}월 ${dd}일`
                : titleDt.toLocaleDateString("en-US", { month: "short", day: "numeric" })}
            </span>
            <span className="text-[16px] font-medium" style={{ color: "var(--text-tertiary)" }}>
              {lang === "ko" ? wdKo : titleDt.toLocaleDateString("en-US", { weekday: "short" })}
            </span>
          </button>
          <button
            className="rounded p-1 opacity-40 transition hover:bg-sunken hover:opacity-100"
            onClick={() => shiftDate(1)}
            aria-label="next day"
          >
            <ChevronRight size={16} />
          </button>
        </div>

        <div className="flex items-center gap-1">
          <button
            className="rounded p-1.5 hover:bg-sunken"
            onClick={() => setRoutineManagerOpen(true)}
            aria-label={t("routine.title")}
            title={t("routine.title")}
          >
            <Layers size={16} />
          </button>
          <button
            className="rounded p-1.5 hover:bg-sunken"
            onClick={() => setPaletteOpen(true)}
            aria-label="⌘K"
            title="⌘K"
          >
            <Search size={16} />
          </button>
          <button
            className="rounded p-1.5 hover:bg-sunken"
            onClick={() => setPreferencesOpen(true)}
            aria-label={t("settings.title")}
            title={t("settings.title")}
          >
            <SettingsIcon size={16} />
          </button>
        </div>
      </div>

      {/* Row 2: week strip */}
      <div className="flex gap-1 pb-2.5">
        {weekCols.map(({ date: dStr, items }) => {
          const [cy, cm, cdd] = dStr.split("-").map(Number);
          const cdt = new Date(cy, cm - 1, cdd);
          const dayNum = cdt.getDate();
          const wdLabel =
            lang === "ko"
              ? ["일", "월", "화", "수", "목", "금", "토"][cdt.getDay()]
              : cdt.toLocaleDateString("en-US", { weekday: "narrow" });
          const isToday = dStr === today;
          const hues = [
            ...new Set(
              items
                .filter((i) => !i.is_skipped)
                .map((i) => categoryById(categories, i.category_id)?.color_hue ?? null),
            ),
          ].slice(0, 5);
          return (
            <button
              key={dStr}
              onClick={() => {
                setDate(dStr);
                setView("today");
              }}
              className="flex flex-1 flex-col items-center gap-1 rounded-lg py-1.5 transition hover:bg-sunken"
            >
              <span
                className="text-[10px] font-semibold"
                style={{ color: isToday ? "var(--accent-oxide-strong)" : "var(--text-tertiary)" }}
              >
                {wdLabel}
              </span>
              <span
                className="flex h-7 w-7 items-center justify-center rounded-full text-[13px] font-semibold transition"
                style={{
                  background: isToday ? "var(--accent-oxide)" : "transparent",
                  color: isToday ? "white" : "var(--text-secondary)",
                  boxShadow: isToday ? "0 2px 8px oklch(0.62 0.1 189 / 0.35)" : undefined,
                }}
              >
                {dayNum}
              </span>
              <span className="flex h-1.5 items-center gap-0.5">
                {hues.map((h, i) => (
                  <span key={i} className="h-1 w-1 rounded-full" style={{ background: categoryColor(h) }} />
                ))}
              </span>
            </button>
          );
        })}
      </div>

      {/* Row 3: segmented tabs */}
      <div className="flex gap-0.5 rounded-lg p-0.5" style={{ background: "var(--surface-sunken)" }}>
        {tabs.map((tb) => (
          <button
            key={tb.key}
            onClick={() => setView(tb.key)}
            className="flex-1 rounded-md px-3 py-1.5 text-[13px] font-semibold transition"
            style={{
              background: view === tb.key ? "var(--surface-raised)" : "transparent",
              color: view === tb.key ? "var(--text-primary)" : "var(--text-secondary)",
              boxShadow: view === tb.key ? "var(--elevation-card)" : undefined,
            }}
          >
            {tb.label}
          </button>
        ))}
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Typecheck**

Run: `cd crates/oxiline-app && npx tsc --noEmit`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/oxiline-app/src/components/Header.tsx
git commit -m "feat(app): week-strip header — 7-day strip, big title, segmented tabs, visible chevrons"
```

---

### Task 5: Full build + visual verification

**Files:** none (verification only)

- [ ] **Step 1: Full typecheck**

Run: `cd crates/oxiline-app && npx tsc --noEmit`
Expected: PASS

- [ ] **Step 2: Production build**

Run: `cd crates/oxiline-app && npm run build`
Expected: PASS (Vite build completes, no errors)

- [ ] **Step 3: Visual smoke test**

Run: `cd crates/oxiline-app && npm run dev`
Open the dev URL. Verify:
- **Header:** big date title (year in oxide accent) flanked by always-visible ±1-day chevrons (muted, brighten on hover); 7-day week strip with today as filled circle + category dots; segmented tab pills with active tab floating
- **Chevron cross-week nav:** click right chevron from Sunday → strip shifts to next week; click left chevron from Monday → strip shifts to previous week
- **Timeline:** floating raised card, oxide minimap handle at top, vertical spine with colored nodes, no horizontal gridlines, quiet time labels, NowLine as spine dot + fading gradient
- **Blocks:** no left accent bars, category-tinted backgrounds, shadow lift on hover, done blocks with green nodes + strikethrough, past-undone with rust ring nodes + faded
- **Interactions:** click a week-strip day → jumps to it; click a block → toggles done; drag a block → moves; click empty timeline → quick-add composer; ⌘K → command palette

- [ ] **Step 4: Stop dev server, final commit if any fixes were needed**

If the smoke test surfaced issues, fix them and commit. Otherwise the work from Tasks 1–4 is complete.
