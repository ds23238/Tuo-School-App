<script lang="ts">
  import { untrack } from "svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import {
    api,
    type Folder,
    type MailDraft,
    type MailMessage,
    type MailStatus,
    type MailSummary,
  } from "../../lib/api";
  import { errorText } from "../../lib/errors";
  import { formatDate, formatShort, formatSize } from "../../lib/format";
  import { i18n } from "../../lib/i18n/index.svelte";
  import { inbox } from "../../lib/inbox.svelte";
  import Avatar from "../Avatar.svelte";
  import Html from "../Html.svelte";
  import Icon from "../Icon.svelte";
  import MailCompose from "./MailCompose.svelte";

  let { status, moodleBase }: { status: MailStatus; moodleBase: string } = $props();

  const folders: { id: Folder; icon: string }[] = [
    { id: "inbox", icon: "inbox" },
    { id: "starred", icon: "star" },
    { id: "sent", icon: "send" },
    { id: "drafts", icon: "edit" },
    { id: "trash", icon: "trash" },
  ];

  let folder = $state<Folder>("inbox");
  let courseId = $state<number | null>(null);
  let search = $state("");
  let list = $state<MailSummary[] | null>(null);
  let total = $state(0);
  let message = $state<MailMessage | null>(null);
  let draft = $state<{ draft: MailDraft; fresh: boolean } | null>(null);
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let loading = $state(false);

  async function load(more = false) {
    loading = true;
    error = null;
    try {
      const page = await api.mail.list(folder, courseId, search, more ? (list?.length ?? 0) : 0);
      list = more ? [...(list ?? []), ...page.messages] : page.messages;
      total = page.total;
    } catch (e) {
      error = errorText(e);
    } finally {
      loading = false;
    }
  }

  function choose(f: Folder) {
    folder = f;
    message = null;
    draft = null;
    load();
  }

  async function openMail(m: MailSummary) {
    notice = null;
    error = null;
    try {
      if (m.draft) {
        message = null;
        draft = { draft: await api.mail.draft(m.id), fresh: false };
        return;
      }
      draft = null;
      message = await api.mail.message(m.id);
      m.unread = false;
    } catch (e) {
      error = errorText(e);
    }
  }

  async function create(kind: "new" | "reply" | "replyAll" | "forward", id: number) {
    error = null;
    notice = null;
    try {
      draft = { draft: await api.mail.create(kind, id), fresh: true };
      message = null;
    } catch (e) {
      error = errorText(e);
    }
  }

  function done(result: "sent" | "saved" | "closed") {
    draft = null;
    notice = result === "sent" ? i18n.t.mail.sentDone : result === "saved" ? i18n.t.mail.saved : null;
    load();
    inbox.refresh();
  }

  async function toggleStar(m: { id: number; starred: boolean }) {
    const next = !m.starred;
    try {
      await api.mail.setFlag(m.id, "starred", next);
      m.starred = next;
      const row = list?.find((r) => r.id === m.id);
      if (row) row.starred = next;
    } catch (e) {
      error = errorText(e);
    }
  }

  async function markUnread(m: MailMessage) {
    await api.mail.setFlag(m.id, "unread", true).catch((e) => (error = errorText(e)));
    message = null;
    load();
    inbox.refresh();
  }

  async function remove(m: MailMessage, deleted: 0 | 1 | 2) {
    if (deleted === 2 && !(await ask(i18n.t.mail.deleteForeverConfirm, { kind: "warning" }))) return;
    try {
      await api.mail.setDeleted(m.id, deleted);
      message = null;
      load();
      inbox.refresh();
    } catch (e) {
      error = errorText(e);
    }
  }

  function openAttachment(messageId: number, name: string) {
    api.mail.openAttachment(messageId, name).catch((e) => (error = errorText(e)));
  }

  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  function onsearch() {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => load(), 350);
  }

  function people(m: MailSummary): string {
    if (folder === "sent" || folder === "drafts") {
      return m.recipients.map((r) => r.name).join(", ") || "–";
    }
    return m.sender.name;
  }

  const compose = $derived(status.courses.length > 0);

  $effect(() => {
    void inbox.generation;
    untrack(() => {
      if (folder === "inbox" && !draft) load();
    });
  });
</script>

