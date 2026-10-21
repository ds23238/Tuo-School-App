<script lang="ts">
  import { api, type Module, type MoodleFile } from "../lib/api";
  import { courseContext } from "../lib/courseContext";
  import { errorText } from "../lib/errors";
  import { fill, formatDeadline, formatSize } from "../lib/format";
  import { i18n } from "../lib/i18n/index.svelte";
  import { moduleIcon, opensInApp, purposeVar } from "../lib/modules";
  import { nav } from "../lib/nav.svelte";
  import CourseSection from "./CourseSection.svelte";
  import FileStatus from "./FileStatus.svelte";
  import Html from "./Html.svelte";
  import Icon from "./Icon.svelte";

  let { module }: { module: Module } = $props();
  const ctx = courseContext();

  let busy = $state<number | null>(null);
  let completing = $state(false);

  const kinds: Record<string, string> = $derived(i18n.t.kinds);
  const kindName = $derived(kinds[module.kind] ?? module.kindName);
  const files = $derived(["resource", "folder"].includes(module.kind) ? module.files : []);
  const single = $derived(module.kind === "resource" && files.length === 1 ? files[0] : null);
  const external = $derived(!opensInApp(module) && module.kind !== "resource" && module.kind !== "folder");
  const now = Date.now() / 1000;

  async function run(id: number | null, action: () => Promise<void>) {
    busy = id;
    try {
      await action();
    } catch (e) {
      ctx.notify(errorText(e), true);
    } finally {
      busy = null;
    }
  }

  function openFile(file: MoodleFile) {
    if (!file.state) return;
    const id = file.state.id;
    const wasRemote = file.state.status === "remote" || file.state.status === "missing";
    run(id, async () => {
      await api.moodle.openFile(id);
      if (wasRemote) await ctx.reload();
    });
  }

  function update(file: MoodleFile) {
    if (!file.state) return;
    const id = file.state.id;
    run(id, async () => {
      await api.moodle.updateFile(id);
      ctx.notify(i18n.t.file.updateDone);
      await ctx.reload();
    });
  }

  function open() {
    if (single) return openFile(single);
    if (module.kind === "folder") return;
    if (opensInApp(module)) return nav.openModule(module);
    if (module.link) return run(null, () => api.openUrl(module.link!));
    if (module.url) run(null, () => api.moodle.openInBrowser(module.url!));
  }

  async function toggleCompletion() {
    completing = true;
    try {
      await api.moodle.setCompletion(module.id, !module.completed);
      module.completed = !module.completed;
    } catch (e) {
      ctx.notify(errorText(e), true);
    } finally {
      completing = false;
    }
  }
</script>

