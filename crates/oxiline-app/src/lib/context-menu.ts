// App-native context menu store + viewport clamp helper.
//
// One store, one component: elements opt in by attaching `onContextMenu` that
// calls `show(clientX, clientY, items)`. The single <ContextMenu/> rendered at
// the App root (body portal) consumes this store. See
// docs/superpowers/specs/2026-08-05-context-menu-and-hud-design.md §A.

import { create } from "zustand";
import type { ComponentType } from "react";

export interface MenuEntry {
  kind: "item";
  label: string;
  icon?: ComponentType<{ size?: number; className?: string }>;
  onSelect: () => void;
  danger?: boolean;
  disabled?: boolean;
}
/** A non-interactive divider. */
export interface MenuSeparator {
  kind: "separator";
}
/** A small uppercase section label (usually the target's identity). */
export interface MenuHeader {
  kind: "header";
  label: string;
}
export type MenuItem = MenuEntry | MenuSeparator | MenuHeader;

interface ContextMenuState {
  x: number;
  y: number;
  items: MenuItem[];
  open: boolean;
  /** Open the menu at viewport coordinates with the given items. Replaces any
   *  menu already open. */
  show: (x: number, y: number, items: MenuItem[]) => void;
  close: () => void;
}

export const useContextMenu = create<ContextMenuState>((set) => ({
  x: 0,
  y: 0,
  items: [],
  open: false,
  show: (x, y, items) => set({ x, y, items, open: true }),
  close: () => set({ open: false, items: [] }),
}));

/**
 * Clamp a menu rectangle so it stays inside the viewport. If the menu would
 * overflow the right or bottom edge, flip it to the opposite side of the
 * cursor; if it still does not fit, clamp it to the edge with `margin` slack.
 * Pure — safe to unit-test in node.
 */
export function clampMenuPosition(
  x: number,
  y: number,
  menu: { width: number; height: number },
  viewport: { width: number; height: number },
  margin = 8,
): { x: number; y: number } {
  const maxX = Math.max(margin, viewport.width - margin - menu.width);
  const maxY = Math.max(margin, viewport.height - margin - menu.height);
  // Prefer opening down-right at the cursor; flip up/left on overflow.
  let nx = x + menu.width > viewport.width - margin ? x - menu.width : x;
  let ny = y + menu.height > viewport.height - margin ? y - menu.height : y;
  // Final clamp: never exceed the viewport, never cross the margin.
  nx = Math.min(Math.max(margin, nx), maxX);
  ny = Math.min(Math.max(margin, ny), maxY);
  return { x: nx, y: ny };
}
