# Timeline + Header Redesign — Design Spec

**Date:** 2026-07-31
**Status:** Design (mockup v2 reviewed)
**Scope:** `DayTimeline.tsx`, `Header.tsx`, `BlockView.tsx`, `NowLine.tsx`, `OxideBar.tsx` (relocation), `App.tsx` (minor)

## 1. Goal

Replace the "system-app" feel of the day timeline and header with a modern, minimal
aesthetic inspired by a node-and-spine calendar reference (see mockup v2). Two surfaces:

1. **DayTimeline** — remove horizontal gridlines; introduce a vertical spine with colored
   nodes; soften blocks; wrap in a raised card.
2. **Header** — introduce a 7-day week strip with per-day category dots; elevate the date
   title; convert tabs to a segmented control; relocate OxideBar to the timeline card.

## 2. Constraints

- **OKLCH token-driven** — all colors via existing CSS variables; dark theme works for free.
- **No left accent bars** on blocks. Category identity is carried by spine-node color +
  subtle background tint + checkbox ring. (Explicit user requirement.)
- **Traffic-light safe** — header keeps `pt-9` top padding and `data-tauri-drag-region`.
- **Drag region preserved** — interactive children use `no-drag` (existing CSS rule).
- **Proportional encoding retained** — block height still = duration × pxPerMin; greedy
  multi-column overlap layout unchanged.
- **DnD + keyboard + click-to-add preserved** — no behavioral regressions to BlockView's
  interactions, DropZone, or quick-add composer.
- **Reduced-motion respected** — node pop-in / hover transforms gated on
  `prefers-reduced-motion`.
- **i18n** — Korean + English; reuse `shift()` / `setDate` / `useTimelineRange`.

## 3. Component Changes

### 3.1 Header.tsx — week-strip header

**Remove:**
- The `OxideBar` row (lines 99–106) and its import. OxideBar relocates to the timeline card
  (§3.4). `tlQ`, `settingsQ`, `dayStart`/`dayEnd`/`dayStartMin`/`totalMin` computations that
  only fed OxideBar are removed; the workload computation that OxideBar needed is no longer
  in the header.
- The bottom `border-b` on the tab row (line 108) — separation now comes from the gap +
  card elevation below.

**Row 1 — date title + chevrons + icons (replaces current date+chevron / icon cluster):**
- Left: ±1-day chevrons flanking a large localized date title. Format:
  - ko: `YYYY` (oxide-strong, 18px) + `M월 D일` (primary, 21px, 700) + `요일` (tertiary, 16px, 500)
  - en: `YYYY` (oxide-strong) + `Mon, Feb 1` (primary)
  - Clicking the title calls `goToToday()`.
- **Chevrons kept but promoted to always-visible** (was hover-reveal). `ChevronLeft`/`ChevronRight`
  at `opacity-40` by default, `opacity-100` on hover, calling `shiftDate(-1)` / `shiftDate(1)`.
  Rationale: a fixed ISO week strip (Mon–Sun) only renders 7 days — clicking within it never
  crosses a week boundary, so a visible ±1-day mouse control is required for cross-week
  navigation. Stepping ±1 from an edge day recomputes the strip to the adjacent week
  automatically. Keyboard `ArrowLeft`/`ArrowRight` (App.tsx:50-53) and `t` (today) still work
  but are not discoverable, so the visible chevrons are the primary mouse affordance.
- Right: icon cluster unchanged (Layers/Search/Settings). Keep `rounded p-1.5 hover:bg-sunken`.

**Row 2 — week strip (new):**
- Compute the ISO week (Monday–Sunday) containing `date`:
  ```ts
  const dow = new Date(date + "T12:00:00").getDay(); // 0=Sun..6=Sat
  const mondayOffset = dow === 0 ? -6 : 1 - dow;
  const monday = shift(date, mondayOffset);
  const sunday = shift(monday, 6);
  ```
- Fetch: `const weekQ = useTimelineRange(monday, sunday);`
- Render 7 `day-cell` buttons. Each cell:
  - Weekday label (tertiary, 10px, uppercase for en / single char for ko).
  - Day number (17px, 600). **Today** = filled `--accent-oxide` circle + glow shadow.
  - Per-day category dots: unique `categoryColor` hues from that day's items, max 5 dots,
    4px each. Empty days render no dots row (preserves vertical rhythm).
  - Click → `setDate(day)` + `setView("today")` (so tapping a day from another view jumps
    into the timeline).
