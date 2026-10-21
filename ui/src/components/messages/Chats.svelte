<script lang="ts">
  import { tick, untrack } from "svelte";
  import { api, type Chat, type Conversation, type Member } from "../../lib/api";
  import { errorText } from "../../lib/errors";
  import { fill, formatDay, formatShort, formatTime } from "../../lib/format";
  import { i18n } from "../../lib/i18n/index.svelte";
  import { inbox } from "../../lib/inbox.svelte";
  import Avatar from "../Avatar.svelte";
  import Html from "../Html.svelte";
  import Icon from "../Icon.svelte";

  let { moodleBase }: { moodleBase: string } = $props();

  let conversations = $state<Conversation[] | null>(null);
  let selected = $state<Conversation | null>(null);
  /** Neue Unterhaltung mit jemandem, an den noch nichts geschickt wurde. */
  let pending = $state<Member | null>(null);
  let chat = $state<Chat | null>(null);
  let error = $state<string | null>(null);
  let text = $state("");
  let sending = $state(false);
  let searching = $state(false);
  let query = $state("");
  let people = $state<Member[] | null>(null);
  let scroller = $state<HTMLElement | null>(null);

  async function loadConversations() {
    try {
      conversations = await api.messages.conversations();
      if (selected) selected = conversations.find((c) => c.id === selected!.id) ?? selected;
    } catch (e) {
      error = errorText(e);
    }
  }

  async function scrollToEnd() {
    await tick();
    if (scroller) scroller.scrollTop = scroller.scrollHeight;
  }

  async function open(c: Conversation) {
    selected = c;
    pending = null;
    searching = false;
    chat = null;
    error = null;
    try {
      chat = await api.messages.chat(c.id);
      scrollToEnd();
      if (c.unread > 0) {
        c.unread = 0;
        await api.messages.markChatRead(c.id);
      }
    } catch (e) {
      error = errorText(e);
    }
  }

  /** Neue Nachrichten in der offenen Unterhaltung (Zählung hat sich geändert). */
  async function reloadOpen() {
    if (!selected || !chat) return;
    try {
      const fresh = await api.messages.chat(selected.id);
      const known = new Set(chat.messages.map((m) => m.id));
      const added = fresh.messages.filter((m) => !known.has(m.id));
      if (added.length > 0) {
        chat.messages.push(...added);
        scrollToEnd();
        await api.messages.markChatRead(selected.id);
      }
    } catch {
      // Beim nächsten Mal wieder.
    }
  }

  async function older() {
    if (!selected || !chat) return;
    const before = scroller?.scrollHeight ?? 0;
    const page = await api.messages.chat(selected.id, chat.messages.length);
    chat.messages.unshift(...page.messages);
    chat.hasMore = page.hasMore;
    await tick();
    if (scroller) scroller.scrollTop = scroller.scrollHeight - before;
  }

  async function send() {
    const body = text.trim();
    if (!body || sending) return;
    sending = true;
    error = null;
    try {
      if (pending) {
        const id = await api.messages.sendToUser(pending.id, body);
        text = "";
        await loadConversations();
        const c = conversations?.find((c) => c.id === id);
        if (c) await open(c);
      } else if (selected && chat) {
        const message = await api.messages.send(selected.id, body);
        chat.messages.push(message);
        text = "";
        scrollToEnd();
        loadConversations();
      }
    } catch (e) {
      error = errorText(e);
    } finally {
      sending = false;
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  }

  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  function search() {
    clearTimeout(searchTimer);
    const q = query.trim();
    if (q.length < 2) {
      people = null;
      return;
    }
    searchTimer = setTimeout(async () => {
      try {
        people = await api.messages.searchPeople(q);
      } catch (e) {
        error = errorText(e);
      }
    }, 300);
  }

  function startWith(person: Member) {
    const existing = conversations?.find(
      (c) => c.kind === "individual" && c.members.some((m) => m.id === person.id),
    );
    searching = false;
    query = "";
    people = null;
    if (existing) {
      open(existing);
      return;
    }
    selected = null;
    chat = { id: 0, messages: [], hasMore: false };
    pending = person;
  }

  const title = $derived(
    pending?.name ?? (selected?.kind === "self" ? i18n.t.messages.self : selected?.name) ?? "",
  );
  const canWrite = $derived(
    pending
      ? pending.canMessage
      : selected?.kind !== "individual" || selected.members.some((m) => m.canMessage),
  );

  function newDay(i: number): boolean {
    if (!chat) return false;
    if (i === 0) return true;
    const a = new Date(chat.messages[i - 1]!.time * 1000).toDateString();
    return a !== new Date(chat.messages[i]!.time * 1000).toDateString();
  }

  // Auch beim ersten Mal; danach, wenn das Backend neue Zahlen meldet.
  $effect(() => {
    void inbox.generation;
    untrack(() => {
      loadConversations();
      reloadOpen();
    });
  });
</script>

<div class="split" class:detail={!!(selected || pending)}>
  <aside>
    <div class="list-head">
      <button class="btn primary small" onclick={() => (searching = !searching)}>
        <Icon name={searching ? "x" : "plus"} size={16} />
        {i18n.t.messages.newChat}
      </button>
    </div>
    {#if searching}
      <div class="people">
        <!-- svelte-ignore a11y_autofocus -->
        <input
          type="text"
          bind:value={query}
          oninput={search}
          placeholder={i18n.t.messages.searchPeople}
          aria-label={i18n.t.messages.searchPeople}
          autofocus
        />
        {#if people && people.length === 0}
          <p class="muted small">{i18n.t.messages.noPeople}</p>
        {/if}
        {#each people ?? [] as person (person.id)}
          <button class="row" onclick={() => startWith(person)}>
            <Avatar name={person.name} image={person.image} size={32} />
            <span class="name">{person.name}</span>
          </button>
        {/each}
      </div>
    {/if}
    <div class="rows">
      {#if conversations?.length === 0}
        <p class="muted small pad">{i18n.t.messages.noChats}</p>
      {/if}
      {#each conversations ?? [] as c (c.id)}
        <button class="row" class:active={selected?.id === c.id} onclick={() => open(c)}>
          <Avatar name={c.kind === "self" ? i18n.t.messages.you : c.name} image={c.image} />
          <span class="text">
            <span class="line">
              <span class="name" class:bold={c.unread > 0}>
                {c.kind === "self" ? i18n.t.messages.self : c.name}
              </span>
              {#if c.last}<span class="time">{formatShort(c.last.time)}</span>{/if}
            </span>
            <span class="line">
              <span class="preview">
                {c.last?.mine ? `${i18n.t.messages.you}: ` : ""}{c.last?.preview ?? c.subtitle ?? ""}
              </span>
              {#if c.unread > 0}
                <span class="badge" title={fill(i18n.t.messages.unread, { n: c.unread })}>{c.unread}</span>
              {/if}
            </span>
          </span>
        </button>
      {/each}
    </div>
  </aside>

  <section class="pane">
    {#if selected || pending}
      <header>
        <button class="btn ghost small back" onclick={() => ((selected = null), (pending = null))}>
          <Icon name="chevronLeft" size={16} />
          <span class="sr">{i18n.t.messages.back}</span>
        </button>
        <Avatar
          name={pending?.name ?? selected?.name ?? ""}
          image={pending?.image ?? selected?.image ?? null}
          size={32}
        />
        <div>
          <h2>{title}</h2>
          {#if selected?.kind === "group"}
            <p class="muted small">{[selected.subtitle, fill(i18n.t.messages.members, { n: selected.memberCount })].filter(Boolean).join(" · ")}</p>
          {/if}
        </div>
      </header>
      <div class="messages" bind:this={scroller}>
        {#if chat?.hasMore}
          <button class="btn ghost small older" onclick={older}>{i18n.t.messages.olderMessages}</button>
        {/if}
        {#each chat?.messages ?? [] as m, i (m.id)}
          {#if newDay(i)}<div class="day">{formatDay(m.time)}</div>{/if}
          <div class="bubble" class:mine={m.mine}>
            {#if !m.mine && selected?.kind === "group"}<div class="from">{m.fromName}</div>{/if}
            <Html html={m.html} {moodleBase} />
            <div class="when">{formatTime(new Date(m.time * 1000))}</div>
          </div>
        {/each}
      </div>
      {#if error}<p class="notice error">{error}</p>{/if}
      {#if canWrite}
        <form class="composer" onsubmit={(e) => (e.preventDefault(), send())}>
          <textarea
            bind:value={text}
            {onkeydown}
            rows="2"
            placeholder={i18n.t.messages.write}
            aria-label={i18n.t.messages.write}
          ></textarea>
          <button class="btn primary" type="submit" disabled={sending || !text.trim()} aria-label={i18n.t.messages.send}>
            <Icon name="send" size={16} />
          </button>
        </form>
      {:else}
        <p class="muted small pad">{i18n.t.messages.cannotMessage}</p>
      {/if}
    {:else}
      <div class="empty">
        <Icon name="message" size={40} stroke={1.5} />
        <p class="muted">{error ?? i18n.t.messages.selectChat}</p>
      </div>
    {/if}
  </section>
</div>

<style>
  .split {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 320px 1fr;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    overflow: hidden;
  }
  aside {
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .list-head {
    padding: 12px;
    border-bottom: 1px solid var(--border);
  }
  .people {
    padding: 12px;
    border-bottom: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .people input {
    width: 100%;
    margin-bottom: 6px;
  }
  .rows {
    overflow-y: auto;
    flex: 1;
  }
  .row {
    display: flex;
    gap: 12px;
    align-items: center;
    width: 100%;
    padding: 10px 12px;
    border: 0;
    background: none;
    text-align: left;
    cursor: pointer;
    color: inherit;
    border-radius: 0;
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
    align-items: center;
  }
  .name {
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .bold {
    font-weight: 700;
  }
  .time {
    font-size: 12px;
    color: var(--muted);
    flex: none;
  }
  .preview {
    font-size: 13px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .badge {
    flex: none;
    min-width: 20px;
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--accent);
    color: var(--on-accent);
    font-size: 12px;
    font-weight: 600;
    text-align: center;
  }
  .pad {
    padding: 12px;
  }
  .pane {
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
  }
  .pane header {
    display: flex;
    gap: 12px;
    align-items: center;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
  }
  .pane h2 {
    font-size: 16px;
  }
  .pane header p {
    margin: 0;
  }
  .back {
    display: none;
  }
  .messages {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    background: var(--surface-2);
  }
  .older {
    align-self: center;
  }
  .day {
    align-self: center;
    font-size: 12px;
    color: var(--muted);
    margin: 10px 0 4px;
  }
  .bubble {
    max-width: min(75%, 560px);
    padding: 8px 12px 6px;
    border-radius: 14px 14px 14px 4px;
    background: var(--surface);
    border: 1px solid var(--border);
    align-self: flex-start;
  }
  .bubble.mine {
    align-self: flex-end;
    background: var(--accent-soft);
    border-color: transparent;
    border-radius: 14px 14px 4px 14px;
  }
  .from {
    font-size: 12px;
    font-weight: 600;
    color: var(--accent-ink);
    margin-bottom: 2px;
  }
  .when {
    font-size: 11px;
    color: var(--muted);
    text-align: right;
    margin-top: 2px;
  }
  .composer {
    display: flex;
    gap: 8px;
    padding: 12px 16px;
    border-top: 1px solid var(--border);
    align-items: flex-end;
  }
  .composer textarea {
    flex: 1;
    resize: none;
    min-height: 44px;
    max-height: 160px;
  }
  .notice {
    margin: 8px 16px;
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
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
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
