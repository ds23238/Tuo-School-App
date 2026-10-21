<script lang="ts">
  import { untrack } from "svelte";
  import { api, type MoodleNotification } from "../../lib/api";
  import { errorText } from "../../lib/errors";
  import { formatShort } from "../../lib/format";
  import { i18n } from "../../lib/i18n/index.svelte";
  import { inbox } from "../../lib/inbox.svelte";
  import Html from "../Html.svelte";
  import Icon from "../Icon.svelte";

  let { moodleBase }: { moodleBase: string } = $props();

  let items = $state<MoodleNotification[] | null>(null);
  let open = $state<number | null>(null);
  let error = $state<string | null>(null);

  async function load() {
    try {
      items = await api.messages.notifications();
    } catch (e) {
      error = errorText(e);
    }
  }

  async function toggle(n: MoodleNotification) {
    open = open === n.id ? null : n.id;
    if (!n.read) {
      n.read = true;
      api.messages.markNotificationRead(n.id).catch(() => {});
    }
  }

  async function readAll() {
    try {
      await api.messages.markAllNotificationsRead();
      items?.forEach((n) => (n.read = true));
    } catch (e) {
      error = errorText(e);
    }
  }

  function follow(url: string) {
    api.moodle.openInBrowser(url).catch((e) => (error = errorText(e)));
  }

  $effect(() => {
    void inbox.generation;
    untrack(load);
  });
</script>

<div class="wrap">
  <div class="tools">
    <button class="btn ghost small" onclick={readAll} disabled={!items?.some((n) => !n.read)}>
      <Icon name="check" size={16} />
      {i18n.t.messages.markAllRead}
    </button>
  </div>
  {#if error}<p class="notice error">{error}</p>{/if}
  {#if items?.length === 0}
    <p class="muted">{i18n.t.messages.noNotifications}</p>
  {/if}
  <ul>
    {#each items ?? [] as n (n.id)}
      <li class:unread={!n.read}>
        <button class="head" onclick={() => toggle(n)} aria-expanded={open === n.id}>
          <span class="dot" aria-hidden="true"></span>
          <span class="subject">{n.subject}</span>
          <span class="time">{formatShort(n.time)}</span>
          <Icon name={open === n.id ? "chevronDown" : "chevronRight"} size={16} />
        </button>
        {#if open === n.id}
          <div class="body">
            <Html html={n.html} {moodleBase} />
            {#if n.url}
              <button class="btn small" onclick={() => follow(n.url!)}>
                <Icon name="external" size={14} />
                {n.urlName ?? i18n.t.messages.openLink}
              </button>
            {/if}
          </div>
        {/if}
      </li>
    {/each}
  </ul>
</div>

<style>
  .wrap {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .tools {
    display: flex;
    justify-content: flex-end;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    overflow: hidden;
  }
  li + li {
    border-top: 1px solid var(--border);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 12px 16px;
    border: 0;
    background: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }
  .head:hover {
    background: var(--hover);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex: none;
    background: transparent;
  }
  .unread .dot {
    background: var(--accent);
  }
  .subject {
    flex: 1;
    min-width: 0;
  }
  .unread .subject {
    font-weight: 600;
  }
  .time {
    font-size: 12px;
    color: var(--muted);
    flex: none;
  }
  .body {
    padding: 0 16px 16px 36px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    align-items: flex-start;
  }
  .notice {
    margin: 0;
  }
</style>
