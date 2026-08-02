//! Reusable modal/dialog wrapper: backdrop + focus trap + dialog semantics +
//! Escape-to-close + focus restoration. Replaces the hand-rolled overlay
//! pattern duplicated across Preferences / Onboarding /
//! CommandPalette (`08-roadmap.md` Phase 2 — 접근성 감사).

import { useRef, type ReactNode, type CSSProperties } from "react";
import { useFocusTrap } from "../lib/a11y";

type Variant = "center" | "top" | "fullscreen" | "drawer-right";

const OVERLAY: Record<Variant, string> = {
  center: "fixed inset-0 z-40 flex items-center justify-center",
  top: "fixed inset-0 z-50 flex items-start justify-center pt-20",
  fullscreen: "fixed inset-0 z-[60] flex items-center justify-center",
  "drawer-right": "fixed inset-0 z-40 flex justify-end",
};

interface Props {
  open: boolean;
  onClose: () => void;
  children: ReactNode;
  /** Backdrop click + Escape close the modal (default true). Onboarding sets false. */
  dismissable?: boolean;
  variant?: Variant;
  backdropStyle?: CSSProperties;
  panelClassName?: string;
  panelStyle?: CSSProperties;
  /** id of the heading labelling this dialog (preferred over ariaLabel). */
  labelledBy?: string;
  ariaLabel?: string;
}

export function Modal({
  open,
  onClose,
  children,
  dismissable = true,
  variant = "center",
  backdropStyle,
  panelClassName,
  panelStyle,
  labelledBy,
  ariaLabel,
}: Props) {
  const ref = useRef<HTMLDivElement>(null);
  useFocusTrap(ref, open);

  if (!open) return null;

  const backdrop = backdropStyle ?? { background: "oklch(0 0 0 / 0.25)" };

  return (
    <div
      className={OVERLAY[variant]}
      style={backdrop}
      onClick={dismissable ? onClose : undefined}
    >
      <div
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-labelledby={labelledBy}
        aria-label={ariaLabel}
        tabIndex={-1}
        className={panelClassName}
        style={panelStyle}
        onClick={(e) => e.stopPropagation()}
        onKeyDown={(e) => {
          if (dismissable && e.key === "Escape") {
            e.stopPropagation();
            onClose();
          }
        }}
      >
        {children}
      </div>
    </div>
  );
}
