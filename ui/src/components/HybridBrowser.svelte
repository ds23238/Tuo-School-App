<script lang="ts">
  import DisplayModeSwitch from "./DisplayModeSwitch.svelte";
  import SavedCredentials from "./SavedCredentials.svelte";
  import { onMount, type Snippet } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import type { Service } from "../lib/api";
  import { browserCommand, type BrowserStatus } from "../lib/hybrid";
  import { errorText } from "../lib/errors";
  import { i18n } from "../lib/i18n/index.svelte";

  let { site, target, title, courseId, onback, onlocal, actions, showModeSwitch = true }: {
    site: Service; target?: string; title: string; courseId?: number;
    onback?: () => void; onlocal: () => void | Promise<void>; actions?: Snippet; showModeSwitch?: boolean;
  } = $props();

  let host: HTMLDivElement;
  let loading = $state(true);
  let error = $state<string | null>(null);
  let downloaded = $state(false);
  let slow = $state(false);

  function bounds() {
    const r = host.getBoundingClientRect();
    return { x: r.x, y: r.y, width: Math.max(1, r.width), height: Math.max(1, r.height) };
  }
  async function action(action: string) {
    error = null;
    if (action === "reload") loading = true;
    try { await browserCommand("hybrid_action", { site, action }); }
    catch (e) { loading = false; error = errorText(e); }
  }

  onMount(() => {
    let disposed = false;
    let opened = false;
    let unlisten: (() => void) | undefined;
    let frame = 0;
    let timer: ReturnType<typeof setTimeout>;
    const resize = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        if (opened && !disposed) browserCommand("hybrid_bounds", { site, bounds: bounds() }).catch((e) => { error = errorText(e); });
      });
    };
    const observer = new ResizeObserver(resize);
    observer.observe(host);
    window.addEventListener("resize", resize);
    window.addEventListener("scroll", resize, true);
    timer = setTimeout(() => { if (loading) slow = true; }, 25000);
    (async () => {
      unlisten = await listen<BrowserStatus>("hybrid-status", ({ payload }) => {
        if (disposed || payload.site !== site) return;
        loading = payload.loading;
        if (payload.error) error = payload.error;
        if (payload.downloaded) downloaded = true;
      });
      if (disposed) { unlisten(); return; }
      await browserCommand("hybrid_open", { site, target: target ?? null, courseId: courseId ?? null, bounds: bounds() });
      opened = true;
      if (!disposed) resize();
    })().catch((e) => { if (!disposed) { loading = false; error = errorText(e); } });
    return () => {
      disposed = true;
      observer.disconnect();
      window.removeEventListener("resize", resize);
      window.removeEventListener("scroll", resize, true);
      cancelAnimationFrame(frame);
      clearTimeout(timer);
      unlisten?.();
      invoke("hybrid_hide", { site }).catch(() => {});
    };
  });
</script>

<section class="browser" aria-label={title}>
  <header>
    <div class="heading">
      {#if onback}<button class="btn small" onclick={onback}>{i18n.t.hybrid.back}</button>{/if}
      <strong>{title}</strong>
      <span class="muted small">{site === "moodle" ? "Moodle" : "WebUntis"}</span>
    </div>
    <div class="controls">
      <button class="btn small" onclick={() => action("back")} title={i18n.t.hybrid.back} aria-label={i18n.t.hybrid.back}>←</button>
      <button class="btn small" onclick={() => action("forward")} title={i18n.t.hybrid.forward} aria-label={i18n.t.hybrid.forward}>→</button>
      <button class="btn small" onclick={() => action("reload")}>{i18n.t.hybrid.reload}</button>
      {@render actions?.()}
      <button class="btn small" onclick={() => action("external")}>{i18n.t.hybrid.external}</button>
      {#if showModeSwitch}<DisplayModeSwitch mode="web" onchange={(mode) => { if (mode === "native") return onlocal(); }} />{/if}
      <SavedCredentials {site} />
    </div>
  </header>
  <div class="status" role="status">
    <span>{loading ? i18n.t.hybrid.loading : i18n.t.hybrid.loginHint}</span>
    {#if site === "moodle"}<span>{i18n.t.hybrid.filesHint}</span>{/if}
    <button class="download" onclick={() => action("downloads")}>{downloaded ? i18n.t.hybrid.downloaded : i18n.t.hybrid.downloads}</button>
  </div>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if slow && loading}<p class="notice warning">{i18n.t.hybrid.slow}</p>{/if}
  <div class="viewport" bind:this={host} aria-label={i18n.t.hybrid.website}>
    <p class="muted">{error ? i18n.t.hybrid.fallback : i18n.t.hybrid.loading}</p>
  </div>
</section>

<style>
  .browser { height: 100%; min-height: 0; display: flex; flex-direction: column; }
  header { display: flex; flex-wrap: wrap; align-items: center; gap: 10px; padding: 14px 18px 10px; }
  .heading, .controls { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; }
  .heading { flex: 1; min-width: 160px; }
  strong { overflow-wrap: anywhere; }
  .status { display: flex; flex-wrap: wrap; gap: 6px 16px; align-items: center; padding: 0 18px 10px; font-size: 12px; color: var(--muted); }
  .download { border: 0; padding: 0; background: none; color: var(--accent-strong); cursor: pointer; font: inherit; }
  .notice { margin: 0 18px 10px; }
  .viewport { flex: 1; min-height: 100px; background: var(--surface); border-top: 1px solid var(--border); overflow: hidden; }
  .viewport p { padding: 24px; }
</style>
