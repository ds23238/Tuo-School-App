<script lang="ts">
  import ActivityPage from "../components/ActivityPage.svelte";
  import Html from "../components/Html.svelte";
  import Icon from "../components/Icon.svelte";
  import { api, type Cached, type Choice, type Course, type Module } from "../lib/api";
  import { errorText } from "../lib/errors";
  import { fill, formatDate } from "../lib/format";
  import { i18n } from "../lib/i18n/index.svelte";

  let { course, module }: { course: Course; module: Module } = $props();

  let choice = $state<Cached<Choice> | null>(null);
  let error = $state<string | null>(null);
  let base = $state<string | null>(null);
  let picked = $state<number[]>([]);
  let busy = $state(false);

  api.listAccounts().then((a) => (base = a.find((x) => x.service === "moodle")?.baseUrl ?? null));

  async function load() {
    try {
      choice = await api.moodle.choice(course.id, module.id);
      picked = choice.data.options.filter((o) => o.checked).map((o) => o.id);
    } catch (e) {
      error = errorText(e);
    }
  }
  load();

  const c = $derived(choice?.data);
  /** Abstimmen bzw. ändern ist möglich. */
  const editable = $derived(!!c && !choice?.offline && c.open && (!c.answered || c.canUpdate));
  const changed = $derived(
    !!c && JSON.stringify([...picked].sort()) !== JSON.stringify(c.options.filter((o) => o.checked).map((o) => o.id).sort()),
  );

  function toggle(id: number) {
    if (!c) return;
    if (c.multiple) picked = picked.includes(id) ? picked.filter((p) => p !== id) : [...picked, id];
    else picked = [id];
  }

  async function submit(options: number[]) {
    if (!c) return;
    busy = true;
    error = null;
    try {
      await api.moodle.choiceAnswer(c.id, options);
      await load();
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }
</script>

<ActivityPage {course} {module} {error} offlineSince={choice?.offline ? choice.fetchedAt : null}>
  {#if module.description}
    <div class="intro"><Html html={module.description} moodleBase={base} /></div>
  {/if}
  {#if c}
    {#if c.opens || c.closes || c.answered || !c.open}
      <div class="meta">
        {#if c.opens && c.opens * 1000 > Date.now()}<span class="pill">{fill(i18n.t.choice.opens, { date: formatDate(c.opens) })}</span>{/if}
        {#if c.closes}<span class="pill" class:accent={c.open}>{fill(i18n.t.choice.closes, { date: formatDate(c.closes) })}</span>{/if}
        {#if c.answered}<span class="pill success"><Icon name="check" size={13} stroke={3} />{i18n.t.choice.voted}</span>{/if}
        {#if !c.open}<span class="pill">{i18n.t.choice.closed}</span>{/if}
      </div>
    {/if}
    <div class="card options">
      {#each c.options as o (o.id)}
        <label class="option" class:on={picked.includes(o.id)} class:disabled={!editable || o.disabled}>
          <input
            type={c.multiple ? "checkbox" : "radio"}
            name="choice"
            checked={picked.includes(o.id)}
            disabled={!editable || o.disabled || busy}
            onchange={() => toggle(o.id)}
          />
          <span class="text">{o.text}</span>
          {#if c.results && o.percent !== null}
            <span class="result">
              <span class="bar"><span style:width="{o.percent}%"></span></span>
              <span class="muted small count">{o.votes === 1 ? i18n.t.choice.vote1 : fill(i18n.t.choice.votes, { n: o.votes ?? 0 })} · {Math.round(o.percent)} %</span>
            </span>
          {/if}
        </label>
      {/each}
    </div>
    {#if !c.results}<p class="muted small">{i18n.t.choice.hidden}</p>{/if}
    {#if editable}
      <div class="actions">
        <button class="btn primary" onclick={() => submit(picked)} disabled={busy || picked.length === 0 || !changed}>
          {c.answered ? i18n.t.choice.change : i18n.t.choice.vote}
        </button>
        {#if c.answered && c.canUpdate}
          <button class="btn ghost" onclick={() => submit([])} disabled={busy}>{i18n.t.choice.remove}</button>
        {/if}
      </div>
    {/if}
  {:else if !error}
    <p class="muted">{i18n.t.page.loading}</p>
  {/if}
</ActivityPage>

<style>
  .intro {
    color: var(--text-2);
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .options {
    display: flex;
    flex-direction: column;
    padding: 6px;
  }
  .option {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 4px 12px;
    align-items: center;
    padding: 12px 14px;
    border-radius: 10px;
    cursor: pointer;
  }
  .option:hover:not(.disabled) {
    background: var(--hover);
  }
  .option.on {
    background: var(--accent-soft);
  }
  .option.disabled {
    cursor: default;
  }
  .option input {
    width: 18px;
    height: 18px;
    accent-color: var(--accent);
  }
  .text {
    font-weight: 500;
  }
  .result {
    grid-column: 2;
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .count {
    min-width: 110px;
    text-align: right;
  }
  .bar {
    flex: 1;
    height: 6px;
    border-radius: 3px;
    background: var(--surface-3);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    background: var(--accent);
  }
  .actions {
    display: flex;
    gap: 8px;
  }
</style>
