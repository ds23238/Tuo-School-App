<script lang="ts">
  import { api, type Homework, type Listed } from "../../lib/api";
  import { errorText } from "../../lib/errors";
  import { fill, formatDate } from "../../lib/format";
  import { i18n } from "../../lib/i18n/index.svelte";
  import { isoDate, parseIso, subjectColor } from "../../lib/untis.svelte";

  let data = $state<Listed<Homework> | null>(null);
  let error = $state<string | null>(null);
  let showDone = $state(false);
  const today = isoDate(new Date());

  api.untis.homework().then(
    (d) => (data = d),
    (e) => (error = errorText(e)),
  );

  const shown = $derived(
    (data?.items ?? []).filter((h) => showDone || (!h.done && h.due >= today)),
  );

  async function toggle(h: Homework) {
    h.done = !h.done;
    await api.untis.setHomeworkDone(h.id, h.done).catch((e) => (error = errorText(e)));
  }

  const day = (iso: string) =>
    new Intl.DateTimeFormat(i18n.locale, { weekday: "short", day: "numeric", month: "short" }).format(
      parseIso(iso),
    );
</script>

<div class="tools">
  {#if data?.offline}
    <span class="pill">{fill(i18n.t.untis.offline, { date: formatDate(data.fetchedAt) })}</span>
  {/if}
  <label class="check">
    <input type="checkbox" bind:checked={showDone} />
    {i18n.t.untis.showDone}
  </label>
</div>
{#if error}<p class="notice error">{error}</p>{/if}
{#if data && shown.length === 0}<p class="muted">{i18n.t.untis.noHomework}</p>{/if}
<ul>
  {#each shown as h (h.id)}
    <li class="card" class:done={h.done} style="--c:{subjectColor(h.subject ?? '')}">
      <input type="checkbox" checked={h.done} onchange={() => toggle(h)} aria-label={i18n.t.untis.done} />
      <div class="body">
        <div class="head">
          <strong>{h.subject ?? "–"}</strong>
          <span class="pill" class:soon={h.due <= today}>{fill(i18n.t.untis.due, { date: day(h.due) })}</span>
        </div>
        <p class="text">{h.text}</p>
        {#if h.remark}<p class="muted small">{h.remark}</p>{/if}
        <p class="muted small">
          {fill(i18n.t.untis.assigned, { date: day(h.assigned) })}{h.teacher ? ` · ${h.teacher}` : ""}
          {#if h.attachments.length} · 📎 {h.attachments.join(", ")}{/if}
        </p>
      </div>
    </li>
  {/each}
</ul>

<style>
  .tools {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 12px;
  }
  .check {
    display: flex;
    gap: 8px;
    align-items: center;
    font-size: 14px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  li {
    display: flex;
    gap: 14px;
    align-items: flex-start;
    padding: 14px 16px;
    border-left: 4px solid var(--c);
  }
  li input {
    margin-top: 4px;
    width: 18px;
    height: 18px;
    accent-color: var(--accent);
  }
  .body {
    flex: 1;
    min-width: 0;
  }
  .head {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    align-items: center;
  }
  .text {
    margin: 6px 0;
    white-space: pre-line;
  }
  .body p.small {
    margin: 2px 0 0;
  }
  .soon {
    background: var(--accent-soft);
    color: var(--accent-ink);
  }
  .done {
    opacity: 0.6;
  }
  .done .text {
    text-decoration: line-through;
  }
  .notice {
    margin: 0;
  }
</style>
