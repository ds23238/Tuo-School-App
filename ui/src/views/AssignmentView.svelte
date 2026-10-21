<script lang="ts">
  import { ask, open } from "@tauri-apps/plugin-dialog";
  import Html from "../components/Html.svelte";
  import Icon from "../components/Icon.svelte";
  import { api, type Assignment, type Course, type Module } from "../lib/api";
  import { errorText } from "../lib/errors";
  import { fill, formatDeadline, formatSize, formatDate } from "../lib/format";
  import { i18n } from "../lib/i18n/index.svelte";
  import { nav } from "../lib/nav.svelte";

  let { course, module }: { course: Course; module: Module } = $props();

  let a = $state<Assignment | null>(null);
  let base = $state<string | null>(null);
  let error = $state<string | null>(null);
  let info = $state<string | null>(null);
  let busy = $state(false);

  // Bearbeitungsmodus
  let editing = $state(false);
  let keep = $state<string[]>([]);
  let add = $state<string[]>([]);
  let text = $state("");
  let statement = $state(false);

  const now = Date.now() / 1000;
  const words = $derived(text.trim() ? text.trim().split(/\s+/).length : 0);
  const fileCount = $derived(keep.length + add.length);
  const tooMany = $derived(!!a?.fileSubmission && fileCount > a.fileSubmission.maxFiles);
  const tooLong = $derived(!!a?.wordLimit && words > a.wordLimit);

  async function load() {
    try {
      a = await api.moodle.assignment(course.id, module.id);
    } catch (e) {
      error = errorText(e);
    }
  }

  function startEdit() {
    if (!a) return;
    keep = a.files.map((f) => f.name);
    add = [];
    text = a.textPlain;
    editing = true;
    info = null;
  }

  async function pick() {
    const picked = await open({ multiple: true, directory: false });
    if (Array.isArray(picked)) add = [...add, ...picked.filter((p) => !add.includes(p))];
  }

  async function save() {
    if (!a) return;
    busy = true;
    error = null;
    try {
      await api.moodle.saveSubmission(course.id, module.id, {
        keep,
        add,
        text: a.textSubmission ? text : null,
      });
      editing = false;
      info = i18n.t.assign.saved;
      await load();
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }

  async function submit() {
    if (!a) return;
    if (!(await ask(i18n.t.assign.submitConfirm, { kind: "warning" }))) return;
    busy = true;
    error = null;
    try {
      await api.moodle.submit(a.id, statement);
      info = i18n.t.assign.submitted;
      await load();
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }

  function openFile(name: string, feedback: boolean) {
    api.moodle.openAssignmentFile(course.id, module.id, feedback, name).catch((e) => (error = errorText(e)));
  }

  function basename(path: string): string {
    return path.split(/[\\/]/).pop() ?? path;
  }

  const statusText = $derived.by(() => {
    switch (a?.status) {
      case "submitted":
        return i18n.t.assign.statusSubmitted;
      case "draft":
        return i18n.t.assign.statusDraft;
      case "reopened":
        return i18n.t.assign.statusReopened;
      default:
        return i18n.t.assign.statusNew;
    }
  });

  api.listAccounts().then((x) => (base = x.find((y) => y.service === "moodle")?.baseUrl ?? null));
  load();
</script>

<div class="page">
  <button class="back" onclick={() => nav.back()}>
    <Icon name="chevronLeft" size={16} />{course.fullName}
  </button>

  <div class="title">
    <div class="tile"><Icon name="assign" /></div>
    <h1>{module.name}</h1>
  </div>

  {#if a}
    <div class="pills">
      {#if a.allowFrom && a.allowFrom > now}<span class="pill">{fill(i18n.t.assign.opens, { date: formatDeadline(a.allowFrom) })}</span>{/if}
      {#if a.dueDate}
        <span class="pill" class:accent={a.dueDate > now}>
          <Icon name="clock" size={13} stroke={2.5} />{fill(i18n.t.assign.due, { date: formatDeadline(a.dueDate) })}
        </span>
      {/if}
      {#if a.cutoffDate}<span class="pill">{fill(i18n.t.assign.cutoff, { date: formatDeadline(a.cutoffDate) })}</span>{/if}
      {#if module.url}
        <button class="btn small" onclick={() => api.moodle.openInBrowser(module.url!)}>
          <Icon name="external" size={14} />{i18n.t.assign.openInMoodle}
        </button>
      {/if}
    </div>
  {/if}

  {#if error}<p class="notice error">{error}</p>{/if}
  {#if info}<p class="notice warning">{info}</p>{/if}

  {#if a}
    {#if a.intro}
      <div class="card block"><Html html={a.intro} moodleBase={base} /></div>
    {/if}

    <section class="card block">
      <div class="head">
        <h2>{i18n.t.assign.yourSubmission}</h2>
        <span class="pill" class:success={a.status === "submitted"} class:accent={a.status === "draft"}>
          {statusText}
        </span>
      </div>
      {#if a.submittedAt && a.status !== "new"}
        <p class="muted small">{fill(i18n.t.assign.submittedAt, { date: formatDate(a.submittedAt) })}</p>
      {/if}

      {#if !editing}
        {#if a.fileSubmission}
          <h3>{i18n.t.assign.files}</h3>
          {#if a.files.length}
            <ul class="files">
              {#each a.files as f (f.name)}
                <li>
                  <button class="file" onclick={() => openFile(f.name, false)}>
                    <Icon name="file" size={16} /><span>{f.name}</span>
                  </button>
                  <span class="muted small">{formatSize(f.size)}</span>
                </li>
              {/each}
            </ul>
          {:else}
            <p class="muted small">{i18n.t.assign.noFiles}</p>
          {/if}
        {/if}
        {#if a.textSubmission}
          <h3>{i18n.t.assign.text}</h3>
          {#if a.text}
            <div class="text"><Html html={a.text} moodleBase={base} /></div>
          {:else}
            <p class="muted small">{i18n.t.assign.noText}</p>
          {/if}
        {/if}

        <div class="buttons">
          {#if a.canEdit}
            <button class="btn" onclick={startEdit}><Icon name="upload" size={16} />{i18n.t.assign.edit}</button>
          {/if}
          {#if a.drafts && a.canSubmit && a.status === "draft"}
            {#if a.requiresStatement}
              <label class="check small"><input type="checkbox" bind:checked={statement} />{i18n.t.assign.statement}</label>
            {/if}
            <button class="btn primary" onclick={submit} disabled={busy || (a.requiresStatement && !statement)}>
              <Icon name="send" size={16} />{i18n.t.assign.submit}
            </button>
          {/if}
          {#if !a.canEdit && a.status !== "submitted"}
            <p class="muted small">{a.locked ? i18n.t.assign.locked : i18n.t.assign.closed}</p>
          {/if}
        </div>
      {:else}
        {#if a.fileSubmission}
          <h3>{i18n.t.assign.files}</h3>
          <p class="muted small">
            {fill(i18n.t.assign.maxFiles, { n: a.fileSubmission.maxFiles })}{#if a.fileSubmission.maxBytes}, {fill(i18n.t.assign.maxSize, { size: formatSize(a.fileSubmission.maxBytes) })}{/if}{#if a.fileSubmission.types.length}. {fill(i18n.t.assign.types, { types: a.fileSubmission.types.join(", ") })}{/if}
          </p>
          {#if keep.length || add.length}
          <ul class="files">
            {#each keep as name (name)}
              <li>
                <span class="file"><Icon name="file" size={16} /><span>{name}</span></span>
                <button class="btn ghost small" onclick={() => (keep = keep.filter((k) => k !== name))}>
                  <Icon name="x" size={14} />{i18n.t.assign.remove}
                </button>
              </li>
            {/each}
            {#each add as path (path)}
              <li>
                <span class="file"><Icon name="plus" size={16} /><span>{basename(path)}</span></span>
                <button class="btn ghost small" onclick={() => (add = add.filter((p) => p !== path))}>
                  <Icon name="x" size={14} />{i18n.t.assign.remove}
                </button>
              </li>
            {/each}
          </ul>
          {/if}
          {#if tooMany}<p class="notice error small">{fill(i18n.t.assign.maxFiles, { n: a.fileSubmission.maxFiles })}</p>{/if}
          <button class="btn add" onclick={pick}><Icon name="plus" size={16} />{i18n.t.assign.addFiles}</button>
        {/if}
        {#if a.textSubmission}
          <h3><label for="text">{i18n.t.assign.text}</label></h3>
          <textarea id="text" bind:value={text} rows="10"></textarea>
          <p class="muted small" class:over={tooLong}>
            {fill(i18n.t.assign.words, { n: words })}{#if a.wordLimit}&nbsp;· {fill(i18n.t.assign.wordLimit, { n: a.wordLimit })}{/if}
          </p>
        {/if}
        <div class="buttons">
          <button class="btn primary" onclick={save} disabled={busy || tooMany || tooLong}>
            {busy ? i18n.t.assign.saving : i18n.t.assign.save}
          </button>
          <button class="btn" onclick={() => (editing = false)} disabled={busy}>{i18n.t.assign.cancel}</button>
        </div>
      {/if}
    </section>

    {#if a.grade || a.feedback || a.feedbackFiles.length}
      <section class="card block">
        <div class="head"><h2>{i18n.t.assign.feedback}</h2></div>
        {#if a.gradedAt}<p class="muted small">{fill(i18n.t.assign.gradedAt, { date: formatDate(a.gradedAt) })}</p>{/if}
        {#if a.grade}
          <h3>{i18n.t.assign.grade}</h3>
          <div class="grade"><Html html={a.grade} /></div>
        {/if}
        {#if a.feedback}<div class="text"><Html html={a.feedback} moodleBase={base} /></div>{/if}
        {#if a.feedbackFiles.length}
          <ul class="files">
            {#each a.feedbackFiles as f (f.name)}
              <li>
                <button class="file" onclick={() => openFile(f.name, true)}>
                  <Icon name="file" size={16} /><span>{f.name}</span>
                </button>
                <span class="muted small">{formatSize(f.size)}</span>
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    {/if}
  {:else if !error}
    <p class="muted">{i18n.t.page.loading}</p>
  {/if}
</div>

<style>
  .page {
    max-width: 900px;
    margin: 0 auto;
    padding: 20px 40px 48px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .back {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    border: 0;
    background: none;
    padding: 6px 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--accent-strong);
    cursor: pointer;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .tile {
    width: 44px;
    height: 44px;
    border-radius: 12px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--p-assessment);
    background: var(--p-assessment-soft);
  }
  h1 {
    font-size: 26px;
    font-weight: 700;
  }
  .pills {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }
  .block {
    padding: 22px 24px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  h2 {
    font-size: 18px;
  }
  h3 {
    font-size: 14px;
    margin-top: 8px;
  }
  p {
    margin: 0;
  }
  .files {
    list-style: none;
    margin: 0;
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
  }
  .file {
    display: flex;
    align-items: center;
    gap: 10px;
    border: 0;
    background: none;
    padding: 6px 0;
    font-size: 14px;
    text-align: left;
    min-width: 0;
  }
  button.file {
    cursor: pointer;
  }
  button.file:hover span {
    color: var(--accent-strong);
    text-decoration: underline;
  }
  .text {
    padding: 12px 14px;
    border-radius: 10px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    font-size: 14px;
  }
  textarea {
    width: 100%;
    resize: vertical;
    line-height: 1.5;
  }
  .over {
    color: var(--danger);
  }
  .buttons {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    margin-top: 8px;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .grade {
    font-size: 18px;
    font-weight: 700;
  }
  .notice {
    margin: 0;
  }
  .add {
    align-self: flex-start;
  }
</style>
