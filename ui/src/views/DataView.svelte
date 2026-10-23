<script lang="ts">
  import ActivityPage from "../components/ActivityPage.svelte";
  import Html from "../components/Html.svelte";
  import Icon from "../components/Icon.svelte";
  import { api, type Cached, type Course, type DataEntry, type Module, type MoodleDatabase } from "../lib/api";
  import { errorText } from "../lib/errors";
  import { fill, formatDate, formatDay } from "../lib/format";
  import { i18n } from "../lib/i18n/index.svelte";

  let { course, module }: { course: Course; module: Module } = $props();

  let db = $state<Cached<MoodleDatabase> | null>(null);
  let error = $state<string | null>(null);
  let base = $state<string | null>(null);
  let query = $state("");

  api.listAccounts().then((a) => (base = a.find((x) => x.service === "moodle")?.baseUrl ?? null));
  async function load() {
    try {
      db = await api.moodle.database(course.id, module.id);
    } catch (e) {
      error = errorText(e);
    }
  }
  load();

  const fields = $derived(new Map((db?.data.fields ?? []).map((f) => [f.id, f])));

  /** Suche im sichtbaren Text aller Felder. */
  const text = (e: DataEntry) =>
    [e.author, ...e.values.map((v) => v.html.replace(/<[^>]*>/g, " "))].join(" ").toLowerCase();
  const shown = $derived.by(() => {
    const entries = [...(db?.data.entries ?? [])].sort((a, b) => b.created - a.created);
    const q = query.trim().toLowerCase();
    return q ? entries.filter((e) => text(e).includes(q)) : entries;
  });

  const pictures = (e: DataEntry) => e.values.filter((v) => fields.get(v.fieldId)?.kind === "picture");
  const others = (e: DataEntry) => e.values.filter((v) => fields.get(v.fieldId)?.kind !== "picture");

  function add() {
    if (db) api.moodle.openInBrowser(db.data.addUrl).catch((e) => (error = errorText(e)));
  }
</script>

<ActivityPage {course} {module} {error} offlineSince={db?.offline ? db.fetchedAt : null}>
  {#snippet actions()}
    {#if db?.data.canAdd}
      <button class="btn primary small" onclick={add}>
        <Icon name="plus" size={14} />{i18n.t.data.add}
        <Icon name="external" size={12} />
      </button>
    {/if}
  {/snippet}
  {#if module.description}
    <div class="intro"><Html html={module.description} moodleBase={base} /></div>
  {/if}
  {#if db}
    <div class="tools">
      <label class="search">
        <Icon name="search" size={16} />
        <input type="search" bind:value={query} placeholder={i18n.t.data.search} />
      </label>
      <span class="muted small">{fill(i18n.t.data.count, { n: db.data.entries.length })}</span>
    </div>
    {#if db.data.entries.length === 0}
      <p class="muted">{i18n.t.data.empty}</p>
    {:else if shown.length === 0}
      <p class="muted">{i18n.t.data.noMatch}</p>
    {/if}
    <div class="grid">
      {#each shown as e (e.id)}
        <article class="card entry">
          {#each pictures(e) as v (v.fieldId)}
            <div class="picture"><Html html={v.html} moodleBase={base} /></div>
          {/each}
          <dl>
            {#each others(e) as v (v.fieldId)}
              <dt>{fields.get(v.fieldId)?.name}</dt>
              <dd>
                {#if v.date !== null}{formatDay(v.date)}{:else}<Html html={v.html} moodleBase={base} />{/if}
              </dd>
            {/each}
          </dl>
          <footer class="muted small">
            {e.author} · {formatDate(e.created)}
            {#if !e.approved}<span class="pill">{i18n.t.data.pending}</span>{/if}
          </footer>
        </article>
      {/each}
    </div>
  {:else if !error}
    <p class="muted">{i18n.t.page.loading}</p>
  {/if}
</ActivityPage>

<style>
  .intro {
    color: var(--text-2);
  }
  .tools {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .search {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px;
    border: 1px solid var(--border-strong);
    border-radius: 10px;
    background: var(--surface);
    color: var(--muted);
  }
  .search input {
    flex: 1;
    border: 0;
    background: none;
    padding: 9px 0;
  }
  .search input:focus {
    outline: none;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 14px;
    align-items: start;
  }
  .entry {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .picture :global(img) {
    display: block;
    width: 100%;
    max-height: 240px;
    object-fit: cover;
  }
  dl {
    margin: 0;
    padding: 14px 16px 4px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  dt {
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
  }
  dd {
    margin: 0 0 8px;
  }
  footer {
    padding: 0 16px 14px;
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }
</style>
