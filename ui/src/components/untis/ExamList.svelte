<script lang="ts">
  import { api, type Exam, type Listed } from "../../lib/api";
  import { errorText } from "../../lib/errors";
  import { fill, formatDate } from "../../lib/format";
  import { i18n } from "../../lib/i18n/index.svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { isoDate, mapping, parseIso, subjectColor } from "../../lib/untis.svelte";
  import Icon from "../Icon.svelte";

  let data = $state<Listed<Exam> | null>(null);
  let error = $state<string | null>(null);
  const today = isoDate(new Date());

  // Formular für selbst eingetragene Prüfungen.
  let adding = $state(false);
  let form = $state({ date: today, start: "", subject: "", name: "", note: "" });

  function load() {
    api.untis.exams().then(
      (d) => (data = d),
      (e) => (error = errorText(e)),
    );
  }
  load();

  async function save(e: SubmitEvent) {
    e.preventDefault();
    try {
      await api.untis.addExam({
        date: form.date,
        start: form.start || null,
        subject: form.subject || null,
        name: form.name,
        note: form.note || null,
      });
      adding = false;
      form = { date: today, start: "", subject: "", name: "", note: "" };
      load();
    } catch (err) {
      error = errorText(err);
    }
  }

  async function remove(exam: Exam) {
    if (!(await ask(i18n.t.untis.deleteExamConfirm, { kind: "warning" }))) return;
    await api.untis.deleteExam(exam.id).catch((err) => (error = errorText(err)));
    load();
  }

  const day = (iso: string) =>
    new Intl.DateTimeFormat(i18n.locale, { weekday: "long", day: "numeric", month: "long" }).format(
      parseIso(iso),
    );
</script>

<div class="tools">
  {#if data?.offline && data.fetchedAt}
    <span class="pill">{fill(i18n.t.untis.offline, { date: formatDate(data.fetchedAt) })}</span>
  {/if}
  <span class="grow"></span>
  <button class="btn small" onclick={() => (adding = !adding)}>
    <Icon name={adding ? "x" : "plus"} size={14} />
    {i18n.t.untis.addExam}
  </button>
</div>

{#if adding}
  <form class="card add" onsubmit={save}>
    <label>
      <span>{i18n.t.untis.examDate}</span>
      <input type="date" bind:value={form.date} required />
    </label>
    <label>
      <span>{i18n.t.untis.examTime}</span>
      <input type="time" bind:value={form.start} />
    </label>
    <label>
      <span>{i18n.t.untis.subject}</span>
      <input type="text" bind:value={form.subject} list="exam-subjects" />
      <datalist id="exam-subjects">
        {#each mapping.entries as m (m.subject)}<option value={m.longName}></option>{/each}
      </datalist>
    </label>
    <label class="wide">
      <span>{i18n.t.untis.examName}</span>
      <input type="text" bind:value={form.name} required placeholder={i18n.t.untis.examNameHint} />
    </label>
    <label class="wide">
      <span>{i18n.t.untis.examNote}</span>
      <textarea bind:value={form.note} rows="2"></textarea>
    </label>
    <div class="wide"><button class="btn primary small" type="submit">{i18n.t.assign.save}</button></div>
  </form>
{/if}
{#if error}<p class="notice error">{error}</p>{/if}
{#if data && data.items.length === 0}<p class="muted">{i18n.t.untis.noExams}</p>{/if}
<ul>
  {#each data?.items ?? [] as e (`${e.local}-${e.id}`)}
    <li class="card" class:past={e.date < today} style="--c:{subjectColor(e.subject ?? e.name)}">
      <div class="date">
        <strong>{parseIso(e.date).getDate()}</strong>
        <span>{new Intl.DateTimeFormat(i18n.locale, { month: "short" }).format(parseIso(e.date))}</span>
      </div>
      <div class="body">
        <div class="head">
          <strong>{e.subject ?? e.name}</strong>
          {#if e.kind}<span class="pill">{e.kind}</span>{/if}
          {#if e.grade}<span class="pill success">{i18n.t.untis.grade}: {e.grade}</span>{/if}
          {#if e.local}
            <span class="pill">{i18n.t.untis.ownExam}</span>
            <button class="btn ghost small del" onclick={() => remove(e)} aria-label={i18n.t.mail.delete} title={i18n.t.mail.delete}>
              <Icon name="trash" size={14} />
            </button>
          {/if}
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
  .tools {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .grow {
    flex: 1;
  }
  .add {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 12px;
    padding: 16px;
  }
  .add label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
    font-weight: 500;
  }
  .add .wide {
    grid-column: 1 / -1;
  }
  .del {
    margin-left: auto;
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
    gap: 16px;
    padding: 14px 16px;
    border-left: 4px solid var(--c);
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
