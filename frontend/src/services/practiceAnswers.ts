// practiceAnswers.ts — the printed answers sheet's one request.
//
// ## Why its own module and not another export from `practice.ts`
//
// That file was already at 417 non-comment lines before this task — over Rule
// 17's limit, and pre-existing. Adding to it would have made a standing
// violation worse to save creating a file, which is the wrong trade in a repo
// whose whole convention is a module per surface.
//
// The seam is honest as well as arithmetical: everything in `practice.ts` is
// read by the practice PAGE on mount. This is read once, by a print tab, when
// Chuck decides he wants paper.

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import { readErrorMessage } from "./fetchUtils";
import { PRACTICE_TIMEOUT_MS } from "./practice";

/** One question's current answer, as the printed answers sheet receives it. */
export type PracticeAnswer = {
  question_id: string;
  /** Her words, exactly as typed. */
  text: string;
  /** `Answered on 22 Aug`, already composed by the server. */
  answered_on: string;
};

/**
 * Every current answer in one scenario — the printed answers sheet's payload.
 *
 * ## Why this is a second request and not part of the deck
 *
 * The deck is fetched on every load of the practice page and this carries every
 * answer's full prose. Riding it along would make Marie wait on text her screen
 * never shows. Chuck asks for it once, deliberately, by opening a print tab.
 */
export async function fetchPracticeAnswers(
  slug: string,
  scenarioId: string,
): Promise<PracticeAnswer[]> {
  const response = await authFetch(
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}/scenarios/` +
      `${encodeURIComponent(scenarioId)}/practice/answers`,
    { timeoutMs: PRACTICE_TIMEOUT_MS },
  );

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to load the answers (HTTP ${response.status}${detail}).`,
    );
  }

  const parsed = (await response.json()) as { answers?: PracticeAnswer[] };
  // An EMPTY list is legitimate — a deck nobody has answered — but it must be
  // an array. `undefined` here would print a sheet claiming every question is
  // unanswered, which is a different fact and one Chuck would act on.
  if (!Array.isArray(parsed.answers)) {
    throw new Error(
      "The answers response is missing its answers array — " +
        "backend/frontend contract mismatch. Report it to the site administrator.",
    );
  }
  return parsed.answers;
}
