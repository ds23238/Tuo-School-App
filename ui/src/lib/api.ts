import { invoke } from "@tauri-apps/api/core";

// Spiegelt die Typen aus crates/core und crates/moodle.

export type Service = "moodle" | "untis";

export interface Settings {
  /** `null` = Systemsprache. */
  locale: string | null;
  syncDir: string;
  syncEnabled: boolean;
  syncIntervalMinutes: number;
  notifications: boolean;
  /** Nur Linux: `auto`, `always`, `never`, `compat`. Wirkt nach Neustart. */
  graphics: string;
  /** `system`, `light` oder `dark`. */
  theme: string;
  /** Eigene Farben je Untis-Fach (Kürzel → `#rrggbb`). */
  subjectColors: Record<string, string>;
}

export interface SyncStatus {
  running: boolean;
  /** Unix-Zeit des letzten abgeschlossenen Syncs. */
  lastAt: number | null;
  error: string | null;
  results: CourseSync[];
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

export interface Counts {
  chats: number;
  notifications: number;
  /** `null` ohne Moodle-Mail. */
  mail: number | null;
  /** Ungelesene Untis-Mitteilungen, `null` ohne WebUntis-Konto. */
  untis: number | null;
}

export interface School {
  server: string;
  loginName: string;
  displayName: string;
  address: string;
}

export interface UntisElement {
  name: string;
  longName: string;
  /** Ursprünglich geplant, z. B. vertretene Lehrkraft. */
  previous: string | null;
  /** Abwesend (in WebUntis durchgestrichen). */
  absent: boolean;
}

export type LessonStatus = "regular" | "cancelled" | "changed" | "exam" | "other";

export interface Lesson {
  id: number;
  /** ISO-Datum. */
  date: string;
  /** `HH:MM`. */
  start: string;
  end: string;
  subject: UntisElement | null;
  teachers: UntisElement[];
  rooms: UntisElement[];
  classes: UntisElement[];
  status: LessonStatus;
  text: string | null;
  info: string | null;
  substitution: string | null;
}

export interface Week {
  monday: string;
  days: { date: string; holiday: string | null }[];
  lessons: Lesson[];
  grid: { name: string; start: string; end: string }[];
  fetchedAt: number;
  offline: boolean;
}

export interface Listed<T> {
  items: T[];
  fetchedAt: number;
  offline: boolean;
}

export interface Homework {
  id: number;
  subject: string | null;
  teacher: string | null;
  assigned: string;
  due: string;
  text: string;
  remark: string | null;
  done: boolean;
  attachments: string[];
}

export interface Exam {
  id: number;
  kind: string | null;
  name: string;
  subject: string | null;
  date: string;
  start: string | null;
  end: string | null;
  teachers: string[];
  rooms: string[];
  text: string | null;
  grade: string | null;
  /** Selbst eingetragen. */
  local: boolean;
}

export interface UntisMessage {
  id: number;
  subject: string;
  preview: string;
  sender: string;
  sent: string;
  read: boolean;
  attachments: boolean;
  /** Bei gesendeten: an wen. */
  recipients: string;
}

export interface UntisMessageDetail {
  id: number;
  subject: string;
  html: string;
  sender: string;
  senderId: number | null;
  sent: string;
  attachments: string[];
}

export interface UntisRecipient {
  id: number;
  name: string;
  group: string;
  tags: string[];
}

export interface UntisCompose {
  allowed: boolean;
  recipients: UntisRecipient[];
  maxFileSize: number | null;
  maxFileCount: number | null;
}

export interface Deadline {
  id: number;
  name: string;
  kind: string;
  courseId: number;
  course: string;
  moduleId: number | null;
  /** Unix-Zeit. */
  time: number;
  url: string | null;
}

export interface RecentFile {
  id: number;
  courseId: number;
  course: string;
  name: string;
  downloadedAt: number;
}

export interface MappingEntry {
  subject: string;
  longName: string;
  courseId: number | null;
  /** Vorschlag, noch nicht bestätigt. */
  suggested: boolean;
}

export interface Member {
  id: number;
  name: string;
  image: string | null;
  canMessage: boolean;
}

export interface Conversation {
  id: number;
  name: string;
  subtitle: string | null;
  image: string | null;
  kind: "individual" | "group" | "self";
  unread: number;
  memberCount: number;
  favourite: boolean;
  muted: boolean;
  members: Member[];
  last: { preview: string; time: number; mine: boolean } | null;
}

export interface ChatMessage {
  id: number;
  from: number;
  fromName: string | null;
  /** Bereinigtes HTML. */
  html: string;
  time: number;
  mine: boolean;
}

export interface Chat {
  id: number;
  /** Älteste zuerst. */
  messages: ChatMessage[];
  hasMore: boolean;
}

export interface MoodleNotification {
  id: number;
  subject: string;
  html: string;
  time: number;
  read: boolean;
  url: string | null;
  urlName: string | null;
  component: string | null;
}

export type Folder = "inbox" | "starred" | "sent" | "drafts" | "trash";

export interface MailCourse {
  id: number;
  shortName: string;
  fullName: string;
  unread: number;
  drafts: number;
}

export interface MailStatus {
  courses: MailCourse[];
  unread: number;
  drafts: number;
}

export interface Person {
  id: number;
  name: string;
  image: string | null;
}

export interface Recipient extends Person {
  kind: "to" | "cc" | "bcc";
}

export interface MailSummary {
  id: number;
  subject: string;
  time: number;
  unread: boolean;
  starred: boolean;
  draft: boolean;
  attachments: number;
  course: string;
  sender: Person;
  recipients: Recipient[];
}

export interface MailAttachment {
  name: string;
  size: number;
  mime: string;
  url: string;
}

export interface MailReference {
  id: number;
  subject: string;
  html: string;
  time: number;
  sender: Person;
  attachments: MailAttachment[];
}

export interface MailMessage {
  id: number;
  subject: string;
  html: string;
  time: number;
  unread: boolean;
  starred: boolean;
  deleted: boolean;
  draft: boolean;
  courseId: number;
  course: string;
  sender: Person;
  recipients: Recipient[];
  attachments: MailAttachment[];
  references: MailReference[];
}

export interface MailDraft {
  id: number;
  courseId: number;
  subject: string;
  text: string;
  quote: string | null;
  quoteHtml: string | null;
  to: Person[];
  cc: Person[];
  bcc: Person[];
  attachments: MailAttachment[];
  references: MailReference[];
}

export interface DraftInput {
  courseId: number;
  to: number[];
  cc: number[];
  bcc: number[];
  subject: string;
  text: string;
  quote: string | null;
  add: string[];
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
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  syncStatus: () => invoke<SyncStatus>("sync_status"),
  syncNow: () => invoke<CourseSync[]>("sync_now"),

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
    courseViewed: (id: number) => invoke<void>("moodle_course_viewed", { id }),
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

