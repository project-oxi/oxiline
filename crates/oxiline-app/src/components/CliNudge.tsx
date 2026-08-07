import { useEffect, useState } from "react";
import { Terminal, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useCliStatus, useInstallCli } from "../hooks";

const DISMISS_KEY = "oxiline.cliNudgeDismissed";

/**
 * One-time banner nudging the user to expose the bundled `oxiline` CLI on
 * PATH. Hidden once dismissed (localStorage) or once installed. Only in
 * the real Tauri shell — never in browser/dev mode.
 */
export function CliNudge() {
  const { t } = useTranslation();
  const status = useCliStatus();
  const install = useInstallCli();
  const [dismissed, setDismissed] = useState(false);

  const inTauri =
    typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

  useEffect(() => {
    if (!inTauri) return;
    if (window.localStorage.getItem(DISMISS_KEY) === "1") return;
    // Show unless the CLI is already installed.
    if (status.data !== undefined && status.data !== "installed") {
      setDismissed(false);
    }
  }, [inTauri, status.data]);

  const onInstall = () => {
    install.mutate(undefined, {
      onSuccess: () => {
        setDismissed(true);
        window.localStorage.setItem(DISMISS_KEY, "1");
      },
    });
  };
  const dismiss = () => {
    window.localStorage.setItem(DISMISS_KEY, "1");
    setDismissed(true);
  };

  if (!inTauri) return null;
  if (status.data === undefined) return null; // still loading
  if (status.data === "installed") return null;
  if (dismissed) return null;

  return (
    <div className="pointer-events-none fixed inset-x-0 top-3 z-30 flex justify-center px-4">
      <div className="pointer-events-auto flex w-full max-w-md items-start gap-2.5 rounded-xl border border-line bg-surface px-3.5 py-2.5 shadow-lg">
        <Terminal size={15} className="mt-0.5 shrink-0 text-text-muted" />
        <div className="min-w-0 flex-1">
          <p className="text-xs font-semibold text-text">{t("settings.cliNudgeTitle")}</p>
          <p className="mt-0.5 text-[11px] leading-relaxed text-text-subtle">
            {t("settings.cliNudgeBody")}
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-1.5">
          <button
            type="button"
            onClick={onInstall}
            disabled={install.isPending}
            className="rounded-lg bg-interactive-primary px-2.5 py-1 text-xs font-medium text-interactive-primary-foreground transition-colors hover:bg-interactive-primary/90 disabled:opacity-50"
          >
            {install.isPending ? "…" : t("settings.cliNudgeInstall")}
          </button>
          <button
            type="button"
            onClick={dismiss}
            aria-label={t("settings.cliNudgeDismiss")}
            className="rounded-lg p-1 text-text-subtle transition-colors hover:bg-surface-muted hover:text-text-muted"
          >
            <X size={14} />
          </button>
        </div>
      </div>
    </div>
  );
}
