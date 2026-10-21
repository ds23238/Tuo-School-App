import { de, type Messages } from "./de";
import { en } from "./en";

export const locales = ["de", "en"] as const;
export type Locale = (typeof locales)[number];

const dictionaries: Record<Locale, Messages> = { de, en };
const storageKey = "locale";

function isLocale(value: unknown): value is Locale {
  return locales.includes(value as Locale);
}

function initialLocale(): Locale {
  try {
    const stored = localStorage.getItem(storageKey);
    if (isLocale(stored)) return stored;
  } catch {
    // Kein Zugriff auf localStorage: Systemsprache verwenden.
  }
  const system = navigator.language.slice(0, 2);
  return isLocale(system) ? system : "en";
}

let current = $state<Locale>(initialLocale());

export const i18n = {
  get locale(): Locale {
    return current;
  },
  set locale(value: Locale) {
    current = value;
    document.documentElement.lang = value;
    try {
      localStorage.setItem(storageKey, value);
    } catch {
      // Auswahl gilt dann nur für diese Sitzung.
    }
  },
  get t(): Messages {
    return dictionaries[current];
  },
};