- `todayStr` imported from store for today comparison.

**Row 3 — segmented tabs (restyled, same data):**
- Container: `bg-sunken rounded-lg p-0.5` track.
- Each tab: `rounded-md px-3 py-1.5 text-[13px] font-semibold`.
- Active tab: `bg-raised text-primary` + `box-shadow: --elevation-card` (floating pill).
- Inactive: `text-secondary`, transparent bg, hover → `text-primary`.

**Imports change:** add `useTimelineRange`, `shift`, `todayStr`, `categoryById`,
`categoryColor`. Keep `ChevronLeft`/`ChevronRight` (lucide-react) for ±1-day nav.
Remove `OxideBar`, `useTimeline`, `useSettings` imports and the `num`/`localeDateLabel` helpers.

### 3.2 DayTimeline.tsx — hybrid spine timeline

**Remove:**
- The `hours.map` block (lines 112–136) that renders full-width 1px gridlines. The `<span>`
  time labels inside it are replaced by standalone floating labels (§below).

**Constants:**
- `GUTTER_PX = 44` (was 56 — labels narrower, time-only).
- `SPINE_X = 54` — x-center of the spine line and nodes, measured from scroll-container left.
- `LANE_GAP = 10`.
- `laneLeft = SPINE_X + 12` (= 66) — blocks start here (node radius 7 + gap 5).
- `pxPerMin = 64/60` (was 56/60) — slightly taller for airiness. All derived math
  (`heightPx`, NowLine, DropZone) follows automatically.

**Add — spine line:**
- One absolutely-positioned `<div>`: `left: SPINE_X - 1, width: 2, top: 0, bottom: 0,
  background: --border-subtle`. Sits at `z-0` behind blocks.

**Add — time labels (quiet, no gridline):**
- Render hour labels at each hour mark as standalone `<span>`: `left: 0, width: GUTTER_PX,
  text-align: right, font-mono, text-[10px], color: --text-tertiary, transform:
  translateY(-5px)`. No line extends right. Pointer-events none.

**Add — spine nodes (per block):**
- For each laid block, render a node `<div>` at the block's `start_minute` Y, centered on
  `SPINE_X`:
  - Size 14px circle, `border: 2px solid var(--surface-raised)` (knockout against spine).
  - Fill: `item.is_done ? var(--signal-success) : categoryColor(cat?.color_hue ?? null)`.
  - Done node: white check icon inside. Past-undone: hollow with `border-color:
    var(--signal-rust)`, fill `--surface-raised`.
  - `z-10` (above spine, below NowLine z-20).
- Nodes are positioned in the same absolute coordinate space as blocks (inside the
  `.relative` height container), computed from `start`/`dayStartMin`/`pxPerMin`.

**Add — raised card wrapper:**
- Root changes from `<div className="flex h-full flex-col">` to:
  ```tsx
  <div className="flex h-full flex-col px-3 pb-3">
    <div className="flex flex-1 flex-col overflow-hidden rounded-2xl bg-raised"
         style={{ boxShadow: "var(--elevation-panel)" }}>
      {/* oxide handle (§3.4) */}
      <div className="relative flex-1 overflow-y-auto px-2 pb-6"> {/* spine timeline */} </div>
      <div className="...footer..."> {/* workload, unchanged */} </div>
    </div>
  </div>
  ```

**Hover hint + quick-add composer:**
- Adjust `left` offset to `laneLeft` (= 66) so they start at the block lane, not the old
  gutter boundary. Restyle: `rounded-lg` border `--accent-oxide`, `bg-raised`,
  `--elevation-panel` (already mostly there).

**Workload footer:** unchanged.

### 3.3 BlockView.tsx — softer cards, no left bar

**Remove:**
- `borderLeft: '4px ...'` from the style object (line 42). **No left accent bar.**
- `border border-border-subtle` from className (line 80).
- The `hover:-mt-0.5` class (line 80) — replaced by transform-based hover.
- `filter: "saturate(0.4)"` for past (line 54) — too aggressive.
- The inset-rust box-shadow for past-undone (line 44–46) — the rust ring node on the spine
  signals overdue instead.

**Change — style object:**
- `background`: `color-mix(in oklch, ${accent} 8%, var(--surface-raised))` where `accent` is
  the category color (or oxide for uncategorized). For done blocks, accent is
  `--signal-success` at 6%.
