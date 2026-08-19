// =============================================================================
// practiceFlow.ts — the writes mockup v3 added
// =============================================================================
//
// The flag, and nothing else yet. A sibling of `practice.ts` rather than more of
// it: that module reached Rule 17's 300-line limit when v3's calls were added to
// it, and the seam is the honest one — `practice.ts` serves the DRILL (deal a
// question, record an answer, end a sitting), and this serves what Marie can do
// to the DECK itself.
//
// Same idioms as its sibling: `authFetch` with an `AbortController` timeout, an
// explicit failure sentence, nothing swallowed.

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import { readErrorMessage } from "./fetchUtils";
import { PRACTICE_TIMEOUT_MS } from "./practice";

export interface NewSitting {
  who: string;
  /**
   * The question ids this sitting will be dealt, IN ORDER.
   *
   * Sent rather than left to the server to draw: the ORDER is the drill, and the
   * screen composing it is the one that also knows what she kept out today. The
   * server stores it, which is what a reload resumes and what "Ended early."
   * is measured against — and it FENCES it, so a queue naming another
   * scenario's questions is a 400 rather than a foreign row on Chuck's sheet.
   */
  queue: string[];
  /** What she chose off the count pills. */
  count: number;
  /** The ids she kept out on the start screen. For the record; never dealt. */
  skippedToday: string[];
}


/**
 * Store — or clear — Marie's flag on one question.
 *
 * ## Why the SERVER's value is returned rather than the typed one
 *
 * The backend trims the note, and a blank note clears the flag. A screen that
 * echoed what she typed would show a flag the database does not have — a
 * leading space, or a "flag" made entirely of whitespace.
 */
export async function savePracticeFlag(
  questionId: string,
  note: string,
): Promise<string | null> {
  const response = await authFetch(
    `${API_BASE_URL}/api/practice/questions/${encodeURIComponent(questionId)}/flag`,
    {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ note }),
      timeoutMs: PRACTICE_TIMEOUT_MS,
    },
  );

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(`Your flag was not saved (HTTP ${response.status}${detail}).`);
  }

  const body = (await response.json()) as { flag_note: string | null };
  return body.flag_note;
}


/** One sitting, as the page re-enters it at its own address. */
export interface Sitting {
  session_id: string;
  scenario_id: string;
  /** `george` | `chuck` | `mixed`. */
  who: string;
  /** The dealt question ids, in order. EMPTY = a sitting that cannot resume. */
  queue: string[];
  /** The questions already dealt. A `skipped` row counts as dealt. */
  answered: string[];
  /** True when the sitting is already closed. */
  ended: boolean;
}

/**
 * Refuse a body that is not a sitting, by NAME.
 *
 * Shared by the two calls that return one, so a contract mismatch reads the same
 * whichever of them met it. The alternative — trusting the cast — puts
 * `undefined.length` in the middle of a witness's session with no clue where it
 * came from.
 */
function asSitting(parsed: Partial<Sitting>): Sitting {
  if (
    typeof parsed.session_id !== "string" ||
    typeof parsed.who !== "string" ||
    !Array.isArray(parsed.queue) ||
    !Array.isArray(parsed.answered)
  ) {
    throw new Error(
      "The practice session response is missing its queue or its side — " +
        "backend/frontend contract mismatch. Report it to the site administrator.",
    );
  }
  return parsed as Sitting;
}

/** Read one sitting — what a reload at `…/session/:id` runs. */
export async function fetchSitting(sessionId: string): Promise<Sitting> {
  const response = await authFetch(
    `${API_BASE_URL}/api/practice/sessions/${encodeURIComponent(sessionId)}`,
    { timeoutMs: PRACTICE_TIMEOUT_MS },
  );

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `That practice session could not be opened (HTTP ${response.status}${detail}).`,
    );
  }
  return asSitting((await response.json()) as Partial<Sitting>);
}

/**
 * Take the unfinished sitting back, and retire any older open ones.
 *
 * The retiring happens on the SERVER and is why this is a POST rather than the
 * read above: pressing Resume is the first moment she has said which sitting she
 * means, and nothing before that may end one on her behalf.
 */
export async function resumeSitting(sessionId: string): Promise<Sitting> {
  const response = await authFetch(
    `${API_BASE_URL}/api/practice/sessions/${encodeURIComponent(sessionId)}/resume`,
    { method: "POST", timeoutMs: PRACTICE_TIMEOUT_MS },
  );

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `That practice session could not be resumed (HTTP ${response.status}${detail}).`,
    );
  }
  return asSitting((await response.json()) as Partial<Sitting>);
}

/**
 * Close the unfinished sitting — and every older one — and start clean.
 *
 * Never a delete. Each closed sitting keeps its answers and gets a Chuck's sheet
 * of its own, which is exactly what the stored hint beside the control promises.
 */
export async function startOverSitting(sessionId: string): Promise<void> {
  const response = await authFetch(
    `${API_BASE_URL}/api/practice/sessions/${encodeURIComponent(sessionId)}/start-over`,
    { method: "POST", timeoutMs: PRACTICE_TIMEOUT_MS },
  );

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `That practice session could not be closed (HTTP ${response.status}${detail}).`,
    );
  }
}

/**
 * Record that she was dealt this question and set it aside.
 *
 * ## Why this is its own call and not `submitPracticeAnswer` with a flag
 *
 * It makes no model call, stores the STORED "doesn't fit" phrase rather than
 * anything she typed, and lands on the row already marked `skipped`. The answer
 * path does the opposite of all three.
 *
 * The ordinary 30-second timeout, and not the answer path's ninety: there is no
 * model in this request, so a slow one is a slow database and not a slow vendor.
 */
export async function skipPracticeQuestion(
  sessionId: string,
  questionId: string,
): Promise<void> {
  const response = await authFetch(`${API_BASE_URL}/api/practice/answers/skip`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ session_id: sessionId, question_id: questionId }),
    timeoutMs: PRACTICE_TIMEOUT_MS,
  });

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Skipping that question was not recorded (HTTP ${response.status}${detail}).`,
    );
  }
}
