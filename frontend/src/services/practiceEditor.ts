// =============================================================================
// practiceEditor.ts — the writes Part B added, and the review page's read
// =============================================================================
//
// Chuck's four edits to the deck, the two note writes, and the review payload.
// A third sibling of `practice.ts` for the reason `practiceFlow.ts` is a second
// one: that module serves the DRILL, `practiceFlow` serves getting into and out
// of a sitting, and this serves what Chuck and Roman do to the deck between
// sittings.
//
// ## Every edit is SIGNED — by the LOGIN, not by a picker
//
// Until 2026-08-19 all four deck writes took an `editingAs` argument, filled by
// a "Who is editing?" dropdown, because the design assumed this build had one
// shared login. It does not: Chuck and Marie have had Authentik accounts since
// March, and every one of these requests already arrives authenticated. The
// picker was asking a question the server could answer itself — and, worse, the
// hook refused to call anything until it was answered, which is how "Edit" came
// to do nothing at all with nothing on screen saying why.
//
// So the argument is gone from all six writes. The server takes the signature
// from the session (`api::practice_editor`, `services::practice_notes::
// attribution`). A change is still signed; the signature is just no longer
// something a screen can get wrong.
//
// Same idioms as its siblings: `authFetch` with an `AbortController` timeout,
// `encodeURIComponent` on every path parameter, an explicit failure sentence,
// nothing swallowed.

import { API_BASE_URL } from "./api";
import type { AuthUser } from "./auth";
import { authFetch } from "./auth";
import { readErrorMessage } from "./fetchUtils";
import {
  PRACTICE_TIMEOUT_MS,
  type PracticeNote,
  type PracticePoint,
  type PracticeQuestion,
  type PracticeWording,
} from "./practice";

/** One attempt at one question, as the review page stacks them. */
export interface PracticeAttempt {
  answer_id: string;
  /** `attempt 2 · Wed 19 Aug 08:40` — composed. */
  heading: string;
  /** The stored mark word. */
  mark: string;
  /** The raw stored value, so the screen colours without matching a sentence. */
  mark_key: string;
  answer: string;
  read_text: string | null;
  read_ok: boolean | null;
  points_to: string[];
  /** `help: opened · boxes: …` — composed. */
  detail: string;
  /**
   * `asked as: "…"` when this attempt's question has been re-worded since.
   *
   * ABSENT from the payload (not null, not "") when the wording has not
   * changed: the server skips serializing it, so the page's check is for a
   * value at all rather than for an empty string that might mean either.
   */
  asked_as?: string;
  notes: PracticeNote[];
}

/** The review page, in one response. */
export interface PracticeReview {
  scenario_id: string;
  code: string;
  title: string;
  question: PracticeQuestion;
  /** `Question 3 · review` — composed. */
  progress: string;
  /** Newest first, which is what the page says out loud. */
  attempts: PracticeAttempt[];
  points: PracticePoint[];
  /** The notes on the QUESTION (Roman's amendment 2). */
  notes: PracticeNote[];
  wording: PracticeWording;
}

/**
 * The signed-in person's name, as a screen prints it.
 *
 * Display-name first, username as the fallback, and `""` while `/api/me` is
 * still in flight — a sentence that briefly reads "Saved as a change by  —"
 * is honest about not knowing yet, where a literal "someone" would be the
 * screen inventing a person. Only ever a LABEL: the attribution that is stored
 * comes from the session on the server, never from this.
 */
export function signedInAs(user: AuthUser | null): string {
  if (user === null) return "";
  return user.display_name.trim() !== "" ? user.display_name : user.username;
}

/** Which field of a question the editor is changing. */
export type EditableField = "text" | "tactic" | "follows" | "watch_for" | "stronger";

/** Throw with the status, or return the decoded body. */
async function orThrow<T>(response: Response, what: string): Promise<T> {
  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(`${what} (HTTP ${response.status}${detail}).`);
  }
  return (await response.json()) as T;
}

