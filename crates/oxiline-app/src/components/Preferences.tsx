import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Check, ChevronDown, ChevronUp, Terminal, X } from "lucide-react";
import { useTraySlots, useUpdateTraySlots, useSettings, useSetSetting, useCategories, useCreateCategory, useDeleteCategory, useCliStatus, useInstallCli, useUninstallCli } from "../hooks";
import { api } from "../lib/api";
import { useQuery, useMutation } from "@tanstack/react-query";
import { useUi } from "../lib/store";
import { applyTheme, setThemeMode, type ThemeMode } from "../lib/theme";
import { changeLang, type Lang } from "../lib/i18n";
import { categoryColor } from "../lib/colors";
import type { CliState, TraySlotPref } from "../types";
import { Modal } from "./Modal";
import { swapOrder } from "../lib/tray-slot-order";

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between py-2">
      <span className="text-[13px] text-text-muted">{label}</span>
      {children}
    </div>
  );
}

function Select<T extends string>(props: { value: T; options: { v: T; label: string }[]; onChange: (v: T) => void }) {
  return (
    <select
      value={props.value}
      onChange={(e) => props.onChange(e.target.value as T)}
      className="rounded bg-transparent px-2 py-1 text-[12px] shadow-[var(--input-shadow)] focus-visible:shadow-[var(--input-shadow-focus)] focus-visible:outline-none"
    >
      {props.options.map((o) => (
        <option key={o.v} value={o.v}>{o.label}</option>
      ))}
    </select>
  );
}

/** Convert a Tauri accelerator string ("CmdOrCtrl+Shift+O") to display glyphs ("⌘⇧O"). */
function formatHotkey(acc: string): string {
  return acc
    .replace(/CmdOrCtrl|CommandOrCtrl|Super/g, "⌘")
    .replace(/Ctrl|Control/g, "⌃")
    .replace(/Alt|Option/g, "⌥")
    .replace(/Shift/g, "⇧")
    .replace(/\+/g, "");
}

/** Static in-app shortcut rows for the §7.10 reference table. */
const SHORTCUT_ROWS: { key: string; actionKey: string; scopeKey: string }[] = [
  { key: "⌘K", actionKey: "settings.actPalette", scopeKey: "settings.scopeApp" },
  { key: "⌘,", actionKey: "settings.actPrefs", scopeKey: "settings.scopeMain" },
  { key: "T", actionKey: "settings.actToday", scopeKey: "settings.scopeViews" },
  { key: "← / →", actionKey: "settings.actPrevNext", scopeKey: "settings.scopeDay" },
];

/** Surfaces the bundled `oxiline` CLI on $PATH via a one-time macOS admin
 *  prompt. Mirrors `oximemo`'s Settings → "Command-line tool". */
function CliSection() {
  const { t } = useTranslation();
  const status = useCliStatus();
  const install = useInstallCli();
  const uninstall = useUninstallCli();
  const state: CliState = status.data ?? "not-installed";
  const busy = install.isPending || uninstall.isPending;

  const onInstall = () => install.mutate(undefined, { onSuccess: () => status.refetch() });
  const onUninstall = () => uninstall.mutate(undefined, { onSuccess: () => status.refetch() });

  return (
    <div className="space-y-2.5">
      <p className="text-[11px] leading-relaxed text-text-subtle">{t("settings.cliDesc")}</p>
      <div className="flex items-center justify-between rounded-lg bg-surface-sunken px-3 py-2">
        <span className="flex items-center gap-1.5 text-xs text-text-muted">
          {state === "installed" && <Check size={13} className="text-status-success" />}
          {state === "installed" ? t("settings.cliInstalled") : t("settings.cliNotInstalled")}
        </span>
        {state === "installed" ? (
          <button
            type="button"
            onClick={onUninstall}
            disabled={busy}
            className="rounded-lg border border-line px-2 py-1 text-xs font-medium text-text-muted transition-colors hover:bg-surface-muted disabled:opacity-50"
          >
            {busy ? "…" : t("settings.cliUninstall")}
          </button>
        ) : (
          <button
            type="button"
            onClick={onInstall}
            disabled={busy}
            className="rounded-lg border border-line px-2 py-1 text-xs font-medium text-text-muted transition-colors hover:bg-surface-muted disabled:opacity-50"
          >
            {busy
              ? t("settings.cliInstalling")
              : state === "stale"
                ? t("settings.cliReinstall")
                : t("settings.cliInstall")}
          </button>
        )}
      </div>
    </div>
  );
}

