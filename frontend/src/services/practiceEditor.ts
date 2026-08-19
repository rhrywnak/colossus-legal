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
// ## Every edit is SIGNED
//
// `editingAs` is a required argument on all four deck writes, not an optional
// one with a default. There is one login, and a change nobody signed is a change
// nobody can ask about — the server refuses it too, which is what makes the
// screen's picker more than a courtesy.
//
// Same idioms as its siblings: `authFetch` with an `AbortController` timeout,
// `encodeURIComponent` on every path parameter, an explicit failure sentence,
// nothing swallowed.

import { API_BASE_URL } from "./api";
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
  editingAs: string,
): Promise<void> {
  const response = await post(
    `/api/practice/questions/${encodeURIComponent(questionId)}/edit`,
    { field, value, editing_as: editingAs },
  );
  await orThrow<unknown>(response, "That edit was not saved");
}

/** Move one question up or down within its own side. */
export async function moveQuestion(
  questionId: string,
  direction: "up" | "down",
  editingAs: string,
): Promise<void> {
  const response = await post(
    `/api/practice/questions/${encodeURIComponent(questionId)}/move`,
    { direction, editing_as: editingAs },
  );
  await orThrow<unknown>(response, "That question was not moved");
}

/** Hide one question, or put it back. Never a delete. */
export async function hideQuestion(
  questionId: string,
  hidden: boolean,
  editingAs: string,
): Promise<void> {
  const response = await post(
    `/api/practice/questions/${encodeURIComponent(questionId)}/hidden`,
    { hidden, editing_as: editingAs },
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
  editingAs: string,
): Promise<void> {
  const response = await post(
    `/api/cases/${encodeURIComponent(slug)}/scenarios/` +
      `${encodeURIComponent(scenarioId)}/practice/questions`,
    { ...question, editing_as: editingAs },
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
  author: string,
  text: string,
): Promise<PracticeNote> {
  const response = await post(
    `/api/cases/${encodeURIComponent(slug)}/scenarios/` +
      `${encodeURIComponent(scenarioId)}/practice/notes`,
    {
      question_id: target.questionId,
      answer_id: target.answerId,
      author,
      text,
    },
  );
  return orThrow<PracticeNote>(response, "That note was not saved");
}

/** Strike one note through. Never a delete. */
export async function strikeNote(noteId: string, author: string): Promise<void> {
  const response = await post(
    `/api/practice/notes/${encodeURIComponent(noteId)}/strike`,
    { author },
  );
  await orThrow<unknown>(response, "That note was not struck");
}

/** The review page for one question. */
export async function fetchQuestionReview(
  slug: string,
  scenarioId: string,
  questionId: string,
): Promise<PracticeReview> {
  const response = await authFetch(
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}/scenarios/` +
      `${encodeURIComponent(scenarioId)}/practice/questions/` +
      `${encodeURIComponent(questionId)}`,
    { timeoutMs: PRACTICE_TIMEOUT_MS },
  );

  const parsed = await orThrow<Partial<PracticeReview>>(
    response,
    "That question's review could not be loaded",
  );
  // The shapes the page cannot render without. `attempts` may be EMPTY — a
  // question nobody has answered is a legitimate screen, and the page says so
  // in the store's words — but it must be an array, and the wording must be
  // there or every label on the page is blank.
  if (
    !Array.isArray(parsed.attempts) ||
    !Array.isArray(parsed.points) ||
    !Array.isArray(parsed.notes) ||
    parsed.question == null ||
    parsed.wording == null
  ) {
    throw new Error(
      "The review response is missing its attempts, points or wording — " +
        "backend/frontend contract mismatch. Report it to the site administrator.",
    );
  }
  return parsed as PracticeReview;
}
