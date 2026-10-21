<script lang="ts">
  import { getVersion } from "@tauri-apps/api/app";
  import { invoke } from "@tauri-apps/api/core";
  import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
  import { ask, open } from "@tauri-apps/plugin-dialog";
  import Icon from "../components/Icon.svelte";
  import { api, type Account, type Settings } from "../lib/api";
  import { errorText } from "../lib/errors";
  import { fill, formatDate } from "../lib/format";
  import { i18n, locales } from "../lib/i18n/index.svelte";
  import { sync } from "../lib/sync.svelte";

  let settings = $state<Settings | null>(null);
  let accounts = $state<Account[]>([]);
  let keyringOk = $state(true);
  let autostart = $state<boolean | null>(null);
  let graphicsConfigurable = $state(false);
  invoke<boolean>("graphics_configurable").then((v) => (graphicsConfigurable = v));
  const intervals = [15, 30, 60, 120, 240];

  function intervalLabel(minutes: number): string {
    if (minutes < 60) return fill(i18n.t.settings.minutes, { n: minutes });
    if (minutes === 60) return i18n.t.settings.hour;
    return fill(i18n.t.settings.hours, { n: minutes / 60 });
  }

  invoke<boolean>("autostart_supported").then(async (supported) => {
    autostart = supported ? await isEnabled().catch(() => null) : null;
  });

  function toggleAutostart() {
    const next = !autostart;
    run(async () => {
      await (next ? enable() : disable());
      autostart = await isEnabled();
    });
  }
  let error = $state<string | null>(null);
  const version = getVersion().catch(() => "–");

  async function run(action: () => Promise<void>) {
    try {
      error = null;
      await action();
    } catch (e) {
      error = errorText(e);
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

<div class="page">
  <h1>{i18n.t.nav.settings}</h1>
  {#if error}<p class="notice error">{error}</p>{/if}
  {#if !keyringOk}<p class="notice warning">{i18n.t.settings.keyringMissing}</p>{/if}

  {#if settings}
    <section class="card">
      <h2>{i18n.t.settings.general}</h2>
      <div class="row">
        <label for="locale">{i18n.t.settings.language}</label>
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
      </div>
      {#if graphicsConfigurable}
        <div class="row">
          <div>
            <label for="graphics">{i18n.t.settings.graphics}</label>
            <div class="muted small">{i18n.t.settings.graphicsHint}</div>
          </div>
          <select id="graphics" value={settings.graphics} onchange={(e) => update({ graphics: e.currentTarget.value })}>
            <option value="auto">{i18n.t.settings.graphicsAuto}</option>
            <option value="always">{i18n.t.settings.graphicsAlways}</option>
            <option value="never">{i18n.t.settings.graphicsNever}</option>
            <option value="compat">{i18n.t.settings.graphicsCompat}</option>
          </select>
        </div>
      {/if}
      <div class="row">
        <span>{i18n.t.settings.syncDir}</span>
        <div class="value">
          <code>{settings.syncDir}</code>
          <button class="btn small" onclick={chooseSyncDir}><Icon name="folder" size={14} />{i18n.t.settings.change}</button>
        </div>
      </div>
    </section>

    <section class="card">
      <h2>{i18n.t.settings.sync}</h2>
      <div class="row">
        <div>
          <div>{i18n.t.settings.autoSync}</div>
          <div class="muted small">{i18n.t.settings.autoSyncHint}</div>
        </div>
        <button
          class="switch"
          role="switch"
          aria-checked={settings.syncEnabled}
          aria-label={i18n.t.settings.autoSync}
          onclick={() => update({ syncEnabled: !settings!.syncEnabled })}
        ></button>
      </div>
      <div class="row">
        <label for="interval">{i18n.t.settings.interval}</label>
        <select
          id="interval"
          value={settings.syncIntervalMinutes}
          disabled={!settings.syncEnabled}
          onchange={(e) => update({ syncIntervalMinutes: Number(e.currentTarget.value) })}
        >
          {#each intervals as m (m)}
            <option value={m}>{intervalLabel(m)}</option>
          {/each}
        </select>
      </div>
      <div class="row">
        <div>
          <div>{i18n.t.settings.notifications}</div>
          <div class="muted small">{i18n.t.settings.notificationsHint}</div>
        </div>
        <button
          class="switch"
          role="switch"
          aria-checked={settings.notifications}
          aria-label={i18n.t.settings.notifications}
          onclick={() => update({ notifications: !settings!.notifications })}
        ></button>
      </div>
      {#if autostart !== null}
        <div class="row">
          <div>
            <div>{i18n.t.settings.autostart}</div>
            <div class="muted small">{i18n.t.settings.autostartHint}</div>
          </div>
          <button
            class="switch"
            role="switch"
            aria-checked={autostart}
            aria-label={i18n.t.settings.autostart}
            onclick={toggleAutostart}
          ></button>
        </div>
      {/if}
      <div class="row">
        <span>{i18n.t.settings.lastSync}</span>
        <span class="muted">{sync.lastAt ? formatDate(sync.lastAt.getTime() / 1000) : "–"}</span>
      </div>
    </section>

    <section class="card">
      <h2>{i18n.t.settings.accounts}</h2>
      {#each accounts as account (account.service)}
        <div class="row">
          <div>
            <div class="strong">{i18n.t.service[account.service]}</div>
            <div class="muted small">{account.displayName ?? account.username} · {new URL(account.baseUrl).host}</div>
          </div>
          <button class="btn small" onclick={() => signOut(account)}>
            <Icon name="logout" size={14} />{i18n.t.settings.signOut}
          </button>
        </div>
      {:else}
        <p class="muted">{i18n.t.settings.noAccounts}</p>
      {/each}
    </section>

    <section class="card">
      <h2>{i18n.t.settings.about}</h2>
      <div class="row">
        <span>{i18n.t.settings.version}</span>
        <span class="muted">TSA-Sync {#await version then v}{v}{/await}</span>
      </div>
    </section>
  {/if}
</div>

<style>
  .page {
    max-width: 760px;
    margin: 0 auto;
    padding: 32px 40px 48px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  h1 {
    font-size: 28px;
    font-weight: 700;
  }
  section {
    padding: 8px 22px;
  }
  h2 {
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--muted);
    padding: 12px 0 4px;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 14px 0;
    border-top: 1px solid var(--border);
  }
  h2 + .row {
    border-top: 0;
  }
  .value {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  code {
    font-size: 13px;
    color: var(--text-2);
    overflow-wrap: anywhere;
  }
  .strong {
    font-weight: 600;
  }
  p {
    margin: 0;
  }
  section > p {
    padding: 12px 0;
  }
</style>
