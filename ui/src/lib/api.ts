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

export interface CourseView {
  courseId: number;
  /** Unix-Zeit des Abrufs. */
  fetchedAt: number;
  sections: Section[];
}

export interface Section {
  id: number;
  number: number;
  name: string;
  /** Bereinigtes HTML. */
  summary: string;
  visible: boolean;
  available: boolean;
  availability: string | null;
  modules: Module[];
}

export interface Module {
  id: number;
  name: string;
  /** Moodle-Modulname, z. B. `resource`, `assign`, `label`. */
  kind: string;
  kindName: string;
  description: string | null;
  icon: string | null;
  url: string | null;
  link: string | null;
  visible: boolean;
  available: boolean;
  availability: string | null;
  indent: number;
  dates: { label: string; timestamp: number }[];
  completed: boolean | null;
  files: MoodleFile[];
  subsection: Section | null;
}

export interface MoodleFile {
  name: string;
  path: string;
  size: number;
  modified: number | null;
  mime: string | null;
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
    course: (id: number) => invoke<CourseView | null>("moodle_course", { id }),
    refreshCourse: (id: number) => invoke<CourseView>("moodle_refresh_course", { id }),
  },
};
