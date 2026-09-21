// questionChat.ts — the question chat's calls (CC_TASK_CHAT_ENGINE_v1).
//
// Four reads and one streamed send. Every failure THROWS a `ChatCallError`
// carrying what kind of failure it was, so the panel picks the right sentence
// without matching message text.
//
// ## Why the send has an IDLE timeout, not a total one
//
// A grounded reply over the whole case record can legitimately take minutes to
// finish; a total timeout would cut off healthy answers (the backend learned this
// on 2026-08-28). What a broken stream looks like is SILENCE, so the abort timer
// is re-armed on every chunk that arrives — keep-alive comments included — and
// fires only when nothing has arrived for `idleTimeoutMs`. The value comes from
// the server (`question_chat_client_idle_timeout_secs`), never from this file.
//
// ## Checked BY EYE against `backend/src/dto/chat_discussion.rs`
//
// Hand-written types, like every practice service; the parsers below refuse a
// payload missing a field the panel renders.

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import { readErrorMessage } from "./fetchUtils";
import { PRACTICE_TIMEOUT_MS } from "./practice";

export type CitationCard = {
  document_title: string;
  document_date: string | null;
  page: number | null;
  quoted_text: string;
};

export type Segment = { text: string; cards: CitationCard[] };

export type ChatMessage = {
  seq: number;
  role: "user" | "assistant";
  author_name: string;
  segments: Segment[];
  at: string;
  failure: string | null;
};

export type ThreadRow = {
  username: string;
  display_name: string;
  initial: string;
  is_viewer: boolean;
  read_only: boolean;
  message_count: number;
  unread: number;
  preview: string | null;
  resumed_from: string | null;
};

export type EarlierRow = { message_count: number; last_on: string; preview: string };

export type ChatThreads = {
  question_id: string;
  viewer: string;
  model_short_name: string;
  grounded: boolean;
  others: string[];
  threads: ThreadRow[];
  earlier: EarlierRow | null;
  client_idle_timeout_secs: number;
  max_turns: number;
};

export type ChatThread = {
  username: string | null;
  display_name: string;
  read_only: boolean;
  messages: ChatMessage[];
};

/** What went wrong: an HTTP refusal, a silent stream, or the network. */
export type ChatFailureKind = "http" | "stalled" | "network";

export class ChatCallError extends Error {
  constructor(
    message: string,
    public readonly kind: ChatFailureKind,
    public readonly status: number | null,
  ) {
    super(message);
    this.name = "ChatCallError";
  }
}

/** One server-sent event from the send. */
export type ChatStreamEvent =
  | { event: "accepted"; data: { seq: number } }
  | { event: "delta"; data: { text: string } }
  | { event: "tool"; data: { state: "started" | "finished" } }
  | { event: "done"; data: { messages: ChatMessage[] } }
  | { event: "failed"; data: { failure: string; detail: string; messages: ChatMessage[] } };

const base = (questionId: string) =>
  `${API_BASE_URL}/api/chat/question/${encodeURIComponent(questionId)}`;
const threadPath = (questionId: string, username: string) =>
  `${base(questionId)}/threads/${encodeURIComponent(username)}`;

async function getJson<T>(url: string, check: (v: unknown) => T): Promise<T> {
  let response: Response;
  try {
    response = await authFetch(url, { timeoutMs: PRACTICE_TIMEOUT_MS });
  } catch (cause) {
    throw new ChatCallError(cause instanceof Error ? cause.message : String(cause), "network", null);
  }
  if (!response.ok) {
    throw new ChatCallError(await readErrorMessage(response), "http", response.status);
  }
  return check(await response.json());
}

function need(value: unknown, fields: string[], what: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null) {
    throw new ChatCallError(`${what}: the server sent no object`, "http", null);
  }
  const obj = value as Record<string, unknown>;
  for (const f of fields) {
    if (!(f in obj)) throw new ChatCallError(`${what}: the server omitted "${f}"`, "http", null);
  }
  return obj;
}

export const asThreads = (v: unknown): ChatThreads =>
  need(v, ["viewer", "model_short_name", "grounded", "others", "threads", "earlier", "client_idle_timeout_secs"], "chat threads") as unknown as ChatThreads;

export const asThread = (v: unknown): ChatThread =>
  need(v, ["display_name", "read_only", "messages"], "chat thread") as unknown as ChatThread;

