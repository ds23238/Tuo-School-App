import { i18n } from "./i18n/index.svelte";

export function formatDate(unixSeconds: number): string {
  return new Intl.DateTimeFormat(i18n.locale, { dateStyle: "medium", timeStyle: "short" }).format(
    unixSeconds * 1000,
  );
}

export function formatSize(bytes: number): string {
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  const digits = unit === 0 || value >= 10 ? 0 : 1;
  return `${new Intl.NumberFormat(i18n.locale, { maximumFractionDigits: digits }).format(value)} ${units[unit]}`;
}

/** Ersetzt `{name}` in übersetzten Texten. */
export function fill(text: string, values: Record<string, string | number>): string {
  return text.replace(/\{(\w+)\}/g, (match, key: string) => String(values[key] ?? match));
}
