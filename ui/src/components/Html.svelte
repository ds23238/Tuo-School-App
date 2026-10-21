<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";

  /** Bereits im Backend bereinigtes HTML. */
  let { html }: { html: string } = $props();

  // Links öffnen im System-Browser, nie im App-Fenster.
  function onclick(e: MouseEvent) {
    const link = (e.target as Element).closest("a[href]");
    if (!link) return;
    e.preventDefault();
    openUrl(link.getAttribute("href")!).catch(() => {});
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="html" {onclick}>{@html html}</div>

<style>
  .html :global(img) {
    max-width: 100%;
    height: auto;
  }
  .html :global(p:first-child) {
    margin-top: 0;
  }
  .html :global(p:last-child) {
    margin-bottom: 0;
  }
  .html :global(a) {
    color: var(--accent);
  }
</style>
