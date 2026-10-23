<script lang="ts">
  import ActivityPage from "../components/ActivityPage.svelte";
  import Html from "../components/Html.svelte";
  import Icon from "../components/Icon.svelte";
  import { api, isCommandError, type Board, type Cached, type Course, type Module } from "../lib/api";
  import { errorText } from "../lib/errors";
  import { formatShort } from "../lib/format";
  import { i18n } from "../lib/i18n/index.svelte";

  let { course, module }: { course: Course; module: Module } = $props();

  let board = $state<Cached<Board> | null>(null);
  let error = $state<string | null>(null);
  let base = $state<string | null>(null);

  api.listAccounts().then((a) => (base = a.find((x) => x.service === "moodle")?.baseUrl ?? null));

  async function load() {
    try {
      board = await api.moodle.board(course.id, module.id);
    } catch (e) {
      error = isCommandError(e) && e.code === "mailLogin" ? i18n.t.board.loginFailed : errorText(e);
    }
  }
  load();

  function openMoodle() {
    if (module.url) api.moodle.openInBrowser(module.url).catch((e) => (error = errorText(e)));
  }
</script>

<ActivityPage {course} {module} {error} offlineSince={board?.offline ? board.fetchedAt : null}>
  {#snippet actions()}
    <button class="btn small" onclick={openMoodle}>
      <Icon name="external" size={14} />{i18n.t.board.openMoodle}
    </button>
  {/snippet}
  {#if module.description}
    <div class="intro"><Html html={module.description} moodleBase={base} /></div>
  {/if}
  {#if board}
    <p class="muted small">{i18n.t.board.readOnly}</p>
    {#if board.data.columns.length === 0}<p class="muted">{i18n.t.board.empty}</p>{/if}
    <div class="columns">
      {#each board.data.columns as column (column.id)}
        <section class="column">
          <h2>{column.name}</h2>
          {#each column.notes as note (note.id)}
            <article class="card note">
              {#if note.heading}<h3>{note.heading}</h3>{/if}
              <Html html={note.html} moodleBase={base} />
              <footer class="muted small">
                {formatShort(note.created)}
                {#if note.rating > 0}<span><Icon name="star" size={12} /> {note.rating}</span>{/if}
              </footer>
            </article>
          {:else}
            <p class="muted small">{i18n.t.board.emptyColumn}</p>
          {/each}
        </section>
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
  .columns {
    display: flex;
    gap: 14px;
    overflow-x: auto;
    padding-bottom: 8px;
    align-items: flex-start;
  }
  .column {
    flex: 0 0 260px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    border-radius: var(--radius);
    background: var(--surface-2);
  }
  h2 {
    font-size: 15px;
  }
  .note {
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 14px;
  }
  h3 {
    font-size: 14px;
  }
  footer {
    display: flex;
    justify-content: space-between;
  }
  footer span {
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }
</style>
