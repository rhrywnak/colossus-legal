// discussPanelView.ts — the discussion panel's pure decisions.
//
// Nothing here renders. Every sentence the panel composes, and every state its
// stream can be in, is decided by a function in this file — because this project
// cannot render a component in a test, and a claim that lives only in JSX is a
// claim nothing checks (the same reason `practiceAnswerPhase` exists).
//
// Every `w("…")` call names its key LITERALLY: the wording-reach test reads this
// source for those calls and fails the build on a key the wire does not carry.

import type { ChatMessage, ChatStreamEvent, ChatThreads, ThreadRow } from "../../../services/questionChat";

export type Words = (key: string) => string;

/** Which thread is open: a person's, or the old dock's shared one. */
export type Selection = { kind: "thread"; username: string } | { kind: "earlier" };

/** Where the panel is: shut, beside the question, or full screen. */
export type DiscussMode = "closed" | "side" | "full";

/** Read the mode from the page's query string (`?discuss=side|full`). */
export function parseDiscussMode(search: string): DiscussMode {
  const value = new URLSearchParams(search).get("discuss");
  return value === "side" || value === "full" ? value : "closed";
}

/** "Chuck and Roman"; "A, B and C"; one name alone. */
export function joinNames(names: string[], and: string): string {
  if (names.length <= 1) return names.join("");
  return `${names.slice(0, -1).join(", ")} ${and} ${names[names.length - 1]}`;
}

export function rowFor(threads: ChatThreads, selection: Selection): ThreadRow | null {
  if (selection.kind !== "thread") return null;
  return threads.threads.find((t) => t.username === selection.username) ?? null;
}

/** The switcher button's own label. */
export function switcherLabel(w: Words, threads: ChatThreads, selection: Selection): string {
  if (selection.kind === "earlier") return w("chat_earlier_label");
  const row = rowFor(threads, selection);
  if (row === null || row.is_viewer) return w("chat_your_thread");
  return w("chat_thread_of_template").replace("{name}", row.display_name);
}

/** The header's line under the switcher — who can read this, or that you can't write. */
export function visibilityLine(w: Words, threads: ChatThreads, selection: Selection): string {
  if (selection.kind === "earlier") return w("chat_earlier_readonly_line");
  const row = rowFor(threads, selection);
  if (row !== null && !row.is_viewer) return w("chat_readonly_line");
  const line = w("chat_visibility_template").replace(
    "{others}",
    joinNames(threads.others, w("chat_and")),
  );
  const resumed = row?.resumed_from ?? null;
  return resumed === null ? line : `${line} · ${w("chat_resumed_template").replace("{date}", resumed)}`;
}

/** One switcher row's title line. */
export function rowTitle(w: Words, row: ThreadRow): string {
  const count = String(row.message_count);
  if (row.is_viewer) {
    if (row.message_count === 0) return w("chat_switcher_own_empty");
    if (row.message_count === 1) return w("chat_switcher_own_one");
    return w("chat_switcher_own_template").replace("{count}", count);
  }
  if (row.message_count === 0) return w("chat_switcher_other_empty").replace("{name}", row.display_name);
  if (row.message_count === 1) return w("chat_switcher_other_one").replace("{name}", row.display_name);
  return w("chat_switcher_other_template").replace("{name}", row.display_name).replace("{count}", count);
}

/** The unread badge, or `null` when there is nothing new. */
export function unreadBadge(w: Words, row: ThreadRow): string | null {
  return row.unread > 0 ? w("chat_unread_template").replace("{count}", String(row.unread)) : null;
}

/** The Earlier team discussion row's meta line. */
export function earlierMeta(w: Words, count: number, lastOn: string): string {
  if (count === 1) return w("chat_earlier_meta_one").replace("{date}", lastOn);
  return w("chat_earlier_meta_template").replace("{count}", String(count)).replace("{date}", lastOn);
}

export function chipText(w: Words, threads: ChatThreads): string {
  return w("chat_grounded_chip_template").replace("{model}", threads.model_short_name);
}

/** Whether the composer is offered. Decided by the server's `read_only`. */
export function canWrite(threads: ChatThreads, selection: Selection): boolean {
  const row = rowFor(threads, selection);
  return row !== null && !row.read_only;
}

/** The sentence for a named failure — stored on a message or raised by a send. */
export function failureSentence(w: Words, failure: string, maxTurns: number): string {
  switch (failure) {
    case "refused":
      return w("chat_refused");
    case "truncated":
      return w("chat_truncated");
    case "stalled":
      return w("chat_stalled");
    case "cap":
      return w("chat_cap_reached_template").replace("{max}", String(maxTurns));
    default:
      return w("chat_send_failed");
  }
}

/** A reply in flight, as the thread shows it until the server says done. */
export type Pending = {
  /** The prose streamed so far. */
  text: string;
  /** The model is reading the record right now. */
  reading: boolean;
  /** Set when the stream ends: the stored messages, or the failure. */
  finished: { messages: ChatMessage[]; failure: string | null } | null;
};

export const NO_PENDING: Pending = { text: "", reading: false, finished: null };

/** Fold one stream event into the pending reply. */
export function applyEvent(p: Pending, e: ChatStreamEvent): Pending {
  switch (e.event) {
    case "accepted":
      return p;
    case "delta":
      return { ...p, text: p.text + e.data.text, reading: false };
    case "tool":
      return { ...p, reading: e.data.state === "started" };
    case "done":
      return { ...p, reading: false, finished: { messages: e.data.messages, failure: null } };
    case "failed":
      return { ...p, reading: false, finished: { messages: e.data.messages, failure: e.data.failure } };
  }
}

/** The one line shown while a reply is on its way. */
export function waitingLine(w: Words, p: Pending): string | null {
  if (p.finished !== null) return null;
  if (p.reading) return w("chat_tool_line");
  return p.text === "" ? w("chat_waiting") : null;
}

/** The last message's text, for the switcher preview — `null` when empty. */
export function previewOf(messages: ChatMessage[]): string | null {
  const last = messages[messages.length - 1];
  if (last === undefined) return null;
  return last.segments.map((s) => s.text).join("") || null;
}
