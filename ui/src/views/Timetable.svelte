<script lang="ts">
  import Icon from "../components/Icon.svelte";
  import UntisLogin from "../components/UntisLogin.svelte";
  import ExamList from "../components/untis/ExamList.svelte";
  import HomeworkList from "../components/untis/HomeworkList.svelte";
  import WeekView from "../components/untis/WeekView.svelte";
  import { api, type Account } from "../lib/api";
  import { i18n } from "../lib/i18n/index.svelte";
  import { mapping } from "../lib/untis.svelte";

  type Tab = "week" | "homework" | "exams";
  let account = $state<Account | null | undefined>(undefined);
  let expired = $state(false);
  let notice = $state<string | null>(null);
  let tab = $state<Tab>("week");

  function load() {
    api.listAccounts().then((accounts) => {
      account = accounts.find((a) => a.service === "untis") ?? null;
    });
  }

  function onlogin(persistent: boolean) {
    expired = false;
    notice = persistent ? null : i18n.t.untis.notPersistent;
    load();
    mapping.load();
  }

  load();
  mapping.load();

  const tabs: { id: Tab; icon: string }[] = [
    { id: "week", icon: "timetable" },
    { id: "homework", icon: "assign" },
    { id: "exams", icon: "quiz" },
  ];
</script>

{#if account === undefined}
  <!-- lädt -->
{:else if !account || expired}
  <div class="page login">
    <UntisLogin notice={expired ? i18n.t.error.untisExpired : null} {onlogin} />
  </div>
{:else}
  <div class="page">
    <header>
      <div>
        <h1>{i18n.t.nav.timetable}</h1>
        <p class="muted small">{account.displayName ?? account.username} · {new URL(account.baseUrl).host}</p>
      </div>
      <div class="tabs" role="tablist">
        {#each tabs as t (t.id)}
          <button role="tab" class="tab" class:on={tab === t.id} aria-selected={tab === t.id} onclick={() => (tab = t.id)}>
            <Icon name={t.icon} size={16} />
            {i18n.t.untis[t.id]}
          </button>
        {/each}
      </div>
    </header>
    {#if notice}<p class="notice warning">{notice}</p>{/if}
    {#if tab === "week"}
      <WeekView onexpired={() => (expired = true)} />
    {:else if tab === "homework"}
      <HomeworkList />
    {:else}
      <ExamList />
    {/if}
  </div>
{/if}

<style>
  .page {
    max-width: 1280px;
    margin: 0 auto;
    padding: 28px 32px 40px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  header {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
  }
  h1 {
    font-size: 28px;
    font-weight: 700;
  }
  header p {
    margin: 6px 0 0;
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
  .login {
    align-items: center;
    padding-top: 64px;
  }
  .notice {
    margin: 0;
  }
</style>
