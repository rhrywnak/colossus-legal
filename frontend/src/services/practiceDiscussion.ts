// practiceDiscussion.ts — "Discuss with AI": one question's thread, and a message.
//
// CC_TASK_QUESTION_CHAT_v1. Two calls over one address. Every failure THROWS a
// `DiscussionError` carrying the HTTP status, so the dock can tell "the cap is
// reached" (409) from "no reply came back" (503) from "the thread will not load"
// without matching sentences.
//
// ## Checked BY EYE against `backend/src/dto/practice_discussion.rs`
//
// Hand-written types, like every practice service: a field the backend stops
// sending is `undefined` at runtime and `tsc` says nothing, so the parse below
// refuses a payload missing any field the dock renders.

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import { readErrorMessage } from "./fetchUtils";
import { PRACTICE_TIMEOUT_MS } from "./practice";

/**
 * How long a SEND may take: it waits on a model that may think before it writes.
 * The house RAG-class budget (`services/ask.ts` uses the same two minutes).
 */
export const DISCUSS_SEND_TIMEOUT_MS = 120000;

/** One message in the thread, composed by the server. */
export type DiscussionTurn = {
  id: string;
  role: "user" | "model";
  author: string;
  model_id: string | null;
  text: string;
  /** `5:36 pm`, in the case's timezone. */
  when: string;
  /** What a model reply cost; `null` on user turns and when not reported. */
  input_tokens: number | null;
  output_tokens: number | null;
  ms: number | null;
};

/** One model the picker offers. */
export type DiscussModel = {
  model_id: string;
  display_name: string;
  /** `billed` or `local`. */
  billing_class: string;
};

/** Everything the dock needs. */
export type DiscussionPayload = {
  question_id: string;
  scenario_code: string;
  position: number;
  turns: DiscussionTurn[];
  models: DiscussModel[];
  default_model: string;
  max_turns: number;
  model_turns: number;
};

/** A failed discussion call, with the status the dock branches on. */
export class DiscussionError extends Error {
  constructor(
    message: string,
    public readonly status: number,
  ) {
    super(message);
    this.name = "DiscussionError";
  }
}

const path = (questionId: string) =>
  `${API_BASE_URL}/api/practice/questions/${encodeURIComponent(questionId)}/discussion`;

/** Refuse a body missing what the dock renders — a contract breach, not an empty thread. */
function asPayload(body: Partial<DiscussionPayload>): DiscussionPayload {
  if (
    typeof body.question_id !== "string" ||
    typeof body.scenario_code !== "string" ||
    typeof body.position !== "number" ||
    !Array.isArray(body.turns) ||
    !Array.isArray(body.models) ||
    typeof body.default_model !== "string" ||
    typeof body.max_turns !== "number" ||
    typeof body.model_turns !== "number"
  ) {
    throw new DiscussionError(
      "The discussion response is missing fields — backend/frontend contract mismatch.",
      0,
    );
  }
  return body as DiscussionPayload;
}

async function orThrow(response: Response, what: string): Promise<DiscussionPayload> {
  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new DiscussionError(`${what} (HTTP ${response.status}${detail}).`, response.status);
  }
  return asPayload((await response.json()) as Partial<DiscussionPayload>);
}

/** Load one question's thread, its model list and its cap. */
export async function fetchDiscussion(questionId: string): Promise<DiscussionPayload> {
  const response = await authFetch(path(questionId), { timeoutMs: PRACTICE_TIMEOUT_MS });
  return orThrow(response, "The discussion could not be loaded");
}

/** What one message sends. `draft_answer` only when it differs from the saved answer. */
export type DiscussionMessage = {
  text: string;
  model: string;
  draft_answer?: string;
};

/** Send one message; the server stores it, asks the model, and returns the thread. */
export async function sendDiscussion(
  questionId: string,
  message: DiscussionMessage,
): Promise<DiscussionPayload> {
  const response = await authFetch(path(questionId), {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(message),
    timeoutMs: DISCUSS_SEND_TIMEOUT_MS,
  });
  return orThrow(response, "The message was not answered");
}
