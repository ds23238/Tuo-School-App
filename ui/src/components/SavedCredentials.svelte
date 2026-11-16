<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { Service } from "../lib/api";
  import { i18n } from "../lib/i18n/index.svelte";
  import { errorText } from "../lib/errors";

  let { site }: { site: Service } = $props();
  let available = $state(false);
  let pending = $state(false);
  let credentials = $state<{ username: string; password: string } | null>(null);
  let error = $state<string | null>(null);
  let alive = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  function hide() { clearTimeout(timer); credentials = null; }
  async function reveal() {
    if (credentials) { hide(); return; }
    pending = true;
    error = null;
    try {
      const result = await invoke<{ username: string; password: string }>("hybrid_saved_credentials", { site });
      if (alive) { credentials = result; timer = setTimeout(hide, 30000); }
    } catch (e) { if (alive) error = errorText(e); }
    finally { if (alive) pending = false; }
  }
  onMount(() => {
    alive = true;
    invoke<boolean>("hybrid_password_available", { site }).then((value) => { if (alive) available = value; }).catch(() => {});
    return () => { alive = false; hide(); };
  });
</script>
<svelte:window onblur={hide} />
{#if available}
  <div class="saved">
    <button class="btn small" onclick={reveal} disabled={pending}>{credentials ? i18n.t.hybrid.hideCredentials : i18n.t.hybrid.credentials}</button>
    {#if credentials}
      <section aria-label={i18n.t.hybrid.credentials} class="credentials">
        <label>{i18n.t.hybrid.savedUsername}<input readonly value={credentials.username} autocomplete="off" spellcheck={false} /></label>
        <label>{i18n.t.hybrid.savedPassword}<input readonly type="text" value={credentials.password} autocomplete="off" spellcheck={false} /></label>
        <span class="muted small">{i18n.t.hybrid.credentialsHint}</span>
      </section>
    {/if}
    {#if error}<span class="notice error" role="alert">{error}</span>{/if}
  </div>
{/if}
<style>
  .saved, .credentials, label { display: flex; flex-direction: column; gap: 6px; }
  .credentials { padding: 10px; border: 1px solid var(--border); border-radius: 8px; background: var(--surface); }
  label { font-size: 12px; }
  input { min-width: 210px; max-width: 100%; }
</style>
