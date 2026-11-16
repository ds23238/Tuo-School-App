<script lang="ts">
  import HybridBrowser from "../components/HybridBrowser.svelte";
  import DisplayModeSwitch from "../components/DisplayModeSwitch.svelte";
  import { appearance } from "../lib/appearance.svelte";
  import type { DisplayMode } from "../lib/api";
  import Icon from "../components/Icon.svelte";
  import Chats from "../components/messages/Chats.svelte";
  import Mail from "../components/messages/Mail.svelte";
  import Notifications from "../components/messages/Notifications.svelte";
  import UntisMessages from "../components/messages/UntisMessages.svelte";
  import { api, isCommandError, type Account, type MailStatus } from "../lib/api";
  import { errorText } from "../lib/errors";
  import { i18n } from "../lib/i18n/index.svelte";
  import { inbox } from "../lib/inbox.svelte";

  type Tab = "chats" | "mail" | "notifications" | "untis";

  let modeError = $state<string | null>(null);
  let untisAccountInfo = $state<Account | null>(null);
  let account = $state<Account | null | undefined>(undefined);
  let untisAccount = $state(false);
  let mail = $state<MailStatus | null>(null);
  let mailError = $state<string | null>(null);
  let tab = $state<Tab>(readTab());

  function readTab(): Tab {
    try {
      const t = localStorage.getItem("messages.tab");
      if (t === "chats" || t === "mail" || t === "notifications" || t === "untis") return t;
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
    untisAccountInfo = accounts.find((a) => a.service === "untis") ?? null;
    untisAccount = !!untisAccountInfo;
    if (!account) return;
    try {
      mail = await api.mail.status();
    } catch (e) {
      // Nur wenn es Mail gibt, die Anmeldung dafür aber gerade scheitert.
      if (isCommandError(e) && e.code === "mailLogin") mailError = errorText(e);
    }
  });

  const showMail = $derived(!!mail || !!mailError);
  type TabInfo = { id: Tab; icon: string; label: string; count: number };
  const tabs = $derived.by(() => {
    const list: TabInfo[] = [];
    if (account) {
      list.push({ id: "chats", icon: "message", label: i18n.t.messages.chats, count: inbox.chats });
      if (showMail) list.push({ id: "mail", icon: "mail", label: i18n.t.messages.mail, count: inbox.mail ?? 0 });
      list.push({
        id: "notifications",
        icon: "bell",
        label: i18n.t.messages.notifications,
        count: inbox.notifications,
      });
    }
    if (untisAccount) {
      list.push({ id: "untis", icon: "timetable", label: i18n.t.untis.messages, count: inbox.untis ?? 0 });
    }
    return list;
  });
  const current = $derived(tabs.some((t) => t.id === tab) ? tab : (tabs[0]?.id ?? "chats"));
  const area = $derived(current === "untis" ? "untisMessages" : "moodleMessages");
  const mode = $derived(current === "untis" ? appearance.untisMessagesDisplay : appearance.moodleMessagesDisplay);
  const webAccount = $derived(current === "untis" ? untisAccountInfo : account);
  const target = $derived(current === "untis" ? undefined : current === "mail" ? "local/mail/view.php" : current === "notifications" ? "message/output/popup/notifications.php" : "message/index.php");
  const title = $derived(tabs.find((t) => t.id === current)?.label ?? i18n.t.messages.title);
  async function setMode(next: DisplayMode) {
    try { modeError = null; await appearance.setDisplay(area, next); }
    catch (e) { modeError = errorText(e); }
  }
</script>

<div class="page">
  <header>
    <h1>{i18n.t.messages.title}</h1>
    {#if account || untisAccount}
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

  {#if account || untisAccount}
    <DisplayModeSwitch {mode} onchange={setMode} />
  {/if}
  {#if modeError}<p class="notice error" role="alert">{modeError}</p>{/if}
  {#if mode === "web" && webAccount}
    <div class="web-content">
      {#key current}<HybridBrowser site={webAccount.service} {target} title={title} showModeSwitch={false} onlocal={() => setMode("native")} />{/key}
    </div>
  {:else if current === "untis" && untisAccount}
    <UntisMessages />
  {:else if account === null}
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
  .web-content { flex: 1; min-height: 0; }
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
