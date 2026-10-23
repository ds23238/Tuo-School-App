<script lang="ts">
  import ActivityPage from "../components/ActivityPage.svelte";
  import Avatar from "../components/Avatar.svelte";
  import Html from "../components/Html.svelte";
  import Icon from "../components/Icon.svelte";
  import { api, type Cached, type Course, type Discussion, type Forum, type ForumPost, type Module, type Thread } from "../lib/api";
  import { errorText } from "../lib/errors";
  import { fill, formatDate, formatShort, formatSize } from "../lib/format";
  import { i18n } from "../lib/i18n/index.svelte";

  let { course, module }: { course: Course; module: Module } = $props();

  let forum = $state<Cached<Forum> | null>(null);
  let thread = $state<Cached<Thread> | null>(null);
  let error = $state<string | null>(null);
  let base = $state<string | null>(null);
  let busy = $state(false);

  // Neues Thema bzw. Antwort auf einen Beitrag.
  let adding = $state(false);
  let draft = $state({ subject: "", text: "" });
  let replyTo = $state<number | null>(null);
  let reply = $state("");

  api.listAccounts().then((a) => (base = a.find((x) => x.service === "moodle")?.baseUrl ?? null));

  async function loadForum() {
    try {
      forum = await api.moodle.forum(course.id, module.id);
    } catch (e) {
      error = errorText(e);
    }
  }
  loadForum();

  async function open(id: number) {
    error = null;
    try {
      thread = await api.moodle.forumThread(id);
      replyTo = null;
      reply = "";
    } catch (e) {
      error = errorText(e);
    }
  }

  function close() {
    thread = null;
    loadForum();
  }

  /** Beiträge als Baum in Lesereihenfolge, mit Einrückung. */
  const tree = $derived.by(() => {
    const posts = thread?.data.posts ?? [];
    const children = new Map<number | null, ForumPost[]>();
    const ids = new Set(posts.map((p) => p.id));
    for (const p of posts) {
      const parent = p.parent !== null && ids.has(p.parent) ? p.parent : null;
      children.set(parent, [...(children.get(parent) ?? []), p]);
    }
    const out: { post: ForumPost; depth: number }[] = [];
    const walk = (parent: number | null, depth: number) => {
      for (const p of children.get(parent) ?? []) {
        out.push({ post: p, depth });
        walk(p.id, depth + 1);
      }
    };
    walk(null, 0);
    return out;
  });

  async function addDiscussion(e: SubmitEvent) {
    e.preventDefault();
    if (!draft.subject.trim() || !draft.text.trim()) return;
    busy = true;
    error = null;
    try {
      const id = await api.moodle.forumAdd(course.id, module.id, draft.subject.trim(), draft.text);
      adding = false;
      draft = { subject: "", text: "" };
      await open(id);
    } catch (err) {
      error = errorText(err);
    } finally {
      busy = false;
    }
  }

  async function sendReply(post: ForumPost) {
    if (!reply.trim() || !thread) return;
    busy = true;
    error = null;
    try {
      const subject = /^re:/i.test(post.subject) ? post.subject : `Re: ${post.subject}`;
      await api.moodle.forumReply(post.id, subject, reply);
      await open(thread.data.id);
    } catch (err) {
      error = errorText(err);
    } finally {
      busy = false;
    }
  }

  function attachment(post: ForumPost, name: string) {
    if (!thread) return;
    api.moodle.forumAttachment(thread.data.id, post.id, name).catch((e) => (error = errorText(e)));
  }

  const sorted = (list: Discussion[]) =>
    [...list].sort((a, b) => Number(b.pinned) - Number(a.pinned) || b.modified - a.modified);
</script>

