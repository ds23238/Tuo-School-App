<script lang="ts">
  import type { FileState } from "../lib/api";
  import { i18n } from "../lib/i18n/index.svelte";
  import Icon from "./Icon.svelte";

  let {
    state,
    onupdate,
    busy = false,
  }: { state: FileState | null; onupdate: () => void; busy?: boolean } = $props();
</script>

{#if state}
  {#if state.status === "current"}
    <span class="pill success">{i18n.t.file.local}</span>
  {:else if state.status === "remote"}
    <span class="pill">{i18n.t.file.remote}</span>
  {:else if state.status === "missing"}
    <span class="pill">{i18n.t.file.missing}</span>
  {:else if state.status === "removed"}
    <span class="pill danger">{i18n.t.file.removed}</span>
  {/if}
  {#if state.modifiedLocally}
    <span class="pill accent">{i18n.t.file.modifiedLocally}</span>
  {/if}
  {#if state.status === "updateAvailable"}
    <button class="pill update" onclick={onupdate} disabled={busy} title={i18n.t.file.updateAvailable}>
      <Icon name="refresh" size={13} stroke={2.5} class={busy ? "spin" : ""} />
      {i18n.t.file.fetchUpdate}
    </button>
  {/if}
{/if}

<style>
  .update {
    border: 1px solid var(--accent);
    background: var(--surface);
    color: var(--accent-ink);
    font-weight: 600;
    cursor: pointer;
    min-height: 26px;
  }
  .update:hover {
    background: var(--accent-soft);
  }
</style>
