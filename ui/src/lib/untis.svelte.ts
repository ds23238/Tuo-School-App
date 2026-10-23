import { api, type Homework, type Lesson, type MappingEntry } from "./api";
import { appearance } from "./appearance.svelte";
import { nav } from "./nav.svelte";

/** Farbton je Fach, damit dasselbe Fach überall gleich aussieht. */
function subjectHue(name: string): number {
  let hash = 0;
  for (const c of name) hash = (hash * 31 + c.charCodeAt(0)) >>> 0;
  return hash % 360;
}

/** Standardfarbe eines Fachs als `#rrggbb` (für Farbwähler). */
export function defaultSubjectColor(short: string): string {
  const h = subjectHue(short) / 360;
  const [s, l] = [0.6, 0.5];
  const f = (n: number) => {
    const k = (n + h * 12) % 12;
    const c = l - s * Math.min(l, 1 - l) * Math.max(-1, Math.min(k - 3, 9 - k, 1));
    return Math.round(c * 255)
      .toString(16)
      .padStart(2, "0");
  };
  return `#${f(0)}${f(8)}${f(4)}`;
}

/**
 * Farbe eines Fachs; `key` ist Kürzel oder ausgeschriebener Name (Hausaufgaben
 * und Prüfungen kennen nur den Namen). Eigene Farben aus den Einstellungen gehen vor.
 */
export function subjectColor(key: string): string {
  const entry = mapping.entries.find((e) => e.subject === key || e.longName === key);
  const short = entry?.subject ?? key;
  return appearance.subjectColors[short] ?? defaultSubjectColor(short);
}

/** ISO-Datum in Ortszeit (nicht UTC wie `toISOString`). */
export function isoDate(date: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

export function parseIso(date: string): Date {
  const [y, m, d] = date.split("-").map(Number);
  return new Date(y!, m! - 1, d!);
}

/** Zuordnung Fach → Kurs, geteilt von Stundenplan und Heute-Ansicht. */
class Mapping {
  entries = $state<MappingEntry[]>([]);

  load() {
    return api.mapping.list().then(
      (e) => (this.entries = e),
      () => {},
    );
  }

  courseFor(lesson: Lesson): number | null {
    const name = lesson.subject?.name;
    return this.entries.find((e) => e.subject === name)?.courseId ?? null;
  }

  async set(subject: string, courseId: number | null) {
    await api.mapping.set(subject, courseId);
    await this.load();
  }
}

export const mapping = new Mapping();

/** Öffnet den Moodle-Kurs zu einer Stunde (Ansicht „Kurse“). */
export async function openCourse(courseId: number) {
  const course = (await api.moodle.courses()).find((c) => c.id === courseId);
  if (course) nav.openCourse(course);
}

export type LessonState =
  | "cancelled"
  | "noTeacher"
  | "substitution"
  | "roomChange"
  | "changed"
  | "exam"
  | "other"
  | "regular";

/** Genauere Art einer Stunde als `status`, z. B. Vertretung ohne Lehrkraft. */
export function lessonState(l: Lesson): LessonState {
  if (l.status === "cancelled") return "cancelled";
  const missing = (name: string) => !name || /^-+$/.test(name) || name === "?";
  const present = l.teachers.filter((t) => !t.absent && !missing(t.name));
  if ((l.teachers.length > 0 && present.length === 0) || (l.status === "changed" && l.teachers.length === 0)) {
    return "noTeacher";
  }
  if (l.teachers.some((t) => t.absent)) return "substitution";
  if (l.teachers.some((t) => t.previous)) return "substitution";
  if (l.rooms.some((r) => r.previous)) return "roomChange";
  if (l.status === "changed") return "changed";
  if (l.status === "exam" || l.status === "other") return l.status;
  return "regular";
}

/** Hausaufgabe gehört zur Stunde: gleiches Fach, fällig oder aufgegeben an dem Tag. */
export function homeworkFor(l: Lesson, homework: Homework[]): { due: Homework[]; assigned: Homework[] } {
  const names = [l.subject?.name, l.subject?.longName]
    .filter((n): n is string => !!n)
    .map((n) => n.toLowerCase());
  const same = (h: Homework) => !!h.subject && names.includes(h.subject.toLowerCase());
  return {
    due: homework.filter((h) => same(h) && h.due === l.date),
    assigned: homework.filter((h) => same(h) && h.assigned === l.date && h.due !== l.date),
  };
}