{#if thread}
  <ActivityPage
    {course}
    {module}
    title={thread.data.name}
    onback={close}
    {error}
    offlineSince={thread.offline ? thread.fetchedAt : null}
  >
    <div class="posts">
      {#each tree as { post, depth } (post.id)}
        <article class="card post" class:unread={post.unread} style:margin-left="{Math.min(depth, 4) * 28}px">
          <header>
            <Avatar name={post.author} size={36} />
            <div>
              <strong>{post.author}</strong>
              <span class="muted small">{formatDate(post.created)}</span>
            </div>
          </header>
          {#if depth > 0 && post.subject && post.subject !== thread.data.name && post.subject !== `Re: ${thread.data.name}`}
            <h3>{post.subject}</h3>
          {/if}
          <Html html={post.html} moodleBase={base} />
          {#if post.attachments.length}
            <div class="files">
              {#each post.attachments as a (a.name)}
                <button class="file" onclick={() => attachment(post, a.name)}>
                  <Icon name="paperclip" size={14} />{a.name}
                  <span class="muted">{formatSize(a.size)}</span>
                </button>
              {/each}
            </div>
          {/if}
          {#if post.canReply && !thread.offline}
            {#if replyTo === post.id}
              <div class="reply">
                <textarea bind:value={reply} rows="4" placeholder={i18n.t.forum.replyPlaceholder} disabled={busy}></textarea>
                <div class="row">
                  <button class="btn primary small" onclick={() => sendReply(post)} disabled={busy || !reply.trim()}>
                    <Icon name="send" size={14} />{i18n.t.forum.send}
                  </button>
                  <button class="btn ghost small" onclick={() => (replyTo = null)} disabled={busy}>{i18n.t.assign.cancel}</button>
                </div>
              </div>
            {:else}
              <button class="btn ghost small replybtn" onclick={() => ((replyTo = post.id), (reply = ""))}>
                <Icon name="reply" size={14} />{i18n.t.forum.reply}
              </button>
            {/if}
          {/if}
        </article>
      {/each}
    </div>
  </ActivityPage>
{:else}
  <ActivityPage {course} {module} {error} offlineSince={forum?.offline ? forum.fetchedAt : null}>
    {#snippet actions()}
      {#if forum?.data.canPost && !forum.offline}
        <button class="btn primary small" onclick={() => (adding = !adding)}>
          <Icon name={adding ? "x" : "plus"} size={14} />{i18n.t.forum.newDiscussion}
        </button>
      {/if}
    {/snippet}
    {#if module.description}
      <div class="intro"><Html html={module.description} moodleBase={base} /></div>
    {/if}
    {#if adding}
      <form class="card compose" onsubmit={addDiscussion}>
        <input type="text" bind:value={draft.subject} placeholder={i18n.t.forum.subject} required disabled={busy} />
        <textarea bind:value={draft.text} rows="6" placeholder={i18n.t.forum.message} required disabled={busy}></textarea>
        <div class="row">
          <button class="btn primary small" type="submit" disabled={busy}>
            <Icon name="send" size={14} />{i18n.t.forum.post}
          </button>
        </div>
      </form>
    {/if}
    {#if forum && forum.data.discussions.length === 0}
      <p class="muted">{i18n.t.forum.empty}</p>
    {:else if !forum && !error}
      <p class="muted">{i18n.t.page.loading}</p>
    {/if}
    <div class="list">
      {#each sorted(forum?.data.discussions ?? []) as d (d.id)}
        <button class="card discussion" class:unread={d.unread > 0} onclick={() => open(d.id)}>
          <Avatar name={d.author} size={38} />
          <span class="text">
            <span class="line">
              <span class="name">
                {#if d.pinned}<Icon name="star" size={14} label={i18n.t.forum.pinned} />{/if}
                {#if d.locked}<Icon name="lock" size={14} label={i18n.t.forum.locked} />{/if}
                {d.name}
              </span>
              <span class="muted small">{formatShort(d.modified)}</span>
            </span>
            <span class="muted small">{d.author} · {d.replies === 1 ? i18n.t.forum.reply1 : fill(i18n.t.forum.replies, { n: d.replies })}{#if d.unread > 0} · <strong class="new">{fill(i18n.t.forum.unread, { n: d.unread })}</strong>{/if}</span>
            {#if d.preview}<span class="preview">{d.preview}</span>{/if}
          </span>
        </button>
      {/each}
    </div>
  </ActivityPage>
{/if}

<style>
  .intro {
    color: var(--text-2);
  }
  .list,
  .posts {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .discussion {
    display: flex;
    gap: 14px;
    align-items: flex-start;
    padding: 14px 16px;
    text-align: left;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .discussion:hover {
    border-color: var(--accent);
  }
  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .line {
    display: flex;
    justify-content: space-between;
    gap: 12px;
  }
  .name {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
  }
  .unread .name {
    font-weight: 800;
  }
  .new {
    color: var(--accent-ink);
  }
  .preview {
    font-size: 14px;
    color: var(--text-2);
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }
  .post {
    padding: 16px 18px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .post.unread {
    border-left: 3px solid var(--accent);
  }
  .post header {
    display: flex;
    gap: 10px;
    align-items: center;
  }
  .post header div {
    display: flex;
    flex-direction: column;
  }
  h3 {
    font-size: 15px;
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
  .file:hover {
    background: var(--hover);
  }
  .replybtn {
    align-self: flex-start;
  }
  .reply,
  .compose {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .compose {
    padding: 16px;
  }
  .row {
    display: flex;
    gap: 8px;
  }
  textarea {
    resize: vertical;
  }
</style>
