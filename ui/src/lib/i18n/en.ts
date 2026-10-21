import type { Messages } from "./de";

export const en: Messages = {
  nav: {
    today: "Today",
    courses: "Courses",
    timetable: "Timetable",
    settings: "Settings",
  },
  placeholder: {
    today: "Your timetable, deadlines and new files will show up here.",
    courses: "No Moodle account connected yet.",
    timetable: "No Untis account connected yet.",
  },
  settings: {
    language: "Language",
    systemLanguage: "System language",
    syncDir: "Sync folder",
    change: "Change",
    accounts: "Accounts",
    noAccounts: "No accounts connected yet.",
    signOut: "Sign out",
    signOutConfirm: "Sign out of this account? Your credentials are removed from the keyring; downloaded files are kept.",
    keyringMissing: "The system keyring is not available. Without it, credentials cannot be stored.",
    version: "Version",
  },
  service: {
    moodle: "Moodle",
    untis: "WebUntis",
  },
  error: {
    generic: "Error",
  },
  locale: {
    de: "Deutsch",
    en: "English",
  },
};
