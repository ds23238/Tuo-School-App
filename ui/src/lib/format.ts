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

export function formatTime(date: Date): string {
  return new Intl.DateTimeFormat(i18n.locale, { timeStyle: "short" }).format(date);
}

/** „Fr., 24. Okt., 23:59“ – kompakt für Fristen. */
export function formatDeadline(unixSeconds: number): string {
  return new Intl.DateTimeFormat(i18n.locale, {
    weekday: "short",
    day: "numeric",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  }).format(unixSeconds * 1000);
}

/** Heute nur die Uhrzeit, sonst das Datum – für Listen von Nachrichten. */
export function formatShort(unixSeconds: number): string {
  const date = new Date(unixSeconds * 1000);
  const now = new Date();
  if (date.toDateString() === now.toDateString()) return formatTime(date);
  const sameYear = date.getFullYear() === now.getFullYear();
  return new Intl.DateTimeFormat(i18n.locale, {
    day: "numeric",
    month: "short",
    year: sameYear ? undefined : "numeric",
  }).format(date);
}

export function formatDay(unixSeconds: number): string {
  return new Intl.DateTimeFormat(i18n.locale, { dateStyle: "full" }).format(unixSeconds * 1000);
}
