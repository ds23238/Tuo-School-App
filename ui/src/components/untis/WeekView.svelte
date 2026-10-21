<script lang="ts">
  import { api, type Lesson, type Week } from "../../lib/api";
  import { errorText } from "../../lib/errors";
  import { fill, formatDate } from "../../lib/format";
  import { i18n } from "../../lib/i18n/index.svelte";
  import { isoDate, parseIso } from "../../lib/untis.svelte";
  import Icon from "../Icon.svelte";
  import LessonCard from "./LessonCard.svelte";
  import LessonDetail from "./LessonDetail.svelte";

  let { onexpired }: { onexpired: () => void } = $props();

  let date = $state(new Date());
  let week = $state<Week | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let selected = $state<Lesson | null>(null);
  const today = isoDate(new Date());

  async function load() {
    loading = true;
    error = null;
    try {
      week = await api.untis.week(isoDate(date));
    } catch (e) {
      error = errorText(e);
      if (typeof e === "object" && e && "code" in e && e.code === "untisExpired") onexpired();
    } finally {
      loading = false;
    }
  }

  function shift(days: number) {
    const next = new Date(date);
    next.setDate(next.getDate() + days);
    date = next;
    selected = null;
    load();
  }

  function goToday() {
    date = new Date();
    selected = null;
    load();
  }

  type Row = { name: string; start: string; end: string };

  /** Zeilen aus dem Zeitraster; Stunden außerhalb davon bekommen eigene. */
  const rows = $derived.by((): Row[] => {
    if (!week) return [];
    const rows: Row[] = week.grid.map((u) => ({ ...u }));
    for (const l of week.lessons) {
      if (!rows.some((r) => r.start <= l.start && l.start < r.end)) {
        rows.push({ name: "", start: l.start, end: l.end });
      }
    }
    rows.sort((a, b) => a.start.localeCompare(b.start));
    // Leere Randstunden (z. B. 9.–12. Stunde ohne Unterricht) ausblenden.
    const used = (r: Row) => week!.lessons.some((l) => r.start <= l.start && l.start < r.end);
    const first = rows.findIndex(used);
    const last = rows.length - 1 - [...rows].reverse().findIndex(used);
    return first < 0 ? rows : rows.slice(first, last + 1);
  });

  function cell(day: string, row: Row): Lesson[] {
    return (week?.lessons ?? []).filter((l) => l.date === day && row.start <= l.start && l.start < row.end);
  }

  const weekday = (day: string) =>
    new Intl.DateTimeFormat(i18n.locale, { weekday: "short" }).format(parseIso(day));
  const dayMonth = (day: string) =>
    new Intl.DateTimeFormat(i18n.locale, { day: "numeric", month: "numeric" }).format(parseIso(day));

  load();
</script>

<div class="toolbar">
  <div class="nav">
    <button class="btn ghost small" onclick={() => shift(-7)} aria-label={i18n.t.untis.prevWeek} title={i18n.t.untis.prevWeek}>
      <Icon name="chevronLeft" size={18} />
    </button>
    <button class="btn small" onclick={goToday}>{i18n.t.untis.thisWeek}</button>
    <button class="btn ghost small" onclick={() => shift(7)} aria-label={i18n.t.untis.nextWeek} title={i18n.t.untis.nextWeek}>
      <Icon name="chevronRight" size={18} />
    </button>
    {#if week}
      <span class="range">{dayMonth(week.days[0]!.date)} – {dayMonth(week.days[week.days.length - 1]!.date)}</span>
    {/if}
    {#if loading}<Icon name="refresh" size={16} class="spin" />{/if}
  </div>
  {#if week?.offline}
    <span class="pill">{fill(i18n.t.untis.offline, { date: formatDate(week.fetchedAt) })}</span>
  {/if}
</div>

{#if error}<p class="notice error">{error}</p>{/if}

{#if week}
  <div class="grid" style="--days:{week.days.length}">
    <div class="corner"></div>
    {#each week.days as d (d.date)}
      <div class="day-head" class:today={d.date === today}>
        <strong>{weekday(d.date)}</strong>
        <span>{dayMonth(d.date)}</span>
      </div>
    {/each}
    {#each rows as row, r (row.start)}
      <div class="unit" style="grid-column:1; grid-row:{r + 2}">
        <strong>{row.name}</strong>
        <span>{row.start}</span>
      </div>
      {#each week.days as d, i (d.date)}
        <div
          class="cell"
          class:today={d.date === today}
          class:holiday={!!d.holiday}
          style="grid-column:{i + 2}; grid-row:{r + 2}"
        >
          {#each cell(d.date, row) as l (l.id)}
            <LessonCard lesson={l} selected={selected?.id === l.id} onselect={() => (selected = l)} />
          {/each}
        </div>
      {/each}
    {/each}
    {#each week.days as d, i (d.date)}
      {#if d.holiday}
        <div class="holiday-label" style="grid-column:{i + 2}; grid-row:2 / span {Math.max(rows.length, 1)}">
          {d.holiday}
        </div>
      {/if}
    {/each}
  </div>
  {#if rows.length === 0 && !loading}<p class="muted">{i18n.t.untis.noLessons}</p>{/if}
{/if}

{#if selected}
  <LessonDetail lesson={selected} onclose={() => (selected = null)} />
{/if}

<style>
  .toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px;
  }
  .nav {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .range {
    margin-left: 8px;
    font-weight: 600;
  }
  .grid {
    display: grid;
    grid-template-columns: 56px repeat(var(--days), minmax(0, 1fr));
    gap: 4px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 8px;
    position: relative;
  }
  .day-head {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 6px 0;
    border-radius: 8px;
    font-size: 13px;
    color: var(--text-2);
  }
  .day-head.today {
    background: var(--accent);
    color: var(--on-accent);
  }
  .unit {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    color: var(--muted);
  }
  .unit strong {
    font-size: 14px;
    color: var(--text-2);
  }
  .cell {
    min-height: 54px;
    display: flex;
    gap: 3px;
    border-radius: 8px;
    background: var(--surface-2);
  }
  .cell.today {
    background: var(--accent-soft);
  }
  .cell.holiday {
    background: transparent;
  }
  .holiday-label {
    display: flex;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 8px;
    border-radius: 8px;
    background: var(--surface-3);
    color: var(--muted);
    font-weight: 600;
    writing-mode: vertical-rl;
    transform: rotate(180deg);
  }
  .notice {
    margin: 0;
  }
</style>