  messages: {
    counts: () => invoke<Counts>("message_counts"),
    refreshCounts: () => invoke<Counts>("refresh_message_counts"),
    conversations: () => invoke<Conversation[]>("moodle_conversations"),
    chat: (conversationId: number, before = 0) =>
      invoke<Chat>("moodle_chat", { conversationId, before }),
    markChatRead: (conversationId: number) =>
      invoke<void>("moodle_mark_chat_read", { conversationId }),
    send: (conversationId: number, text: string) =>
      invoke<ChatMessage>("moodle_send_chat", { conversationId, text }),
    sendToUser: (userId: number, text: string) =>
      invoke<number>("moodle_send_to_user", { userId, text }),
    searchPeople: (text: string) => invoke<Member[]>("moodle_search_people", { text }),
    notifications: () => invoke<MoodleNotification[]>("moodle_notifications"),
    markNotificationRead: (id: number) => invoke<void>("moodle_mark_notification_read", { id }),
    markAllNotificationsRead: () => invoke<void>("moodle_mark_all_notifications_read"),
  },

  untis: {
    search: (query: string) => invoke<School[]>("untis_search", { query }),
    login: (school: School, username: string, password: string, remember: boolean) =>
      invoke<{ account: Account; persistent: boolean }>("untis_login", {
        school,
        username,
        password,
        remember,
      }),
    week: (date: string) => invoke<Week>("untis_week", { date }),
    upcoming: () => invoke<Lesson[]>("untis_upcoming"),
    homework: () => invoke<Listed<Homework>>("untis_homework"),
    setHomeworkDone: (id: number, done: boolean) =>
      invoke<void>("untis_set_homework_done", { id, done }),
    exams: () => invoke<Listed<Exam>>("untis_exams"),
    addExam: (exam: { date: string; start: string | null; subject: string | null; name: string; note: string | null }) =>
      invoke<void>("untis_add_exam", { exam }),
    deleteExam: (id: number) => invoke<void>("untis_delete_exam", { id }),
    messages: () => invoke<UntisMessage[]>("untis_messages"),
    sentMessages: () => invoke<UntisMessage[]>("untis_sent_messages"),
    message: (id: number) => invoke<UntisMessageDetail>("untis_message", { id }),
    dayNews: () => invoke<{ subject: string; html: string }[]>("untis_day_news"),
    openWeb: () => invoke<void>("untis_open_web"),
    compose: () => invoke<UntisCompose>("untis_compose"),
    send: (message: { subject: string; content: string; recipients: number[] }, files: string[]) =>
      invoke<void>("untis_send_message", { message, files }),
  },

  today: {
    deadlines: () => invoke<Deadline[]>("moodle_deadlines"),
    recentFiles: () => invoke<RecentFile[]>("moodle_recent_files"),
  },

  mapping: {
    list: () => invoke<MappingEntry[]>("mapping_list"),
    set: (subject: string, courseId: number | null) =>
      invoke<void>("mapping_set", { subject, courseId }),
  },

  mail: {
    status: () => invoke<MailStatus | null>("mail_status"),
    list: (folder: Folder, courseId: number | null, search: string, offset: number) =>
      invoke<{ messages: MailSummary[]; total: number }>("mail_list", {
        folder,
        courseId,
        search,
        offset,
      }),
    message: (id: number) => invoke<MailMessage>("mail_message", { id }),
    setFlag: (id: number, flag: "starred" | "unread", value: boolean) =>
      invoke<void>("mail_set_flag", { id, flag, value }),
    /** 0 = wiederherstellen, 1 = Papierkorb, 2 = endgültig. */
    setDeleted: (id: number, deleted: 0 | 1 | 2) => invoke<void>("mail_set_deleted", { id, deleted }),
    searchUsers: (courseId: number, text: string) =>
      invoke<Person[]>("mail_search_users", { courseId, text }),
    create: (kind: "new" | "reply" | "replyAll" | "forward", id: number) =>
      invoke<MailDraft>("mail_create", { kind, id }),
    draft: (id: number) => invoke<MailDraft>("mail_draft", { id }),
    save: (id: number, input: DraftInput, send: boolean) =>
      invoke<void>("mail_save", { id, input, send }),
    openAttachment: (messageId: number, name: string) =>
      invoke<void>("mail_open_attachment", { messageId, name }),
  },
};
