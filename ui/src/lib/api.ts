import { invoke } from "@tauri-apps/api/core";

// Spiegelt die Typen aus crates/core und crates/moodle.

export type Service = "moodle" | "untis";

export interface Settings {
  /** `null` = Systemsprache. */
  locale: string | null;
  syncDir: string;
}

export interface Account {
  service: Service;
  baseUrl: string;
  username: string;
  displayName: string | null;
  /** Unix-Zeit in Sekunden. */
  createdAt: number;
}

export interface SiteCheck {
  url: string;
  name: string;
  ssoOnly: boolean;
}

export interface Course {
  id: number;
  fullName: string;
  shortName: string;
  visible: boolean;
  enrolled: boolean;
  sync: boolean;
  /** Unix-Zeit in Sekunden. */
  startDate: number | null;
  endDate: number | null;
  lastAccess: number | null;
}

/** Fehler aus dem Backend, siehe src-tauri/src/error.rs. */
export interface CommandError {
  code: string;
  message: string;
}

export function isCommandError(e: unknown): e is CommandError {
  return typeof e === "object" && e !== null && "code" in e && "message" in e;
}

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  listAccounts: () => invoke<Account[]>("list_accounts"),
  removeAccount: (service: Service) => invoke<void>("remove_account", { service }),
  checkKeyring: () => invoke<void>("check_keyring"),

  moodle: {
    checkSite: (url: string) => invoke<SiteCheck>("moodle_check_site", { url }),
    login: (url: string, username: string, password: string, remember: boolean) =>
      invoke<Account>("moodle_login", { url, username, password, remember }),
    courses: () => invoke<Course[]>("moodle_courses"),
    refreshCourses: () => invoke<Course[]>("moodle_refresh_courses"),
    setCourseSync: (id: number, sync: boolean) =>
      invoke<void>("moodle_set_course_sync", { id, sync }),
  },
};
