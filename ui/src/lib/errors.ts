import { isCommandError } from "./api";
import { i18n } from "./i18n/index.svelte";

/** Übersetzte Meldung zu einem Fehler aus dem Backend. */
export function errorText(e: unknown): string {
  if (!isCommandError(e)) return `${i18n.t.error.generic}: ${String(e)}`;
  const errors: Record<string, string> = i18n.t.error;
  const text = errors[e.code];
  if (!text) return `${i18n.t.error.generic}: ${e.message}`;
  // Diese Meldungen sind schon für Menschen geschrieben und nennen den Grund.
  return ["moodle", "open", "io", "sync", "autostart"].includes(e.code) ? `${text}: ${e.message}` : text;
}
