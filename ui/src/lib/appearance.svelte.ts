import { getCurrentWindow } from "@tauri-apps/api/window";
import { api, type Settings } from "./api";

/** Farbschema und Fachfarben aus den Einstellungen, für alle Ansichten. */
class Appearance {
  theme = $state("system");
  subjectColors = $state<Record<string, string>>({});

  constructor() {
    api.getSettings().then(
      (s) => this.apply(s),
      () => {},
    );
  }

  apply(settings: Settings) {
    this.theme = settings.theme;
    this.subjectColors = settings.subjectColors ?? {};
    const root = document.documentElement;
    if (settings.theme === "light" || settings.theme === "dark") root.dataset.theme = settings.theme;
    else delete root.dataset.theme;
    // Auch die Titelleiste des Fensters (soweit das System es erlaubt).
    const theme = settings.theme === "light" || settings.theme === "dark" ? settings.theme : null;
    getCurrentWindow()
      .setTheme(theme)
      .catch(() => {});
  }
}

export const appearance = new Appearance();