/** POST JSON to one practice path, with the house timeout. */
function post(path: string, body: unknown): Promise<Response> {
  return authFetch(`${API_BASE_URL}${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
    timeoutMs: PRACTICE_TIMEOUT_MS,
  });
}

/**
 * Change one field on one question.
 *
 * A blank `value` CLEARS the optional fields and is refused on `text` — a
 * question with no words is not a question. The server decides that; this only
 * sends what was typed.
 */
export async function editQuestion(
  questionId: string,
  field: EditableField,
  value: string | null,
): Promise<void> {
  const response = await post(
    `/api/practice/questions/${encodeURIComponent(questionId)}/edit`,
    { field, value },
  );
  await orThrow<unknown>(response, "That edit was not saved");
}

/** Move one question up or down within its own side. */
export async function moveQuestion(
  questionId: string,
  direction: "up" | "down",
): Promise<void> {
  const response = await post(
    `/api/practice/questions/${encodeURIComponent(questionId)}/move`,
    { direction },
  );
  await orThrow<unknown>(response, "That question was not moved");
}

/** Hide one question, or put it back. Never a delete. */
export async function hideQuestion(
  questionId: string,
  hidden: boolean,
): Promise<void> {
  const response = await post(
    `/api/practice/questions/${encodeURIComponent(questionId)}/hidden`,
    { hidden },
  );
  await orThrow<unknown>(response, "That question was not hidden");
}

/** What the add form sends. Mirrors the backend request, field for field. */
export interface NewQuestion {
  kind: "cross" | "direct" | "redirect";
  text: string;
  tactic: number | null;
  follows: string | null;
  watch_for: string | null;
  source_kind: string | null;
  source_index: number | null;
}

/** Add a question somebody typed on the page. */
export async function addQuestion(
  slug: string,
  scenarioId: string,
  question: NewQuestion,
): Promise<void> {
  const response = await post(
    `/api/cases/${encodeURIComponent(slug)}/scenarios/` +
      `${encodeURIComponent(scenarioId)}/practice/questions`,
    question,
  );
  await orThrow<unknown>(response, "That question was not added");
}

/** Where a note is being written: the scenario, a question, or one attempt. */
export interface NoteTarget {
  questionId: string | null;
  answerId: string | null;
}

/**
 * Write one note, and return it as the SERVER stored it.
 *
 * The stored `created_at` is the server's, so a panel that dated a new note by
 * the browser's clock would disagree with itself the moment it reloaded.
 */
export async function saveNote(
  slug: string,
  scenarioId: string,
  target: NoteTarget,
  text: string,
): Promise<PracticeNote> {
  const response = await post(
    `/api/cases/${encodeURIComponent(slug)}/scenarios/` +
      `${encodeURIComponent(scenarioId)}/practice/notes`,
    {
      question_id: target.questionId,
      answer_id: target.answerId,
      text,
    },
  );
  return orThrow<PracticeNote>(response, "That note was not saved");
}

/**
 * Strike one note through. Never a delete.
 *
 * No body at all: who struck it is the signed-in user, which the server reads
 * from the session. Any signed-in user may strike any note (Roman's rule for
 * §3.10) — the striker is recorded, so a struck note still says who struck it.
 */
export async function strikeNote(noteId: string): Promise<void> {
  const response = await post(
    `/api/practice/notes/${encodeURIComponent(noteId)}/strike`,
    {},
  );
  await orThrow<unknown>(response, "That note was not struck");
}


/**
 * Place one question where a drag dropped it.
 *
 * `before` is the question it lands immediately ABOVE; `null` means the end of
 * its side. A drop that names no position (onto itself, or across sides) is a
 * 200 that changed nothing — see the handler's note on why it is not a 400.
 *
 * A separate call from `moveQuestion` because they are different operations:
 * the arrows move one step and swap two rows, this re-sequences a side.
 */
export async function reorderQuestion(
  questionId: string,
  before: string | null,
): Promise<void> {
  const response = await post(
    `/api/practice/questions/${encodeURIComponent(questionId)}/reorder`,
    { before },
  );
  await orThrow<unknown>(response, "That question was not moved");
}
