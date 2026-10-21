import type { Course, Module } from "./api";

export type View = "today" | "courses" | "messages" | "timetable" | "settings";

/** Aktuelle Ansicht; Kurs und Aktivität nur innerhalb von „Kurse“. */
class Nav {
  view = $state<View>("today");
  course = $state<Course | null>(null);
  module = $state<Module | null>(null);

  go(view: View) {
    this.view = view;
    this.course = null;
    this.module = null;
  }

  openCourse(course: Course) {
    this.view = "courses";
    this.course = course;
    this.module = null;
  }

  openModule(module: Module) {
    this.module = module;
  }

  back() {
    if (this.module) this.module = null;
    else this.course = null;
  }
}

export const nav = new Nav();
