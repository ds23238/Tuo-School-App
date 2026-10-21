<script lang="ts">
  import { untrack } from "svelte";
  import { ask, open } from "@tauri-apps/plugin-dialog";
  import { api, type MailCourse, type MailDraft, type Person } from "../../lib/api";
  import { errorText } from "../../lib/errors";
  import { formatSize } from "../../lib/format";
  import { i18n } from "../../lib/i18n/index.svelte";
  import Html from "../Html.svelte";
  import Icon from "../Icon.svelte";

  let {
    draft,
    courses,
    fresh,
    moodleBase,
    ondone,
  }: {
    draft: MailDraft;
    courses: MailCourse[];
    /** Gerade erst angelegt: Abbrechen löscht den Entwurf wieder. */
    fresh: boolean;
    moodleBase: string;
    ondone: (result: "sent" | "saved" | "closed") => void;
  } = $props();

  type Kind = "to" | "cc" | "bcc";
  // Die Ansicht wird je Entwurf neu erzeugt, daher genügt der Anfangswert.
  const initial = untrack(() => draft);
  let courseId = $state(initial.courseId);
  let recipients = $state<(Person & { kind: Kind })[]>([
    ...initial.to.map((p) => ({ ...p, kind: "to" as Kind })),
    ...initial.cc.map((p) => ({ ...p, kind: "cc" as Kind })),
    ...initial.bcc.map((p) => ({ ...p, kind: "bcc" as Kind })),
  ]);
  let subject = $state(initial.subject);
  let text = $state(initial.text);
  let add = $state<string[]>([]);
  let query = $state("");
  let found = $state<Person[] | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);

  // Antworten gehören zum Kurs der ursprünglichen Mail.
  const courseFixed = initial.references.length > 0;

  let timer: ReturnType<typeof setTimeout> | undefined;
  function search() {
    clearTimeout(timer);
    const q = query.trim();
    if (!q) {
      found = null;
      return;
    }
    timer = setTimeout(async () => {
      try {
        const people = await api.mail.searchUsers(courseId, q);
        found = people.filter((p) => !recipients.some((r) => r.id === p.id));
      } catch (e) {
        error = errorText(e);
      }
    }, 250);
  }

  function pick(p: Person) {
    recipients.push({ ...p, kind: "to" });
    query = "";
    found = null;
  }

  async function attach() {
    const picked = await open({ multiple: true, directory: false });
    if (Array.isArray(picked)) add = [...add, ...picked.filter((p) => !add.includes(p))];
  }

  function fileName(path: string): string {
    return path.split(/[\\/]/).pop() ?? path;
  }

  async function save(send: boolean) {
    if (send && recipients.length === 0) {
      error = i18n.t.mail.needRecipient;
      return;
    }
    if (send && !subject.trim()) {
      error = i18n.t.mail.needSubject;
      return;
    }
    busy = true;
    error = null;
    const ids = (kind: Kind) => recipients.filter((r) => r.kind === kind).map((r) => r.id);
    try {
      await api.mail.save(
        draft.id,
        {
          courseId,
          to: ids("to"),
          cc: ids("cc"),
          bcc: ids("bcc"),
          subject,
          text,
          quote: draft.quote,
          add,
        },
        send,
      );
      ondone(send ? "sent" : "saved");
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }

  async function close() {
    if (fresh) {
      if (text.trim() && !(await ask(i18n.t.mail.discardConfirm, { kind: "warning" }))) return;
      await api.mail.setDeleted(draft.id, 2).catch(() => {});
    }
    ondone("closed");
  }

  async function discard() {
    if (!(await ask(i18n.t.mail.discardConfirm, { kind: "warning" }))) return;
    await api.mail.setDeleted(draft.id, 2).catch(() => {});
    ondone("closed");
  }
</script>

<form class="compose" onsubmit={(e) => (e.preventDefault(), save(true))}>
  <div class="bar">
    <h2>{subject || i18n.t.mail.compose}</h2>
    <button type="button" class="btn ghost small" onclick={close} aria-label={i18n.t.assign.cancel}>
      <Icon name="x" size={16} />
    </button>
  </div>

  <label class="field">
    <span>{i18n.t.mail.course}</span>
    <select bind:value={courseId} disabled={courseFixed || busy}>
      {#each courses as c (c.id)}
        <option value={c.id}>{c.fullName}</option>
      {/each}
    </select>
  </label>

  <div class="field">
    <span>{i18n.t.mail.to}</span>
    <div class="recipients">
      {#each recipients as r, i (r.id)}
        <span class="chip">
          <select bind:value={r.kind} aria-label={r.name}>
            <option value="to">{i18n.t.mail.to}</option>
            <option value="cc">{i18n.t.mail.cc}</option>
            <option value="bcc">{i18n.t.mail.bcc}</option>
          </select>
          {r.name}
          <button type="button" onclick={() => recipients.splice(i, 1)} aria-label={i18n.t.assign.remove}>
            <Icon name="x" size={12} />
          </button>
        </span>
      {/each}
      <div class="picker">
        <input
          type="text"
          bind:value={query}
          oninput={search}
          placeholder={i18n.t.mail.addRecipient}
          aria-label={i18n.t.mail.addRecipient}
        />
        {#if found}
          <div class="found">
            {#if found.length === 0}<p class="muted small">{i18n.t.messages.noPeople}</p>{/if}
            {#each found as p (p.id)}
              <button type="button" onclick={() => pick(p)}>{p.name}</button>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  </div>

  <label class="field">
    <span>{i18n.t.mail.subject}</span>
    <input type="text" bind:value={subject} disabled={busy} />
  </label>

  <textarea bind:value={text} rows="10" aria-label={i18n.t.mail.message} disabled={busy}></textarea>

  {#if draft.quoteHtml}
    <details class="quote" open>
      <summary>{i18n.t.mail.quoted}</summary>
      <Html html={draft.quoteHtml} {moodleBase} />
    </details>
  {/if}
  {#each draft.references as ref (ref.id)}
    <details class="quote">
      <summary>{i18n.t.mail.earlier}: {ref.sender.name} – {ref.subject}</summary>
      <Html html={ref.html} {moodleBase} />
    </details>
  {/each}

  <div class="files">
    {#each draft.attachments as a (a.name)}
      <span class="file"><Icon name="paperclip" size={14} /> {a.name} <span class="muted">{formatSize(a.size)}</span></span>
    {/each}
    {#each add as path (path)}
      <span class="file">
        <Icon name="paperclip" size={14} />
        {fileName(path)}
        <button type="button" onclick={() => (add = add.filter((p) => p !== path))} aria-label={i18n.t.assign.remove}>
          <Icon name="x" size={12} />
        </button>
      </span>
    {/each}
  </div>

  {#if error}<p class="notice error">{error}</p>{/if}

  <div class="actions">
    <button type="submit" class="btn primary" disabled={busy}>
      <Icon name="send" size={16} />
      {busy ? i18n.t.mail.sending : i18n.t.mail.send}
    </button>
    <button type="button" class="btn" onclick={() => save(false)} disabled={busy}>{i18n.t.mail.save}</button>
    <button type="button" class="btn ghost" onclick={attach} disabled={busy}>
      <Icon name="paperclip" size={16} />
      {i18n.t.mail.addFiles}
    </button>
    <span class="grow"></span>
    {#if !fresh}
      <button type="button" class="btn ghost" onclick={discard} disabled={busy}>
        <Icon name="trash" size={16} />
        {i18n.t.mail.discard}
      </button>
    {/if}
  </div>
</form>

<style>
  .compose {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 20px;
    overflow-y: auto;
  }
  .bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }
  h2 {
    font-size: 18px;
  }
  .field {
    display: grid;
    grid-template-columns: 70px 1fr;
    align-items: center;
    gap: 8px;
  }
  .field > span {
    font-size: 13px;
    color: var(--muted);
  }
  .recipients {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 4px 2px 2px;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--accent-ink);
    font-size: 13px;
  }
  .chip select {
    border: 0;
    background: var(--surface);
    border-radius: 999px;
    padding: 2px 8px;
    font-size: 12px;
  }
  .chip button,
  .file button {
    display: inline-flex;
    border: 0;
    background: none;
    color: inherit;
    cursor: pointer;
    padding: 2px;
  }
  .picker {
    position: relative;
    flex: 1;
    min-width: 180px;
  }
  .picker input {
    width: 100%;
  }
  .found {
    position: absolute;
    z-index: 5;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    max-height: 240px;
    overflow-y: auto;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: 10px;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.12);
    padding: 4px;
  }
  .found p {
    margin: 6px 8px;
  }
  .found button {
    display: block;
    width: 100%;
    text-align: left;
    border: 0;
    background: none;
    color: inherit;
    padding: 8px 10px;
    border-radius: 6px;
    cursor: pointer;
  }
  .found button:hover {
    background: var(--hover);
  }
  textarea {
    resize: vertical;
    min-height: 160px;
  }
  .quote {
    border-left: 3px solid var(--border-strong);
    padding-left: 12px;
    color: var(--text-2);
  }
  .quote summary {
    cursor: pointer;
    font-size: 13px;
    color: var(--muted);
    margin-bottom: 6px;
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
    padding: 4px 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 13px;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }
  .grow {
    flex: 1;
  }
  .notice {
    margin: 0;
  }
</style>