export function Preferences() {
  const { t } = useTranslation();
  const { preferencesOpen: open, setPreferencesOpen } = useUi();
  const settingsQ = useSettings();
  const setSetting = useSetSetting();
  const catsQ = useCategories();
  const createCat = useCreateCategory();
  const delCat = useDeleteCategory();
  const permQ = useQuery({
    queryKey: ["notification-permission"],
    queryFn: () => api.isNotificationPermissionGranted(),
    staleTime: 5000,
  });
  const requestPerm = useMutation({
    mutationFn: () => api.requestNotificationPermission(),
    onSuccess: () => permQ.refetch(),
  });
  const [catName, setCatName] = useState("");
  const [catHue, setCatHue] = useState(200);

  const s = settingsQ.data ?? {};

  const changeTheme = (mode: ThemeMode) => {
    setSetting.mutate({ key: "theme", value: mode });
    setThemeMode(mode);
    applyTheme(mode);
  };
  const changeLanguage = (lang: Lang) => {
    setSetting.mutate({ key: "locale", value: lang });
    changeLang(lang);
  };

  // ---- tray-slot preferences section ----
  const traySlots = useTraySlots();
  const updateTraySlots = useUpdateTraySlots();

  function toggleSlot(idx: number) {
    const next = traySlots.map((s, i) => (i === idx ? { ...s, on: !s.on } : s));
    updateTraySlots.mutate(next);
  }

  function moveSlot(idx: number, dir: -1 | 1) {
    updateTraySlots.mutate(swapOrder(traySlots, idx, dir));
  }

  const slotLabel = (kind: TraySlotPref["kind"]) => {
    switch (kind) {
      case "now_recording": return t("settings.slotNowRecording");
      case "now_next": return t("settings.slotNowNext");
      case "state_dot": return t("settings.slotStateDot");
      default: return String(kind);
    }
  };

  return (
    <Modal
      open={open}
      onClose={() => setPreferencesOpen(false)}
      labelledBy="prefs-title"
      panelClassName="max-h-[80%] w-full max-w-lg overflow-y-auto rounded-lg border border-border bg-surface-raised p-5"
    >
        <div className="mb-3 flex items-center justify-between">
          <h2 id="prefs-title" className="text-[16px] font-semibold">{t("settings.title")}</h2>
          <button className="rounded p-1 hover:bg-surface-sunken" onClick={() => setPreferencesOpen(false)} aria-label={t("common.close")}>
            <X size={16} />
          </button>
        </div>

        <section className="mb-4">
          <h3 className="mb-1 text-[12px] font-semibold uppercase text-text-subtle">{t("settings.general")}</h3>
          <Row label={t("settings.language")}>
            <Select value={((s.locale as string) === "en" ? "en" : "ko")} onChange={changeLanguage} options={[{ v: "ko", label: "한국어" }, { v: "en", label: "English" }]} />
          </Row>
          <Row label={t("settings.theme")}>
            <Select<ThemeMode>
              value={(s.theme as ThemeMode) ?? "system"}
              onChange={changeTheme}
              options={[
                { v: "system", label: t("settings.themeSystem") },
                { v: "light", label: t("settings.themeLight") },
                { v: "dark", label: t("settings.themeDark") },
              ]}
            />
          </Row>
          <Row label={t("settings.launchAtLogin")}>
            <input type="checkbox" checked={s.launch_at_login === true} onChange={(e) => setSetting.mutate({ key: "launch_at_login", value: String(e.target.checked) })} />
          </Row>
        </section>

        <section className="mb-4">
          <h3 className="mb-1 text-[12px] font-semibold uppercase text-text-subtle">{t("settings.timeline")}</h3>
          <Row label={t("settings.dayStart")}>
            <input type="number" defaultValue={(s.day_start_hour as number) ?? 5} className="w-16 rounded bg-transparent px-2 py-1 text-[12px] shadow-[var(--input-shadow)] focus-visible:shadow-[var(--input-shadow-focus)] focus-visible:outline-none" onBlur={(e) => setSetting.mutate({ key: "day_start_hour", value: e.target.value })} />
          </Row>
          <Row label={t("settings.dayEnd")}>
            <input type="number" defaultValue={(s.day_end_hour as number) ?? 26} className="w-16 rounded bg-transparent px-2 py-1 text-[12px] shadow-[var(--input-shadow)] focus-visible:shadow-[var(--input-shadow-focus)] focus-visible:outline-none" onBlur={(e) => setSetting.mutate({ key: "day_end_hour", value: e.target.value })} />
          </Row>
          <Row label={t("settings.workloadWarning")}>
            <input type="number" defaultValue={(s.workload_warning_minutes as number) ?? 600} className="w-20 rounded bg-transparent px-2 py-1 text-[12px] shadow-[var(--input-shadow)] focus-visible:shadow-[var(--input-shadow-focus)] focus-visible:outline-none" onBlur={(e) => setSetting.mutate({ key: "workload_warning_minutes", value: e.target.value })} />
          </Row>
        </section>

        <section className="mb-4">
          <h3 className="mb-1 text-[12px] font-semibold uppercase text-text-subtle">{t("settings.shortcut")}</h3>
          <Row label={t("settings.globalHotkey")}>
            <input
              defaultValue={(s.global_hotkey as string) ?? "CmdOrCtrl+Shift+O"}
              className="w-44 rounded bg-transparent px-2 py-1 font-mono text-[12px] shadow-[var(--input-shadow)] focus-visible:shadow-[var(--input-shadow-focus)] focus-visible:outline-none"
              onBlur={(e) => setSetting.mutate(
                { key: "global_hotkey", value: e.target.value },
                { onSuccess: () => void api.reloadShortcuts() },
              )}
            />
          </Row>
          <Row label="빠른 녹화 (⌘⇧R)">
            <input
              defaultValue={(s.quick_record_hotkey as string) ?? "CmdOrCtrl+Shift+R"}
              className="w-44 rounded bg-transparent px-2 py-1 font-mono text-[12px] shadow-[var(--input-shadow)] focus-visible:shadow-[var(--input-shadow-focus)] focus-visible:outline-none"
              onBlur={(e) => setSetting.mutate(
                { key: "quick_record_hotkey", value: e.target.value },
                { onSuccess: () => void api.reloadShortcuts() },
              )}
            />
          </Row>
          <Row label={t("settings.hudDuration")}>
            <input type="number" defaultValue={Math.round(((s.hud_duration_ms as number) ?? 2000) / 1000)} min={1} max={5} className="w-16 rounded bg-transparent px-2 py-1 text-[12px] shadow-[var(--input-shadow)] focus-visible:shadow-[var(--input-shadow-focus)] focus-visible:outline-none" onBlur={(e) => setSetting.mutate({ key: "hud_duration_ms", value: String(Number(e.target.value) * 1000) })} />
          </Row>
          {/* §7.10 keyboard shortcuts reference table */}
          <table className="mt-3 w-full border-collapse text-[12px]">
            <thead>
              <tr className="text-text-subtle">
                <th className="py-1 text-left font-medium">{t("settings.scKey")}</th>
                <th className="py-1 text-left font-medium">{t("settings.scAction")}</th>
                <th className="py-1 text-left font-medium">{t("settings.scScope")}</th>
              </tr>
            </thead>
            <tbody>
              <tr className="border-t border-border">
                <td className="py-1.5 pr-2 font-mono text-interactive-primary">{formatHotkey((s.global_hotkey as string) ?? "CmdOrCtrl+Shift+O")}</td>
                <td className="py-1.5 pr-2 text-text-muted">{t("settings.actHud")}</td>
                <td className="py-1.5 text-text-subtle">{t("settings.scopeGlobal")}</td>
              </tr>
              <tr className="border-t border-border">
                <td className="py-1.5 pr-2 font-mono text-interactive-primary">{formatHotkey((s.quick_record_hotkey as string) ?? "CmdOrCtrl+Shift+R")}</td>
                <td className="py-1.5 pr-2 text-text-muted">빠른 녹화 토글</td>
                <td className="py-1.5 text-text-subtle">{t("settings.scopeGlobal")}</td>
              </tr>
              <tr className="border-t border-border">
                <td className="py-1.5 pr-2 font-mono text-text-muted">⌘⇧A</td>
                <td className="py-1.5 pr-2 text-text-muted">활동 전환 (녹화)</td>
                <td className="py-1.5 text-text-subtle">{t("settings.scopeApp")}</td>
              </tr>
              <tr className="border-t border-border">
                <td className="py-1.5 pr-2 font-mono text-text-muted">⌘N</td>
                <td className="py-1.5 pr-2 text-text-muted">{t("settings.actPalette")} (오늘)</td>
                <td className="py-1.5 text-text-subtle">{t("settings.scopeMain")}</td>
              </tr>
              {SHORTCUT_ROWS.map((r) => (
                <tr key={r.key} className="border-t border-border">
                  <td className="py-1.5 pr-2 font-mono text-text-muted">{r.key}</td>
                  <td className="py-1.5 pr-2 text-text-muted">{t(r.actionKey)}</td>
                  <td className="py-1.5 text-text-subtle">{t(r.scopeKey)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>

        <section className="mb-4">
          <h3 className="mb-1 text-[12px] font-semibold uppercase text-text-subtle">{t("notifications.section")}</h3>
          <Row label={t("notifications.enable")}>
            <input
              type="checkbox"
              checked={s.notifications_enabled === true}
              onChange={(e) =>
                setSetting.mutate({ key: "notifications_enabled", value: String(e.target.checked) })
              }
            />
          </Row>
          <div className="py-1 text-[12px] text-text-subtle">{t("notifications.enableHelp")}</div>
          <Row label={t("notifications.leadMinutes")}>
            <input
              type="range"
              min={1}
              max={30}
              defaultValue={(s.notification_lead_minutes as number) ?? 5}
              onChange={(e) =>
                setSetting.mutate({ key: "notification_lead_minutes", value: e.target.value })
              }
              className="w-24"
            />
            <span className="ml-2 w-6 text-center text-[12px]">{String(s.notification_lead_minutes ?? 5)}</span>
          </Row>
          <div className="flex items-center gap-2 py-1">
            {permQ.data ? (
              <span className="text-[12px] text-status-success">{t("notifications.granted")}</span>
            ) : (
              <>
                <span className="text-[12px] text-status-error">{t("notifications.denied")}</span>
                <button
                  className="rounded px-2 py-1 text-[12px]"
                  style={{ background: "var(--color-interactive-primary)", color: "var(--color-interactive-primary-foreground)" }}
                  onClick={() => requestPerm.mutate()}
                >
                  {t("notifications.requestPermission")}
                </button>
                <button
                  className="rounded bg-surface-muted px-2 py-1 text-[12px]"
                  onClick={() => api.openNotificationSettings()}
                >
                  {t("notifications.openSystemSettings")}
                </button>
              </>
            )}
          </div>
        </section>

        <section className="mb-4">
          <h3 className="mb-1 text-[12px] font-semibold uppercase text-text-subtle">{t("settings.menubar")}</h3>
          <p className="mb-2 text-[12px] text-text-subtle">{t("settings.menubarHelp")}</p>
          {traySlots.length === 0 ? (
            <p className="text-[12px] text-text-subtle">{t("settings.menubarEmptyHint")}</p>
          ) : (
            <ul className="space-y-1">
              {traySlots.map((s, i) => (
                <li key={s.kind} className="flex items-center gap-2 rounded-md px-2 py-1.5 hover:bg-surface-sunken">
                  <input
                    type="checkbox"
                    checked={s.on}
                    onChange={() => toggleSlot(i)}
                    aria-label={slotLabel(s.kind)}
                  />
                  <span className="flex-1 text-[13px]">{slotLabel(s.kind)}</span>
                  <span className="text-[11px] text-text-subtle">{s.on ? "켬" : "끔"}</span>
                  <button
                    type="button"
                    className="rounded p-1 hover:bg-surface-muted disabled:opacity-30"
                    disabled={i === 0}
                    onClick={() => moveSlot(i, -1)}
                    aria-label="위로"
                  >
                    <ChevronUp size={14} />
                  </button>
                  <button
                    type="button"
                    className="rounded p-1 hover:bg-surface-muted disabled:opacity-30"
                    disabled={i === traySlots.length - 1}
                    onClick={() => moveSlot(i, 1)}
                    aria-label="아래로"
                  >
                    <ChevronDown size={14} />
                  </button>
                </li>
              ))}
            </ul>
          )}
          <p className="mt-2 text-[11px] text-text-subtle">{t("settings.menubarAllOffNote")}</p>
        </section>
        <section className="mb-4">
          <h3 className="mb-1 text-[12px] font-semibold uppercase text-text-subtle">{t("settings.categories")}</h3>
          <ul className="mb-2 space-y-1">
            {(catsQ.data ?? []).map((c) => (
              <li key={c.id} className="flex items-center gap-2 py-1">
                <span className="h-3 w-3 rounded-full" style={{ background: categoryColor(c.color_hue) }} />
                <span className="flex-1 text-[13px]">{c.name}</span>
                {!c.is_builtin && (
                  <button className="text-[11px] text-status-error hover:underline" onClick={() => delCat.mutate(c.id)}>{t("common.delete")}</button>
                )}
              </li>
            ))}
          </ul>
          <div className="flex items-center gap-2">
            <span className="h-7 w-7 shrink-0 rounded-[var(--input-radius)]" style={{ background: categoryColor(catHue) }} aria-hidden />
            <input type="range" min={0} max={360} value={catHue} onChange={(e) => setCatHue(Number(e.target.value))} className="flex-1" />
            <input value={catName} onChange={(e) => setCatName(e.target.value)} placeholder={t("settings.categoryName")} className="w-28 rounded bg-transparent px-2 py-1 text-[12px] shadow-[var(--input-shadow)] focus-visible:shadow-[var(--input-shadow-focus)] focus-visible:outline-none" />
            <button
              className="rounded bg-interactive-primary px-2 py-1 text-[12px] text-interactive-primary-foreground"
              onClick={() => { if (catName.trim()) { createCat.mutate({ name: catName.trim(), hue: catHue, icon: null }); setCatName(""); } }}
            >
              {t("common.add")}
            </button>
          </div>
        </section>

        <section className="mb-4">
          <h3 className="mb-1 text-[12px] font-semibold uppercase text-text-subtle">{t("settings.data")}</h3>
          <Row label={t("settings.dbPath")}>
            <code className="max-w-[60%] truncate text-[11px] text-text-subtle">{String(s.__dbPath ?? "")}</code>
          </Row>
        </section>

        <section className="mb-4">
          <h3 className="mb-1 flex items-center gap-1.5 text-[12px] font-semibold uppercase text-text-subtle">
            <Terminal size={12} />
            {t("settings.sectionCli")}
          </h3>
          <CliSection />
        </section>

        <section>
          <h3 className="mb-1 text-[12px] font-semibold uppercase text-text-subtle">{t("settings.about")}</h3>
          <p className="text-[12px] text-text-muted">{t("settings.version")} 0.1.0</p>
          <p className="mt-1 text-[12px] text-text-subtle">{t("settings.livesInMenubar")}</p>
        </section>
    </Modal>
  );
}