<div class="toolbar">
  <div class="folders">
    {#each folders as f (f.id)}
      <button class="chip" class:on={folder === f.id} onclick={() => choose(f.id)}>
        <Icon name={f.icon} size={14} />
        {i18n.t.mail[f.id]}
        {#if f.id === "inbox" && (inbox.mail ?? status.unread) > 0}
          <span class="count">{inbox.mail ?? status.unread}</span>
        {/if}
      </button>
    {/each}
  </div>
  <div class="filters">
    <label class="search">
      <Icon name="search" size={16} />
      <input type="search" bind:value={search} oninput={onsearch} placeholder={i18n.t.mail.search} aria-label={i18n.t.mail.search} />
    </label>
    <select bind:value={courseId} onchange={() => load()} aria-label={i18n.t.mail.course}>
      <option value={null}>{i18n.t.mail.allCourses}</option>
      {#each status.courses as c (c.id)}
        <option value={c.id}>{c.shortName}</option>
      {/each}
    </select>
    {#if compose}
      <button class="btn primary" onclick={() => create("new", courseId ?? status.courses[0]!.id)}>
        <Icon name="plus" size={16} />
        {i18n.t.mail.compose}
      </button>
    {/if}
  </div>
</div>

<div class="split" class:detail={!!(message || draft)}>
  <aside>
    {#if list?.length === 0 && !loading}<p class="muted small pad">{i18n.t.mail.empty}</p>{/if}
    {#each list ?? [] as m (m.id)}
      <button class="row" class:active={message?.id === m.id || draft?.draft.id === m.id} class:unread={m.unread} onclick={() => openMail(m)}>
        <Avatar name={people(m)} image={folder === "sent" || folder === "drafts" ? null : m.sender.image} size={34} />
        <span class="text">
          <span class="line">
            <span class="who">{people(m)}</span>
            <span class="time">{formatShort(m.time)}</span>
          </span>
          <span class="line">
            <span class="subject">
              {#if m.draft && folder !== "drafts"}<span class="pill">{i18n.t.mail.draftLabel}</span>{/if}
              {m.subject || i18n.t.mail.noSubject}
            </span>
            <span class="marks">
              {#if m.attachments > 0}<Icon name="paperclip" size={14} />{/if}
              {#if m.starred}<span class="star"><Icon name="star" size={14} /></span>{/if}
            </span>
          </span>
          <span class="course">{m.course}</span>
        </span>
      </button>
    {/each}
    {#if list && list.length < total}
      <button class="btn ghost small more" onclick={() => load(true)} disabled={loading}>{i18n.t.mail.more}</button>
    {/if}
  </aside>

  <section class="pane">
    {#if notice}<p class="notice">{notice}</p>{/if}
    {#if error}<p class="notice error">{error}</p>{/if}
    {#if draft}
      {#key draft.draft.id}
        <MailCompose draft={draft.draft} fresh={draft.fresh} courses={status.courses} {moodleBase} ondone={done} />
      {/key}
    {:else if message}
      {@const m = message}
      <article>
        <header>
          <button class="btn ghost small back" onclick={() => (message = null)} aria-label={i18n.t.messages.back}>
            <Icon name="chevronLeft" size={16} />
          </button>
          <h2>{m.subject || i18n.t.mail.noSubject}</h2>
          <button class="btn ghost small" class:starred={m.starred} onclick={() => toggleStar(m)} aria-label={m.starred ? i18n.t.mail.unstar : i18n.t.mail.star} title={m.starred ? i18n.t.mail.unstar : i18n.t.mail.star}>
            <Icon name="star" size={18} />
          </button>
        </header>
        <div class="meta">
          <Avatar name={m.sender.name} image={m.sender.image} size={40} />
          <div>
            <div><strong>{m.sender.name}</strong> <span class="pill">{m.course}</span></div>
            <div class="muted small">
              {i18n.t.mail.to}: {m.recipients.filter((r) => r.kind === "to").map((r) => r.name).join(", ")}
              {#if m.recipients.some((r) => r.kind === "cc")}
                · {i18n.t.mail.cc}: {m.recipients.filter((r) => r.kind === "cc").map((r) => r.name).join(", ")}
              {/if}
            </div>
            <div class="muted small">{formatDate(m.time)}</div>
          </div>
        </div>
        <div class="actions">
          {#if !m.deleted}
            <button class="btn small" onclick={() => create("reply", m.id)}><Icon name="reply" size={14} />{i18n.t.mail.reply}</button>
            {#if m.recipients.length > 1}
              <button class="btn small" onclick={() => create("replyAll", m.id)}><Icon name="replyAll" size={14} />{i18n.t.mail.replyAll}</button>
            {/if}
            <button class="btn small" onclick={() => create("forward", m.id)}><Icon name="forward" size={14} />{i18n.t.mail.forward}</button>
            <button class="btn ghost small" onclick={() => markUnread(m)}><Icon name="eyeOff" size={14} />{i18n.t.mail.markUnread}</button>
            <button class="btn ghost small" onclick={() => remove(m, 1)}><Icon name="trash" size={14} />{i18n.t.mail.delete}</button>
          {:else}
            <button class="btn small" onclick={() => remove(m, 0)}><Icon name="undo" size={14} />{i18n.t.mail.restore}</button>
            <button class="btn ghost small" onclick={() => remove(m, 2)}><Icon name="trash" size={14} />{i18n.t.mail.deleteForever}</button>
          {/if}
        </div>
        <div class="content"><Html html={m.html} {moodleBase} /></div>
        {#if m.attachments.length > 0}
          <div class="files">
            {#each m.attachments as a (a.name)}
              <button class="file" onclick={() => openAttachment(m.id, a.name)}>
                <Icon name="paperclip" size={14} /> {a.name} <span class="muted">{formatSize(a.size)}</span>
              </button>
            {/each}
          </div>
        {/if}
        {#each m.references as ref (ref.id)}
          <details class="quote">
            <summary>{ref.sender.name} · {formatDate(ref.time)} · {ref.subject}</summary>
            <Html html={ref.html} {moodleBase} />
            {#each ref.attachments as a (a.name)}
              <button class="file" onclick={() => openAttachment(m.id, a.name)}>
                <Icon name="paperclip" size={14} /> {a.name}
              </button>
            {/each}
          </details>
        {/each}
      </article>
    {:else}
      <div class="empty">
        <Icon name="mail" size={40} stroke={1.5} />
        <p class="muted">{i18n.t.mail.selectMail}</p>
      </div>
    {/if}
  </section>
</div>

<style>
  .toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    justify-content: space-between;
    align-items: center;
  }
  .folders,
  .filters {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    border: 1px solid var(--border-strong);
    background: var(--surface);
    border-radius: 999px;
    padding: 5px 12px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-2);
    cursor: pointer;
  }
  .chip.on {
    background: var(--text);
    border-color: var(--text);
    color: var(--bg);
  }
  .count {
    background: var(--accent);
    color: var(--on-accent);
    border-radius: 999px;
    padding: 0 6px;
    font-size: 11px;
    font-weight: 700;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 10px;
    border: 1px solid var(--border-strong);
    border-radius: 10px;
    background: var(--surface);
    color: var(--muted);
  }
  .search input {
    border: 0;
    background: none;
    padding: 8px 0;
    outline: none;
    color: var(--text);
    width: 180px;
  }
  .split {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 340px 1fr;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    overflow: hidden;
  }
  aside {
    border-right: 1px solid var(--border);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }
  .row {
    display: flex;
    gap: 10px;
    padding: 10px 12px;
    width: 100%;
    border: 0;
    border-bottom: 1px solid var(--border);
    background: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }
  .row:hover {
    background: var(--hover);
  }
  .row.active {
    background: var(--accent-soft);
  }
  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .line {
    display: flex;
    gap: 8px;
    justify-content: space-between;
    align-items: center;
  }
  .who,
  .subject {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .unread .who,
  .unread .subject {
    font-weight: 700;
  }
  .subject {
    font-size: 13px;
  }
  .time,
  .course {
    font-size: 12px;
    color: var(--muted);
  }
  .marks {
    display: flex;
    gap: 4px;
    color: var(--muted);
    flex: none;
  }
  .star,
  .starred {
    color: var(--accent);
  }
  .star :global(svg),
  .starred :global(svg) {
    fill: currentColor;
  }
  .more {
    margin: 10px auto;
  }
  .pad {
    padding: 12px;
  }
  .pane {
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
    overflow-y: auto;
  }
  article {
    padding: 16px 20px 24px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  article header {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  article h2 {
    flex: 1;
    font-size: 20px;
  }
  .back {
    display: none;
  }
  .meta {
    display: flex;
    gap: 12px;
    align-items: flex-start;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .content {
    border-top: 1px solid var(--border);
    padding-top: 14px;
  }
  .files {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .file {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface-2);
    color: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .file:hover {
    border-color: var(--accent);
  }
  .quote {
    border-left: 3px solid var(--border-strong);
    padding-left: 12px;
  }
  .quote summary {
    cursor: pointer;
    font-size: 13px;
    color: var(--muted);
  }
  .notice {
    margin: 12px 20px 0;
  }
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--muted);
  }
  @media (max-width: 900px) {
    .split {
      grid-template-columns: 1fr;
    }
    .split.detail aside,
    .split:not(.detail) .pane {
      display: none;
    }
    .back {
      display: inline-flex;
    }
  }
</style>
