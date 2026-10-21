<script lang="ts">
  import Icon from "../components/Icon.svelte";
  import Chats from "../components/messages/Chats.svelte";
  import Mail from "../components/messages/Mail.svelte";
  import Notifications from "../components/messages/Notifications.svelte";
  import { api, isCommandError, type Account, type MailStatus } from "../lib/api";
  import { errorText } from "../lib/errors";
  import { i18n } from "../lib/i18n/index.svelte";
  import { inbox } from "../lib/inbox.svelte";

  type Tab = "chats" | "mail" | "notifications";

  let account = $state<Account | null | undefined>(undefined);
  let mail = $state<MailStatus | null>(null);
  let mailError = $state<string | null>(null);
  let tab = $state<Tab>(readTab());

  function readTab(): Tab {
    try {
      const t = localStorage.getItem("messages.tab");
      if (t === "chats" || t === "mail" || t === "notifications") return t;
    } catch {
      // Standard
    }
    return "chats";
  }

  function choose(t: Tab) {
    tab = t;
    try {
      localStorage.setItem("messages.tab", t);
    } catch {
      // Nur bis zum Neustart.
    }
  }

  // Frische Zahlen, z. B. direkt nach der Anmeldung.
  inbox.refresh();
  api.listAccounts().then(async (accounts) => {
    account = accounts.find((a) => a.service === "moodle") ?? null;
    if (!account) return;
    try {
      mail = await api.mail.status();
    } catch (e) {
      // Nur wenn es Mail gibt, die Anmeldung dafür aber gerade scheitert.
      if (isCommandError(e) && e.code === "mailLogin") mailError = errorText(e);
    }
  });

  const showMail = $derived(!!mail || !!mailError);
  const tabs = $derived(
    [
      { id: "chats" as Tab, icon: "message", label: i18n.t.messages.chats, count: inbox.chats },
      showMail && { id: "mail" as Tab, icon: "mail", label: i18n.t.messages.mail, count: inbox.mail ?? 0 },
      {
        id: "notifications" as Tab,
        icon: "bell",
        label: i18n.t.messages.notifications,
        count: inbox.notifications,
      },
    ].filter((t) => !!t),
  );
  const current = $derived(tab === "mail" && !showMail ? "chats" : tab);
</script>

<div class="page">
  <header>
    <h1>{i18n.t.messages.title}</h1>
    {#if account}
      <div class="tabs" role="tablist">
        {#each tabs as t (t.id)}
          <button role="tab" class="tab" class:on={current === t.id} aria-selected={current === t.id} onclick={() => choose(t.id)}>
            <Icon name={t.icon} size={16} />
            {t.label}
            {#if t.count > 0}<span class="count">{t.count}</span>{/if}
          </button>
        {/each}
      </div>
    {/if}
  </header>

  {#if account === null}
    <p class="muted">{i18n.t.messages.signIn}</p>
  {:else if account}
    {#if current === "chats"}
      <Chats moodleBase={account.baseUrl} />
    {:else if current === "mail"}
      {#if mail}
        <Mail status={mail} moodleBase={account.baseUrl} />
      {:else}
        <p class="notice error">{mailError}</p>
      {/if}
    {:else}
      <div class="scroll"><Notifications moodleBase={account.baseUrl} /></div>
    {/if}
  {/if}
</div>

<style>
  .page {
    height: 100vh;
    max-width: 1280px;
    margin: 0 auto;
    padding: 28px 32px 24px;
    display: flex;
    flex-direction: column;
    gap: 18px;
    box-sizing: border-box;
  }
  header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  h1 {
    font-size: 28px;
    font-weight: 700;
  }
  .tabs {
    display: flex;
    gap: 4px;
    padding: 4px;
    background: var(--surface-3);
    border-radius: 12px;
  }
  .tab {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    border: 0;
    background: none;
    padding: 8px 14px;
    border-radius: 9px;
    font-weight: 500;
    color: var(--text-2);
    cursor: pointer;
  }
  .tab.on {
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow), 0 1px 3px rgba(0, 0, 0, 0.08);
  }
  .count {
    background: var(--accent);
    color: var(--on-accent);
    border-radius: 999px;
    padding: 0 7px;
    font-size: 12px;
    font-weight: 700;
  }
  .scroll {
    overflow-y: auto;
    min-height: 0;
  }
  .notice {
    margin: 0;
  }
</style>
