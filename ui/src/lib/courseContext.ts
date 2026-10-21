import { getContext, setContext } from "svelte";

/** Was Aktivitätszeilen von ihrer Kursseite brauchen. */
export interface CourseContext {
  courseId: number;
  /** Basisadresse der Moodle-Seite, z. B. `https://moodle.example/`. */
  base: string;
  reload: () => Promise<void>;
  notify: (message: string, error?: boolean) => void;
}

const key = Symbol("course");

export function setCourseContext(ctx: CourseContext) {
  setContext(key, ctx);
}

export function courseContext(): CourseContext {
  return getContext<CourseContext>(key);
}
