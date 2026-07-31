# OxiLine UI 리디자인 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refine the DayTimeline surface toward a flat-premium minimal aesthetic — flatten block elevation, add a category accent rail, soften the outer card/spine, give the header breathing room, and make the NowLine label legible — without regressing the already-solved oxi design system.

**Architecture:** Extend styles.css Tier-3 component-token layer with a timeline-specific token set (elevation ladder, flat block fill/border, rail, spine, tick). The four component edits consume these tokens purely via `var()` / Tailwind arbitrary-value utilities — they share only the CSS-variable contract, so after Task 1 they are fully independent and parallelizable.

**Tech Stack:** React 19 + TypeScript, Tailwind v4 (`@theme inline`), Tauri v2 (Rust shell, unchanged), oxi-design-system OKLCH tokens.

## Global Constraints

- **No regression:** do NOT reintroduce hour/30-min gridlines (user dislikes them). Do NOT revert styles.css fonts/tokens (SUIT/SUITE/Geist Mono, 6-hue palette, `.dark` class) back toward the superseded `06-design-system.md` (Pretendard/JetBrains/verdigris).
- **oxi component rules:** consume Tailwind utilities or canonical semantic `var(--color-*)`. No `dark:` variant in component files. No `onMouseEnter/Leave` for hover — CSS/Tailwind `hover:` only. OKLCH only (already in token layer).
- **Verification is build + visual, not unit tests** — these are CSS/JSX structural changes with no new behavior. Each task's gate is `bun run build` (tsc+vite) passing; final gate adds a mocked-Tauri browser screenshot.
- **Rust is untouched** — no `src-tauri/` edits. `cargo build` is a smoke check only.
- Exact token names below are the cross-task contract — every consumer must spell them identically.

### Token contract (defined in Task 1, consumed by Tasks 2-5)

| Token | Value (light) | Value (dark) | Consumer |
|---|---|---|---|
| `--shadow-card` | `var(--shadow-sm)` | same | DayTimeline outer card |
| `--shadow-block-rest` | `none` | same | BlockView rest |
| `--shadow-block-hover` | `var(--shadow-md)` | same | BlockView hover |
| `--shadow-block-drag` | `var(--shadow-lg)` | same | BlockView drag |
| `--color-block-bg` | `oklch(96.5% 0.005 95)` | `oklch(24% 0.016 265)` | BlockView fill |
| `--color-block-border` | `oklch(90% 0.007 95)` | `oklch(33% 0.015 265)` | BlockView hairline |
| `--tl-rail-width` | `3px` | same | BlockView rail |
| `--tl-spine-width` | `1px` | same | DayTimeline spine |
| `--tl-tick-color` | `oklch(0% 0 0 / 0.06)` | `oklch(100% 0 0 / 0.05)` | DayTimeline hour tick |

`--color-block-bg` and `--color-block-border` are also exposed in `@theme inline`.

---

### Task 1: Timeline token layer in styles.css

**Files:**
- Modify: `crates/oxiline-app/src/styles.css` (Tier-3 `:root` block at lines ~138-178; `.dark` shadow block ~179-184; `@theme inline` ~190-251)

**Interfaces:**
- Consumes: existing `--shadow-sm/md/lg`, `--radius-*` (all already defined).
- Produces: every token in the contract table above — exact spellings.

- [ ] **Step 1: Add timeline tokens to the Tier-3 `:root` component block**

Append to the `:root { ... }` that holds radius/shadow/motion tokens (after the `--ease-in-out` line, before the closing `}` at ~line 178):

```css
  /* ── Timeline surface (UI-redesign 2026-07-31, spec §3.1) ─────────────── */
  --shadow-card:        var(--shadow-sm);   /* outer timeline card (was shadow-lg) */
  --shadow-block-rest:  none;               /* flat blocks — separation via hairline */
  --shadow-block-hover: var(--shadow-md);   /* hover lift */
  --shadow-block-drag:  var(--shadow-lg);   /* dragging */

  --color-block-bg:      oklch(96.5% 0.005 95);
  --color-block-border:  oklch(90% 0.007 95);

  --tl-rail-width:   3px;
  --tl-spine-width:  1px;
  --tl-tick-color:   oklch(0% 0 0 / 0.06);
```

