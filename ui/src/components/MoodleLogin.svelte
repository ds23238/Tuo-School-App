<script lang="ts">
  import { api, isCommandError, type Account } from "../lib/api";
  import { errorText } from "../lib/errors";
  import { i18n } from "../lib/i18n/index.svelte";

  let {
    url = "",
    username = "",
    notice = null,
    onlogin,
  }: {
    url?: string;
    username?: string;
    notice?: string | null;
    onlogin: (account: Account) => void;
  } = $props();

  let password = $state("");
  let remember = $state(false);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let ssoOnly = $state(false);

  async function checkSite() {
    if (!url.trim()) return;
    ssoOnly = await api.moodle.checkSite(url).then(
      (site) => site.ssoOnly,
      () => false,
    );
  }

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    busy = true;
    error = null;
    try {
      onlogin(await api.moodle.login(url, username, password, remember));
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
    <h2>{i18n.t.moodle.signIn}</h2>
    <p class="muted small">{i18n.t.moodle.signInHint}</p>
  </div>
  {#if notice}<p class="notice warning">{notice}</p>{/if}
  {#if error}<p class="notice error">{error}</p>{/if}

  <label>
    <span>{i18n.t.moodle.url}</span>
    <!-- svelte-ignore a11y_autofocus -->
    <input
      type="text"
      inputmode="url"
      bind:value={url}
      onchange={checkSite}
      placeholder={i18n.t.moodle.urlPlaceholder}
      required
      autocomplete="url"
      autofocus={!url}
    />
  </label>
  {#if ssoOnly}<p class="notice warning">{i18n.t.moodle.ssoHint}</p>{/if}

  <label>
    <span>{i18n.t.moodle.username}</span>
    <input type="text" bind:value={username} required autocomplete="username" />
  </label>

  <label>
    <span>{i18n.t.moodle.password}</span>
    <input type="password" bind:value={password} required autocomplete="current-password" />
  </label>

  <label class="check">
    <input type="checkbox" bind:checked={remember} />
    <span>
      {i18n.t.moodle.remember}
      <small class="muted">{i18n.t.moodle.rememberHint}</small>
    </span>
  </label>

  <button class="btn primary" type="submit" disabled={busy}>
    {busy ? i18n.t.moodle.signingIn : i18n.t.moodle.submit}
  </button>
</form>

<style>
  form {
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-width: 440px;
    padding: 28px;
  }
  h2 {
    font-size: 20px;
  }
  p {
    margin: 4px 0 0;
  }
  label {
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
  button {
    align-self: flex-start;
    min-width: 120px;
  }
</style>
