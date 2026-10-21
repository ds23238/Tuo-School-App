<script lang="ts">
  import { api, type Exam, type Listed } from "../../lib/api";
  import { errorText } from "../../lib/errors";
  import { fill, formatDate } from "../../lib/format";
  import { i18n } from "../../lib/i18n/index.svelte";
  import { isoDate, parseIso, subjectHue } from "../../lib/untis.svelte";

  let data = $state<Listed<Exam> | null>(null);
  let error = $state<string | null>(null);
  const today = isoDate(new Date());

  api.untis.exams().then(
    (d) => (data = d),
    (e) => (error = errorText(e)),
  );

  const day = (iso: string) =>
    new Intl.DateTimeFormat(i18n.locale, { weekday: "long", day: "numeric", month: "long" }).format(
      parseIso(iso),
    );
</script>

{#if data?.offline}
  <span class="pill">{fill(i18n.t.untis.offline, { date: formatDate(data.fetchedAt) })}</span>
{/if}
{#if error}<p class="notice error">{error}</p>{/if}
{#if data && data.items.length === 0}<p class="muted">{i18n.t.untis.noExams}</p>{/if}
<ul>
  {#each data?.items ?? [] as e (e.id)}
    <li class="card" class:past={e.date < today} style="--hue:{subjectHue(e.subject ?? e.name)}">
      <div class="date">
        <strong>{parseIso(e.date).getDate()}</strong>
        <span>{new Intl.DateTimeFormat(i18n.locale, { month: "short" }).format(parseIso(e.date))}</span>
      </div>
      <div class="body">
        <div class="head">
          <strong>{e.subject ?? e.name}</strong>
          {#if e.kind}<span class="pill">{e.kind}</span>{/if}
          {#if e.grade}<span class="pill success">{i18n.t.untis.grade}: {e.grade}</span>{/if}
        </div>
        {#if e.subject && e.name && e.name !== e.subject}<p>{e.name}</p>{/if}
        <p class="muted small">
          {day(e.date)}{e.start ? ` · ${e.start}–${e.end ?? ""}` : ""}
          {#if e.rooms.length} · {e.rooms.join(", ")}{/if}
          {#if e.teachers.length} · {e.teachers.join(", ")}{/if}
        </p>
        {#if e.text}<p class="text">{e.text}</p>{/if}
      </div>
    </li>
  {/each}
</ul>

<style>
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
    gap: 16px;
    padding: 14px 16px;
    border-left: 4px solid hsl(var(--hue) 60% 50%);
  }
  .date {
    display: flex;
    flex-direction: column;
    align-items: center;
    min-width: 44px;
    color: var(--text-2);
  }
  .date strong {
    font-size: 22px;
    line-height: 1.1;
  }
  .date span {
    font-size: 12px;
    text-transform: uppercase;
  }
  .body {
    flex: 1;
    min-width: 0;
  }
  .head {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }
  p {
    margin: 4px 0 0;
  }
  .text {
    white-space: pre-line;
  }
  .past {
    opacity: 0.55;
  }
  .notice {
    margin: 0;
  }
</style>
