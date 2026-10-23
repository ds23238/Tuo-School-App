<script lang="ts">
  import ActivityPage from "../components/ActivityPage.svelte";
  import Html from "../components/Html.svelte";
  import Icon from "../components/Icon.svelte";
  import { api, type Cached, type Course, type Glossary, type Module } from "../lib/api";
  import { errorText } from "../lib/errors";
  import { formatSize } from "../lib/format";
  import { i18n } from "../lib/i18n/index.svelte";

  let { course, module }: { course: Course; module: Module } = $props();

  let glossary = $state<Cached<Glossary> | null>(null);
  let error = $state<string | null>(null);
  let base = $state<string | null>(null);
  let query = $state("");
  let letter = $state<string | null>(null);
  let adding = $state(false);
  let draft = $state({ concept: "", text: "" });
  let busy = $state(false);

  api.listAccounts().then((a) => (base = a.find((x) => x.service === "moodle")?.baseUrl ?? null));

  async function load() {
    try {
      glossary = await api.moodle.glossary(course.id, module.id);
    } catch (e) {
      error = errorText(e);
    }
  }
  load();

  const initial = (concept: string) => {
    const c = concept.trim().charAt(0).toLocaleUpperCase(i18n.locale);
    return /\p{L}/u.test(c) ? c : "#";
  };
  const entries = $derived(
    [...(glossary?.data.entries ?? [])].sort((a, b) => a.concept.localeCompare(b.concept, i18n.locale)),
  );
  const letters = $derived([...new Set(entries.map((e) => initial(e.concept)))]);
  const shown = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return entries.filter(
      (e) =>
        (!letter || initial(e.concept) === letter) &&
        (!q || e.concept.toLowerCase().includes(q) || e.html.replace(/<[^>]*>/g, " ").toLowerCase().includes(q)),
    );
  });

  async function add(e: SubmitEvent) {
    e.preventDefault();
    if (!glossary || !draft.concept.trim() || !draft.text.trim()) return;
    busy = true;
    error = null;
    try {
      await api.moodle.glossaryAdd(glossary.data.id, draft.concept.trim(), draft.text);
      draft = { concept: "", text: "" };
      adding = false;
      await load();
    } catch (err) {
      error = errorText(err);
    } finally {
      busy = false;
    }
  }

  function attachment(entryId: number, name: string) {
    api.moodle.glossaryAttachment(course.id, module.id, entryId, name).catch((e) => (error = errorText(e)));
  }
</script>

<ActivityPage {course} {module} {error} offlineSince={glossary?.offline ? glossary.fetchedAt : null}>
  {#snippet actions()}
    {#if glossary?.data.canAdd && !glossary.offline}
      <button class="btn primary small" onclick={() => (adding = !adding)}>
        <Icon name={adding ? "x" : "plus"} size={14} />{i18n.t.glossary.add}
      </button>
    {/if}
  {/snippet}
  {#if module.description}
    <div class="intro"><Html html={module.description} moodleBase={base} /></div>
  {/if}
  {#if adding}
    <form class="card compose" onsubmit={add}>
      <input type="text" bind:value={draft.concept} placeholder={i18n.t.glossary.concept} required disabled={busy} />
      <textarea bind:value={draft.text} rows="5" placeholder={i18n.t.glossary.definition} required disabled={busy}></textarea>
      <div><button class="btn primary small" type="submit" disabled={busy}>{i18n.t.glossary.save}</button></div>
    </form>
  {/if}
  {#if glossary}
    <label class="search">
      <Icon name="search" size={16} />
      <input type="search" bind:value={query} placeholder={i18n.t.glossary.search} />
    </label>
    {#if letters.length > 1}
      <div class="letters">
        <button class="chip" class:on={letter === null} onclick={() => (letter = null)}>{i18n.t.glossary.all}</button>
        {#each letters as l (l)}
          <button class="chip" class:on={letter === l} onclick={() => (letter = l)}>{l}</button>
        {/each}
      </div>
    {/if}
    {#if entries.length === 0}
      <p class="muted">{i18n.t.glossary.empty}</p>
    {:else if shown.length === 0}
      <p class="muted">{i18n.t.glossary.noMatch}</p>
    {/if}
    <dl class="card">
      {#each shown as e (e.id)}
        <div class="entry">
          <dt>
            {e.concept}
            {#if !e.approved}<span class="pill">{i18n.t.glossary.pending}</span>{/if}
          </dt>
          <dd>
            <Html html={e.html} moodleBase={base} />
            {#if e.attachments.length}
              <div class="files">
                {#each e.attachments as a (a.name)}
                  <button class="file" onclick={() => attachment(e.id, a.name)}>
                    <Icon name="paperclip" size={14} />{a.name} <span class="muted">{formatSize(a.size)}</span>
                  </button>
                {/each}
              </div>
            {/if}
            <span class="muted small">{e.author}</span>
          </dd>
        </div>
      {/each}
    </dl>
  {:else if !error}
    <p class="muted">{i18n.t.page.loading}</p>
  {/if}
</ActivityPage>

<style>
  .intro {
    color: var(--text-2);
  }
  .compose {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 16px;
  }
  textarea {
    resize: vertical;
  }
  .search {
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
  .letters {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    min-width: 32px;
    border: 1px solid var(--border-strong);
    background: var(--surface);
    border-radius: 999px;
    padding: 3px 10px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-2);
    cursor: pointer;
  }
  .chip.on {
    background: var(--text);
    border-color: var(--text);
    color: var(--bg);
  }
  dl {
    margin: 0;
    padding: 4px 20px;
  }
  .entry {
    padding: 14px 0;
  }
  .entry + .entry {
    border-top: 1px solid var(--border);
  }
  dt {
    font-weight: 700;
    font-size: 16px;
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
  }
  dd {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .files {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .file {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    color: inherit;
    font-size: 13px;
    cursor: pointer;
  }
</style>
