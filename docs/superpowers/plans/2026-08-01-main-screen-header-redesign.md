# Main-Screen Header Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans (inline). Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the OxiLine main-screen header more minimal/clean, better-utilize the titlebar, and fix two design-system deviations — by refining `Header.tsx` + adding one token to `styles.css`.

**Architecture:** Pure presentational change to a single React component (`Header.tsx`). Three edits: (1) titlebar date hero — drop the blue-year fragment, render a clean 2-part hero anchored after the traffic lights; (2) view tabs — replace the filled segmented control with §6.10 underline tabs; (3) week-strip today indicator — move inline `--color-*` reads to utilities + a new `--shadow-today-node` component token. No behavior, routing, data, or DnD changes.

**Tech Stack:** React 19, Tailwind v4 (`@theme inline` tokens), TypeScript 5, oxi-design-system OKLCH tokens, lucide-react icons, react-i18next.

**Spec:** `docs/superpowers/specs/2026-08-01-main-screen-header-redesign-design.md`

## Global Constraints

- 420px-wide Tauri overlay window (min 360px); `titleBarStyle: Overlay`, native traffic lights. `data-tauri-drag-region` + the `no-drag` CSS rule on interactive children are preserved.
- Components consume Tailwind utilities / component tokens only — never `--color-*` semantic vars in component code (DESIGN.md §2.1).
- `.dark` is the single dark trigger; no `dark:` variant in components.
- Korean + English via existing `lang` (`i18n.language.startsWith("en") ? "en" : "ko"`). No new i18n keys.
- All behavior preserved: `shiftDate`, `goToToday`, `setDate`+`setView("today")` (week strip), `setView` (tabs), icon actions, keyboard shortcuts.

## File Structure

- **Modify** `crates/oxiline-app/src/components/Header.tsx` — all three presentational edits.
- **Modify** `crates/oxiline-app/src/styles.css` — add `--shadow-today-node` component token (light + dark).
- No new files. No changes to `App.tsx`, `DayTimeline.tsx`, `BlockView.tsx`, `NowLine.tsx`, `OxideBar.tsx`, `tauri.conf.json`, or i18n JSON.

## Verification approach

This is a pure UI/className change with **no new behavioral contract**, so per project test guidance the proof is a **browser visual check** (drive the live render) + **typecheck** + the **existing test suite** (regression guard). No new unit tests (would test source styling, not behavior).

---

### Task 1: Add the `--shadow-today-node` component token

**Files:**
- Modify: `crates/oxiline-app/src/styles.css` (component-token block, `:root` + `.dark`)

**Interfaces:**
- Produces: `--shadow-today-node` (a box-shadow value) referenced by Header's today circle as `shadow-[var(--shadow-today-node)]`.

- [ ] **Step 1: Add the token to `:root`** (after the existing `--tl-tick-color` line in the component-token block):

```css
  /* Today-node glow for the week-strip circle (DESIGN.md §6.10 / header-redesign) */
  --shadow-today-node: 0 2px 8px oklch(0.45 0.14 250 / 0.35);
```

- [ ] **Step 2: Add the dark override** (inside the existing `.dark { ... }` component-token block, after `--tl-tick-color`):

```css
  --shadow-today-node: 0 2px 8px oklch(0.70 0.14 250 / 0.40);
```

- [ ] **Step 3: Verify token compiles** — `bun run build` is heavy; instead confirm via the dev render in Task 4. No separate step.

---

### Task 2: Refactor `Header.tsx` — hero date, underline tabs, week-strip token fix

**Files:**
- Modify: `crates/oxiline-app/src/components/Header.tsx` (date button ~L50-65, today circle ~L135-148, tabs ~L159-175)

**Interfaces:**
- Consumes: `useUi` (`date, view, setView, setDate, shiftDate, goToToday, setPaletteOpen, setPreferencesOpen, setRoutineManagerOpen`), `useTimelineRange`, `useCategories`, `todayStr`, `shift`, `categoryById`, `categoryColor`, lucide icons, `useTranslation`. All already imported.
- Produces: same `Header()` export, same props (none). No external interface change.

- [ ] **Step 1: Replace the date button (titlebar) with the 2-part hero.** Swap the current date button (`yy` span + `mm/dd` span + weekday span) for:

