import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import ko from "../locales/ko.json";
import en from "../locales/en.json";

export type Lang = "ko" | "en";

export function detectInitialLang(setting: string | undefined): Lang {
  if (setting === "ko" || setting === "en") return setting;
  // "system": sniff the browser/OS locale.
  const nav = (typeof navigator !== "undefined" ? navigator.language : "en") || "en";
  return nav.toLowerCase().startsWith("ko") ? "ko" : "en";
}

export async function initI18n(initialSetting?: string) {
  const lang = detectInitialLang(initialSetting);
  await i18n.use(initReactI18next).init({
    resources: {
      ko: { translation: ko },
      en: { translation: en },
    },
    lng: lang,
    fallbackLng: "en",
    interpolation: { escapeValue: false },
  });
  return i18n;
}

export function changeLang(lang: Lang) {
  void i18n.changeLanguage(lang);
}
