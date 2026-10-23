<script lang="ts">
  import CourseCard from "../components/CourseCard.svelte";
  import Icon from "../components/Icon.svelte";
  import MoodleLogin from "../components/MoodleLogin.svelte";
  import { api, isCommandError, type Account, type Course, type CourseFiles } from "../lib/api";
  import { errorText } from "../lib/errors";
  import { i18n } from "../lib/i18n/index.svelte";
  import { nav } from "../lib/nav.svelte";
  import { sync } from "../lib/sync.svelte";
  import AssignmentView from "./AssignmentView.svelte";
  import CourseView from "./CourseView.svelte";
  import DataView from "./DataView.svelte";
  import ForumView from "./ForumView.svelte";
  import PageView from "./PageView.svelte";

  type Filter = "current" | "past" | "all";
  type Sort = "name" | "nameDesc" | "recent" | "progress" | "start";

  /** Auswahl bleibt pro Gerät erhalten; ohne Speicher gilt der Standard. */
  function stored<T extends string>(key: string, fallback: T): T {
    try {
      return (localStorage.getItem(key) as T | null) ?? fallback;
    } catch {
      return fallback;
    }
  }
  function store(key: string, value: string) {
    try {
      localStorage.setItem(key, value);
    } catch {
      // Dann gilt die Auswahl nur bis zum Neustart.
    }
  }

  let account = $state<Account | null>(null);
  let courses = $state<Course[]>([]);
  let stats = $state<Record<string, CourseFiles>>({});
  let loaded = $state(false);
  let refreshing = $state(false);
  let expired = $state(false);
  let error = $state<string | null>(null);
  let filter = $state<Filter>(stored("courses.filter", "current"));
  let sort = $state<Sort>(stored("courses.sort", "name"));
  let query = $state("");

  $effect(() => store("courses.filter", filter));
  $effect(() => store("courses.sort", sort));

  /** Für die Suche: klein, ohne Akzente („Französisch“ findet „franzosisch“). */
  const fold = (s: string) =>
    s.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();

  const sorters: Record<Sort, (a: Course, b: Course) => number> = {
    name: (a, b) => a.fullName.localeCompare(b.fullName, i18n.locale),
    nameDesc: (a, b) => b.fullName.localeCompare(a.fullName, i18n.locale),
    recent: (a, b) => (b.lastAccess ?? 0) - (a.lastAccess ?? 0),
    progress: (a, b) => (b.progress ?? -1) - (a.progress ?? -1),
    start: (a, b) => (b.startDate ?? 0) - (a.startDate ?? 0),
  };

  const now = Date.now() / 1000;
  const isPast = (c: Course) => !!c.endDate && c.endDate < now;
  const inFilter = $derived(
    courses.filter((c) =>
      filter === "all" ? true : filter === "past" ? isPast(c) : !isPast(c) && (c.enrolled || c.sync),
    ),
  );
  const shown = $derived.by(() => {
    const words = fold(query).split(/\s+/).filter(Boolean);
    return inFilter
      .filter((c) => {
        const text = fold(`${c.fullName} ${c.shortName}`);
        return words.every((w) => text.includes(w));
      })
      .sort(sorters[sort]);
  });

  function open(course: Course) {
    course.lastAccess = Math.floor(Date.now() / 1000);
    api.moodle.courseViewed(course.id).catch(() => {});
    nav.openCourse(course);
  }

  async function load() {
    try {
      const accounts = await api.listAccounts();
      account = accounts.find((a) => a.service === "moodle") ?? null;
      if (account) {
        [courses, stats] = await Promise.all([api.moodle.courses(), api.moodle.fileStats()]);
        if (courses.length === 0) await refresh();
      }
    } catch (e) {
      error = errorText(e);
    } finally {
      loaded = true;
    }
  }

  async function refresh() {
    refreshing = true;
    error = null;
    try {
      courses = await api.moodle.refreshCourses();
    } catch (e) {
      if (isCommandError(e) && e.code === "invalidToken") expired = true;
      else error = errorText(e);
    } finally {
      refreshing = false;
    }
  }

  async function onlogin(a: Account) {
    account = a;
    expired = false;
    await refresh();
  }

  async function toggle(course: Course) {
    const value = !course.sync;
    try {
      await api.moodle.setCourseSync(course.id, value);
      course.sync = value;
    } catch (e) {
      error = errorText(e);
    }
  }

  // Nach jedem Sync die Dateizahlen neu holen.
  $effect(() => {
    if (sync.generation > 0 && account) {
      api.moodle.fileStats().then((s) => (stats = s), () => {});
    }
  });

  load();
</script>