- [ ] **Step 2: Add dark-mode parity in the `.dark { ... }` shadow block**

Append to the `.dark` block at ~line 179-184 (after `--shadow-lg` dark line, before closing `}`):

```css
  /* Timeline tokens — dark parity (UI-redesign) */
  --color-block-bg:     oklch(24% 0.016 265);
  --color-block-border: oklch(33% 0.015 265);
  --tl-tick-color:      oklch(100% 0 0 / 0.05);
```

(`--shadow-card/block-*` resolve through `var(--shadow-*)` which already have dark overrides, so no dark re-declaration needed.)

- [ ] **Step 3: Expose the two block color tokens in `@theme inline`**

In the `@theme inline { ... }` block, add alongside the other surface/border exposures (e.g. after `--color-focus-ring` ~line 206):

```css
  /* Block surface (UI-redesign) */
  --color-block-bg: var(--color-block-bg);
  --color-block-border: var(--color-block-border);
```

- [ ] **Step 4: Build to verify CSS parses**

Run: `cd crates/oxiline-app && bun run build`
Expected: Vite build succeeds (Tailwind v4 compiles the new tokens without error).

- [ ] **Step 5: Commit**

```bash
git add crates/oxiline-app/src/styles.css
git commit -m "style(oxiline): add timeline token layer — flat block elevation, rail, spine, tick

Extends Tier-3 component tokens (no regression): shadow-card softens the
outer card; shadow-block-{rest,hover,drag} gives a 3-step elevation
ladder; color-block-{bg,border} provide flat separation without shadow;
tl-rail-width/spine-width/tick-color drive the refined timeline chrome."
```

---

### Task 2: BlockView — flat fill + accent rail + elevation ladder

**Files:**
- Modify: `crates/oxiline-app/src/components/BlockView.tsx` (style object ~85-109; JSX root + content ~136-194)

**Interfaces:**
- Consumes: `--color-block-bg`, `--color-block-border`, `--color-border-strong`, `--shadow-block-rest/hover/drag`, `--tl-rail-width`, `--color-status-success` (Task 1).
- Produces: none (leaf component).

- [ ] **Step 1: Flatten the fill + border in the `style` object**

In `BlockView.tsx`, the `style` object (line 85) currently sets:
```ts
    background: item.is_done
      ? "color-mix(in oklch, var(--color-status-success) 6%, var(--color-surface-raised))"
      : `color-mix(in oklch, ${accent} 8%, var(--color-surface-raised))`,
```
Replace that `background:` value with:
```ts
    background: item.is_done
      ? "color-mix(in oklch, var(--color-status-success) 8%, var(--color-block-bg))"
      : "var(--color-block-bg)",
```
Then add (keep the existing `top`, `height`, `left`, `width`, `opacity`, `transform`, `zIndex`, `transition`, `cursor` lines unchanged) a drag-state shadow — change the existing `transform:` line:
```ts
    transform: isDragging
      ? `${CSS.Translate.toString(transform)} scale(1.02)`
      : CSS.Translate.toString(transform),
```
and add right after `zIndex:`:
```ts
    boxShadow: isDragging ? "var(--shadow-block-drag)" : undefined,
```

- [ ] **Step 2: Switch the root className from shadow utilities to the token ladder + hairline border**

The root `<div>` (line 140) currently has:
```tsx
      className={`absolute overflow-hidden rounded-lg ${
        isDragging
          ? "shadow-lg"
          : "shadow-sm hover:shadow-lg"
      }`}
```
Replace with a static className (drag shadow is now driven by inline `boxShadow` from Step 1, which beats the class):
```tsx
      className="absolute overflow-hidden rounded-lg border border-[var(--color-block-border)] hover:border-[var(--color-border-strong)] shadow-[var(--shadow-block-rest)] hover:shadow-[var(--shadow-block-hover)]"
```

- [ ] **Step 3: Add the category accent rail as the first child of the root div**

Immediately after the root `<div ...>` opening tag (before the content `<div role="presentation" ...>`), insert:
```tsx
      <div
        aria-hidden
        className="absolute left-0 top-0 bottom-0"
        style={{ width: "var(--tl-rail-width)", background: accent }}
      />
```
(`accent` is already computed at line 75: `item.is_done ? "var(--color-status-success)" : color`. Done → green rail; past-undone dims via the existing whole-block `opacity: 0.55`.)

