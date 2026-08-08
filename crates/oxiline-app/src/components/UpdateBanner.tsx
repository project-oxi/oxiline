import { useTranslation } from "react-i18next";
import { Download, Settings, X } from "lucide-react";
import { useUpdate } from "../lib/updater";
import { useUi } from "../lib/store";

/** Slim top banner shown when the launch/periodic auto-check finds a newer
 *  GitHub release. One-click install (download + relaunch), or open
 *  Preferences for the full release notes. Dismissable until the next check. */
export function UpdateBanner() {
  const { t } = useTranslation();
  const { status, install, reset } = useUpdate();
  const setPreferencesOpen = useUi((s) => s.setPreferencesOpen);
  if (status.kind !== "available") return null;
  return (
    <div className="flex items-center gap-2 border-b border-interactive-primary/30 bg-interactive-primary-subtle px-4 py-1.5 text-[12px]">
      <Download size={13} className="shrink-0 text-interactive-primary" />
      <span className="flex-1 text-text-muted">
        {t("updater.available", { version: status.version })}
      </span>
      <button
        className="rounded bg-interactive-primary px-2.5 py-1 font-medium text-interactive-primary-foreground"
        onClick={() => void install()}
      >
        {t("updater.install")}
      </button>
      <button
        className="rounded p-1 text-text-subtle hover:bg-surface-sunken"
        aria-label={t("settings.title")}
        onClick={() => setPreferencesOpen(true)}
      >
        <Settings size={14} />
      </button>
      <button
        className="rounded p-1 text-text-subtle hover:bg-surface-sunken"
        aria-label={t("common.close")}
        onClick={reset}
      >
        <X size={13} />
      </button>
    </div>
  );
}
