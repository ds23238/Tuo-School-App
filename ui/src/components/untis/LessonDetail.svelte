<script lang="ts">
  import type { Homework, Lesson, UntisElement } from "../../lib/api";
  import { fill } from "../../lib/format";
  import { i18n } from "../../lib/i18n/index.svelte";
  import { homeworkFor, lessonState, mapping, openCourse, parseIso } from "../../lib/untis.svelte";
  import Icon from "../Icon.svelte";

  /** Popup zu einer Stunde: Art, Lehrkraft mit Kürzel, Raum, Hausaufgaben. */
  let {
    lesson,
    homework = [],
    onclose,
  }: { lesson: Lesson; homework?: Homework[]; onclose: () => void } = $props();

  const courseId = $derived(mapping.courseFor(lesson));
  const state = $derived(lessonState(lesson));
  const tasks = $derived(homeworkFor(lesson, homework));
  const day = $derived(
    new Intl.DateTimeFormat(i18n.locale, { weekday: "long", day: "numeric", month: "long" }).format(
      parseIso(lesson.date),
    ),
  );

  /** „Müller (MUE)“; Kürzel allein, wenn kein Name bekannt ist. */
  function label(e: UntisElement): string {
    if (!e.name || /^-+$/.test(e.name)) return "–";
    return e.longName && e.longName !== e.name ? `${e.longName} (${e.name})` : e.name;
  }

  function names(list: UntisElement[]): string {
    return list
      .map((e) => (e.previous ? `${label(e)} – ${fill(i18n.t.untis.instead, { name: e.previous })}` : label(e)))
      .join(", ");
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
  }
</script>

<svelte:window {onkeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onclose}>
  <div
    class="card dialog {state}"
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    aria-label={lesson.subject?.longName ?? lesson.text ?? ""}
    onclick={(e) => e.stopPropagation()}
  >
    <div class="head">
      <div>
        <h3>{lesson.subject?.longName ?? lesson.text ?? "–"}</h3>
        <p class="muted small">{day} · {lesson.start}–{lesson.end}</p>
      </div>
      <button class="btn ghost small" onclick={onclose} aria-label={i18n.t.assign.cancel}>
        <Icon name="x" size={16} />
      </button>
    </div>

    <span class="pill status">{i18n.t.untis.state[state]}</span>

    <dl>
      {#if lesson.subject}
        <dt>{i18n.t.untis.subject}</dt>
        <dd>{label(lesson.subject)}</dd>
      {/if}
      <dt>{i18n.t.untis.teacher}</dt>
      <dd>{lesson.teachers.length ? names(lesson.teachers) : "–"}</dd>
      {#if lesson.rooms.length}<dt>{i18n.t.untis.room}</dt><dd>{names(lesson.rooms)}</dd>{/if}
      {#if lesson.classes.length}<dt>{i18n.t.untis.classes}</dt><dd>{lesson.classes.map((c) => c.name).join(", ")}</dd>{/if}
      {#if lesson.substitution}<dt>{i18n.t.untis.substitution}</dt><dd>{lesson.substitution}</dd>{/if}
      {#if lesson.info}<dt>{i18n.t.untis.info}</dt><dd>{lesson.info}</dd>{/if}
      {#if lesson.text && lesson.subject}<dt>{i18n.t.untis.lessonText}</dt><dd>{lesson.text}</dd>{/if}
    </dl>

    <div class="homework">
      <h4><Icon name="assign" size={16} /> {i18n.t.untis.homework}</h4>
      {#each tasks.due as h (h.id)}
        <p class:done={h.done}><span class="pill accent">{i18n.t.untis.dueToday}</span> {h.text}</p>
      {/each}
      {#each tasks.assigned as h (h.id)}
        <p class:done={h.done}>
          <span class="pill">{fill(i18n.t.untis.due, { date: new Intl.DateTimeFormat(i18n.locale, { day: "numeric", month: "numeric" }).format(parseIso(h.due)) })}</span>
          {h.text}
        </p>
      {/each}
      {#if tasks.due.length === 0 && tasks.assigned.length === 0}
        <p class="muted small">{i18n.t.untis.noLessonHomework}</p>
      {/if}
    </div>

    {#if courseId}
      <button class="btn small" onclick={() => (onclose(), openCourse(courseId))}>
        <Icon name="courses" size={16} />
        {i18n.t.untis.openCourse}
      </button>
    {/if}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: rgba(0, 0, 0, 0.35);
  }
  .dialog {
    width: min(480px, 100%);
    max-height: 100%;
    overflow-y: auto;
    padding: 20px 22px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    align-items: flex-start;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.25);
  }
  .head {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    width: 100%;
  }
  h3 {
    font-size: 18px;
  }
  h4 {
    display: flex;
    gap: 6px;
    align-items: center;
    font-size: 14px;
    margin-bottom: 4px;
  }
  .head p {
    margin: 2px 0 0;
  }
  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 6px 16px;
    margin: 0;
    font-size: 14px;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
  }
  .homework {
    width: 100%;
    border-top: 1px solid var(--border);
    padding-top: 10px;
  }
  .homework p {
    margin: 4px 0;
    white-space: pre-line;
  }
  .homework .done {
    text-decoration: line-through;
    opacity: 0.6;
  }
  .status {
    background: var(--success-soft);
    color: var(--success);
  }
  .cancelled .status,
  .noTeacher .status {
    background: var(--danger-soft);
    color: var(--danger);
  }
  .substitution .status,
  .roomChange .status,
  .changed .status {
    background: var(--accent-soft);
    color: var(--accent-ink);
  }
  .exam .status {
    background: var(--p-assessment-soft);
    color: var(--p-assessment);
  }
  .other .status {
    background: var(--surface-3);
    color: var(--muted);
  }
</style>
