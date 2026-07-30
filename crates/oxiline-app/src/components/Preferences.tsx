import { useState } from "react";
import { useTranslation } from "react-i18next";
import { X } from "lucide-react";
import { useSettings, useSetSetting, useCategories, useCreateCategory, useDeleteCategory } from "../hooks";
import { useUi } from "../lib/store";
import { applyTheme, setThemeMode, type ThemeMode } from "../lib/theme";
import { changeLang, type Lang } from "../lib/i18n";

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between py-2">
      <span className="text-[13px]" style={{ color: "var(--text-secondary)" }}>{label}</span>
      {children}
    </div>
  );
}

function Select<T extends string>(props: { value: T; options: { v: T; label: string }[]; onChange: (v: T) => void }) {
  return (
    <select
      value={props.value}
      onChange={(e) => props.onChange(e.target.value as T)}
      className="rounded border border-border-subtle bg-transparent px-2 py-1 text-[12px]"
    >
      {props.options.map((o) => (
        <option key={o.v} value={o.v}>{o.label}</option>
      ))}
    </select>
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
  const [catName, setCatName] = useState("");
  const [catHue, setCatHue] = useState(200);

  if (!open) return null;
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

  return (
    <div className="fixed inset-0 z-40 flex items-center justify-center" style={{ background: "oklch(0 0 0 / 0.25)" }} onClick={() => setPreferencesOpen(false)}>
      <div
        className="max-h-[80%] w-full max-w-lg overflow-y-auto rounded-lg border border-border-subtle p-5"
        style={{ background: "var(--surface-raised)" }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="mb-3 flex items-center justify-between">
          <h2 className="text-[16px] font-semibold">{t("settings.title")}</h2>
          <button className="rounded p-1 hover:bg-sunken" onClick={() => setPreferencesOpen(false)} aria-label={t("common.close")}>
            <X size={16} />
          </button>
        </div>

        <section className="mb-4">
          <h3 className="mb-1 text-[12px] font-semibold uppercase" style={{ color: "var(--text-tertiary)" }}>{t("settings.general")}</h3>
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
          <h3 className="mb-1 text-[12px] font-semibold uppercase" style={{ color: "var(--text-tertiary)" }}>{t("settings.timeline")}</h3>
          <Row label={t("settings.dayStart")}>
            <input type="number" defaultValue={(s.day_start_hour as number) ?? 5} className="w-16 rounded border border-border-subtle bg-transparent px-2 py-1 text-[12px]" onBlur={(e) => setSetting.mutate({ key: "day_start_hour", value: e.target.value })} />
          </Row>
          <Row label={t("settings.dayEnd")}>
            <input type="number" defaultValue={(s.day_end_hour as number) ?? 26} className="w-16 rounded border border-border-subtle bg-transparent px-2 py-1 text-[12px]" onBlur={(e) => setSetting.mutate({ key: "day_end_hour", value: e.target.value })} />
          </Row>
          <Row label={t("settings.workloadWarning")}>
            <input type="number" defaultValue={(s.workload_warning_minutes as number) ?? 600} className="w-20 rounded border border-border-subtle bg-transparent px-2 py-1 text-[12px]" onBlur={(e) => setSetting.mutate({ key: "workload_warning_minutes", value: e.target.value })} />
          </Row>
        </section>

        <section className="mb-4">
          <h3 className="mb-1 text-[12px] font-semibold uppercase" style={{ color: "var(--text-tertiary)" }}>{t("settings.shortcut")}</h3>
          <Row label={t("settings.globalHotkey")}>
            <input
              defaultValue={(s.global_hotkey as string) ?? "CmdOrCtrl+Shift+O"}
              className="w-44 rounded border border-border-subtle bg-transparent px-2 py-1 font-mono text-[12px]"
              onBlur={(e) => setSetting.mutate({ key: "global_hotkey", value: e.target.value })}
            />
          </Row>
          <Row label={t("settings.hudDuration")}>
            <input type="number" defaultValue={Math.round(((s.hud_duration_ms as number) ?? 2000) / 1000)} min={1} max={5} className="w-16 rounded border border-border-subtle bg-transparent px-2 py-1 text-[12px]" onBlur={(e) => setSetting.mutate({ key: "hud_duration_ms", value: String(Number(e.target.value) * 1000) })} />
          </Row>
        </section>

        <section className="mb-4">
          <h3 className="mb-1 text-[12px] font-semibold uppercase" style={{ color: "var(--text-tertiary)" }}>{t("settings.categories")}</h3>
          <ul className="mb-2 space-y-1">
            {(catsQ.data ?? []).map((c) => (
              <li key={c.id} className="flex items-center gap-2 py-1">
                <span className="h-3 w-3 rounded-full" style={{ background: `oklch(0.62 0.09 ${c.color_hue})` }} />
                <span className="flex-1 text-[13px]">{c.name}</span>
                {!c.is_builtin && (
                  <button className="text-[11px] hover:underline" style={{ color: "var(--signal-rust)" }} onClick={() => delCat.mutate(c.id)}>{t("routine.delete")}</button>
                )}
              </li>
            ))}
          </ul>
          <div className="flex items-center gap-2">
            <input type="color" value={`hsl(${catHue}, 70%, 50%)`} onChange={() => {}} className="h-7 w-7" />
            <input type="range" min={0} max={360} value={catHue} onChange={(e) => setCatHue(Number(e.target.value))} className="flex-1" />
            <input value={catName} onChange={(e) => setCatName(e.target.value)} placeholder={t("routine.name")} className="w-28 rounded border border-border-subtle bg-transparent px-2 py-1 text-[12px]" />
            <button
              className="rounded px-2 py-1 text-[12px] text-white"
              style={{ background: "var(--accent-oxide)" }}
              onClick={() => { if (catName.trim()) { createCat.mutate({ name: catName.trim(), hue: catHue, icon: null }); setCatName(""); } }}
            >
              {t("common.add")}
            </button>
          </div>
        </section>

        <section className="mb-4">
          <h3 className="mb-1 text-[12px] font-semibold uppercase" style={{ color: "var(--text-tertiary)" }}>{t("settings.data")}</h3>
          <Row label={t("settings.dbPath")}>
            <code className="max-w-[60%] truncate text-[11px]" style={{ color: "var(--text-tertiary)" }}>{String(s.__dbPath ?? "")}</code>
          </Row>
        </section>

        <section>
          <h3 className="mb-1 text-[12px] font-semibold uppercase" style={{ color: "var(--text-tertiary)" }}>{t("settings.about")}</h3>
          <p className="text-[12px]" style={{ color: "var(--text-secondary)" }}>{t("settings.version")} 0.1.0</p>
          <p className="mt-1 text-[12px]" style={{ color: "var(--text-tertiary)" }}>{t("settings.livesInMenubar")}</p>
        </section>
      </div>
    </div>
  );
}