```tsx
<button
  onClick={goToToday}
  className="flex items-baseline gap-1.5 rounded px-1 hover:bg-surface-sunken"
  title={t("nav.today")}
>
  <span className="text-[18px] font-semibold tracking-tight text-text">
    {lang === "ko"
      ? `${mm}월 ${dd}일`
      : titleDt.toLocaleDateString("en-US", { month: "short", day: "numeric" })}
  </span>
  <span className="text-[12px] font-medium text-text-muted">
    {lang === "ko"
      ? wdKo + "요일"
      : titleDt.toLocaleDateString("en-US", { weekday: "short" })}
  </span>
  {yy !== Number(today.slice(0, 4)) && (
    <span className="text-[11px] font-medium text-text-subtle">{yy}</span>
  )}
</button>
```

`wdKo` is the existing `["일","월","화","수","목","금","토"][titleDt.getDay()]` const — keep it; reuse here. Drop the old `max-[379px]:hidden` year span and the separate weekday span.

- [ ] **Step 2: Fix the week-strip today circle (§2.1).** Replace the inline-`style` today circle with a utility-driven one. The `<span>` for the day number becomes:

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

This removes the three inline `var(--color-*)` reads (`--color-interactive-primary`, `-foreground`, `--color-text-muted`) and the inline `color-mix` shadow. The `categoryColor(h)` dot below is untouched (legit dynamic value).

- [ ] **Step 3: Replace the segmented tabs with §6.10 underline tabs.** Swap the filled-track container + pill buttons for:

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

- [ ] **Step 4: Typecheck** — Run: `bun x tsc -b` (or `bun run build`'s tsc stage). Expected: no errors. The `wdKo` const stays used; `today` is already imported from `../lib/store`.

- [ ] **Step 5: Commit**

```bash
git add crates/oxiline-app/src/components/Header.tsx crates/oxiline-app/src/styles.css
git commit -m "style(header): hero date + underline tabs + token-compliant today node"
```

---

### Task 3: Visual verification + cleanup

**Files:**
- Temporary (verify-only): `crates/oxiline-app/audit/` (mock harness — already exists)
- Remove before finishing: `crates/oxiline-app/audit/`

- [ ] **Step 1: Render the redesigned header in the audit harness.** The audit harness (`audit/index.html` → mocks `__TAURI_INTERNALS__`) already boots the real app at `http://localhost:1420/audit/index.html`. Drive it: open in browser, wait for boot, screenshot `#app-shell`.

- [ ] **Step 2: Inspect the screenshot** via `inspect_image` — confirm: (a) date is a clean 2-part hero (`8월 1일` + `토요일`), no separate blue year; (b) tabs are underline style (no filled pill track); (c) today circle still filled blue with glow; (d) layout balanced. Also screenshot dark mode (toggle `localStorage['oxi-theme']='dark'` + reload) to confirm parity.

- [ ] **Step 3: Regression test** — Run: `bun run test`. Expected: existing suite (`timeline-math`, `sanity`) still passes (changes don't touch tested code).

- [ ] **Step 4: Remove the audit scaffolding** — `rm -rf crates/oxiline-app/audit`. It is dev-only scaffolding, never a production build input (`vite.config.ts rollupOptions.input` = `index.html` + `hud.html` only). Confirm `git status` shows it untracked / gone.

- [ ] **Step 5: Final commit (cleanup + plan)** — commit the plan doc; the `audit/` removal needs no commit (untracked).

```bash
git add docs/superpowers/plans/2026-08-01-main-screen-header-redesign.md
git commit -m "docs(plan): main-screen header redesign implementation plan"
```

## Self-review checklist (done inline before execution)

- **Spec coverage:** §5.1 hero date → Task 2 step 1 ✓ · §5.2 week-strip token fix → Task 2 step 2 ✓ · §5.3 underline tabs → Task 2 step 3 ✓ · §6 token → Task 1 ✓.
- **Placeholders:** none.
- **Type/name consistency:** `wdKo` reused (defined + consumed in same file); `--shadow-today-node` defined (Task 1) before consumed (Task 2 step 2); `today`, `shift`, `categoryById`, `categoryColor` already imported.
