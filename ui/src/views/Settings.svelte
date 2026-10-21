<script lang="ts">
  import { getVersion } from "@tauri-apps/api/app";
  import { ask, open } from "@tauri-apps/plugin-dialog";
  import { api, type Account, type Settings } from "../lib/api";
  import { i18n, locales } from "../lib/i18n/index.svelte";

  let settings = $state<Settings | null>(null);
  let accounts = $state<Account[]>([]);
  let keyringOk = $state(true);
  let error = $state<string | null>(null);
  const version = getVersion().catch(() => "–");

  async function run(action: () => Promise<void>) {
    try {
      error = null;
      await action();
    } catch (e) {
      error = String(e);
    }
  }

  run(async () => {
    [settings, accounts] = await Promise.all([api.getSettings(), api.listAccounts()]);
    keyringOk = await api.checkKeyring().then(
      () => true,
      () => false,
    );
  });

  function update(patch: Partial<Settings>) {
    if (!settings) return;
    const next = { ...settings, ...patch };
    run(async () => {
      await api.saveSettings(next);
      settings = next;
      i18n.use(next.locale);
    });
  }

  async function chooseSyncDir() {
    if (!settings) return;
    const dir = await open({ directory: true, defaultPath: settings.syncDir });
    if (typeof dir === "string") update({ syncDir: dir });
  }

  async function signOut(account: Account) {
    if (!(await ask(i18n.t.settings.signOutConfirm, { kind: "warning" }))) return;
    run(async () => {
      await api.removeAccount(account.service);
      accounts = accounts.filter((a) => a.service !== account.service);
    });
  }
</script>

{#if error}
  <p class="notice error">{i18n.t.error.generic}: {error}</p>
{/if}
{#if !keyringOk}
  <p class="notice warning">{i18n.t.settings.keyringMissing}</p>
{/if}

{#if settings}
  <dl>
    <dt><label for="locale">{i18n.t.settings.language}</label></dt>
    <dd>
      <select
        id="locale"
        value={settings.locale ?? ""}
        onchange={(e) => update({ locale: e.currentTarget.value || null })}
      >
        <option value="">{i18n.t.settings.systemLanguage}</option>
        {#each locales as l (l)}
          <option value={l}>{i18n.t.locale[l]}</option>
        {/each}
      </select>
    </dd>

    <dt>{i18n.t.settings.syncDir}</dt>
    <dd class="row">
      <code>{settings.syncDir}</code>
      <button onclick={chooseSyncDir}>{i18n.t.settings.change}</button>
    </dd>

    <dt>{i18n.t.settings.accounts}</dt>
    <dd>
      {#each accounts as account (account.service)}
        <div class="row">
          <span>
            <strong>{i18n.t.service[account.service]}</strong>
            {account.displayName ?? account.username} · {account.baseUrl}
          </span>
          <button onclick={() => signOut(account)}>{i18n.t.settings.signOut}</button>
        </div>
      {:else}
        <span class="muted">{i18n.t.settings.noAccounts}</span>
      {/each}
    </dd>

    <dt>{i18n.t.settings.version}</dt>
    <dd>{#await version then v}{v}{/await}</dd>
  </dl>
{/if}

<style>
  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 16px 24px;
    align-items: baseline;
    margin: 0;
  }
  dd {
    margin: 0;
  }
  .row {
    display: flex;
    gap: 12px;
    align-items: baseline;
  }
  .muted {
    color: var(--text-muted);
  }
  code {
    word-break: break-all;
  }
  .notice {
    padding: 8px 12px;
    border-radius: 6px;
    margin: 0 0 16px;
  }
  .warning {
    background: var(--warning-soft);
  }
  .error {
    background: var(--error-soft);
  }
</style>
