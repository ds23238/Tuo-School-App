import { api, type Lesson, type MappingEntry } from "./api";
import { nav } from "./nav.svelte";

/** Farbton je Fach, damit dasselbe Fach überall gleich aussieht. */
export function subjectHue(name: string): number {
  let hash = 0;
  for (const c of name) hash = (hash * 31 + c.charCodeAt(0)) >>> 0;
  return hash % 360;
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