{#if !loaded}
  <!-- lädt -->
{:else if !account || expired}
  <div class="page login">
    <MoodleLogin
      url={account?.baseUrl}
      username={account?.username}
      notice={expired ? i18n.t.moodle.expired : null}
      {onlogin}
    />
  </div>
{:else if nav.course && nav.module?.kind === "data"}
  {#key nav.module.id}<DataView course={nav.course} module={nav.module} />{/key}
{:else if nav.course && nav.module?.kind === "forum"}
  {#key nav.module.id}<ForumView course={nav.course} module={nav.module} />{/key}
{:else if nav.course && nav.module?.kind === "assign"}
  {#key nav.module.id}<AssignmentView course={nav.course} module={nav.module} />{/key}
{:else if nav.course && nav.module}
  {#key nav.module.id}<PageView course={nav.course} module={nav.module} />{/key}
{:else if nav.course}
  {#key nav.course.id}<CourseView course={nav.course} {account} />{/key}
{:else}
  <div class="page">
    <header>
      <div>
        <h1>{i18n.t.courses.title}</h1>
        <p class="muted small">
          {account.displayName ?? account.username} · {new URL(account.baseUrl).host}
        </p>
      </div>
      <div class="filters">
        {#each [["current", i18n.t.courses.current], ["past", i18n.t.courses.past], ["all", i18n.t.courses.all]] as [id, label] (id)}
          <button class="chip" class:on={filter === id} onclick={() => (filter = id as Filter)}>
            {label}
          </button>
        {/each}
        <button
          class="btn ghost"
          onclick={refresh}
          disabled={refreshing}
          title={i18n.t.courses.refresh}
          aria-label={i18n.t.courses.refresh}
        >
          <Icon name="refresh" size={16} class={refreshing ? "spin" : ""} />
        </button>
      </div>
    </header>
    <div class="tools">
      <label class="search">
        <Icon name="search" size={18} />
        <input
          type="search"
          bind:value={query}
          placeholder={i18n.t.courses.search}
          aria-label={i18n.t.courses.search}
        />
      </label>
      <label class="sort">
        <Icon name="sort" size={16} />
        <select bind:value={sort} aria-label={i18n.t.courses.sort} title={i18n.t.courses.sort}>
          <option value="name">{i18n.t.courses.sortName}</option>
          <option value="nameDesc">{i18n.t.courses.sortNameDesc}</option>
          <option value="recent">{i18n.t.courses.sortRecent}</option>
          <option value="start">{i18n.t.courses.sortStart}</option>
          <option value="progress">{i18n.t.courses.sortProgress}</option>
        </select>
        <Icon name="chevronDown" size={16} />
      </label>
    </div>
    {#if error}<p class="notice error">{error}</p>{/if}

    {#if shown.length === 0 && !refreshing}
      <p class="muted">{query ? i18n.t.courses.noMatch : i18n.t.courses.empty}</p>
    {:else}
      <div class="grid">
        {#each shown as course (course.id)}
          <CourseCard
            {course}
            files={stats[course.id]}
            onopen={() => open(course)}
            ontoggle={() => toggle(course)}
          />
        {/each}
      </div>
    {/if}
  </div>
{/if}

<style>
  .page {
    max-width: 1160px;
    margin: 0 auto;
    padding: 32px 40px 48px;
    display: flex;
    flex-direction: column;
    gap: 24px;
  }
  header {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
  }
  h1 {
    font-size: 28px;
    font-weight: 700;
  }
  header p {
    margin: 6px 0 0;
  }
  .filters {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }
  .chip {
    border: 1px solid var(--border-strong);
    background: var(--surface);
    border-radius: 999px;
    padding: 6px 14px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-2);
    cursor: pointer;
    min-height: 34px;
  }
  .chip.on {
    background: var(--text);
    border-color: var(--text);
    color: var(--bg);
  }
  .tools {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
  }
  .search {
    flex: 1;
    min-width: 220px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 14px;
    border: 1px solid var(--border-strong);
    border-radius: 12px;
    background: var(--surface);
    color: var(--muted);
  }
  .search:focus-within {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  .search input {
    flex: 1;
    border: 0;
    padding: 10px 0;
    background: none;
    outline: none;
    color: var(--text);
  }
  .sort {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px;
    border: 1px solid var(--border-strong);
    border-radius: 12px;
    background: var(--surface);
    color: var(--muted);
  }
  .sort select {
    appearance: none;
    -webkit-appearance: none;
    border: 0;
    background: none;
    padding: 10px 0;
    color: var(--text);
    cursor: pointer;
  }
  .sort select:focus-visible {
    outline: none;
  }
  .sort:focus-within {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(270px, 1fr));
    gap: 20px;
  }
  .notice {
    margin: 0;
  }
  .login {
    align-items: center;
    padding-top: 64px;
  }
  .login :global(form) {
    width: 100%;
  }
</style>
