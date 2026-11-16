<script lang="ts">
  import type { DisplayMode } from "../lib/api";
  import { i18n } from "../lib/i18n/index.svelte";
  let { mode, onchange }: { mode: DisplayMode; onchange: (mode: DisplayMode) => void | Promise<void> } = $props();
  let busy = $state(false);
  async function choose(next: DisplayMode) {
    if (busy || mode === next) return;
    busy = true;
    try { await onchange(next); } finally { busy = false; }
  }
</script>
<div class="modes" role="radiogroup" aria-label={i18n.t.hybrid.display}>
  <button class="btn small" role="radio" aria-checked={mode === "native"} disabled={busy} onclick={() => choose("native")}>{i18n.t.hybrid.native}</button>
  <button class="btn small" role="radio" aria-checked={mode === "web"} disabled={busy} onclick={() => choose("web")}>{i18n.t.hybrid.web}</button>
</div>
<style>
  .modes { display: flex; gap: 4px; flex-wrap: wrap; }
  [aria-checked="true"] { border-color: var(--accent-strong); color: var(--accent-strong); }
</style>