- `boxShadow`: `var(--elevation-card)` always; hover promotes to `var(--elevation-panel)`.
  Active (current, not past, not done) block: add `0 0 0 1.5px color-mix(in oklch, ${accent} 35%, transparent)` ring.
- Past-undone: `opacity: 0.55`, `color: --text-secondary` on title. No filter.
- Done: `opacity: 0.6`, title `line-through` + `--text-secondary`.
- Hover: `transform: translateX(2px)` (not margin-top; avoids vertical jitter). Keep the
  existing dnd `transform` via `CSS.Translate` — combine: `transform: CSS.Translate.toString(transform)
  || 'translateX(0)'` and apply the hover via a CSS class toggle (`hover:translate-x-0.5`)
  only when `!isDragging`.

**Keep unchanged:**
- The inner JSX (checkbox circle, title, rangeLabel, duration) — structure stays, only the
  checkbox border color uses the category color instead of `--border-default` when not done.
- `onKeyDown` handler, dnd attributes/listeners, aria-label.

### 3.4 OxideBar.tsx — relocate to card handle

**No component change.** OxideBar already has a `compact` prop (6px height). It moves from
`Header.tsx` to the top of `DayTimeline`'s card, rendered as a thin sticky handle:
```tsx
<div className="px-3 pt-2.5 pb-1">
  <OxideBar items={items} categories={...} dayStartMin={dayStartMin} totalMin={totalMin}
            compact onClickMinute={(m) => setAdding({ minute: snap(m) })} />
</div>
```
This preserves the day-minimap feature (clicking seeks + opens quick-add) while decluttering
the header. `useCreateTask` / `setAdding` / `snap` already exist in DayTimeline.

### 3.5 NowLine.tsx — spine node treatment

**Change:**
- The full-width 2px line (lines 53–56) becomes a short right-extending gradient: `left:
  SPINE_X, right: ...` → render as `height: 1.5px, background: linear-gradient(90deg,
  var(--accent-oxide-strong), transparent)`, spanning from the spine to the right edge of
  the content lane (not the full gutter width).
- The dot (lines 57–70) stays but repositions to sit centered on `SPINE_X` (left offset =
  `SPINE_X - 5`). Keep the pulse animation.
- The label stays (right of the node).
- `SPINE_X` must be passed as a prop (currently NowLine only receives `pxPerMin` +
  `dayStartMin`). Add `spineX: number` to Props.

### 3.6 App.tsx — no change

The card wrapping lives entirely inside DayTimeline; App's `<div className="flex flex-1
flex-col overflow-hidden">` is reused as-is. Header renders above it on `--surface-canvas`.

## 4. Geometry Summary

| Element        | x (from scroll left) | width    |
|----------------|----------------------|----------|
| Time labels    | 0                    | 44px     |
| Spine line     | 53                   | 2px      |
| Spine nodes    | 47–61 (centered 54)  | 14px     |
| Block lane     | 66 → right edge      | flexible |
| NowLine dot    | centered 54          | 10px     |
| NowLine stroke | 60 → right edge      | flexible |

Vertical: `pxPerMin = 64/60` (≈1.067 px/min, 64px/hour). `heightPx = totalMin × pxPerMin`.

## 5. New i18n keys

None required. Weekday labels use `Date.toLocaleDateString(locale, { weekday: "short" })`
(existing pattern from WeekView). Day titles reuse existing date formatting logic
(`localeDateLabel` in Header, simplified).

## 6. Non-Goals

- WeekView / BacklogView / ReportView are not redesigned in this pass.
- No new backend queries (week strip reuses `useTimelineRange`).
- No changes to the DnD system, keyboard shortcuts, or command palette.
- No animation library; transitions via CSS + existing motion tokens.

## 7. Risks

- **Spine-node vs. overlap columns:** when blocks are in side-by-side columns (overlap
  cluster), each column's blocks start at different `left` but their nodes all sit on the
  single spine at `SPINE_X`. The node marks the block's *start time*, not its column —
  correct behavior (two overlapping blocks at the same start get coincident nodes; rare, and
  visually they stack with a slightly larger outline). Acceptable.
- **Very short blocks** (< 22px tall): node (14px) may exceed block height. Node is
  positioned at start-time Y (top of block), so it sits at the block's top edge regardless
- **OxideBar in card handle** adds ~14px to card top. Net header height drops (removed
  OxideBar row), so timeline gains vertical room overall.
