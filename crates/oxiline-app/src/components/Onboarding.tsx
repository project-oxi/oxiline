import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useUi } from "../lib/store";
import { api } from "../lib/api";
import { changeLang, type Lang } from "../lib/i18n";
import { Modal } from "./Modal";

export function Onboarding() {
  const { t } = useTranslation();
  const { onboardingOpen: open, setOnboardingOpen } = useUi();
  const [step, setStep] = useState(0);
  const [lang, setLang] = useState<Lang>("ko");


  const finish = async () => {
    try {
      await api.setSetting("locale", lang);
      changeLang(lang);
      await api.setOnboardingDone();
    } catch {
      /* ignore */
    }
    setOnboardingOpen(false);
  };

  const steps = [
    {
      title: t("onboarding.welcome"),
      body: t("onboarding.welcomeBody"),
      extra: (
        <div className="mt-3 flex justify-center gap-2">
          {(["ko", "en"] as Lang[]).map((l) => (
            <button
              key={l}
              onClick={() => setLang(l)}
              className="rounded-md px-3 py-1.5 text-[13px] font-medium"
              style={{
                background: lang === l ? "var(--color-interactive-primary)" : "var(--color-surface-sunken)",
                color: lang === l ? "var(--color-interactive-primary-foreground)" : "var(--color-text-muted)",
              }}
            >
              {l === "ko" ? "한국어" : "English"}
            </button>
          ))}
        </div>
      ),
    },
    { title: t("onboarding.background"), body: t("onboarding.backgroundBody"), extra: null },
    { title: t("onboarding.shortcut"), body: t("onboarding.shortcutBody"), extra: null },
  ];
  const cur = steps[step];

  return (
    <Modal
      open={open}
      onClose={() => {}}
      variant="fullscreen"
      dismissable={false}
      backdropStyle={{ background: "var(--color-surface)" }}
      panelClassName="w-full max-w-sm px-8 text-center"
      labelledBy="onboarding-title"
    >
        <div className="mx-auto mb-5 h-12 w-12 rounded-lg" style={{ background: "var(--color-interactive-primary)" }} />
        <h1 id="onboarding-title" className="text-[22px] font-bold font-display">{cur.title}</h1>
        <p className="mx-auto mt-2 max-w-xs text-[14px] text-text-muted">
          {cur.body}
        </p>
        {cur.extra}

        <div className="mt-6 flex items-center justify-between">
          <button className="text-[12px] text-text-subtle hover:underline" onClick={() => setStep((s) => s + 1)}>
            {t("onboarding.skip")}
          </button>
          <div className="flex items-center gap-2">
            <div className="flex gap-1">
              {steps.map((_, i) => (
                <span key={i} className="h-1.5 w-1.5 rounded-full" style={{ background: i === step ? "var(--color-interactive-primary)" : "var(--color-border-strong)" }} />
              ))}
            </div>
            <button
              className="rounded-md px-4 py-1.5 text-[13px] font-medium text-interactive-primary-foreground"
              style={{ background: "var(--color-interactive-primary)" }}
              onClick={() => (step < steps.length - 1 ? setStep((s) => s + 1) : finish())}
            >
              {step < steps.length - 1 ? t("onboarding.next") : t("onboarding.done")}
            </button>
          </div>
        </div>
    </Modal>
  );
}
