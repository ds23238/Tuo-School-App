<script lang="ts">
  import MoodleLogin from "../components/MoodleLogin.svelte";
  import { api, isCommandError, type Account, type Course } from "../lib/api";
  import { errorText } from "../lib/errors";
  import { i18n } from "../lib/i18n/index.svelte";

  let account = $state<Account | null>(null);
  let courses = $state<Course[]>([]);
  let loaded = $state(false);
  let refreshing = $state(false);
  let expired = $state(false);
  let error = $state<string | null>(null);

  const now = Date.now() / 1000;
  const current = $derived(courses.filter((c) => c.enrolled && !(c.endDate && c.endDate < now)));
  const past = $derived(courses.filter((c) => c.enrolled && c.endDate && c.endDate < now));
  const notEnrolled = $derived(courses.filter((c) => !c.enrolled));

  async function load() {
    try {
      const accounts = await api.listAccounts();
      account = accounts.find((a) => a.service === "moodle") ?? null;
      if (account) {
        courses = await api.moodle.courses();
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
    const sync = !course.sync;
    try {
      await api.moodle.setCourseSync(course.id, sync);
      course.sync = sync;
    } catch (e) {
      error = errorText(e);
    }
  }

  load();
</script>

{#snippet list(items: Course[])}
  <ul>
    {#each items as course (course.id)}
      <li>
        <label>
          <input
            type="checkbox"
            checked={course.sync}
            onchange={() => toggle(course)}
            aria-label={i18n.t.moodle.sync}
          />
          <span class="name">{course.fullName}</span>
          {#if course.shortName !== course.fullName}
            <span class="muted">{course.shortName}</span>
          {/if}
          {#if !course.visible}<span class="badge">{i18n.t.moodle.hidden}</span>{/if}
        </label>
      </li>
    {/each}
  </ul>
{/snippet}

{#if !loaded}
  <!-- lädt -->
{:else if !account || expired}
  <MoodleLogin
    url={account?.baseUrl}
    username={account?.username}
    notice={expired ? i18n.t.moodle.expired : null}
    {onlogin}
  />
{:else}
  <div class="bar">
    <span class="muted">
      {account.displayName ?? account.username} · {new URL(account.baseUrl).host}
    </span>
    <button onclick={refresh} disabled={refreshing}>
      {refreshing ? i18n.t.moodle.refreshing : i18n.t.moodle.refresh}
    </button>
  </div>
  {#if error}<p class="notice error">{error}</p>{/if}

  {#if courses.length === 0 && !refreshing}
    <p class="muted">{i18n.t.moodle.noCourses}</p>
  {:else}
    <p class="muted">{i18n.t.moodle.syncHint}</p>
    <h2>{i18n.t.moodle.current}</h2>
    {@render list(current)}
    {#if notEnrolled.length}
      <h2>{i18n.t.moodle.notEnrolled}</h2>
      {@render list(notEnrolled)}
    {/if}
    {#if past.length}
      <details>
        <summary><h2>{i18n.t.moodle.past} ({past.length})</h2></summary>
        {@render list(past)}
      </details>
    {/if}
  {/if}
{/if}

<style>
  .bar {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 16px;
    margin-bottom: 8px;
  }
  h2 {
    font-size: 1rem;
    margin: 20px 0 8px;
  }
  summary h2 {
    display: inline;
  }
  summary {
    cursor: pointer;
    margin: 20px 0 8px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li label {
    display: flex;
    align-items: baseline;
    gap: 10px;
    padding: 6px 4px;
    border-radius: 6px;
    cursor: pointer;
  }
  li label:hover {
    background: var(--hover);
  }
  .badge {
    font-size: 0.8em;
    padding: 0 6px;
    border-radius: 4px;
    background: var(--surface-2);
    border: 1px solid var(--border);
  }
</style>
