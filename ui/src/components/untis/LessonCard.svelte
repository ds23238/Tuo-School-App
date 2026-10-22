<script lang="ts">
  import type { Lesson } from "../../lib/api";
  import { lessonState, subjectHue } from "../../lib/untis.svelte";

  /** Eine Stunde im Raster: Fach, Raum, Lehrkraft; Änderungen farbig. */
  let {
    lesson,
    selected = false,
    onselect,
  }: { lesson: Lesson; selected?: boolean; onselect: () => void } = $props();

  const hue = $derived(subjectHue(lesson.subject?.name ?? lesson.text ?? "?"));
  const changedRoom = $derived(lesson.rooms.some((r) => r.previous));
  const changedTeacher = $derived(lesson.teachers.some((t) => t.previous));
</script>

<button
  class="lesson {lesson.status} {lessonState(lesson)}"
  class:selected
  style="--hue:{hue}"
  onclick={onselect}
  title={[lesson.subject?.longName, lesson.substitution, lesson.info].filter(Boolean).join("\n")}
>
  <span class="subject">{lesson.subject?.name ?? lesson.text ?? "–"}</span>
  <span class="meta">
    <span class:mark={changedRoom}>{lesson.rooms.map((r) => r.name).join(", ")}</span>
    <span class:mark={changedTeacher}>{lesson.teachers.map((t) => t.name).join(", ")}</span>
  </span>
</button>

<style>
  .lesson {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px 8px;
    border: 1px solid transparent;
    border-left: 4px solid hsl(var(--hue) 60% 50%);
    border-radius: 8px;
    background: hsl(var(--hue) 70% 50% / 0.1);
    color: var(--text);
    text-align: left;
    cursor: pointer;
    font: inherit;
    overflow: hidden;
  }
  .lesson:hover,
  .lesson.selected {
    border-color: hsl(var(--hue) 60% 50%);
  }
  .subject {
    font-weight: 700;
    font-size: 13px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    column-gap: 6px;
    font-size: 11.5px;
    color: var(--text-2);
  }
  .mark {
    color: var(--accent-ink);
    font-weight: 700;
  }
  .cancelled {
    border-left-color: var(--danger);
    background: var(--danger-soft);
  }
  .cancelled .subject,
  .cancelled .meta {
    text-decoration: line-through;
    color: var(--danger);
  }
  .changed {
    border-left-color: var(--accent);
    background: var(--accent-soft);
  }
  .noTeacher {
    border-left-color: var(--danger);
    background: var(--danger-soft);
  }
  .exam {
    border-left-color: var(--p-assessment);
    background: var(--p-assessment-soft);
  }
  .other {
    opacity: 0.7;
  }
</style>
