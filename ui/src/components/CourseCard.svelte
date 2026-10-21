<script lang="ts">
  import type { Course, CourseFiles } from "../lib/api";
  import { fill } from "../lib/format";
  import { i18n } from "../lib/i18n/index.svelte";

  let {
    course,
    files,
    onopen,
    ontoggle,
  }: {
    course: Course;
    files: CourseFiles | undefined;
    onopen: () => void;
    ontoggle: () => void;
  } = $props();

  const tint = $derived(course.id % 6);
  const initials = $derived(
    course.shortName
      .split(/\s+/)
      .filter(Boolean)
      .slice(0, 2)
      .map((w) => w.slice(0, w.length > 3 ? 2 : 3))
      .join(" ")
      .slice(0, 6),
  );
  const status = $derived.by(() => {
    if (!course.sync) return i18n.t.courses.notSynced;
    if (!files || files.total === 0) return i18n.t.courses.noFiles;
    if (files.updates > 0) return fill(i18n.t.courses.updates, { n: files.updates });
    return fill(i18n.t.courses.files, { n: files.downloaded });
  });
</script>

<article class="card">
  <button class="open" onclick={onopen}>
    <div class="banner tint-{tint}">
      {#if course.image}
        <img src={course.image} alt="" />
      {:else}
        <span class="shape a"></span>
        <span class="shape b"></span>
        <span class="initials">{initials}</span>
      {/if}
    </div>
    <div class="body">
      <div class="title">{course.fullName}</div>
      <div class="muted small meta">
        {#if course.shortName !== course.fullName}<span>{course.shortName}</span>{/if}
        {#if !course.enrolled}<span class="pill">{i18n.t.courses.notEnrolled}</span>{/if}
        {#if !course.visible}<span class="pill">{i18n.t.courses.hidden}</span>{/if}
      </div>
      {#if course.progress !== null}
        <div class="progress">
          <div class="bar"><div style:width="{Math.round(course.progress)}%"></div></div>
          <span class="muted">{Math.round(course.progress)} %</span>
        </div>
      {/if}
    </div>
  </button>
  <div class="foot">
    <span class="small" class:muted={!files?.updates} class:accent={course.sync && !!files?.updates}>
      {status}
    </span>
    <label class="small">
      {i18n.t.courses.sync}
      <button
        class="switch"
        role="switch"
        aria-checked={course.sync}
        aria-label={fill(i18n.t.courses.syncAria, { name: course.fullName })}
        onclick={ontoggle}
      ></button>
    </label>
  </div>
</article>

<style>
  article {
    content-visibility: auto;
    contain-intrinsic-size: auto 260px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .open {
    display: block;
    border: 0;
    padding: 0;
    background: none;
    text-align: left;
    cursor: pointer;
  }
  .open:hover .title {
    color: var(--accent-strong);
  }
  .banner {
    position: relative;
    height: 108px;
    overflow: hidden;
    background: var(--t-bg);
  }
  .banner img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .shape {
    position: absolute;
    background: var(--t-shape);
  }
  .shape.a {
    width: 110px;
    height: 110px;
    border-radius: 50%;
    right: -18px;
    top: -34px;
  }
  .shape.b {
    width: 86px;
    height: 86px;
    border-radius: 18px;
    right: 90px;
    bottom: -46px;
    transform: rotate(18deg);
  }
  .initials {
    position: absolute;
    left: 18px;
    bottom: 12px;
    font-size: 24px;
    font-weight: 700;
    color: var(--t-ink);
  }
  .tint-0 {
    --t-bg: #e3f1ec;
    --t-shape: #cfe7de;
    --t-ink: #1f5e4a;
  }
  .tint-1 {
    --t-bg: #e6eefb;
    --t-shape: #d3e1f7;
    --t-ink: #264a86;
  }
  .tint-2 {
    --t-bg: #fbebdd;
    --t-shape: #f6dac2;
    --t-ink: #8a3d08;
  }
  .tint-3 {
    --t-bg: #f1e8f8;
    --t-shape: #e4d5f2;
    --t-ink: #5b2e86;
  }
  .tint-4 {
    --t-bg: #e4f2f6;
    --t-shape: #cfe6ee;
    --t-ink: #1e5b6e;
  }
  .tint-5 {
    --t-bg: #f6efe2;
    --t-shape: #ece0c8;
    --t-ink: #6b4f1d;
  }
  @media (prefers-color-scheme: dark) {
    .banner {
      filter: brightness(0.82) saturate(0.9);
    }
  }
  .body {
    padding: 14px 18px 10px;
  }
  .title {
    font-weight: 600;
    font-size: 16px;
    line-height: 1.3;
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
    margin-top: 4px;
    min-height: 20px;
  }
  .progress {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 12px;
    font-size: 12px;
  }
  .bar {
    flex: 1;
    height: 6px;
    border-radius: 999px;
    background: var(--surface-3);
    overflow: hidden;
  }
  .bar div {
    height: 100%;
    border-radius: 999px;
    background: var(--accent);
  }
  .foot {
    margin-top: auto;
    padding: 10px 18px 14px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    border-top: 1px solid var(--border);
  }
  .accent {
    color: var(--accent-ink);
    font-weight: 600;
  }
  label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 500;
  }
</style>
