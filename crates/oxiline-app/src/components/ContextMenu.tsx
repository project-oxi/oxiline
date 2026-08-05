/**
 * ContextMenu — the single app-native right-click menu.
 *
 * Rendered once at the App root via a body portal. Consumes the
 * `useContextMenu` store; elements open it via `show(clientX, clientY, items)`.
 * Viewport-clamped (flips on overflow), keyboard-navigable, closes on outside
 * pointer / blur / scroll / resize. See
 * docs/superpowers/specs/2026-08-05-context-menu-and-hud-design.md §A.
 */
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useContextMenu, clampMenuPosition, type MenuItem } from "../lib/context-menu";

/** Indices of actionable rows (item kind, not disabled). */
function selectableIndices(items: MenuItem[]): number[] {
  return items
    .map((it, i) => (it.kind === "item" && !it.disabled ? i : -1))
    .filter((i) => i >= 0);
}

export function ContextMenu() {
  const { x, y, items, open, close } = useContextMenu();
  const panelRef = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ x, y });
  const [sel, setSel] = useState(0);
  const selectable = useMemo(() => selectableIndices(items), [items]);

  // Measure + clamp on open. useLayoutEffect so the menu never paints in the
  // wrong place (e.g. past the bottom-right edge) before flipping.
  useLayoutEffect(() => {
    if (!open) return;
    const el = panelRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const clamped = clampMenuPosition(
      x,
      y,
      { width: rect.width, height: rect.height },
      { width: window.innerWidth, height: window.innerHeight },
    );
    setPos(clamped);
    setSel(0);
  }, [open, x, y, items]);

  // Close on outside pointerdown, Escape, window blur, scroll, resize.
  useEffect(() => {
    if (!open) return;
    const onPointer = (e: PointerEvent) => {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) close();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        close();
      } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault();
        if (selectable.length === 0) return;
        const cur = selectable.indexOf(sel);
        const next =
          e.key === "ArrowDown"
            ? selectable[(cur + 1) % selectable.length]
            : selectable[(cur - 1 + selectable.length) % selectable.length];
        setSel(next);
      } else if (e.key === "Enter") {
        const entry = items[sel];
        if (entry && entry.kind === "item" && !entry.disabled) {
          e.preventDefault();
          close();
          entry.onSelect();
        }
      }
    };
    const onBlur = () => close();
    const onScrollOrResize = () => close();
    document.addEventListener("pointerdown", onPointer, true);
    document.addEventListener("keydown", onKey, true);
    window.addEventListener("blur", onBlur);
    window.addEventListener("scroll", onScrollOrResize, true);
    window.addEventListener("resize", onScrollOrResize);
    return () => {
      document.removeEventListener("pointerdown", onPointer, true);
      document.removeEventListener("keydown", onKey, true);
      window.removeEventListener("blur", onBlur);
      window.removeEventListener("scroll", onScrollOrResize, true);
      window.removeEventListener("resize", onScrollOrResize);
    };
  }, [open, sel, items, selectable, close]);

  if (!open) return null;

  return createPortal(
    <div
      ref={panelRef}
      className="context-menu fixed z-[60] min-w-[190px] select-none rounded-lg border border-border bg-surface-raised p-1 text-text shadow-[var(--shadow-lg)]"
      style={{ left: pos.x, top: pos.y }}
      onContextMenu={(e) => e.preventDefault()}
    >
      {items.map((it, i) => {
        if (it.kind === "separator") return <div key={i} className="my-1 h-px bg-border" />;
        if (it.kind === "header")
          return (
            <div
              key={i}
              className="px-2.5 pb-0.5 pt-1 text-[10px] font-semibold uppercase tracking-wide text-text-subtle"
            >
              {it.label}
            </div>
          );
        const Icon = it.icon;
        const active = i === sel;
        return (
          <button
            key={i}
            type="button"
            disabled={it.disabled}
            onMouseEnter={() => setSel(i)}
            onClick={(e) => {
              e.stopPropagation();
              close();
              it.onSelect();
            }}
            className={`flex w-full items-center gap-2 rounded px-2.5 py-1.5 text-left text-[13px] transition-colors disabled:pointer-events-none disabled:opacity-40 ${
              it.danger ? "text-status-error" : "text-text"
            } ${active ? "bg-surface-sunken" : ""} ${it.danger && active ? "bg-status-error-subtle" : ""} hover:bg-surface-sunken`}
          >
            {Icon && <Icon size={14} className={it.danger ? "text-status-error" : "text-text-subtle"} />}
            <span className="flex-1 truncate">{it.label}</span>
          </button>
        );
      })}
    </div>,
    document.body,
  );
}
