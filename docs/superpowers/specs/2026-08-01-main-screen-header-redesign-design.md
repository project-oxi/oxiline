# Main-Screen Header Redesign — Design Spec

**Date:** 2026-08-01
**Status:** Design (direction A approved)
**Scope:** `Header.tsx` (primary), `styles.css` (one component token). No other files.
**Predecessor:** `2026-07-31-timeline-header-redesign-design.md` (this iterates on that result).

## 1. Goal

Make the main-screen header more minimal and cleaner, and utilize the titlebar strip
better — while fixing two design-system deviations found in the adherence audit.

**Approved direction — "refined 3-band":** keep the three header bands (titlebar → week
strip → view tabs) but clean the execution of each. No behavior change.

## 2. Adherence audit (vs `oxi-design-system/DESIGN.md` v1.0)

Token layer is compliant (3-tier OKLCH, `.dark` single trigger, no `dark:` in components,
SUIT/SUITE/Geist Mono, `font-display` on the date). Two deviations this redesign fixes:

| # | Rule | Current | Fix |
|---|------|---------|-----|
| 1 | **§6.10 Tabs** | Filled segmented control — `bg-surface-sunken` track + raised-pill active (`Header.tsx:160`) | **Underline tabs** — `border-b border-line` track, active `border-b-2 border-interactive-primary` |
| 2 | **§2.1 Single Rule** | Today indicator reads `--color-interactive-primary`, `-foreground`, `--color-text-muted` directly in inline `style` (`Header.tsx:137–145`) | Utilities (`bg-interactive-primary`, `text-interactive-primary-foreground`, `text-text-muted`) + a component token for the glow |

The `categoryColor(h)` hue dot (`Header.tsx:151`) is a legitimately dynamic value — unchanged.

## 3. Problems confirmed by the live screenshot

1. **Titlebar under-utilized / lopsided** — date huddles left-of-center after the 56px
   traffic-light clearance; icons far right → a wide dead gap in the strip.
2. **Date is busy** — three mismatched fragments in one strip: blue `2026` + `8월 1일`
   (21px SUITE) + gray `토` (16px). No single clean hierarchy.
3. **Heavy chrome** — the filled segmented-control band is the heaviest visual weight
   before any schedule content.

## 4. Constraints

- **Tauri overlay, 420px wide** (min 360px). Titlebar keeps `data-tauri-drag-region`;
  interactive children remain drag-free via the existing `no-drag` CSS rule. The
  56px left clearance for native traffic lights is preserved.
- **No behavior change** — `shiftDate`, `goToToday`, `setDate`+`setView("today")` from
  the week strip, `setView` tabs, `setPaletteOpen`/`setPreferencesOpen`/`setRoutineManagerOpen`
  icon actions, keyboard shortcuts — all preserved exactly.
- **i18n** — Korean + English. No new keys.
- **Design-system compliant** — every change consumes Tailwind utilities or component
  tokens, never `--color-*` in component code.
- **Reduced motion respected.**

## 5. Component changes — `Header.tsx`

### 5.1 Row 1 — titlebar strip (clean hero date)

**Remove** the separate blue `2026` year span (the noisiest fragment).

