<script lang="ts">
  import { api } from "../lib/api";
  import { i18n } from "../lib/i18n/index.svelte";

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
<div
  class="html"
  {onclick}
  style:--embed-page={JSON.stringify(i18n.t.course.embedPage)}
  style:--embed-video={JSON.stringify(i18n.t.course.embedVideo)}
  style:--embed-interactive={JSON.stringify(i18n.t.course.embedInteractive)}
>
  {@html html}
</div>

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
  /* Bildausrichtung aus dem Moodle-Editor. */
  .html :global(img.atto_image_button_left),
  .html :global(img.float-left),
  .html :global(img.float-start) {
    float: left;
    margin: 0 1em 0.5em 0;
  }
  .html :global(img.atto_image_button_right),
  .html :global(img.float-right),
  .html :global(img.float-end) {
    float: right;
    margin: 0 0 0.5em 1em;
  }
  .html :global(img.atto_image_button_middle) {
    vertical-align: middle;
  }
  .html :global(img.atto_image_button_text-top) {
    vertical-align: text-top;
  }
  .html :global(img.atto_image_button_text-bottom) {
    vertical-align: text-bottom;
  }
  .html::after {
    content: "";
    display: block;
    clear: both;
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
  /* Eingebettete Seiten (iframes) erscheinen wie ein Link-Element im Kurs,
     siehe html.rs. Symbole als Maske, damit sie die Farbe übernehmen. */
  .html :global(a.embed) {
    --c: var(--p-content);
    --bg: var(--p-content-soft);
    --icon: url("data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><path d='M10 13a5 5 0 0 0 7.5.5l3-3a5 5 0 0 0-7-7l-1.7 1.7'/><path d='M14 11a5 5 0 0 0-7.5-.5l-3 3a5 5 0 0 0 7 7l1.7-1.7'/></svg>");
    display: flex;
    align-items: center;
    gap: 14px;
    max-width: 520px;
    padding: 10px 14px 10px 10px;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--surface);
    color: var(--text);
    text-decoration: none;
  }
  .html :global(a.embed.video) {
    --icon: url("data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><path d='M2 6h14v12H2z'/><path d='m16 10 6-4v12l-6-4'/></svg>");
    --label: var(--embed-video);
  }
  .html :global(a.embed.interactive) {
    --c: var(--p-interactive);
    --bg: var(--p-interactive-soft);
    --icon: url("data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><path d='M12 2a10 10 0 1 0 0 20a10 10 0 1 0 0-20'/><path d='m10 8 6 4-6 4z'/></svg>");
    --label: var(--embed-interactive);
  }
  .html :global(.embed-tile) {
    flex: none;
    width: 40px;
    height: 40px;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg);
  }
  .html :global(.embed-tile)::before {
    content: "";
    width: 22px;
    height: 22px;
    background: var(--c);
    -webkit-mask: var(--icon) center / contain no-repeat;
    mask: var(--icon) center / contain no-repeat;
  }
  .html :global(a.embed)::after {
    content: "";
    flex: none;
    width: 14px;
    height: 14px;
    margin-left: auto;
    background: var(--muted);
    -webkit-mask: url("data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><path d='M15 3h6v6M10 14 21 3M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6'/></svg>") center / contain no-repeat;
    mask: url("data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><path d='M15 3h6v6M10 14 21 3M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6'/></svg>") center / contain no-repeat;
  }
  .html :global(a.embed:hover) {
    border-color: var(--c);
  }
  .html :global(.embed-text) {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .html :global(.embed-kind) {
    font-size: 12px;
    font-weight: 500;
    color: var(--muted);
  }
  .html :global(.embed-kind)::before {
    content: var(--label, var(--embed-page)) " · ";
  }
  .html :global(.embed-kind:empty)::before {
    content: var(--label, var(--embed-page));
  }
  .html :global(.embed-name) {
    font-weight: 600;
    font-size: 15px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