export function fetchChatThreads(questionId: string): Promise<ChatThreads> {
  return getJson(`${base(questionId)}/threads`, asThreads);
}

export function fetchChatThread(questionId: string, username: string): Promise<ChatThread> {
  return getJson(threadPath(questionId, username), asThread);
}

export function fetchEarlierDiscussion(questionId: string): Promise<ChatThread> {
  return getJson(`${base(questionId)}/earlier`, asThread);
}

/** Move the viewer's read mark on `username`'s thread to `seq`. */
export async function markChatRead(questionId: string, username: string, seq: number): Promise<void> {
  let response: Response;
  try {
    response = await authFetch(`${threadPath(questionId, username)}/read`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ seq }),
      timeoutMs: PRACTICE_TIMEOUT_MS,
    });
  } catch (cause) {
    throw new ChatCallError(cause instanceof Error ? cause.message : String(cause), "network", null);
  }
  if (!response.ok) {
    throw new ChatCallError(await readErrorMessage(response), "http", response.status);
  }
}

/**
 * Incremental `text/event-stream` parser: chunks in, whole events out.
 * Pure, so the framing rules are testable without a socket.
 */
export class SseParser {
  private buffer = "";

  push(chunk: string): ChatStreamEvent[] {
    this.buffer += chunk.replace(/\r\n/g, "\n");
    const out: ChatStreamEvent[] = [];
    let at = this.buffer.indexOf("\n\n");
    while (at !== -1) {
      const block = this.buffer.slice(0, at);
      this.buffer = this.buffer.slice(at + 2);
      const parsed = parseBlock(block);
      if (parsed !== null) out.push(parsed);
      at = this.buffer.indexOf("\n\n");
    }
    return out;
  }
}

function parseBlock(block: string): ChatStreamEvent | null {
  let name = "";
  const data: string[] = [];
  for (const line of block.split("\n")) {
    if (line.startsWith("event:")) name = line.slice(6).trim();
    else if (line.startsWith("data:")) data.push(line.slice(5).replace(/^ /, ""));
    // `:` comments (keep-alive) and other fields carry nothing.
  }
  if (name === "" || data.length === 0) return null;
  try {
    return { event: name, data: JSON.parse(data.join("\n")) } as ChatStreamEvent;
  } catch (cause) {
    // A malformed event is said out loud and dropped; the stream's `done` or
    // `failed` still settles the send.
    console.error("question chat: an event could not be parsed", name, cause);
    return null;
  }
}

/**
 * Send one message; call `onEvent` for every event as it arrives. Resolves when
 * the stream ends. Throws `ChatCallError` for an HTTP refusal (with its status),
 * a stream silent for `idleTimeoutMs` ("stalled"), or a network failure.
 */
export async function sendChatMessage(
  questionId: string,
  username: string,
  text: string,
  idleTimeoutMs: number,
  onEvent: (event: ChatStreamEvent) => void,
): Promise<void> {
  const controller = new AbortController();
  let timer = window.setTimeout(() => controller.abort(), idleTimeoutMs);
  const rearm = () => {
    window.clearTimeout(timer);
    timer = window.setTimeout(() => controller.abort(), idleTimeoutMs);
  };
  try {
    const response = await authFetch(`${threadPath(questionId, username)}/messages`, {
      method: "POST",
      headers: { "Content-Type": "application/json", Accept: "text/event-stream" },
      body: JSON.stringify({ text }),
      signal: controller.signal,
    });
    if (!response.ok) {
      throw new ChatCallError(await readErrorMessage(response), "http", response.status);
    }
    if (response.body === null) {
      throw new ChatCallError("the server sent no reply stream", "network", response.status);
    }
    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    const parser = new SseParser();
    for (;;) {
      const { value, done } = await reader.read();
      if (done) break;
      rearm();
      for (const event of parser.push(decoder.decode(value, { stream: true }))) onEvent(event);
    }
  } catch (cause) {
    if (cause instanceof ChatCallError) throw cause;
    if (controller.signal.aborted) {
      throw new ChatCallError(`no word from the server for ${idleTimeoutMs / 1000}s`, "stalled", null);
    }
    throw new ChatCallError(cause instanceof Error ? cause.message : String(cause), "network", null);
  } finally {
    window.clearTimeout(timer);
  }
}
