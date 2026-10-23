<script lang="ts">
  import { api, isCommandError, type School } from "../lib/api";
  import { errorText } from "../lib/errors";
  import { i18n } from "../lib/i18n/index.svelte";
  import Icon from "./Icon.svelte";

  let {
    notice = null,
    onlogin,
  }: {
    notice?: string | null;
    /** `persistent`: Anmeldung übersteht einen Neustart. */
    onlogin: (persistent: boolean) => void;
  } = $props();

  let query = $state("");
  let schools = $state<School[] | null>(null);
  let school = $state<School | null>(null);
  let username = $state("");
  let password = $state("");
  let remember = $state(false);
  let busy = $state(false);
  let searching = $state(false);
  let error = $state<string | null>(null);

  let timer: ReturnType<typeof setTimeout> | undefined;
  function search() {
    clearTimeout(timer);
    const q = query.trim();
    if (q.length < 3) {
      schools = null;
      return;
    }
    timer = setTimeout(async () => {
      searching = true;
      error = null;
      try {
        schools = await api.untis.search(q);
      } catch (e) {
        schools = null;
        error = errorText(e);
      } finally {
        searching = false;
      }
    }, 400);
  }

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!school) return;
    busy = true;
    error = null;
    try {
      const result = await api.untis.login(school, username, password, remember);
      onlogin(result.persistent);
    } catch (err) {
      error = errorText(err);
      if (isCommandError(err) && err.code === "invalidLogin") password = "";
    } finally {
      busy = false;
    }
  }
</script>

<form class="card" onsubmit={submit}>
  <div>
    <h2>{i18n.t.untis.signIn}</h2>
    <p class="muted small">{i18n.t.untis.signInHint}</p>
  </div>
  {#if notice}<p class="notice warning">{notice}</p>{/if}
  {#if error}<p class="notice error">{error}</p>{/if}

  <div class="field">
    <span>{i18n.t.untis.school}</span>
    {#if school}
      <div class="chosen">
        <div>
          <strong>{school.displayName}</strong>
          <div class="muted small">{school.address}</div>
        </div>
        <button type="button" class="btn ghost small" onclick={() => (school = null)}>
          {i18n.t.untis.change}
        </button>
      </div>
    {:else}
      <label class="search">
        <Icon name={searching ? "refresh" : "search"} size={16} class={searching ? "spin" : ""} />
        <!-- svelte-ignore a11y_autofocus -->
        <input
          type="text"
          bind:value={query}
          oninput={search}
          placeholder={i18n.t.untis.schoolPlaceholder}
          aria-label={i18n.t.untis.school}
          autofocus
        />
      </label>
      {#if schools}
        <div class="results">
          {#if schools.length === 0}<p class="muted small">{i18n.t.untis.noSchools}</p>{/if}
          {#each schools as s (s.loginName + s.server)}
            <button type="button" onclick={() => (school = s)}>
              <strong>{s.displayName}</strong>
              <span class="muted small">{s.address}</span>
            </button>
          {/each}
        </div>
      {/if}
    {/if}
  </div>

  <label>
    <span>{i18n.t.untis.username}</span>
    <input type="text" bind:value={username} required autocomplete="username" />
  </label>

  <label>
    <span>{i18n.t.untis.password}</span>
    <input type="password" bind:value={password} required autocomplete="current-password" />
  </label>

  <label class="check">
    <input type="checkbox" bind:checked={remember} />
    <span>
      {i18n.t.untis.remember}
      <small class="muted">{i18n.t.untis.rememberHint}</small>
    </span>
  </label>

  <button class="btn primary" type="submit" disabled={busy || !school}>
    {busy ? i18n.t.untis.signingIn : i18n.t.untis.submit}
  </button>
</form>

<style>
  form {
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-width: 440px;
    width: 100%;
    padding: 28px;
    box-sizing: border-box;
  }
  h2 {
    font-size: 20px;
  }
  p {
    margin: 4px 0 0;
  }
  label,
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 14px;
    font-weight: 500;
  }
  label.check {
    flex-direction: row;
    align-items: baseline;
    gap: 10px;
    font-weight: 400;
  }
  small {
    display: block;
  }
  .search {
    flex-direction: row;
    align-items: center;
    gap: 8px;
    padding: 0 12px;
    border: 1px solid var(--border-strong);
    border-radius: 10px;
    color: var(--muted);
    background: var(--surface);
  }
  .search input {
    flex: 1;
    border: 0;
    padding: 10px 0;
    outline: none;
    background: none;
    color: var(--text);
  }
  .search:focus-within {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  .results {
    display: flex;
    flex-direction: column;
    max-height: 240px;
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: 10px;
  }
  .results button {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: 8px 12px;
    border: 0;
    border-bottom: 1px solid var(--border);
    background: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
    font-weight: 400;
  }
  .results button:hover {
    background: var(--hover);
  }
  .results p {
    margin: 8px 12px;
  }
  .chosen {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface-2);
    font-weight: 400;
  }
  button.btn.primary {
    align-self: flex-start;
    min-width: 120px;
  }
</style>
