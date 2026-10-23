<script lang="ts">
  import { untrack } from "svelte";
  import { api, type UntisCompose as ComposeData, type UntisMessage, type UntisMessageDetail } from "../../lib/api";
  import { errorText } from "../../lib/errors";
  import { fill, formatDate, formatShort } from "../../lib/format";
  import { i18n } from "../../lib/i18n/index.svelte";
  import { inbox } from "../../lib/inbox.svelte";
  import Avatar from "../Avatar.svelte";
  import Icon from "../Icon.svelte";
  import UntisCompose from "./UntisCompose.svelte";

  let box = $state<"received" | "sent">("received");
  let list = $state<UntisMessage[] | null>(null);
  let news = $state<{ subject: string; html: string }[]>([]);
  let detail = $state<UntisMessageDetail | null>(null);
  let error = $state<string | null>(null);
  let info = $state<string | null>(null);
  let compose = $state<ComposeData | null>(null);
  let writing = $state<{ to: number[]; subject: string } | null>(null);

  /** Neue Mitteilung; bei Antworten an die Absenderin bzw. den Absender. */
  async function write(reply?: UntisMessageDetail) {
    error = null;
    info = null;
    try {
      compose ??= await api.untis.compose();
      if (!compose.allowed) {
        error = i18n.t.untis.composeNotAllowed;
        return;
      }
      if (!reply) {
        writing = { to: [], subject: "" };
        return;
      }
      const sender =
        compose.recipients.find((r) => r.id === reply.senderId) ??
        compose.recipients.find((r) => r.name === reply.sender);
      writing = {
        to: sender ? [sender.id] : [],
        subject: /^(aw|re):/i.test(reply.subject) ? reply.subject : i18n.t.untis.replyPrefix + reply.subject,
      };
    } catch (e) {
      error = errorText(e);
    }
  }

  function written(sent: boolean) {
    writing = null;
    if (sent) {
      info = i18n.t.untis.sentDone;
      if (box === "sent") load();
    }
  }

  async function load() {
    error = null;
    try {
      list = box === "sent" ? await api.untis.sentMessages() : await api.untis.messages();
    } catch (e) {
      list = [];
      error = errorText(e);
    }
  }

  function choose(next: "received" | "sent") {
    box = next;
    detail = null;
    writing = null;
    list = null;
    load();
  }

  async function open(m: UntisMessage) {
    error = null;
    info = null;
    writing = null;
    try {
      detail = await api.untis.message(m.id);
      m.read = true;
    } catch (e) {
      error = errorText(e);
    }
  }

  let opening = $state<string | null>(null);
  async function openAttachment(id: string) {
    if (!detail) return;
    opening = id;
    error = null;
    try {
      await api.untis.openAttachment(detail.id, id);
    } catch (e) {
      error = errorText(e);
    } finally {
      opening = null;
    }
  }

  const time = (sent: string) => {
    const t = Date.parse(sent);
    return Number.isNaN(t) ? sent : formatShort(t / 1000);
  };

  api.untis.dayNews().then(
    (n) => (news = n),
    () => {},
  );
  $effect(() => {
    void inbox.generation;
    untrack(load);
  });
</script>

