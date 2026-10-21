<script lang="ts">
  import { i18n } from "./lib/i18n/index.svelte";
  import Placeholder from "./views/Placeholder.svelte";
  import Settings from "./views/Settings.svelte";

  type View = "today" | "courses" | "timetable" | "settings";
  const views: View[] = ["today", "courses", "timetable", "settings"];

  let view = $state<View>("today");

  $effect(() => {
    document.documentElement.lang = i18n.locale;
  });
</script>

<div class="layout">
  <nav>
    {#each views as v (v)}
      <button class:active={view === v} onclick={() => (view = v)}>
        {i18n.t.nav[v]}
      </button>
    {/each}
  </nav>
  <main>
    <h1>{i18n.t.nav[view]}</h1>
    {#if view === "settings"}
      <Settings />
    {:else}
      <Placeholder text={i18n.t.placeholder[view]} />
    {/if}
  </main>
</div>

<style>
  .layout {
    display: grid;
    grid-template-columns: 200px 1fr;
    height: 100vh;
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 12px 8px;
    background: var(--surface-2);
    border-right: 1px solid var(--border);
  }
  nav button {
    text-align: left;
    padding: 8px 12px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--text);
    font: inherit;
    cursor: pointer;
  }
  nav button:hover {
    background: var(--hover);
  }
  nav button.active {
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }
  main {
    padding: 24px 32px;
    overflow: auto;
  }
  h1 {
    margin: 0 0 16px;
    font-size: 1.4rem;
  }
</style>
