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
    version: "Version",
  },
  locale: {
    de: "Deutsch",
    en: "English",
  },
};

type Widen<T> = { [K in keyof T]: T[K] extends string ? string : Widen<T[K]> };
export type Messages = Widen<typeof de>;