- [ ] **Step 4: Build to verify**

Run: `cd crates/oxiline-app && bun run build`
Expected: success (TS + Vite). No type errors — no new props, only style/className/JSX edits.

- [ ] **Step 5: Commit**

```bash
git add crates/oxiline-app/src/components/BlockView.tsx
git commit -m "style(oxiline): flatten block cards + add category accent rail

Blocks use a flat tinted fill + 1px hairline (rest) that brightens on hover,
lifting to shadow-md; dragging scales 1.02 + shadow-lg. A 3px left rail
encodes category/done color (color-is-data), reducing reliance on shadow
for separation. Resolves scannability + shadow-heaviness gaps (spec §3.2)."
```

---

### Task 3: DayTimeline — soften outer card, refine spine, add hour ticks

**Files:**
- Modify: `crates/oxiline-app/src/components/DayTimeline.tsx` (outer card ~126-129; spine ~145-148; hours label map ~150-171)

**Interfaces:**
- Consumes: `--shadow-card`, `--tl-spine-width`, `--tl-tick-color`, `--color-border` (Task 1).
- Produces: none.

- [ ] **Step 1: Soften the outer card shadow**

Line 128 currently:
```tsx
        style={{ background: "var(--color-surface-raised)", boxShadow: "var(--shadow-lg)" }}
```
Change `boxShadow` value to `"var(--shadow-card)"`:
```tsx
        style={{ background: "var(--color-surface-raised)", boxShadow: "var(--shadow-card)" }}
```

- [ ] **Step 2: Thin the spine to 1px, keep it centered on SPINE_X**

Lines 145-148 currently:
```tsx
            <div
              className="pointer-events-none absolute"
              style={{ left: SPINE_X - 1, top: 0, bottom: 0, width: 2, background: "var(--color-border)" }}
            />
```
Replace with (1px width, recentered; `SPINE_X - 0.5` keeps the 1px line centered on SPINE_X so the dots at `SPINE_X - 7` still align):
```tsx
            <div
              className="pointer-events-none absolute"
              style={{ left: SPINE_X - 0.5, top: 0, bottom: 0, width: "var(--tl-spine-width)", background: "var(--color-border)" }}
            />
```

- [ ] **Step 3: Add subtle hour tick notches on the spine**

Immediately AFTER the hours-label map closes (after the `})` at line 171) and before the `{/* spine nodes */}` comment (line 173), insert a new map:
```tsx
            {/* subtle hour tick notches — rhythm without gridlines (spec §3.3) */}
            {hours.map((h) => {
              const top = (h * 60 - dayStartMin) * pxPerMin;
              return (
                <div
                  key={`tick-${h}`}
                  className="pointer-events-none absolute"
                  style={{ left: SPINE_X - 6, top, width: 6, height: 1, background: "var(--tl-tick-color)" }}
                />
              );
            })}
```

- [ ] **Step 4: Build to verify**

Run: `cd crates/oxiline-app && bun run build`
Expected: success.

- [ ] **Step 5: Commit**

```bash
git add crates/oxiline-app/src/components/DayTimeline.tsx
git commit -m "style(oxiline): soften timeline card, thin spine to 1px, add hour ticks

Outer card shadow lg→sm (shadow-card) flattens the surface; spine 2px→1px
recentered on SPINE_X; faint 6px hour tick notches on the spine give rhythm
without reintroducing the gridlines the user disliked (spec §3.3)."
```

---

### Task 4: NowLine — legible time label pill

**Files:**
- Modify: `crates/oxiline-app/src/components/NowLine.tsx` (label span ~79-83)

**Interfaces:**
- Consumes: `--color-surface-raised`, `--shadow-xs`, `--radius-xs`, `--color-interactive-primary` (existing).
- Produces: none.

- [ ] **Step 1: Give the HH:MM label a background pill**

