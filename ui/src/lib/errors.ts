import { isCommandError } from "./api";
import { i18n } from "./i18n/index.svelte";

/** Übersetzte Meldung zu einem Fehler aus dem Backend. */
export function errorText(e: unknown): string {
  if (!isCommandError(e)) return `${i18n.t.error.generic}: ${String(e)}`;
  const errors: Record<string, string> = i18n.t.error;
  const text = errors[e.code];
  if (!text) return `${i18n.t.error.generic}: ${e.message}`;
  // Moodle-Meldungen sind schon für Menschen geschrieben.
  return e.code === "moodle" ? `${text}: ${e.message}` : text;
}
