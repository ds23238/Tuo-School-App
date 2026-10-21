<script lang="ts">
  import type { Module, Section } from "../lib/api";
  import { courseContext } from "../lib/courseContext";
  import { fill } from "../lib/format";
  import { i18n } from "../lib/i18n/index.svelte";
  import Html from "./Html.svelte";
  import Icon from "./Icon.svelte";
  import ModuleRow from "./ModuleRow.svelte";

  let { section, nested = false }: { section: Section; nested?: boolean } = $props();
  const ctx = courseContext();

  let open = $state(true);
  const title = $derived(section.name || fill(i18n.t.course.section, { n: section.number }));

  function tracked(modules: Module[]): Module[] {
    return modules.flatMap((m) => (m.subsection ? tracked(m.subsection.modules) : m.completed === null ? [] : [m]));
  }
  const withCompletion = $derived(tracked(section.modules));
  const done = $derived(withCompletion.filter((m) => m.completed).length);
</script>

<section id="section-{section.id}" class:nested class:card={!nested} class:unavailable={!section.available}>
  <button class="head" onclick={() => (open = !open)} aria-expanded={open}>
    <span class="chevron" class:closed={!open}><Icon name="chevronDown" size={18} /></span>
    {#if !section.available}<Icon name="lock" size={18} />{/if}
    <svelte:element this={nested ? "h3" : "h2"}>{title}</svelte:element>
    {#if !section.visible}<span class="pill">{i18n.t.course.hiddenFromStudents}</span>{/if}
    {#if withCompletion.length}
      <span class="count muted">{fill(i18n.t.course.doneOf, { done, total: withCompletion.length })}</span>
    {/if}
  </button>
  {#if open}
    <div class="content">
      {#if section.availability}
        <div class="availability"><Html html={section.availability} moodleBase={ctx.base} /></div>
      {/if}
      {#if section.summary}
        <div class="summary"><Html html={section.summary} moodleBase={ctx.base} /></div>
      {/if}
      {#each section.modules as module (module.id)}
        <ModuleRow {module} />
      {/each}
    </div>
  {/if}
</section>

<style>
  section {
    padding: 18px 22px;
    scroll-margin-top: 16px;
  }
  section.nested {
    margin: 12px 0 4px;
    padding: 14px 16px;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--surface-2);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    border: 0;
    padding: 0;
    background: none;
    cursor: pointer;
    text-align: left;
  }
  .chevron {
    display: flex;
    color: var(--muted);
    transition: transform 0.15s;
  }
  .chevron.closed {
    transform: rotate(-90deg);
  }
  h2 {
    font-size: 18px;
    font-weight: 700;
    flex: 1;
  }
  h3 {
    font-size: 15px;
    font-weight: 700;
    flex: 1;
  }
  .count {
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
  }
  .content {
    margin-top: 12px;
  }
  .summary {
    margin-bottom: 12px;
    font-size: 14px;
    color: var(--text-2);
  }
  .availability {
    margin-bottom: 12px;
    padding: 10px 14px;
    border-radius: 10px;
    background: var(--surface-3);
    font-size: 13px;
    color: var(--text-2);
  }
  .unavailable h2 {
    color: var(--muted);
  }
</style>
