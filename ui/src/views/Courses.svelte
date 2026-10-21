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
  import PageView from "./PageView.svelte";

  type Filter = "current" | "past" | "all";

  let account = $state<Account | null>(null);
  let courses = $state<Course[]>([]);
  let stats = $state<Record<string, CourseFiles>>({});
  let loaded = $state(false);
  let refreshing = $state(false);
  let expired = $state(false);
  let error = $state<string | null>(null);
  let filter = $state<Filter>("current");

  const now = Date.now() / 1000;
  const isPast = (c: Course) => !!c.endDate && c.endDate < now;
  const shown = $derived(
    courses.filter((c) =>
      filter === "all" ? true : filter === "past" ? isPast(c) : !isPast(c) && (c.enrolled || c.sync),
    ),
  );

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
    {#if error}<p class="notice error">{error}</p>{/if}

    {#if shown.length === 0 && !refreshing}
      <p class="muted">{i18n.t.courses.empty}</p>
    {:else}
      <div class="grid">
        {#each shown as course (course.id)}
          <CourseCard
            {course}
            files={stats[course.id]}
            onopen={() => nav.openCourse(course)}
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
