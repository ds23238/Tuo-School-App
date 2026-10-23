<script lang="ts">
  import type { Snippet } from "svelte";
  import type { Course, Module } from "../lib/api";
  import { fill, formatDate } from "../lib/format";
  import { i18n } from "../lib/i18n/index.svelte";
  import { nav } from "../lib/nav.svelte";
  import Icon from "./Icon.svelte";

  /** Rahmen für Aktivitäten in der App: Zurück, Titel, Stand, Fehler. */
  let {
    course,
    module,
    title = null,
    onback = null,
    error = null,
    offlineSince = null,
    actions,
    children,
  }: {
    course: Course;
    module: Module;
    /** Statt des Aktivitätsnamens, z. B. ein Forumsthema. */
    title?: string | null;
    /** Statt zurück zum Kurs, z. B. zurück zur Themenliste. */
    onback?: (() => void) | null;
    error?: string | null;
    offlineSince?: number | null;
    actions?: Snippet;
    children: Snippet;
  } = $props();
</script>

<div class="page">
  <button class="back" onclick={() => (onback ? onback() : nav.back())}>
    <Icon name="chevronLeft" size={16} />{onback ? module.name : course.fullName}
  </button>
  <div class="head">
    <h1>{title ?? module.name}</h1>
    {#if actions}<div class="actions">{@render actions()}</div>{/if}
  </div>
  {#if error}<p class="notice error">{error}</p>{/if}
  {#if offlineSince}
    <p class="notice warning">{fill(i18n.t.course.offline, { date: formatDate(offlineSince) })}</p>
  {/if}
  {@render children()}
</div>

<style>
  .page {
    max-width: 960px;
    margin: 0 auto;
    padding: 20px 40px 48px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .back {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    border: 0;
    background: none;
    padding: 6px 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--accent-strong);
    cursor: pointer;
  }
  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
  }
  h1 {
    font-size: 28px;
    font-weight: 700;
  }
  .actions {
    display: flex;
    gap: 8px;
  }
  .notice {
    margin: 0;
  }
  @media (max-width: 900px) {
    .page {
      padding: 16px;
    }
  }
</style>