**Date hero** — replace the three-fragment title with a clean two-part hero, anchored
immediately after the traffic-light clearance (utilizes the strip's left side):

```tsx
<button onClick={goToToday} className="flex items-baseline gap-1.5 rounded px-1 hover:bg-surface-sunken" title={t("nav.today")}>
  <span className="text-[20px] font-semibold font-display tracking-tight text-text">
    {lang === "ko" ? `${mm}월 ${dd}일` : titleDt.toLocaleDateString("en-US", { month: "short", day: "numeric" })}
  </span>
  <span className="text-[12px] font-medium text-text-muted">
    {lang === "ko"
      ? ["일","월","화","수","목","금","토"][titleDt.getDay()] + "요일"
      : titleDt.toLocaleDateString("en-US", { weekday: "short" })}
  </span>
  {/* Year shown only when the displayed year differs from today's (keeps normal use minimal) */}
  {yy !== Number(today.slice(0, 4)) && (
    <span className="text-[11px] font-medium text-text-subtle">{yy}</span>
  )}
</button>
```

- ko hero: `8월 1일` + `토요일` · en hero: `Aug 1` + `Sat`.
- ±1-day chevrons flank the hero, unchanged (`ChevronLeft`/`ChevronRight`, 16px,
  `opacity-40` → `hover:opacity-100`, `shiftDate(∓1)`). Now they read as flanking the
  hero rather than competing with a year fragment.
- Right icon cluster (Layers / Search / Settings) — unchanged.
- `wdKo` helper reused for the weekday index; remove the now-unused bare `wdKo` render
  (it collapses into the caption string above).

### 5.2 Row 2 — week strip (compliance fix only)

Keep the 7-day strip (clean per audit; today's filled circle is the anchor). **Fix the
§2.1 violation** in the today cell — move the inline `var(--color-*)` reads to utilities
+ a component token:

```tsx
<span
  className={`flex h-7 w-7 items-center justify-center rounded-full text-[13px] font-semibold transition ${
    isToday
      ? "bg-interactive-primary text-interactive-primary-foreground shadow-[var(--shadow-today-node)]"
      : "text-text-muted"
  }`}
>
  {dayNum}
</span>
```

- `background` → `bg-interactive-primary`
- text color (today) → `text-interactive-primary-foreground`
- text color (other) → `text-text-muted` (was inline `var(--color-text-muted)`)
- glow → `shadow-[var(--shadow-today-node)]` (component token; see §6)
- The conditional now reads `--shadow-today-node` (a component token), not `--color-*`.
  All three forbidden reads are gone.

Weekday-label active color stays `text-interactive-primary` (already a utility). Day-cell
structure, category dots, `setDate`+`setView` behavior — unchanged.

### 5.3 Row 3 — view tabs (§6.10 underline tabs)

Replace the segmented control with the canonical underline tab pattern:

```tsx
<div role="tablist" className="flex gap-1 border-b border-line">
  {tabs.map((tb) => {
    const on = view === tb.key;
    return (
      <button
        key={tb.key}
        role="tab"
        aria-selected={on}
        onClick={() => setView(tb.key)}
        className={`-mb-px border-b-2 px-3 py-2 text-[13px] transition ${
          on
            ? "border-interactive-primary text-text font-semibold"
            : "border-transparent text-text-muted font-medium hover:text-text"
        }`}
      >
        {tb.label}
      </button>
    );
  })}
</div>
```

- Track: `border-b border-line`. Active: `border-interactive-primary` underline +
  `text-text font-semibold`. Inactive: `text-text-muted font-medium`, transparent
  border, `hover:text-text`.
- The filled sunken track + raised-pill shadow are removed → one less heavy band.
- `gap-4` (16px) between tabs; left-aligned. Left padding inherits the strip's `px-4`.

## 6. Token layer — `styles.css` (one addition)

Add a **component token** for the today-node glow (folds the dynamic shadow out of
component code, per the adherence fix):

```css
:root {
  --shadow-today-node: 0 2px 8px oklch(0.45 0.14 250 / 0.35);
}
.dark {
  --shadow-today-node: 0 2px 8px oklch(0.70 0.14 250 / 0.40);
}
```

(`oklch(...)` lives only in the token layer — compliant. The component references it via
`shadow-[var(--shadow-today-node)]`.)

## 7. Geometry / spacing summary

| Band | Content | Notes |
|------|---------|-------|
| Titlebar (drag) | lights(56px) · `‹` · `8월 1일` 20px SUITE + `토요일` 12px · `›` … icons | hero anchored after lights |
| Week strip | 7 day cells · today filled circle | §2.1 fix only |
| View tabs | underline tabs, left-aligned, `border-b` | §6.10 |

No change to `App.tsx`, `DayTimeline.tsx`, `BlockView.tsx`, or the OxideBar (already in
the timeline card). No change to `tauri.conf.json` window size.

## 8. i18n

No new keys. Weekday caption: ko uses the existing weekday array + `"요일"` suffix;
en uses `toLocaleDateString("en-US", { weekday: "short" })`. Year caption is numeric.

## 9. Non-goals

- `DayTimeline`, `BlockView`, `NowLine`, `OxideBar` — not touched.
- No DnD, keyboard, or palette changes.
- WeekView / BacklogView / ReportView — not redesigned.
- No animation library; transitions via CSS + existing motion tokens (`--duration-fast`).

## 10. Risks

- **Underline-tab weight vs. segmented pill.** Underline tabs are subtler; the active
  state relies on `border-interactive-primary` + `font-semibold` + `text-text`. If the
  contrast reads too quiet at 420px, bump active to `font-bold` — but start spec-faithful.
- **Weekday-caption redundancy** with the week strip's `토` header. Acceptable: the hero
  caption gives full `토요일`; the strip gives the at-a-glance week. Standard in calendar
  apps (Cron/Things show both).
- **Year-only-when-different** is a minor conditional; verify it renders nothing on a
  normal same-year day (no layout shift).
