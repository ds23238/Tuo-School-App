<script lang="ts">
  import Html from "../components/Html.svelte";
  import Icon from "../components/Icon.svelte";
  import { api, type Course, type Module, type PageContent } from "../lib/api";
  import { errorText } from "../lib/errors";
  import { fill, formatDate } from "../lib/format";
  import { i18n } from "../lib/i18n/index.svelte";
  import { nav } from "../lib/nav.svelte";

  let { course, module }: { course: Course; module: Module } = $props();

  let page = $state<PageContent | null>(null);
  let error = $state<string | null>(null);
  let base = $state<string | null>(null);

  api.listAccounts().then((a) => (base = a.find((x) => x.service === "moodle")?.baseUrl ?? null));
  async function load() {
    try {
      page = await api.moodle.page(course.id, module.id);
    } catch (e) {
      error = errorText(e);
    }
  }
  load();

  function jump(i: number) {
    document.getElementById(`chapter-${i}`)?.scrollIntoView({ behavior: "smooth" });
  }
</script>

<div class="page">
  <button class="back" onclick={() => nav.back()}>
    <Icon name="chevronLeft" size={16} />{course.fullName}
  </button>
  <h1>{module.name}</h1>
  {#if error}<p class="notice error">{error}</p>{/if}
  {#if page?.offline}
    <p class="notice warning">{fill(i18n.t.course.offline, { date: formatDate(page.fetchedAt) })}</p>
  {/if}

  {#if page}
    <div class="layout">
      {#if page.chapters.length > 1}
        <aside class="card" aria-label={i18n.t.page.contents}>
          <div class="index-title">{i18n.t.page.contents}</div>
          {#each page.chapters as chapter, i (i)}
            <button class="idx" style:padding-left="{10 + chapter.level * 14}px" onclick={() => jump(i)}>
              {chapter.title}
            </button>
          {/each}
        </aside>
      {/if}
      <article class="card">
        {#each page.chapters as chapter, i (i)}
          <section id="chapter-{i}">
            {#if page.chapters.length > 1}<h2 class:sub={chapter.level > 0}>{chapter.title}</h2>{/if}
            <Html html={chapter.html} moodleBase={base} />
          </section>
        {/each}
      </article>
    </div>
  {:else if !error}
    <p class="muted">{i18n.t.page.loading}</p>
  {/if}
</div>

<style>
  .page {
    max-width: 1160px;
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
  h1 {
    font-size: 28px;
    font-weight: 700;
  }
  .layout {
    display: flex;
    gap: 24px;
    align-items: flex-start;
  }
  aside {
    width: 240px;
    flex: none;
    padding: 12px 8px;
    position: sticky;
    top: 16px;
  }
  .index-title {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--muted);
    padding: 4px 10px 8px;
  }
  .idx {
    display: block;
    width: 100%;
    padding: 7px 10px;
    border: 0;
    border-radius: 8px;
    background: none;
    font-size: 13px;
    color: var(--text-2);
    text-align: left;
    cursor: pointer;
  }
  .idx:hover {
    background: var(--hover);
  }
  article {
    flex: 1;
    min-width: 0;
    padding: 28px 32px;
    font-size: 15px;
  }
  section + section {
    margin-top: 32px;
    padding-top: 24px;
    border-top: 1px solid var(--border);
  }
  section {
    scroll-margin-top: 16px;
  }
  h2 {
    font-size: 20px;
    margin-bottom: 12px;
  }
  h2.sub {
    font-size: 17px;
  }
  .notice {
    margin: 0;
  }
  @media (max-width: 900px) {
    aside {
      display: none;
    }
    .page {
      padding: 16px;
    }
  }
</style>
