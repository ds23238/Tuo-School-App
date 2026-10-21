import { api, type CourseSync } from "./api";
import { errorText } from "./errors";

/** Gemeinsamer Sync-Zustand für Seitenleiste, Kursliste und Kursseite. */
class SyncState {
  running = $state(false);
  lastAt = $state<Date | null>(null);
  error = $state<string | null>(null);
  results = $state<CourseSync[]>([]);
  /** Erhöht sich nach jedem Lauf, damit Ansichten neu laden. */
  generation = $state(0);

  async run() {
    if (this.running) return;
    this.running = true;
    this.error = null;
    try {
      this.results = await api.moodle.sync();
      const failed = this.results.find((r) => r.error);
      if (failed) this.error = failed.error;
      this.lastAt = new Date();
    } catch (e) {
      this.error = errorText(e);
    } finally {
      this.running = false;
      this.generation++;
    }
  }
}

export const sync = new SyncState();