<div class="split" class:detail={!!detail || !!writing}>
  <aside>
    <div class="boxes">
      <button class="chip" class:on={box === "received"} onclick={() => choose("received")}>
        <Icon name="inbox" size={14} />{i18n.t.untis.received}
      </button>
      <button class="chip" class:on={box === "sent"} onclick={() => choose("sent")}>
        <Icon name="send" size={14} />{i18n.t.untis.sent}
      </button>
      <span class="grow"></span>
      <button class="btn small primary" onclick={() => write()} title={i18n.t.untis.compose}>
        <Icon name="edit" size={14} />{i18n.t.untis.write}
      </button>
    </div>
    {#each box === "received" ? news : [] as n, i (i)}
      <div class="news">
        <div class="label"><Icon name="bell" size={14} /> {i18n.t.untis.dayNews}</div>
        {#if n.subject}<strong>{n.subject}</strong>{/if}
        <div class="html">{@html n.html}</div>
      </div>
    {/each}
    {#if list?.length === 0 && !error}
      <p class="muted small pad">{box === "sent" ? i18n.t.untis.noSent : i18n.t.untis.noMessages}</p>
    {/if}
    {#each list ?? [] as m (m.id)}
      <button class="row" class:active={detail?.id === m.id} class:unread={!m.read} onclick={() => open(m)}>
        <Avatar name={box === "sent" ? m.recipients || "?" : m.sender} size={34} />
        <span class="text">
          <span class="line">
            <span class="who">{box === "sent" ? fill(i18n.t.untis.to, { names: m.recipients || "–" }) : m.sender}</span>
            <span class="time">{time(m.sent)}</span>
          </span>
          <span class="subject">
            {#if m.attachments}<Icon name="paperclip" size={12} />{/if}
            {m.subject}
          </span>
          <span class="preview">{m.preview}</span>
        </span>
      </button>
    {/each}
  </aside>
  <section class="pane">
    {#if error}<p class="notice error">{error}</p>{/if}
    {#if info}<p class="notice">{info}</p>{/if}
    {#if writing && compose}
      {#key writing}
        <UntisCompose {compose} to={writing.to} subject={writing.subject} ondone={written} />
      {/key}
    {:else if detail}
      <article>
        <header>
          <button class="btn ghost small back" onclick={() => (detail = null)} aria-label={i18n.t.messages.back}>
            <Icon name="chevronLeft" size={16} />
          </button>
          <h2>{detail.subject}</h2>
        </header>
        <p class="muted small">{detail.sender} · {Number.isNaN(Date.parse(detail.sent)) ? detail.sent : formatDate(Date.parse(detail.sent) / 1000)}</p>
        <!-- Im Backend mit ammonia bereinigt. -->
        <div class="html">{@html detail.html}</div>
        {#if detail.attachments.length}
          <div class="files">
            {#each detail.attachments as a (a.id + a.name)}
              <button class="file" onclick={() => openAttachment(a.id)} disabled={opening === a.id}>
                <Icon name="paperclip" size={14} />
                {a.name}
              </button>
            {/each}
          </div>
        {/if}
        <div class="actions">
          {#if box === "received"}
            <button class="btn small primary" onclick={() => detail && write(detail)}>
              <Icon name="reply" size={14} />
              {i18n.t.untis.reply}
            </button>
          {/if}
          <button class="btn small" onclick={() => api.untis.openWeb().catch((e) => (error = errorText(e)))}>
            <Icon name="external" size={14} />
            {i18n.t.untis.openWeb}
          </button>
        </div>
      </article>
    {:else}
      <div class="empty">
        <Icon name="mail" size={40} stroke={1.5} />
        <p class="muted">{i18n.t.untis.selectMessage}</p>
      </div>
    {/if}
  </section>
</div>

<style>
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
  }
  .grow {
    flex: 1;
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
    background: var(--surface);
    color: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .file:hover {
    background: var(--hover);
  }
  .file:disabled {
    opacity: 0.6;
    cursor: progress;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .boxes {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    border: 1px solid var(--border-strong);
    background: var(--surface);
    border-radius: 999px;
    padding: 4px 12px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-2);
    cursor: pointer;
    white-space: nowrap;
  }
  .boxes .btn {
    white-space: nowrap;
  }
  .chip.on {
    background: var(--text);
    border-color: var(--text);
    color: var(--bg);
  }
  .news {
    padding: 12px;
    border-bottom: 1px solid var(--border);
    background: var(--warning-soft);
    font-size: 13px;
  }
  .label {
    display: flex;
    gap: 6px;
    align-items: center;
    font-size: 12px;
    color: var(--muted);
    margin-bottom: 4px;
  }
  .row {
    display: flex;
    gap: 10px;
    width: 100%;
    padding: 10px 12px;
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
    justify-content: space-between;
    gap: 8px;
  }
  .who,
  .subject,
  .preview {
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
  .preview {
    font-size: 12px;
    color: var(--muted);
  }
  .pad {
    padding: 12px;
  }
  .pane {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow-y: auto;
  }
  article {
    padding: 16px 20px 24px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    align-items: flex-start;
  }
  article header {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  article h2 {
    font-size: 20px;
  }
  article p {
    margin: 0;
  }
  .html {
    overflow-wrap: anywhere;
    line-height: 1.55;
  }
  .html :global(p) {
    margin: 0 0 0.6em;
  }
  .back {
    display: none;
  }
  .notice {
    margin: 12px 20px 0;
    white-space: pre-line;
    overflow-wrap: anywhere;
    user-select: text;
    -webkit-user-select: text;
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
