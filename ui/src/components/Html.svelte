<script lang="ts">
  import { api } from "../lib/api";

  /** Bereits im Backend bereinigtes HTML. */
  let { html, moodleBase = null }: { html: string; moodleBase?: string | null } = $props();

  // Links öffnen im System-Browser, nie im App-Fenster; Moodle-Links
  // möglichst schon angemeldet.
  function onclick(e: MouseEvent) {
    const link = (e.target as Element).closest("a[href]");
    if (!link) return;
    e.preventDefault();
    const href = link.getAttribute("href")!;
    const open = moodleBase && href.startsWith(moodleBase) ? api.moodle.openInBrowser(href) : api.openUrl(href);
    open.catch(() => {});
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="html" {onclick}>{@html html}</div>

<style>
  .html {
    overflow-wrap: anywhere;
    line-height: 1.55;
  }
  .html :global(img),
  .html :global(video),
  .html :global(audio) {
    max-width: 100%;
    height: auto;
  }
  .html :global(*) {
    max-width: 100%;
  }
  /* Moodle-Symbole und SVGs ohne eigene Größe würden sonst die volle
     Breite einnehmen (Markierung siehe crates/moodle/src/html.rs). */
  .html :global(img.icon),
  .html :global(img.iconlarge),
  .html :global(img[src$="#icon"]) {
    width: 1.5em;
    height: 1.5em;
    vertical-align: middle;
    margin-right: 0.3em;
  }
  .html :global(img.iconsmall) {
    width: 1em;
    height: 1em;
  }
  .html :global(img[src$="#svg"]:not([width])) {
    max-width: min(100%, 320px);
  }
  .html :global(p) {
    margin: 0 0 0.7em;
  }
  .html :global(> :last-child) {
    margin-bottom: 0;
  }
  .html :global(a) {
    color: var(--accent-strong);
  }
  /* Eingebettete Seiten (iframes) erscheinen als Link, siehe html.rs. */
  .html :global(a.embed) {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface-2);
    text-decoration: none;
    font-weight: 500;
  }
  .html :global(a.embed)::before {
    content: "▶";
    font-size: 0.8em;
  }
  .html :global(a.embed:hover) {
    border-color: var(--accent);
  }
  .html :global(table) {
    border-collapse: collapse;
    max-width: 100%;
    display: block;
    overflow-x: auto;
  }
  .html :global(td),
  .html :global(th) {
    border: 1px solid var(--border);
    padding: 4px 8px;
  }
  .html :global(blockquote) {
    margin: 0.5em 0;
    padding-left: 12px;
    border-left: 3px solid var(--border-strong);
    color: var(--text-2);
  }
  .html :global(pre),
  .html :global(code) {
    background: var(--surface-3);
    border-radius: 6px;
  }
  .html :global(pre) {
    padding: 8px 12px;
    overflow-x: auto;
  }
</style>
