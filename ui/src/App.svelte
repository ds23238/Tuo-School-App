<script lang="ts">
  import Icon from "./components/Icon.svelte";
  import { fill, formatDate, formatTime } from "./lib/format";
  import { i18n } from "./lib/i18n/index.svelte";
  import { nav, type View } from "./lib/nav.svelte";
  import { sync } from "./lib/sync.svelte";
  import Courses from "./views/Courses.svelte";
  import Placeholder from "./views/Placeholder.svelte";
  import Settings from "./views/Settings.svelte";

  const items: { view: View; icon: string }[] = [
    { view: "today", icon: "today" },
    { view: "courses", icon: "courses" },
    { view: "timetable", icon: "timetable" },
    { view: "settings", icon: "settings" },
  ];

  const status = $derived.by(() => {
    if (sync.running) return i18n.t.sync.running;
    if (sync.error) return i18n.t.sync.failed;
    if (sync.lastAt) {
      const today = sync.lastAt.toDateString() === new Date().toDateString();
      const time = today ? formatTime(sync.lastAt) : formatDate(sync.lastAt.getTime() / 1000);
      return fill(i18n.t.sync.done, { time });
    }
    return i18n.t.sync.never;
  });
</script>

<div class="shell">
  <nav aria-label="Navigation">
    <div class="brand">
      <svg width="32" height="32" viewBox="0 0 512 512" aria-hidden="true">
        <rect x="16" y="16" width="480" height="480" rx="104" fill="#f07c1e" />
        <path d="M180 182 256 110l76 72z" fill="#fff" />
        <rect x="196" y="176" width="120" height="226" fill="#fff" />
        <rect x="100" y="246" width="312" height="156" fill="#fff" />
        <rect x="80" y="396" width="352" height="26" rx="6" fill="#fff" />
        <path d="M230 402v-60a26 26 0 0 1 52 0v60z" fill="#f07c1e" />
      </svg>
    </div>
    {#each items as item (item.view)}
      <button
        class="nav-item"
        class:active={nav.view === item.view}
        aria-current={nav.view === item.view ? "page" : undefined}
        onclick={() => nav.go(item.view)}
      >
        <Icon name={item.icon} />
        <span>{i18n.t.nav[item.view]}</span>
      </button>
    {/each}
    <div class="spacer"></div>
    <div class="sync">
      <div class="status" title={sync.error ?? undefined}>
        <span class="dot" class:error={!!sync.error} class:running={sync.running}></span>
        <span>{status}</span>
      </div>
      <button class="btn" onclick={() => sync.run()} disabled={sync.running}>
        <Icon name="refresh" size={16} class={sync.running ? "spin" : ""} />
        <span>{i18n.t.sync.now}</span>
      </button>
    </div>
  </nav>

  <main>
    {#if nav.view === "courses"}
      <Courses />
    {:else if nav.view === "settings"}
      <Settings />
    {:else}
      <Placeholder icon={nav.view} title={i18n.t.nav[nav.view]} text={i18n.t.placeholder[nav.view]} />
    {/if}
  </main>
</div>

<style>
  .shell {
    display: flex;
    height: 100vh;
  }
  nav {
    width: 232px;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 18px 12px 16px;
    background: var(--surface);
    border-right: 1px solid var(--border);
  }
  .brand {
    padding: 2px 8px 18px;
  }
  .nav-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-radius: 10px;
    border: 0;
    background: none;
    color: var(--text-2);
    font-weight: 500;
    font-size: 14px;
    cursor: pointer;
    text-align: left;
  }
  .nav-item:hover {
    background: var(--hover);
    color: var(--text);
  }
  .nav-item.active {
    background: var(--accent-soft);
    color: var(--accent-ink);
  }
  .spacer {
    flex: 1;
  }
  .sync {
    border-top: 1px solid var(--border);
    padding: 14px 4px 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--muted);
    padding: 0 4px;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--success);
    flex: none;
  }
  .dot.running {
    background: var(--accent);
  }
  .dot.error {
    background: var(--danger);
  }
  main {
    flex: 1;
    min-width: 0;
    overflow: auto;
  }
  @media (max-width: 760px) {
    nav {
      width: 68px;
      padding: 18px 8px;
    }
    .nav-item span,
    .sync span {
      display: none;
    }
    .nav-item {
      justify-content: center;
    }
  }
</style>
