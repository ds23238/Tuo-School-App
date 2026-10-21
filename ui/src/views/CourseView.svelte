<script lang="ts">
  import CourseSection from "../components/CourseSection.svelte";
  import Icon from "../components/Icon.svelte";
  import { api, type Account, type Course, type CourseView, type Module } from "../lib/api";
  import { setCourseContext } from "../lib/courseContext";
  import { errorText } from "../lib/errors";
  import { fill, formatDate } from "../lib/format";
  import { i18n } from "../lib/i18n/index.svelte";
  import { nav } from "../lib/nav.svelte";
  import { sync } from "../lib/sync.svelte";

  let { course, account }: { course: Course; account: Account } = $props();

  let view = $state<CourseView | null>(null);
  let refreshing = $state(true);
  let syncing = $state(false);
  let error = $state<string | null>(null);
  let message = $state<{ text: string; error: boolean } | null>(null);

  async function reload() {
    view = (await api.moodle.course(course.id).catch(() => null)) ?? view;
  }

  async function load() {
    // Erst den gespeicherten Stand zeigen, dann aktualisieren.
    await reload();
    try {
      view = await api.moodle.refreshCourse(course.id);
      error = null;
    } catch (e) {
      error = errorText(e);
    } finally {
      refreshing = false;
    }
  }

  function notify(text: string, isError = false) {
    message = { text, error: isError };
  }

  setCourseContext({
    get courseId() {
      return course.id;
    },
    get base() {
      return account.baseUrl;
    },
    reload,
    notify,
  });

  // Hinweise verschwinden nach ein paar Sekunden von selbst.
  $effect(() => {
    if (!message) return;
    const timer = setTimeout(() => (message = null), message.error ? 10000 : 5000);
    return () => clearTimeout(timer);
  });

  async function syncCourse() {
    syncing = true;
    try {
      const n = await api.moodle.syncCourse(course.id);
      notify(fill(i18n.t.sync.downloaded, { n }));
      await reload();
    } catch (e) {
      notify(errorText(e), true);
    } finally {
      syncing = false;
    }
  }

  function action(run: () => Promise<void>) {
    run().catch((e) => notify(errorText(e), true));
  }

  function tracked(modules: Module[]): Module[] {
    return modules.flatMap((m) => (m.subsection ? tracked(m.subsection.modules) : m.completed === null ? [] : [m]));
  }

  const progress = $derived.by(() => {
    if (!view) return course.progress;
    const all = view.sections.flatMap((s) => tracked(s.modules));
    return all.length ? (all.filter((m) => m.completed).length / all.length) * 100 : null;
  });

  // Ein globaler Sync aktualisiert auch die Dateizustände hier.
  $effect(() => {
    if (sync.generation > 0) reload();
  });

  function jump(id: number) {
    document.getElementById(`section-${id}`)?.scrollIntoView({ behavior: "smooth" });
  }

  load();
</script>

