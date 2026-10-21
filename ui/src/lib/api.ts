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
  progress: number | null;
  /** Medien-Adresse des Kursbilds. */
  image: string | null;
}

export interface CourseFiles {
  total: number;
  downloaded: number;
  updates: number;
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

export type Purpose =
  | "content"
  | "assessment"
  | "communication"
  | "collaboration"
  | "interactivecontent"
  | "administration"
  | "other";

export interface Module {
  id: number;
  instance: number | null;
  name: string;
  /** Moodle-Modulname, z. B. `resource`, `assign`, `label`. */
  kind: string;
  kindName: string;
  purpose: Purpose | string;
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
  manualCompletion: boolean;
  files: MoodleFile[];
  subsection: Section | null;
}

export type FileStatus = "remote" | "current" | "updateAvailable" | "missing" | "removed";

export interface FileState {
  id: number;
  status: FileStatus;
  modifiedLocally: boolean;
}

export interface MoodleFile {
  state: FileState | null;
  name: string;
  path: string;
  size: number;
  modified: number | null;
  mime: string | null;
}

export interface CourseSync {
  courseId: number;
  downloaded: number;
  error: string | null;
}

export interface PageContent {
  moduleId: number;
  name: string;
  chapters: { title: string; level: number; html: string }[];
  fetchedAt: number;
  offline: boolean;
}

export interface AssignFile {
  name: string;
  size: number;
  modified: number | null;
}

export interface Assignment {
  id: number;
  moduleId: number;
  name: string;
  intro: string;
  allowFrom: number | null;
  dueDate: number | null;
  cutoffDate: number | null;
  fileSubmission: { maxFiles: number; maxBytes: number; types: string[] } | null;
  textSubmission: boolean;
  wordLimit: number | null;
  drafts: boolean;
  requiresStatement: boolean;
  status: "new" | "draft" | "submitted" | "reopened" | string | null;
  submittedAt: number | null;
  files: AssignFile[];
  text: string | null;
  textPlain: string;
  canEdit: boolean;
  canSubmit: boolean;
  locked: boolean;
  gradingStatus: string | null;
  grade: string | null;
  gradedAt: number | null;
  feedback: string | null;
  feedbackFiles: AssignFile[];
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
    fileStats: () => invoke<Record<string, CourseFiles>>("moodle_file_stats"),
    course: (id: number) => invoke<CourseView | null>("moodle_course", { id }),
    refreshCourse: (id: number) => invoke<CourseView>("moodle_refresh_course", { id }),
    sync: () => invoke<CourseSync[]>("moodle_sync"),
    syncCourse: (id: number) => invoke<number>("moodle_sync_course", { id }),
    openFile: (id: number) => invoke<void>("moodle_open_file", { id }),
    updateFile: (id: number) => invoke<void>("moodle_update_file", { id }),
    openCourseFolder: (id: number) => invoke<void>("moodle_open_course_folder", { id }),
    openInBrowser: (url: string) => invoke<void>("moodle_open_in_browser", { url }),
    setCompletion: (moduleId: number, completed: boolean) =>
      invoke<void>("moodle_set_completion", { moduleId, completed }),
    page: (courseId: number, moduleId: number) =>
      invoke<PageContent>("moodle_page", { courseId, moduleId }),
    assignment: (courseId: number, moduleId: number) =>
      invoke<Assignment>("moodle_assignment", { courseId, moduleId }),
    saveSubmission: (
      courseId: number,
      moduleId: number,
      draft: { keep: string[]; add: string[]; text: string | null },
    ) => invoke<void>("moodle_save_submission", { courseId, moduleId, draft }),
    submit: (assignId: number, acceptStatement: boolean) =>
      invoke<void>("moodle_submit", { assignId, acceptStatement }),
    openAssignmentFile: (courseId: number, moduleId: number, feedback: boolean, name: string) =>
      invoke<void>("moodle_open_assignment_file", { courseId, moduleId, feedback, name }),
  },
};
