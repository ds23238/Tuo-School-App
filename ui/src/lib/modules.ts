import type { Module } from "./api";

const icons: Record<string, string> = {
  resource: "fileText",
  folder: "folder",
  assign: "assign",
  forum: "forum",
  chat: "forum",
  quiz: "quiz",
  url: "link",
  page: "page",
  book: "book",
  lesson: "page",
  glossary: "book",
  wiki: "users",
  workshop: "users",
  data: "chart",
  choice: "chart",
  feedback: "chart",
  survey: "chart",
  h5pactivity: "play",
  scorm: "play",
  lti: "puzzle",
  bigbluebuttonbn: "video",
  subsection: "folder",
};

export function moduleIcon(kind: string): string {
  return icons[kind] ?? "file";
}

/** CSS-Variablen-Name für die Farbe nach Moodle-Zweck. */
export function purposeVar(purpose: string): string {
  switch (purpose) {
    case "content":
    case "assessment":
    case "communication":
    case "collaboration":
      return purpose;
    case "interactivecontent":
      return "interactive";
    default:
      return "other";
  }
}

/** Aktivitäten, die die App selbst anzeigt; alles andere öffnet Moodle. */
export function opensInApp(m: Module): boolean {
  return ["page", "book", "assign"].includes(m.kind);
}
