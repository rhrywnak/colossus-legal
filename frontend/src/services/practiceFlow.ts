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

