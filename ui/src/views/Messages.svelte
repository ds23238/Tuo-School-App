<script lang="ts">
  import HybridBrowser from "../components/HybridBrowser.svelte";
  import MessagesLocal from "./MessagesLocal.svelte";
  import { api, type Account } from "../lib/api";
  import { i18n } from "../lib/i18n/index.svelte";
  import { errorText } from "../lib/errors";

  type Tab = "chats" | "mail" | "notifications" | "untis";
  let accounts = $state<Account[]>([]);
  let loaded = $state(false);
  let error = $state<string | null>(null);
  let local = $state(false);
  let hasMail = $state(false);
  let tab = $state<Tab>("chats");
  const moodle = $derived(accounts.find((a) => a.service === "moodle"));
  const untis = $derived(accounts.find((a) => a.service === "untis"));
  const selected = $derived(tab === "untis" ? untis : moodle);
  const target = $derived(tab === "mail" ? "local/mail/view.php" : tab === "notifications" ? "message/output/popup/notifications.php" : "message/index.php");
  const label = $derived(tab === "mail" ? i18n.t.messages.mail : tab === "notifications" ? i18n.t.messages.notifications : tab === "untis" ? i18n.t.untis.messages : i18n.t.messages.chats);
  api.listAccounts().then(async (value) => {
    accounts = value;
    if (!value.some((a) => a.service === "moodle")) tab = "untis";
    else {
      try { hasMail = !!(await api.mail.status()); }
      catch (e) { hasMail = !!e && typeof e === "object" && "code" in e && e.code === "mailLogin"; }
    }
  }).catch((e) => (error = errorText(e))).finally(() => (loaded = true));
</script>

<div class="messages">
  <div class="tabs" aria-label={i18n.t.messages.title}>
    {#if moodle}
      <button class="btn small" class:on={tab === "chats" && !local} onclick={() => { tab = "chats"; local = false; }}>{i18n.t.messages.chats}</button>
      {#if hasMail}<button class="btn small" class:on={tab === "mail" && !local} onclick={() => { tab = "mail"; local = false; }}>{i18n.t.messages.mail}</button>{/if}
      <button class="btn small" class:on={tab === "notifications" && !local} onclick={() => { tab = "notifications"; local = false; }}>{i18n.t.messages.notifications}</button>
    {/if}
    {#if untis}<button class="btn small" class:on={tab === "untis" && !local} onclick={() => { tab = "untis"; local = false; }}>WebUntis</button>{/if}
    {#if local}<button class="btn small" onclick={() => (local = false)}>{i18n.t.hybrid.original}</button>{/if}
  </div>
  {#if error}<p class="notice error">{error}</p>{/if}
  <div class="content">
    {#if local}<MessagesLocal />
    {:else if selected}
      {#key tab}<HybridBrowser site={selected.service} {target} title={label} onlocal={() => (local = true)} />{/key}
    {:else if loaded}<p class="empty muted">{i18n.t.hybrid.noAccount}</p>{/if}
  </div>
</div>

<style>
  .messages { height: 100%; display: flex; flex-direction: column; min-height: 0; }
  .tabs { padding: 14px 18px 0; display: flex; flex-wrap: wrap; gap: 8px; }
  .on { border-color: var(--accent-strong); color: var(--accent-strong); }
  .content { flex: 1; min-height: 0; overflow: auto; }
  .empty { padding: 28px; }
</style>
