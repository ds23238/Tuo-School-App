<script lang="ts">
  import Icon from "../components/Icon.svelte";
  import {
    api,
    type Account,
    type Deadline,
    type Exam,
    type Homework,
    type Lesson,
    type Module,
    type RecentFile,
    type Section,
  } from "../lib/api";
  import { errorText } from "../lib/errors";
  import { formatShort, formatTime } from "../lib/format";
  import { i18n } from "../lib/i18n/index.svelte";
  import { nav } from "../lib/nav.svelte";
  import LessonDetail from "../components/untis/LessonDetail.svelte";
  import { isoDate, lessonState, mapping, parseIso, subjectHue } from "../lib/untis.svelte";

  let moodle = $state<Account | null>(null);
  let untis = $state<Account | null>(null);
  let lessons = $state<Lesson[] | null>(null);
  let deadlines = $state<Deadline[]>([]);
  let homework = $state<Homework[]>([]);
  let exams = $state<Exam[]>([]);
  let files = $state<RecentFile[]>([]);
  let errors = $state<string[]>([]);
  const today = isoDate(new Date());
  const now = Date.now() / 1000;

  function fail(e: unknown) {
    errors = [...errors, errorText(e)];
  }

  api.listAccounts().then((accounts) => {
    moodle = accounts.find((a) => a.service === "moodle") ?? null;
    untis = accounts.find((a) => a.service === "untis") ?? null;
    if (untis) {
      mapping.load();
      api.untis.upcoming().then((l) => (lessons = l), fail);
      api.untis.homework().then((h) => (homework = h.items), fail);
      api.untis.exams().then((e) => (exams = e.items), fail);
    }
    if (moodle) {
      api.today.deadlines().then((d) => (deadlines = d), fail);
      api.today.recentFiles().then((f) => (files = f), fail);
    }
  });

  /** Heute und der nächste Tag mit Unterricht. */
  const days = $derived.by(() => {
    const dates = [...new Set((lessons ?? []).map((l) => l.date))].sort();
    const shown = [today, ...dates.filter((d) => d > today).slice(0, 1)];
    return shown.map((date) => ({ date, lessons: (lessons ?? []).filter((l) => l.date === date) }));
  });

  type Task = {
    key: string;
    kind: "deadline" | "homework" | "exam";
    title: string;
    detail: string;
    /** Unix-Zeit für die Sortierung. */
    time: number;
    timeLabel: string;
    overdue: boolean;
    homework?: Homework;
    deadline?: Deadline;
  };

  const dayLabel = (iso: string) =>
    new Intl.DateTimeFormat(i18n.locale, { weekday: "short", day: "numeric", month: "short" }).format(parseIso(iso));

  const tasks = $derived.by(() => {
    const horizon = now + 14 * 24 * 3600;
    const list: Task[] = [];
    for (const d of deadlines) {
      if (d.time > now + 30 * 24 * 3600) continue;
      list.push({
        key: `d${d.id}`,
        kind: "deadline",
        title: d.name,
        detail: d.course,
        time: d.time,
        timeLabel: `${formatShort(d.time)} ${formatTime(new Date(d.time * 1000))}`,
        overdue: d.time < now,
        deadline: d,
      });
    }
    for (const h of homework) {
      const t = parseIso(h.due).getTime() / 1000;
      if (h.done || h.due < today || t > horizon) continue;
      list.push({
        key: `h${h.id}`,
        kind: "homework",
        title: h.text,
        detail: h.subject ?? "",
        time: t + 12 * 3600,
        timeLabel: dayLabel(h.due),
        overdue: false,
        homework: h,
      });
    }
    for (const e of exams) {
      const t = parseIso(e.date).getTime() / 1000;
      if (e.date < today || t > horizon) continue;
      list.push({
        key: `e${e.id}`,
        kind: "exam",
        title: e.subject ?? e.name,
        detail: [e.name !== e.subject ? e.name : "", e.rooms.join(", ")].filter(Boolean).join(" · "),
        time: t + 8 * 3600,
        timeLabel: dayLabel(e.date) + (e.start ? ` ${e.start}` : ""),
        overdue: false,
      });
    }
    return list.sort((a, b) => a.time - b.time);
  });

  function findModule(sections: Section[], id: number): Module | null {
    for (const s of sections) {
      for (const m of s.modules) {
        if (m.id === id) return m;
        const inner = m.subsection ? findModule([m.subsection], id) : null;
        if (inner) return inner;
      }
    }
    return null;
  }

  async function openDeadline(d: Deadline) {
    const course = (await api.moodle.courses()).find((c) => c.id === d.courseId);
    const view = course && d.moduleId ? await api.moodle.course(course.id).catch(() => null) : null;
    const module = view && d.moduleId ? findModule(view.sections, d.moduleId) : null;
    if (course && module && (module.kind === "assign" || module.kind === "page" || module.kind === "book")) {
      nav.openCourse(course);
      nav.openModule(module);
    } else if (course) {
      nav.openCourse(course);
    } else if (d.url) {
      api.moodle.openInBrowser(d.url).catch(fail);
    }
  }

  async function toggleHomework(h: Homework) {
    h.done = !h.done;
    homework = [...homework];
    await api.untis.setHomeworkDone(h.id, h.done).catch(fail);
  }

  let selected = $state<Lesson | null>(null);

  const title = new Intl.DateTimeFormat(i18n.locale, { weekday: "long", day: "numeric", month: "long" }).format(new Date());
