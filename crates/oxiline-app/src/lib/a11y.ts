//! Accessibility helpers: focus trap for modals + a global aria-live announcer.
//! (`08-roadmap.md` Phase 2 — 접근성 감사)

import { useEffect } from "react";

/** CSS selector for all natively focusable / programmatically tabbable elements. */
const FOCUSABLE =
  'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])';

/**
 * Trap keyboard focus inside `ref` while `active`.
 *
 * - On activation: record the previously-focused element, then move focus to the
 *   first focusable child of the container.
 * - Tab / Shift+Tab cycle within the container, never escaping.
 * - On deactivation: restore focus to the element that opened the modal.
 */
export function useFocusTrap(ref: React.RefObject<HTMLElement | null>, active: boolean) {
  useEffect(() => {
    if (!active || !ref.current) return;
    const container = ref.current;

    const previouslyFocused = document.activeElement as HTMLElement | null;

    // Move focus into the dialog on open.
    const enter = () => {
      const first = container.querySelector<HTMLElement>(FOCUSABLE);
      (first ?? container).focus({ preventScroll: true });
    };
    // Defer one frame so the just-mounted subtree is queryable.
    const id = requestAnimationFrame(enter);

    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Tab") return;
      const nodes = Array.from(container.querySelectorAll<HTMLElement>(FOCUSABLE));
      if (nodes.length === 0) {
        e.preventDefault();
        return;
      }
      const first = nodes[0];
      const last = nodes[nodes.length - 1];
      const activeEl = document.activeElement;

      if (e.shiftKey) {
        if (activeEl === first || !container.contains(activeEl)) {
          e.preventDefault();
          last.focus({ preventScroll: true });
        }
      } else {
        if (activeEl === last) {
          e.preventDefault();
          first.focus({ preventScroll: true });
        }
      }
    };

    document.addEventListener("keydown", onKey);
    return () => {
      cancelAnimationFrame(id);
      document.removeEventListener("keydown", onKey);
      // Restore focus to whatever opened the dialog.
      previouslyFocused?.focus?.({ preventScroll: true });
    };
  }, [ref, active]);
}

let liveEl: HTMLDivElement | null = null;

function ensureLiveRegion(): HTMLDivElement {
  if (liveEl) return liveEl;
  const el = document.createElement("div");
  el.setAttribute("aria-live", "polite");
  el.setAttribute("aria-atomic", "true");
  el.setAttribute("role", "status");
  // Visually hidden but readable by assistive tech.
  el.style.cssText =
    "position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap;border:0";
  document.body.appendChild(el);
  liveEl = el;
  return el;
}

/**
 * Announce a message to screen readers via a polite live region.
 * Clears after a beat so repeated identical messages re-announce.
 */
export function announce(message: string) {
  const el = ensureLiveRegion();
  el.textContent = "";
  // Force re-announcement on the next tick.
  requestAnimationFrame(() => {
    el.textContent = message;
  });
}
