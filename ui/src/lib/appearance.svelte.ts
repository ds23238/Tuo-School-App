import { getCurrentWindow } from "@tauri-apps/api/window";
import { api, type Settings, type DisplayMode, type DisplayArea } from "./api";

/** Farbschema und Fachfarben aus den Einstellungen, für alle Ansichten. */
class Appearance {
  theme = $state("system");
  courseDisplay = $state<DisplayMode>("web");
  moodleMessagesDisplay = $state<DisplayMode>("native");
  untisMessagesDisplay = $state<DisplayMode>("web");

  async setDisplay(area: DisplayArea, mode: DisplayMode) {
    const field = { courses: "courseDisplay", moodleMessages: "moodleMessagesDisplay", untisMessages: "untisMessagesDisplay" }[area];
    const settings = await api.getSettings();
    const next = { ...settings, [field]: mode };
    await api.saveSettings(next);
    this.apply(next);
  }
  subjectColors = $state<Record<string, string>>({});

  constructor() {
    api.getSettings().then(
      (s) => this.apply(s),
      () => {},
    );
  }

  apply(settings: Settings) {
    this.theme = settings.theme;
    this.courseDisplay = settings.courseDisplay ?? "web";
    this.moodleMessagesDisplay = settings.moodleMessagesDisplay ?? "native";
    this.untisMessagesDisplay = settings.untisMessagesDisplay ?? "web";
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
