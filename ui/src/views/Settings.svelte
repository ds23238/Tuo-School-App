<script lang="ts">
  import { getVersion } from "@tauri-apps/api/app";
  import { invoke } from "@tauri-apps/api/core";
  import { ask, open } from "@tauri-apps/plugin-dialog";
  import Icon from "../components/Icon.svelte";
  import { api, type Account, type Course, type Settings } from "../lib/api";
  import { nav } from "../lib/nav.svelte";
  import { appearance } from "../lib/appearance.svelte";
  import { defaultSubjectColor, mapping } from "../lib/untis.svelte";
  import { errorText } from "../lib/errors";
  import { fill, formatDate } from "../lib/format";
  import { i18n, locales } from "../lib/i18n/index.svelte";
  import { sync } from "../lib/sync.svelte";

  let settings = $state<Settings | null>(null);
  let accounts = $state<Account[]>([]);
  let courses = $state<Course[]>([]);
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

  invoke<boolean>("autostart_enabled").then(
    (v) => (autostart = v),
    () => (autostart = null),
  );

  function toggleAutostart() {
    const next = !autostart;
    run(async () => {
      await invoke("set_autostart", { enabled: next });
      autostart = await invoke<boolean>("autostart_enabled");
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
    if (accounts.some((a) => a.service === "untis")) {
      mapping.load();
      if (accounts.some((a) => a.service === "moodle")) {
        courses = (await api.moodle.courses()).filter((c) => c.enrolled);
      }
    }
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
      appearance.apply(next);
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
      <div class="row">
        <label for="theme">{i18n.t.settings.theme}</label>
        <select id="theme" value={settings.theme} onchange={(e) => update({ theme: e.currentTarget.value })}>
          <option value="system">{i18n.t.settings.themeSystem}</option>
          <option value="light">{i18n.t.settings.themeLight}</option>
          <option value="dark">{i18n.t.settings.themeDark}</option>
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
      {#each (["moodle", "untis"] as const).filter((s) => !accounts.some((a) => a.service === s)) as service (service)}
        <div class="row">
          <div class="strong">{i18n.t.service[service]}</div>
          <button class="btn small primary" onclick={() => nav.go(service === "moodle" ? "courses" : "timetable")}>
            {i18n.t.today.connect}
          </button>
        </div>
      {/each}
    </section>

    {#if accounts.some((a) => a.service === "untis")}
      {@const withMoodle = accounts.some((a) => a.service === "moodle")}
      <section class="card">
        <h2>{i18n.t.mapping.title}</h2>
        <p class="muted small hint">{withMoodle ? i18n.t.mapping.hint : i18n.t.mapping.colorHint}</p>
        {#each mapping.entries as entry (entry.subject)}
          {@const custom = settings.subjectColors[entry.subject]}
          <div class="row">
            <div class="subject">
              <input
                type="color"
                class="swatch"
                value={custom ?? defaultSubjectColor(entry.subject)}
                onchange={(e) =>
                  update({ subjectColors: { ...settings!.subjectColors, [entry.subject]: e.currentTarget.value } })}
                aria-label={`${i18n.t.mapping.color}: ${entry.longName}`}
                title={i18n.t.mapping.color}
              />
              <div>
                <div class="strong">{entry.longName}</div>
                <div class="muted small">
                  {entry.subject}
                  {#if custom}
                    · <button class="link" onclick={() => {
                      const { [entry.subject]: _, ...rest } = settings!.subjectColors;
                      update({ subjectColors: rest });
                    }}>{i18n.t.mapping.resetColor}</button>
                  {/if}
                </div>
              </div>
            </div>
            {#if withMoodle}<div class="value">
              {#if entry.suggested && entry.courseId}
                <span class="pill accent">{i18n.t.mapping.suggested}</span>
                <button class="btn small" onclick={() => mapping.set(entry.subject, entry.courseId)}>
                  {i18n.t.mapping.confirm}
                </button>
              {/if}
              <select
                value={entry.courseId ?? ""}
                onchange={(e) => {
                  const v = (e.currentTarget as HTMLSelectElement).value;
                  mapping.set(entry.subject, v ? Number(v) : null);
                }}
                aria-label={entry.longName}
              >
                <option value="">{i18n.t.mapping.none}</option>
                {#each courses as c (c.id)}
                  <option value={c.id}>{c.fullName}</option>
                {/each}
              </select>
            </div>{/if}
          </div>
        {:else}
          <p class="muted">{i18n.t.mapping.empty}</p>
        {/each}
      </section>
    {/if}

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
  .subject {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .swatch {
    width: 28px;
    height: 28px;
    padding: 0;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: none;
    cursor: pointer;
  }
  .link {
    border: 0;
    background: none;
    padding: 0;
    color: var(--accent-strong);
    cursor: pointer;
    font-size: inherit;
  }
  .hint {
    margin: 0 0 8px;
  }
  .value select {
    max-width: 280px;
  }
  section > p {
    padding: 12px 0;
  }
</style>
