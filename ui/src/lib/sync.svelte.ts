import { listen } from "@tauri-apps/api/event";
import { api, type CourseSync, type SyncStatus } from "./api";
import { errorText } from "./errors";

/**
 * Sync-Zustand aus dem Backend. Der Sync selbst läuft dort (auch im
 * Hintergrund); hier kommt nur sein Stand per Event an.
 */
class SyncState {
  running = $state(false);
  lastAt = $state<Date | null>(null);
  error = $state<string | null>(null);
  results = $state<CourseSync[]>([]);
  /** Erhöht sich nach jedem Lauf, damit Ansichten neu laden. */
  generation = $state(0);

  constructor() {
    api.syncStatus().then((s) => this.apply(s), () => {});
    listen<SyncStatus>("sync-status", (e) => this.apply(e.payload));
  }

  private apply(s: SyncStatus) {
    const finished = this.running && !s.running;
    this.running = s.running;
    this.lastAt = s.lastAt ? new Date(s.lastAt * 1000) : null;
    this.error = s.error;
    this.results = s.results;
    if (finished) this.generation++;
  }

  async run() {
    try {
      await api.syncNow();
    } catch (e) {
      this.error = errorText(e);
    }
  }
}

export const sync = new SyncState();
