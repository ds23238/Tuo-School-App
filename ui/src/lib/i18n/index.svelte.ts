import { de, type Messages } from "./de";
import { en } from "./en";

export const locales = ["de", "en"] as const;
export type Locale = (typeof locales)[number];

const dictionaries: Record<Locale, Messages> = { de, en };

export function isLocale(value: unknown): value is Locale {
  return locales.includes(value as Locale);
}

function systemLocale(): Locale {
  const system = navigator.language.slice(0, 2);
  return isLocale(system) ? system : "en";
}

let current = $state<Locale>(systemLocale());

export const i18n = {
  get locale(): Locale {
    return current;
  },
  /** `null` = Systemsprache. */
  use(locale: string | null) {
    current = isLocale(locale) ? locale : systemLocale();
    document.documentElement.lang = current;
  },
  get t(): Messages {
    return dictionaries[current];
  },
};
