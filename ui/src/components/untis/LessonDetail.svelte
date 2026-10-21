<script lang="ts">
  import type { Lesson, UntisElement } from "../../lib/api";
  import { fill } from "../../lib/format";
  import { i18n } from "../../lib/i18n/index.svelte";
  import { mapping, openCourse, parseIso } from "../../lib/untis.svelte";
  import Icon from "../Icon.svelte";

  let { lesson, onclose }: { lesson: Lesson; onclose: () => void } = $props();
  let element = $state<HTMLElement | null>(null);

  // Liegt die Ansicht unterhalb des Rasters, sichtbar machen.
  $effect(() => {
    void lesson.id;
    element?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  });

  const courseId = $derived(mapping.courseFor(lesson));
  const day = $derived(
    new Intl.DateTimeFormat(i18n.locale, { weekday: "long", day: "numeric", month: "long" }).format(
      parseIso(lesson.date),
    ),
  );
  const statusText = $derived(
    lesson.status === "cancelled"
      ? i18n.t.untis.cancelled
      : lesson.status === "changed"
        ? i18n.t.untis.changed
        : lesson.status === "exam"
          ? i18n.t.untis.exam
          : null,
  );

  function names(list: UntisElement[]): string {
    return list
      .map((e) =>
        e.previous ? `${e.longName || e.name} (${fill(i18n.t.untis.instead, { name: e.previous })})` : e.longName || e.name,
      )
      .join(", ");
  }
</script>

<aside class="card detail {lesson.status}" bind:this={element}>
  <div class="head">
    <div>
      <h3>{lesson.subject?.longName ?? lesson.text ?? "–"}</h3>
      <p class="muted small">{day} · {lesson.start}–{lesson.end}</p>
    </div>
    <button class="btn ghost small" onclick={onclose} aria-label={i18n.t.assign.cancel}>
      <Icon name="x" size={16} />
    </button>
  </div>
  {#if statusText}<span class="pill status">{statusText}</span>{/if}
  <dl>
    {#if lesson.teachers.length}<dt>{i18n.t.untis.teacher}</dt><dd>{names(lesson.teachers)}</dd>{/if}
    {#if lesson.rooms.length}<dt>{i18n.t.untis.room}</dt><dd>{names(lesson.rooms)}</dd>{/if}
    {#if lesson.classes.length}<dt>{i18n.t.untis.classes}</dt><dd>{names(lesson.classes)}</dd>{/if}
    {#if lesson.substitution}<dt>{i18n.t.untis.substitution}</dt><dd>{lesson.substitution}</dd>{/if}
    {#if lesson.info}<dt>{i18n.t.untis.info}</dt><dd>{lesson.info}</dd>{/if}
    {#if lesson.text && lesson.subject}<dt>{i18n.t.untis.info}</dt><dd>{lesson.text}</dd>{/if}
  </dl>
  {#if courseId}
    <button class="btn small" onclick={() => openCourse(courseId)}>
      <Icon name="courses" size={16} />
      {i18n.t.untis.openCourse}
    </button>
  {/if}
</aside>

<style>
  .detail {
    padding: 16px 18px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    align-items: flex-start;
  }
  .head {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    width: 100%;
  }
  h3 {
    font-size: 17px;
  }
  .head p {
    margin: 2px 0 0;
  }
  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 4px 14px;
    margin: 0;
    font-size: 14px;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
  }
  .cancelled .status {
    background: var(--danger-soft);
    color: var(--danger);
  }
  .changed .status {
    background: var(--accent-soft);
    color: var(--accent-ink);
  }
  .exam .status {
    background: var(--p-assessment-soft);
    color: var(--p-assessment);
  }
</style>