</script>

{#if selected}
  <LessonDetail lesson={selected} {homework} onclose={() => (selected = null)} />
{/if}

<div class="page">
  <header>
    <h1>{i18n.t.today.title}</h1>
    <p class="muted">{title}</p>
  </header>
  {#each errors as e, i (i)}<p class="notice error">{e}</p>{/each}

  <div class="columns">
    <section class="card">
      <h2><Icon name="timetable" size={18} /> {i18n.t.today.lessons}</h2>
      {#if !untis}
        <p class="muted">{i18n.t.today.connectUntis}</p>
        <button class="btn small" onclick={() => nav.go("timetable")}>{i18n.t.today.connect}</button>
      {:else}
        {#each days as day (day.date)}
          <h3>{day.date === today ? i18n.t.today.today : dayLabel(day.date)}</h3>
          {#if day.lessons.length === 0 && lessons}
            <p class="muted small">{i18n.t.today.noSchool}</p>
          {/if}
          <ul class="lessons">
            {#each day.lessons as l (l.id)}
              <li>
                <button class="lesson {l.status}" style="--hue:{subjectHue(l.subject?.name ?? '')}" onclick={() => (selected = l)}>
                  <span class="time">{l.start}<br /><span class="muted">{l.end}</span></span>
                  <span class="what">
                    <strong>{l.subject?.longName ?? l.text ?? "–"}</strong>
                    <span class="muted small">
                      {l.rooms.map((r) => r.name).join(", ")}
                      {#if l.teachers.length} · {l.teachers.map((t) => (t.absent ? `(${t.name})` : t.name)).join(", ")}{/if}
                    </span>
                    {#if l.substitution || l.info}<span class="small note">{l.substitution ?? l.info}</span>{/if}
                  </span>
                  {#if lessonState(l) !== "regular"}
                    {@const s = lessonState(l)}
                    <span class="pill" class:danger={s === "cancelled" || s === "noTeacher"} class:accent={s !== "cancelled" && s !== "noTeacher" && s !== "exam"}>{i18n.t.untis.state[s]}</span>
                  {/if}
                </button>
              </li>
            {/each}
          </ul>
        {/each}
      {/if}
    </section>

    <div class="side">
      <section class="card">
        <h2><Icon name="assign" size={18} /> {i18n.t.today.tasks}</h2>
        {#if tasks.length === 0}<p class="muted">{i18n.t.today.noTasks}</p>{/if}
        <ul class="tasks">
          {#each tasks as t (t.key)}
            <li class:overdue={t.overdue}>
              {#if t.homework}
                <input type="checkbox" checked={t.homework.done} onchange={() => toggleHomework(t.homework!)} aria-label={i18n.t.untis.done} />
              {:else}
                <Icon name={t.kind === "exam" ? "quiz" : "clock"} size={18} />
              {/if}
              <button class="task" onclick={() => (t.deadline ? openDeadline(t.deadline) : nav.go("timetable"))}>
                <span class="title">{t.title}</span>
                <span class="muted small">
                  <span class="kind {t.kind}">{t.kind === "deadline" ? i18n.t.today.deadline : t.kind === "exam" ? i18n.t.today.exam : i18n.t.today.homework}</span>
                  {t.detail}
                </span>
              </button>
              <span class="when" class:late={t.overdue}>{t.overdue ? i18n.t.today.overdue : t.timeLabel}</span>
            </li>
          {/each}
        </ul>
      </section>

      {#if moodle}
        <section class="card">
          <h2><Icon name="file" size={18} /> {i18n.t.today.newFiles}</h2>
          {#if files.length === 0}<p class="muted">{i18n.t.today.noFiles}</p>{/if}
          <ul class="files">
            {#each files as f (f.id)}
              <li>
                <button onclick={() => api.moodle.openFile(f.id).catch(fail)}>
                  <Icon name="fileText" size={16} />
                  <span class="title">{f.name}</span>
                  <span class="muted small">{f.course}</span>
                </button>
              </li>
            {/each}
          </ul>
        </section>
      {/if}
    </div>
  </div>
</div>

<style>
  .page {
    max-width: 1160px;
    margin: 0 auto;
    padding: 32px 40px 48px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }
  h1 {
    font-size: 28px;
    font-weight: 700;
  }
  header p {
    margin: 4px 0 0;
  }
  .columns {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 20px;
    align-items: start;
  }
  .side {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }
  section {
    padding: 18px 20px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  h2 {
    display: flex;
    gap: 8px;
    align-items: center;
    font-size: 16px;
  }
  h3 {
    font-size: 13px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--muted);
    margin-top: 6px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .lesson {
    display: flex;
    gap: 12px;
    align-items: center;
    width: 100%;
    padding: 8px 10px;
    border: 0;
    border-left: 4px solid hsl(var(--hue) 60% 50%);
    border-radius: 8px;
    background: var(--surface-2);
    color: inherit;
    text-align: left;
    cursor: pointer;
  }
  .lesson:hover {
    background: var(--hover);
  }
  .lesson.cancelled {
    border-left-color: var(--danger);
  }
  .lesson.cancelled strong {
    text-decoration: line-through;
  }
  .lesson.changed {
    border-left-color: var(--accent);
  }
  .time {
    font-size: 13px;
    font-variant-numeric: tabular-nums;
    min-width: 44px;
  }
  .what {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .note {
    color: var(--accent-ink);
  }
  .tasks li {
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 6px 0;
    border-bottom: 1px solid var(--border);
    color: var(--muted);
  }
  .tasks input {
    width: 18px;
    height: 18px;
    accent-color: var(--accent);
  }
  .task {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    border: 0;
    background: none;
    color: var(--text);
    text-align: left;
    cursor: pointer;
    padding: 0;
  }
  .title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 500;
  }
  .kind {
    font-weight: 600;
    margin-right: 4px;
  }
  .kind.deadline {
    color: var(--p-assessment);
  }
  .kind.homework {
    color: var(--p-content);
  }
  .kind.exam {
    color: var(--danger);
  }
  .when {
    font-size: 12px;
    white-space: nowrap;
  }
  .late {
    color: var(--danger);
    font-weight: 600;
  }
  .files button {
    display: flex;
    gap: 8px;
    align-items: center;
    width: 100%;
    border: 0;
    background: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
    padding: 4px 0;
  }
  .files .title {
    flex: 1;
  }
  .notice {
    margin: 0;
  }
  @media (max-width: 900px) {
    .columns {
      grid-template-columns: 1fr;
    }
  }
</style>
