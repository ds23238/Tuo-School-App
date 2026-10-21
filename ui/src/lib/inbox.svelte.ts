import { listen } from "@tauri-apps/api/event";
import { api, type Counts } from "./api";

/** Ungelesene Mitteilungen; das Backend zählt jede Minute neu. */
class InboxState {
  chats = $state(0);
  notifications = $state(0);
  mail = $state<number | null>(null);
  untis = $state<number | null>(null);
  /** Erhöht sich bei jeder neuen Zählung, damit offene Listen nachladen. */
  generation = $state(0);

  get total(): number {
    return this.chats + this.notifications + (this.mail ?? 0) + (this.untis ?? 0);
  }

  constructor() {
    api.messages.counts().then((c) => this.apply(c), () => {});
    listen<Counts>("message-counts", (e) => this.apply(e.payload));
  }

  private apply(c: Counts) {
    const changed =
      c.chats !== this.chats ||
      c.notifications !== this.notifications ||
      c.mail !== this.mail ||
      c.untis !== this.untis;
    this.chats = c.chats;
    this.notifications = c.notifications;
    this.mail = c.mail;
    this.untis = c.untis;
    if (changed) this.generation++;
  }

  refresh() {
    api.messages.refreshCounts().then((c) => this.apply(c), () => {});
  }
}

export const inbox = new InboxState();
