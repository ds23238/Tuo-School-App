export const de = {
  nav: {
    today: "Heute",
    courses: "Kurse",
    timetable: "Stundenplan",
    settings: "Einstellungen",
  },
  placeholder: {
    today: "Hier erscheinen Stundenplan, Fristen und neue Dateien.",
    courses: "Noch kein Moodle-Konto verbunden.",
    timetable: "Noch kein Untis-Konto verbunden.",
  },
  settings: {
    language: "Sprache",
    systemLanguage: "Systemsprache",
    syncDir: "Sync-Ordner",
    change: "Ändern",
    accounts: "Konten",
    noAccounts: "Noch keine Konten verbunden.",
    signOut: "Abmelden",
    signOutConfirm: "Konto abmelden? Die Zugangsdaten werden aus dem Schlüsselbund entfernt; heruntergeladene Dateien bleiben erhalten.",
    keyringMissing: "Der Schlüsselbund des Systems ist nicht erreichbar. Ohne ihn können keine Zugangsdaten gespeichert werden.",
    version: "Version",
  },
  service: {
    moodle: "Moodle",
    untis: "WebUntis",
  },
  error: {
    generic: "Fehler",
  },
  locale: {
    de: "Deutsch",
    en: "English",
  },
};

type Widen<T> = { [K in keyof T]: T[K] extends string ? string : Widen<T[K]> };
export type Messages = Widen<typeof de>;