{#if module.subsection}
  <CourseSection section={module.subsection} nested />
{:else if module.kind === "label"}
  {#if module.description}
    <div class="label" style:margin-left="{module.indent * 24}px">
      <Html html={module.description} moodleBase={ctx.base} />
    </div>
  {/if}
{:else}
  <div class="row" class:unavailable={!module.available} style:margin-left="{module.indent * 24}px">
    <div class="tile" style:--c="var(--p-{purposeVar(module.purpose)})" style:--bg="var(--p-{purposeVar(module.purpose)}-soft)">
      <Icon name={moduleIcon(module.kind)} />
    </div>
    <div class="main">
      <div class="top">
        <div class="heading">
          <div class="kind">
            {kindName}
            {#if module.kind === "folder"}· {fill(i18n.t.course.files, { n: files.length })}{/if}
            {#if !module.visible}· {i18n.t.course.hiddenFromStudents}{/if}
          </div>
          {#if module.available && module.kind !== "folder"}
            <button class="name link" onclick={open} disabled={busy !== null}>
              <span>{module.name}</span>
              {#if external}<Icon name="external" size={14} label={i18n.t.course.opensInMoodle} />{/if}
            </button>
          {:else}
            <div class="name">{module.name}</div>
          {/if}
        </div>
        {#if module.completed !== null}
          {#if module.manualCompletion}
            <button class="done" class:on={module.completed} onclick={toggleCompletion} disabled={completing}>
              {#if module.completed}<Icon name="check" size={14} stroke={3} />{i18n.t.course.completed}{:else}{i18n.t.course.markDone}{/if}
            </button>
          {:else if module.completed}
            <span class="done on static"><Icon name="check" size={14} stroke={3} />{i18n.t.course.completed}</span>
          {/if}
        {/if}
      </div>

      {#if module.dates.length || single}
        <div class="pills">
          {#each module.dates as date (date.label)}
            <span class="pill" class:accent={date.timestamp > now}>
              <Icon name="clock" size={13} stroke={2.5} />
              {date.label} {formatDeadline(date.timestamp)}
            </span>
          {/each}
          {#if single}
            <span class="pill">{single.name.split(".").pop()?.toUpperCase()} · {formatSize(single.size)}</span>
            <FileStatus state={single.state} busy={busy === single.state?.id} onupdate={() => update(single)} />
          {/if}
        </div>
      {/if}

      {#if module.availability}
        <div class="availability"><Icon name="lock" size={14} /><Html html={module.availability} moodleBase={ctx.base} /></div>
      {/if}
      {#if module.description}
        <div class="description"><Html html={module.description} moodleBase={ctx.base} /></div>
      {/if}

      {#if module.kind === "folder" && files.length}
        <ul class="files">
          {#each files as file (file.path + file.name)}
            <li>
              <button class="file" onclick={() => openFile(file)} disabled={busy !== null}>
                <Icon name="file" size={16} class={busy === file.state?.id ? "spin" : ""} />
                <span class="fname">{file.path === "/" ? "" : file.path.slice(1)}{file.name}</span>
              </button>
              <span class="fmeta">
                <FileStatus state={file.state} busy={busy === file.state?.id} onupdate={() => update(file)} />
                <span class="muted small">{formatSize(file.size)}</span>
              </span>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </div>
{/if}

<style>
  .label {
    padding: 12px 0;
    border-top: 1px solid var(--border);
  }
  .row {
    display: flex;
    align-items: flex-start;
    gap: 14px;
    padding: 14px 0;
    border-top: 1px solid var(--border);
  }
  .tile {
    width: 40px;
    height: 40px;
    border-radius: 10px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--c);
    background: var(--bg);
  }
  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .top {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }
  .heading {
    min-width: 0;
  }
  .kind {
    font-size: 12px;
    font-weight: 500;
    color: var(--muted);
  }
  .name {
    font-weight: 600;
    font-size: 15px;
    text-align: left;
  }
  .link {
    display: inline;
    align-self: flex-start;
    border: 0;
    padding: 0;
    background: none;
    cursor: pointer;
    color: var(--text);
  }
  .link:hover {
    color: var(--accent-strong);
  }
  .link :global(svg) {
    vertical-align: -2px;
    margin-left: 6px;
  }
  .pills {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin-top: 6px;
  }
  .availability {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    margin-top: 8px;
    padding: 8px 12px;
    border-radius: 10px;
    background: var(--surface-3);
    font-size: 13px;
    color: var(--text-2);
  }
  .description {
    margin-top: 6px;
    font-size: 14px;
    color: var(--text-2);
  }
  .files {
    list-style: none;
    margin: 8px 0 0;
    padding: 4px;
    border: 1px solid var(--border);
    border-radius: 10px;
  }
  .files li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 2px 6px;
    border-radius: 8px;
  }
  .files li:hover {
    background: var(--surface-2);
  }
  .file {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    border: 0;
    background: none;
    padding: 6px 0;
    cursor: pointer;
    text-align: left;
    font-size: 13px;
    color: var(--text-2);
  }
  .file:hover .fname {
    color: var(--accent-strong);
    text-decoration: underline;
  }
  .fname {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .fmeta {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: none;
  }
  .done {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-height: 32px;
    padding: 0 12px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 600;
    flex: none;
    border: 1px solid var(--border-strong);
    background: var(--surface);
    color: var(--text-2);
    cursor: pointer;
  }
  .done.on {
    border-color: transparent;
    background: var(--success-soft);
    color: var(--success);
  }
  .done.static {
    cursor: default;
  }
  .unavailable .tile,
  .unavailable .name {
    opacity: 0.6;
  }
</style>
