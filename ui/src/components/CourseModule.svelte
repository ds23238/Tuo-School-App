<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { Module } from "../lib/api";
  import { formatDate, formatSize } from "../lib/format";
  import { i18n } from "../lib/i18n/index.svelte";
  import CourseSection from "./CourseSection.svelte";
  import Html from "./Html.svelte";

  let { module }: { module: Module } = $props();

  const target = $derived(module.link ?? module.url);
  // Seiten und Bücher liefern ihren Inhalt intern als Dateien mit.
  const files = $derived(["resource", "folder"].includes(module.kind) ? module.files : []);

  function open() {
    if (target) openUrl(target).catch(() => {});
  }
</script>

{#if module.subsection}
  <CourseSection section={module.subsection} nested />
{:else if module.kind === "label"}
  <div class="label" style:margin-left="{module.indent * 24}px">
    {#if module.description}<Html html={module.description} />{/if}
  </div>
{:else}
  <div class="module" class:unavailable={!module.available} style:margin-left="{module.indent * 24}px">
    <div class="head">
      {#if module.icon}<img class="icon" src={module.icon} alt="" />{/if}
      <div class="main">
        <div class="name">
          {#if target && module.available}
            <button class="link" onclick={open} title={i18n.t.course.openInBrowser}>{module.name}</button>
          {:else}
            <span>{module.name}</span>
          {/if}
          <span class="muted kind">{module.kindName}</span>
          {#if module.completed}<span class="badge done">✓ {i18n.t.course.completed}</span>{/if}
          {#if !module.visible}<span class="badge">{i18n.t.course.hiddenFromStudents}</span>{/if}
        </div>
        {#each module.dates as date (date.label)}
          <div class="muted small">{date.label} {formatDate(date.timestamp)}</div>
        {/each}
        {#if module.availability}
          <div class="muted small"><Html html={module.availability} /></div>
        {/if}
        {#if module.description}
          <div class="description"><Html html={module.description} /></div>
        {/if}
        {#if files.length}
          <ul class="files">
            {#each files as file (file.path + file.name)}
              <li>
                <span>{file.path === "/" ? "" : file.path.replace(/^\//, "")}{file.name}</span>
                <span class="muted small">{formatSize(file.size)}</span>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .label {
    margin: 8px 0;
  }
  .module {
    padding: 8px 0;
  }
  .head {
    display: flex;
    gap: 10px;
    align-items: flex-start;
  }
  .icon {
    width: 24px;
    height: 24px;
    flex: none;
    margin-top: 1px;
  }
  .main {
    min-width: 0;
    flex: 1;
  }
  .name {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 10px;
    align-items: baseline;
  }
  button.link {
    border: 0;
    padding: 0;
    background: none;
    color: var(--accent);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  button.link:hover {
    text-decoration: underline;
  }
  .kind,
  .small {
    font-size: 0.85em;
  }
  .description {
    margin-top: 4px;
    font-size: 0.95em;
  }
  .files {
    list-style: none;
    margin: 6px 0 0;
    padding: 0;
  }
  .files li {
    display: flex;
    gap: 10px;
    align-items: baseline;
    padding: 2px 0;
  }
  .done {
    color: var(--success);
  }
  .unavailable {
    opacity: 0.6;
  }
</style>
