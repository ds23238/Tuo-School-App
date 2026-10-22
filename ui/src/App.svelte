<script lang="ts">
  import Icon from "./components/Icon.svelte";
  import { fill, formatDate, formatTime } from "./lib/format";
  import { i18n } from "./lib/i18n/index.svelte";
  import { nav, type View } from "./lib/nav.svelte";
  import { sync } from "./lib/sync.svelte";
  import "./lib/appearance.svelte";
  import { inbox } from "./lib/inbox.svelte";
  import Courses from "./views/Courses.svelte";
  import Messages from "./views/Messages.svelte";
  import Settings from "./views/Settings.svelte";
  import Timetable from "./views/Timetable.svelte";
  import Today from "./views/Today.svelte";

  const items: { view: View; icon: string }[] = [
    { view: "today", icon: "today" },
    { view: "courses", icon: "courses" },
    { view: "messages", icon: "message" },
    { view: "timetable", icon: "timetable" },
    { view: "settings", icon: "settings" },
  ];

  /** Eingeklappte Seitenleiste zeigt nur Symbole; Wahl bleibt erhalten. */
  let collapsed = $state(readCollapsed());

  function readCollapsed(): boolean {
    try {
      return localStorage.getItem("sidebar.collapsed") === "1";
    } catch {
      return false;
    }
  }

  function toggleSidebar() {
    collapsed = !collapsed;
    try {
      localStorage.setItem("sidebar.collapsed", collapsed ? "1" : "0");
    } catch {
      // Dann gilt es nur bis zum Neustart.
    }
  }

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
  <nav aria-label="Navigation" class:collapsed>
    <div class="brand">
      <svg width="32" height="32" viewBox="0 0 512 512" aria-hidden="true">
        <rect x="16" y="16" width="480" height="480" rx="104" fill="#f07c1e" />
        <path d="M180 182 256 110l76 72z" fill="#fff" />
        <rect x="196" y="176" width="120" height="226" fill="#fff" />
        <rect x="100" y="246" width="312" height="156" fill="#fff" />
        <rect x="80" y="396" width="352" height="26" rx="6" fill="#fff" />
        <path d="M230 402v-60a26 26 0 0 1 52 0v60z" fill="#f07c1e" />
      </svg>
      <button
        class="collapse"
        onclick={toggleSidebar}
        aria-label={collapsed ? i18n.t.nav.expand : i18n.t.nav.collapse}
        title={collapsed ? i18n.t.nav.expand : i18n.t.nav.collapse}
        aria-expanded={!collapsed}
      >
        <Icon name={collapsed ? "chevronRight" : "chevronLeft"} size={18} />
      </button>
    </div>
    {#each items as item (item.view)}
      <button
        class="nav-item"
        title={collapsed ? i18n.t.nav[item.view] : undefined}
        class:active={nav.view === item.view}
        aria-current={nav.view === item.view ? "page" : undefined}
        onclick={() => nav.go(item.view)}
      >
        <span class="icon">
          <Icon name={item.icon} />
          {#if item.view === "messages" && inbox.total > 0}
            <span class="badge" aria-label={String(inbox.total)}>{inbox.total > 99 ? "99+" : inbox.total}</span>
          {/if}
        </span>
        <span class="label">{i18n.t.nav[item.view]}</span>
      </button>
    {/each}
    <div class="spacer"></div>
    <div class="sync">
      <div class="status" title={sync.error ?? undefined}>
        <span class="dot" class:error={!!sync.error} class:running={sync.running}></span>
        <span>{status}</span>
      </div>
      <button class="btn" onclick={() => sync.run()} disabled={sync.running} title={collapsed ? i18n.t.sync.now : undefined}>
        <Icon name="refresh" size={16} class={sync.running ? "spin" : ""} />
        <span>{i18n.t.sync.now}</span>
      </button>
    </div>
  </nav>

  <main>
    {#if nav.view === "courses"}
      <Courses />
    {:else if nav.view === "messages"}
      <Messages />
    {:else if nav.view === "settings"}
      <Settings />
    {:else if nav.view === "timetable"}
      <Timetable />
    {:else}
      <Today />
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
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 2px 4px 18px 8px;
  }
  .collapse {
    display: flex;
    border: 0;
    background: none;
    color: var(--muted);
    padding: 6px;
    border-radius: 8px;
    cursor: pointer;
  }
  .collapse:hover {
    background: var(--hover);
    color: var(--text);
  }
  nav {
    transition: width 0.15s;
  }
  nav.collapsed {
    width: 68px;
    padding: 18px 8px 16px;
  }
  nav.collapsed .brand {
    flex-direction: column;
    padding: 2px 0 18px;
  }
  nav.collapsed .nav-item .label,
  nav.collapsed .sync span:not(.dot) {
    display: none;
  }
  nav.collapsed .nav-item {
    justify-content: center;
  }
  nav.collapsed .status {
    justify-content: center;
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
  .icon {
    position: relative;
    display: flex;
  }
  .badge {
    position: absolute;
    top: -6px;
    right: -9px;
    min-width: 17px;
    height: 17px;
    padding: 0 4px;
    box-sizing: border-box;
    border-radius: 999px;
    background: var(--accent);
    color: var(--on-accent);
    font-size: 10.5px;
    font-weight: 700;
    line-height: 17px;
    text-align: center;
    border: 2px solid var(--surface);
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
    .nav-item .label,
    .sync span:not(.dot) {
      display: none;
    }
    .nav-item {
      justify-content: center;
    }
  }
</style>