<div class="page">
  <button class="back" onclick={() => nav.back()}>
    <Icon name="chevronLeft" size={16} />{i18n.t.course.back}
  </button>

  <header class="tint-{course.id % 6}">
    <div class="info">
      {#if course.shortName !== course.fullName}<div class="sub">{course.shortName}</div>{/if}
      <h1>{course.fullName}</h1>
      {#if progress !== null}
        <div class="progress">
          <div class="bar"><div style:width="{Math.round(progress)}%"></div></div>
          <span>{fill(i18n.t.courses.progress, { n: Math.round(progress) })}</span>
        </div>
      {/if}
    </div>
    <div class="actions">
      <button class="btn" onclick={syncCourse} disabled={syncing}>
        <Icon name="refresh" size={16} class={syncing ? "spin" : ""} />{i18n.t.course.syncCourse}
      </button>
      <button class="btn" onclick={() => action(() => api.moodle.openCourseFolder(course.id))}>
        <Icon name="folder" size={16} />{i18n.t.course.openFolder}
      </button>
      <button
        class="btn"
        onclick={() => action(() => api.moodle.openInBrowser(`${account.baseUrl}course/view.php?id=${course.id}`))}
      >
        <Icon name="external" size={16} />{i18n.t.course.openInMoodle}
      </button>
    </div>
  </header>

  {#if error}
    <p class="notice {view ? 'warning' : 'error'}">
      {#if view}{fill(i18n.t.course.offline, { date: formatDate(view.fetchedAt) })} · {/if}{error}
    </p>
  {/if}
  {#if message}
    <div class="toast {message.error ? 'error' : ''}" role="status">
      <span>{message.text}</span>
      <button class="btn ghost small" onclick={() => (message = null)} aria-label="OK"><Icon name="x" size={14} /></button>
    </div>
  {/if}

  {#if view}
    <div class="layout">
      <aside class="card" aria-label={i18n.t.course.index}>
        <div class="index-title">{i18n.t.course.index}</div>
        {#each view.sections as section (section.id)}
          {@const all = tracked(section.modules)}
          {@const complete = all.length > 0 && all.every((m) => m.completed)}
          <button class="idx" onclick={() => jump(section.id)}>
            {#if !section.available}
              <Icon name="lock" size={15} />
            {:else if complete}
              <span class="ok"><Icon name="check" size={15} stroke={2.5} /></span>
            {:else}
              <span class="ring"></span>
            {/if}
            <span>{section.name || fill(i18n.t.course.section, { n: section.number })}</span>
          </button>
        {/each}
      </aside>
      <div class="sections">
        {#each view.sections as section (section.id)}
          <CourseSection {section} />
        {:else}
          <p class="muted">{i18n.t.course.empty}</p>
        {/each}
      </div>
    </div>
  {:else if refreshing}
    <p class="muted">{i18n.t.course.loading}</p>
  {/if}
</div>

<style>
  .page {
    max-width: 1160px;
    margin: 0 auto;
    padding: 20px 40px 48px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .back {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    border: 0;
    background: none;
    padding: 6px 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--accent-strong);
    cursor: pointer;
  }
  header {
    border-radius: 16px;
    padding: 26px 28px 22px;
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    justify-content: space-between;
    gap: 20px;
    background: var(--t-bg);
    color: var(--t-ink);
  }
  .tint-0 { --t-bg: #e3f1ec; --t-ink: #133d30; }
  .tint-1 { --t-bg: #e6eefb; --t-ink: #1c3766; }
  .tint-2 { --t-bg: #fbebdd; --t-ink: #6e3006; }
  .tint-3 { --t-bg: #f1e8f8; --t-ink: #47226a; }
  .tint-4 { --t-bg: #e4f2f6; --t-ink: #174756; }
  .tint-5 { --t-bg: #f6efe2; --t-ink: #523c15; }
  @media (prefers-color-scheme: dark) {
    header {
      background: var(--surface);
      border: 1px solid var(--border);
      color: var(--text);
    }
  }
  .sub {
    font-size: 13px;
    font-weight: 600;
    opacity: 0.85;
  }
  h1 {
    font-size: 30px;
    font-weight: 700;
    margin: 4px 0 12px;
  }
  .progress {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 280px;
    max-width: 100%;
    font-size: 12px;
    font-weight: 600;
  }
  .bar {
    flex: 1;
    height: 6px;
    border-radius: 999px;
    background: color-mix(in srgb, currentColor 15%, transparent);
    overflow: hidden;
  }
  .bar div {
    height: 100%;
    background: currentColor;
    border-radius: 999px;
  }
  .actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .actions .btn {
    color: var(--text);
  }
  .toast {
    position: fixed;
    right: 24px;
    bottom: 24px;
    max-width: 440px;
    z-index: 10;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 12px 12px 16px;
    border-radius: 12px;
    background: var(--text);
    color: var(--bg);
    font-size: 14px;
    box-shadow: 0 6px 24px rgba(0, 0, 0, 0.18);
  }
  .toast.error {
    background: var(--danger);
    color: #ffffff;
  }
  .toast .btn {
    color: inherit;
  }
  .notice {
    margin: 0;
  }
  .layout {
    display: flex;
    gap: 24px;
    align-items: flex-start;
  }
  aside {
    width: 250px;
    flex: none;
    padding: 12px 8px;
    position: sticky;
    top: 16px;
    max-height: calc(100vh - 32px);
    overflow: auto;
  }
  .index-title {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--muted);
    padding: 4px 10px 8px;
  }
  .idx {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 10px;
    border: 0;
    border-radius: 8px;
    background: none;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-2);
    text-align: left;
    cursor: pointer;
  }
  .idx:hover {
    background: var(--hover);
    color: var(--text);
  }
  .ok {
    display: flex;
    color: var(--success);
  }
  .ring {
    width: 13px;
    height: 13px;
    border-radius: 50%;
    border: 2px solid var(--border-strong);
    margin: 0 1px;
    flex: none;
  }
  .sections {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  @media (max-width: 900px) {
    aside {
      display: none;
    }
    .page {
      padding: 16px;
    }
  }
</style>
