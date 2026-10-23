<script lang="ts">
  import { untrack } from "svelte";
  import { ask, open } from "@tauri-apps/plugin-dialog";
  import { api, type UntisCompose, type UntisRecipient } from "../../lib/api";
  import { errorText } from "../../lib/errors";
  import { fill } from "../../lib/format";
  import { i18n } from "../../lib/i18n/index.svelte";
  import Icon from "../Icon.svelte";

  let {
    compose,
    to = [],
    subject: initialSubject = "",
    ondone,
  }: {
    compose: UntisCompose;
    to?: number[];
    subject?: string;
    ondone: (sent: boolean) => void;
  } = $props();

  // Die Ansicht wird je Mitteilung neu erzeugt, daher genügt der Anfangswert.
  let recipients = $state<UntisRecipient[]>(
    untrack(() => compose.recipients.filter((r) => to.includes(r.id))),
  );
  let subject = $state(untrack(() => initialSubject));
  let text = $state("");
  let files = $state<string[]>([]);
  let query = $state("");
  let busy = $state(false);
  let error = $state<string | null>(null);

  const found = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return null;
    return compose.recipients
      .filter((r) => !recipients.some((p) => p.id === r.id))
      .filter((r) => r.name.toLowerCase().includes(q) || r.tags.some((t) => t.toLowerCase().includes(q)))
      .slice(0, 30);
  });

  function pick(r: UntisRecipient) {
    recipients.push(r);
    query = "";
  }

  async function attach() {
    const picked = await open({ multiple: true, directory: false });
    if (Array.isArray(picked)) files = [...files, ...picked.filter((p) => !files.includes(p))];
  }

  const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;

  async function send(e: SubmitEvent) {
    e.preventDefault();
    if (recipients.length === 0) return (error = i18n.t.mail.needRecipient);
    if (!subject.trim()) return (error = i18n.t.mail.needSubject);
    if (compose.maxFileCount != null && files.length > compose.maxFileCount)
      return (error = fill(i18n.t.untis.tooMany, { count: String(compose.maxFileCount) }));
    busy = true;
    error = null;
    try {
      await api.untis.send(
        { subject: subject.trim(), content: text, recipients: recipients.map((r) => r.id) },
        files,
      );
      ondone(true);
    } catch (err) {
      error = errorText(err);
    } finally {
      busy = false;
    }
  }

  async function close() {
    if (text.trim() && !(await ask(i18n.t.mail.discardConfirm, { kind: "warning" }))) return;
    ondone(false);
  }
</script>

<form class="compose" onsubmit={send}>
  <div class="bar">
    <h2>{subject || i18n.t.untis.compose}</h2>
    <button type="button" class="btn ghost small" onclick={close} aria-label={i18n.t.assign.cancel}>
      <Icon name="x" size={16} />
    </button>
  </div>

  <div class="field">
    <span>{i18n.t.untis.toLabel}</span>
    <div class="recipients">
      {#each recipients as r, i (r.id)}
        <span class="chip">
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
          placeholder={i18n.t.untis.addRecipient}
          aria-label={i18n.t.untis.addRecipient}
        />
        {#if found}
          <div class="found">
            {#if found.length === 0}<p class="muted small">{i18n.t.untis.noRecipients}</p>{/if}
            {#each found as r (r.id)}
              <button type="button" onclick={() => pick(r)}>
                {r.name}
                {#if r.tags.length}<span class="muted small">{r.tags.join(", ")}</span>{/if}
              </button>
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

  {#if files.length}
    <div class="files">
      {#each files as path (path)}
        <span class="file">
          <Icon name="paperclip" size={14} />
          {fileName(path)}
          <button type="button" onclick={() => (files = files.filter((p) => p !== path))} aria-label={i18n.t.assign.remove}>
            <Icon name="x" size={12} />
          </button>
        </span>
      {/each}
    </div>
  {/if}

  {#if error}<p class="notice error">{error}</p>{/if}

  <div class="actions">
    <button type="submit" class="btn primary" disabled={busy}>
      <Icon name="send" size={16} />
      {busy ? i18n.t.mail.sending : i18n.t.mail.send}
    </button>
    {#if compose.maxFileCount !== 0}
      <button type="button" class="btn ghost" onclick={attach} disabled={busy}>
        <Icon name="paperclip" size={16} />
        {i18n.t.mail.addFiles}
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
    padding: 2px 4px 2px 10px;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--accent-ink);
    font-size: 13px;
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
    display: flex;
    justify-content: space-between;
    gap: 8px;
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
    gap: 8px;
    align-items: center;
  }
  .notice {
    margin: 0;
  }
</style>
