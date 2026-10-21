<script lang="ts">
  import type { Section } from "../lib/api";
  import { fill } from "../lib/format";
  import { i18n } from "../lib/i18n/index.svelte";
  import CourseModule from "./CourseModule.svelte";
  import Html from "./Html.svelte";

  let { section, nested = false }: { section: Section; nested?: boolean } = $props();

  const title = $derived(section.name || fill(i18n.t.course.section, { n: section.number }));
</script>

<details class:nested class:unavailable={!section.available} open>
  <summary>
    <span class="title">{title}</span>
    {#if !section.visible}<span class="badge">{i18n.t.course.hiddenFromStudents}</span>{/if}
  </summary>
  {#if section.availability}
    <div class="availability"><Html html={section.availability} /></div>
  {/if}
  {#if section.summary}
    <div class="summary"><Html html={section.summary} /></div>
  {/if}
  {#each section.modules as module (module.id)}
    <CourseModule {module} />
  {/each}
</details>

<style>
  details {
    border-top: 1px solid var(--border);
    padding: 12px 0;
  }
  details.nested {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 12px;
    margin-top: 8px;
  }
  summary {
    cursor: pointer;
    display: flex;
    gap: 8px;
    align-items: baseline;
  }
  summary::before {
    content: "▸";
    color: var(--text-muted);
    width: 1em;
  }
  details[open] > summary::before {
    content: "▾";
  }
  .title {
    font-weight: 600;
    font-size: 1.05rem;
  }
  .nested .title {
    font-size: 1rem;
  }
  .summary,
  .availability {
    margin: 8px 0;
  }
  .availability {
    color: var(--text-muted);
    font-size: 0.9em;
  }
  .unavailable > summary {
    opacity: 0.6;
  }
</style>
