<script lang="ts">
  import CourseSection from "../components/CourseSection.svelte";
  import { api, type Course, type CourseView } from "../lib/api";
  import { errorText } from "../lib/errors";
  import { fill, formatDate } from "../lib/format";
  import { i18n } from "../lib/i18n/index.svelte";

  let { course, onback }: { course: Course; onback: () => void } = $props();

  let view = $state<CourseView | null>(null);
  let refreshing = $state(true);
  let error = $state<string | null>(null);

  async function load() {
    // Erst den gespeicherten Stand zeigen, dann aktualisieren.
    view = await api.moodle.course(course.id).catch(() => null);
    try {
      view = await api.moodle.refreshCourse(course.id);
      error = null;
    } catch (e) {
      error = errorText(e);
    } finally {
      refreshing = false;
    }
  }

  load();
</script>

<div class="bar">
  <button onclick={onback}>← {i18n.t.course.back}</button>
  {#if refreshing}<span class="muted">{i18n.t.moodle.refreshing}</span>{/if}
</div>
<h2>{course.fullName}</h2>

{#if error}
  <p class="notice {view ? 'warning' : 'error'}">
    {#if view}{fill(i18n.t.course.offline, { date: formatDate(view.fetchedAt) })} · {/if}{error}
  </p>
{/if}

{#if view}
  {#each view.sections as section (section.id)}
    <CourseSection {section} />
  {:else}
    <p class="muted">{i18n.t.course.empty}</p>
  {/each}
{:else if refreshing}
  <p class="muted">{i18n.t.course.loading}</p>
{/if}

<style>
  .bar {
    display: flex;
    gap: 16px;
    align-items: baseline;
  }
  h2 {
    margin: 16px 0 8px;
    font-size: 1.3rem;
  }
</style>