Lines 79-83 currently:
```tsx
        <span
          ref={labelRef}
          className="absolute font-mono text-[10px]"
          style={{ left: 10, top: -7, color: "var(--color-interactive-primary)" }}
        />
```
Replace with (pill: raised bg + xs shadow + radius + padding; nudge top to vertically center given the padding):
```tsx
        <span
          ref={labelRef}
          className="absolute font-mono text-[10px] font-medium"
          style={{
            left: 10,
            top: -9,
            padding: "1px 5px",
            borderRadius: "var(--radius-xs)",
            background: "var(--color-surface-raised)",
            boxShadow: "var(--shadow-xs)",
            color: "var(--color-interactive-primary)",
          }}
        />
```

- [ ] **Step 2: Build to verify**

Run: `cd crates/oxiline-app && bun run build`
Expected: success.

- [ ] **Step 3: Commit**

```bash
git add crates/oxiline-app/src/components/NowLine.tsx
git commit -m "style(oxiline): give NowLine time label a legible pill

The bare mono HH:MM collided with block titles when crossing a block. A
raised-bg + xs-shadow pill keeps it readable over any surface (spec §3.4)."
```

---

### Task 5: Header — top breathing room

**Files:**
- Modify: `crates/oxiline-app/src/components/Header.tsx` (container line 37)

**Interfaces:**
- Consumes: none new.
- Produces: none.

- [ ] **Step 1: Add top padding for traffic-light clearance**

Line 37 currently:
```tsx
    <div className="shrink-0 select-none px-4 pb-2">
```
Change to add `pt-2`:
```tsx
    <div className="shrink-0 select-none px-4 pb-2 pt-2">
```

- [ ] **Step 2: Build to verify**

Run: `cd crates/oxiline-app && bun run build`
Expected: success.

- [ ] **Step 3: Commit**

```bash
git add crates/oxiline-app/src/components/Header.tsx
git commit -m "style(oxiline): add header top padding for traffic-light breathing room

Overlay titlebar content previously started ~6px from the window top. pt-2
gives the date title + tool buttons clearance beneath the traffic lights
without changing the drag region (spec §3.5)."
```

---

### Task 6: Verify — build, Rust smoke, mocked-Tauri visual

**Files:**
- None modified (verification only).

- [ ] **Step 1: Clean frontend build**

Run: `cd crates/oxiline-app && bun run build`
Expected: success, no TS errors, `dist/` emitted.

- [ ] **Step 2: Rust smoke (workspace still compiles — no Rust was touched)**

Run: `cargo build` (from repo root)
Expected: success.

- [ ] **Step 3: Regression grep — no gridlines reintroduced, no forbidden patterns**

Confirm via search:
- `color-mix.*accent 8%` no longer present in BlockView (flattened).
- No `data-theme` / `dark:` variant added in the four component files.
- styles.css still references SUIT/SUITE/Geist Mono (fonts unchanged).

- [ ] **Step 4: Visual proof — mocked-Tauri browser screenshot**

Run the vite dev server, mock `window.__TAURI_INTERNALS__` with seed timeline data (per `tauri-v2-browser-audit-mock` skill), and screenshot the DayTimeline. Confirm visually:
- Blocks are flat with a visible colored left rail; hover lifts them.
- Outer card shadow is soft (not heavy lg).
- Spine is a hairline with faint hour ticks.
- Header has top breathing room.
- NowLine label (if visible) sits in a pill.

If the mock path is infeasible (white-screen), fall back to `cargo tauri dev` manual check and record the outcome.

- [ ] **Step 5: (No commit — verification only)**

---

## Self-Review (completed)

- **Spec coverage:** §3.1→Task1, §3.2→Task2, §3.3→Task3, §3.4→Task4, §3.5→Task5, §5(acceptance)→Task6. All spec sections mapped.
- **Placeholder scan:** none — every step has exact code or exact search/build commands.
- **Type/name consistency:** token spellings identical across Task 1 (definition) and Tasks 2-5 (consumption); `accent`, `SPINE_X`, `CSS`, `transform`, `isDragging` are all pre-existing in BlockView. No new symbols introduced downstream.
- **Execution order:** Task 1 is a hard prerequisite (defines tokens). Tasks 2-5 touch disjoint files and share only the CSS-var contract → safe to run in parallel after Task 1. Task 6 runs last.
