// =============================================================================
// practice.ts — client for Marie's practice drill (PRACTICE v0)
// -----------------------------------------------------------------------------
// Endpoints:
//   GET  /api/cases/:slug/scenarios/:id/practice          → the whole page
//   POST /api/cases/:slug/scenarios/:id/practice/sessions → open a sitting
//   POST /api/practice/answers                            → one answer + the read
//   POST /api/practice/answers/:id/help                   → she opened the drawer
//   POST /api/practice/sessions/:id/end                   → close it, get the sheet
//
// Same idioms as `rehearsal.ts`: `authFetch` (credentials + an AbortController
// timeout), `encodeURIComponent` on every path parameter, `readErrorMessage` to
// surface the backend's `{message}`, and every non-2xx throws.
//
// ## Why the answer call gets a LONGER timeout
//
// It makes an LLM call inside the request. The house rule is 30s normally and 90s
// for synthesis; a one-sentence read is far smaller than a synthesis, but it is
// still a model round trip, and a 30s abort would show Marie "your answer was not
// recorded" for an answer the server was in the middle of recording. 90s.
//
// ## What this file deliberately does NOT do
//
// No client-side judging. The red/green sentence comes from the server or does
// not come at all; this module never composes one, and there is no local
// fallback text anywhere in it. The mockup's `judge()` function was a MOCKUP.
// =============================================================================

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import { readErrorMessage } from "./fetchUtils";
import type { NewSitting } from "./practiceFlow";

/** How long an ordinary practice call may take. The house default. */
export const PRACTICE_TIMEOUT_MS = 30000;

/** How long the answer call may take — it makes a model call inside. */
const READ_TIMEOUT_MS = 90000;

/** One question, with everything its two screens render. */
export type PracticeQuestion = {
  id: string;
  /** `george` or `chuck`. */
  side: string;
  /** True when it braids several barrage rows — a third pill. */
  braid: boolean;
  text: string;
  /** The tactic's NAME, already resolved. `null` withdraws the tag. */
  tactic: string | null;
  /** The "Built from: …" line. `null` renders no source line. */
  receipt: string | null;
  braid_rows: string | null;
  watch_for: string | null;
  /** Present together or not at all. */
  pair_said: string | null;
  pair_admitted: string | null;
  /** `null` renders the stored "no receipt for this one" line. */
  stronger: string | null;
  stronger_lean: string | null;
  /** Marie's one line saying what is wrong with this question. `null` = none. */
  flag_note: string | null;
};

/** One of Marie's talking points. */
export type PracticePoint = {
  position: number;
  text: string;
  /** `null` renders the stored named-absence line — never a blank. */
  exhibit: string | null;
};

/**
 * Every string the four screens speak.
 *
 * `Record<string, string>` rather than a named field for each: this mirrors a
 * backend block that is generated from one migration, the page reads it by key,
 * and writing eighty names here would produce eighty chances to typo one into a
 * silent `undefined`. The `wordingOf` helper below is the single reader, and it
 * throws by NAME on a key the payload does not carry.
 */
export type PracticeWording = Record<string, string>;

/** Everything the page needs, in one response. */
export type PracticeDeck = {
  scenario_id: string;
  code: string;
  title: string;
  /** EMPTY is a legitimate state: the page says "no practice deck yet". */
  questions: PracticeQuestion[];
  points: PracticePoint[];
  last_session_line: string;
  wording: PracticeWording;
};

/** Her four self-check boxes. */
export type SelfCheck = {
  only_asked: boolean;
  accepted_premise: boolean;
  explained_unasked: boolean;
  guessed: boolean;
};

/** What the reveal shows about the read. */
export type AnswerResult = {
  answer_id: string;
  /** `null` → the screen shows "no system read this time". */
  read_text: string | null;
  /** `true` green, `false` red, `null` no read. Three states. */
  read_ok: boolean | null;
};

/** One row of Chuck's sheet — every cell already a word. */
export type PracticeSheetRow = {
  number: number;
  from: string;
  tactic: string;
  question: string;
  answer: string;
  mark: string;
  help_opened: boolean;
  help: string;
};

/** Chuck's sheet, composed. */
export type PracticeSheet = {
  kicker: string;
  heading: string;
  rows: PracticeSheetRow[];
  /**
   * The deck's flagged questions, already composed server-side into the
   * sentences the sheet prints. EMPTY withdraws the whole block.
   */
  flagged: string[];
  /** The block's heading and its sentence. Both empty when `flagged` is. */
  flagged_heading: string;
  flagged_hint: string;
};

/**
 * Read one stored string, or throw naming the key.
 *
 * ## Why this throws instead of returning a fallback
 *
 * There is no literal to fall back to (the wording law, v2 §2b), and a missing
 * key means the store and this build disagree. A `?? ""` here would put a blank
 * button in front of a witness; a `?? "Answer"` would put a sentence in the
 * product that no migration can change. Throwing puts the page's own failure
 * notice on screen with the key in it, which is the only honest option.
 */
export function wordingOf(wording: PracticeWording, key: string): string {
  const value = wording[key];
  if (typeof value !== "string" || value.trim() === "") {
    throw new Error(
      `The practice page has no stored wording for "${key}". The backend and ` +
        `this build disagree about the wording store; report it to the site ` +
        `administrator.`,
    );
  }
  return value;
}

/** The deck, the points, the words, and the last-session line — one request. */
export async function fetchPracticeDeck(
  slug: string,
  scenarioId: string,
): Promise<PracticeDeck> {
  const response = await authFetch(
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}/scenarios/` +
      `${encodeURIComponent(scenarioId)}/practice`,
    { timeoutMs: PRACTICE_TIMEOUT_MS },
  );

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to load the practice deck (HTTP ${response.status}${detail}).`,
    );
  }

  const parsed = (await response.json()) as Partial<PracticeDeck>;
  // The shapes the page cannot render without. `questions` may be EMPTY — that
  // is the un-seeded scenario, and a legitimate screen — but it must be an
  // array, and `wording` must be there or every label on all four screens is
  // blank.
  if (
    !Array.isArray(parsed.questions) ||
    !Array.isArray(parsed.points) ||
    parsed.wording == null ||
    typeof parsed.last_session_line !== "string"
  ) {
    throw new Error(
      "The practice response is missing questions/points/wording — " +
        "backend/frontend contract mismatch. Report it to the site administrator.",
    );
  }
  return parsed as PracticeDeck;
}

/** Open a session. `who` is `george`, `chuck` or `mixed`. */
export async function startPracticeSession(
  slug: string,
  scenarioId: string,
  sitting: NewSitting,
): Promise<string> {
  const response = await authFetch(
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}/scenarios/` +
      `${encodeURIComponent(scenarioId)}/practice/sessions`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        who: sitting.who,
        queue: sitting.queue,
        count: sitting.count,
        skipped_today: sitting.skippedToday,
      }),
      timeoutMs: PRACTICE_TIMEOUT_MS,
    },
  );

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Could not start the practice session (HTTP ${response.status}${detail}).`,
    );
  }
  const parsed = (await response.json()) as { session_id?: string };
  if (typeof parsed.session_id !== "string") {
    throw new Error("The session response carried no session id.");
  }
  return parsed.session_id;
}

/**
 * Record one answer and return the read.
 *
 * ## Why the mark and the boxes are NOT sent here
 *
 * Both are decided after she has read the reveal, and this call is what produces
 * the reveal. The row opens provisional (no boxes, marked fine) and
 * `closePracticeAnswer` settles it when she moves on — which means her typed
 * answer is already stored if she closes the laptop mid-screen.
 */
export async function submitPracticeAnswer(input: {
  sessionId: string;
  questionId: string;
  answerText: string;
  dontRecall: boolean;
}): Promise<AnswerResult> {
  const response = await authFetch(`${API_BASE_URL}/api/practice/answers`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      session_id: input.sessionId,
      question_id: input.questionId,
      answer_text: input.answerText,
      dont_recall: input.dontRecall,
    }),
    timeoutMs: READ_TIMEOUT_MS,
  });

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Your answer was not recorded (HTTP ${response.status}${detail}).`,
    );
  }
  const parsed = (await response.json()) as Partial<AnswerResult>;
  if (typeof parsed.answer_id !== "string") {
    throw new Error("The answer response carried no answer id.");
  }
  return {
    answer_id: parsed.answer_id,
    read_text: parsed.read_text ?? null,
    read_ok: parsed.read_ok ?? null,
  };
}

/** Settle one answer when she leaves the reveal: her mark and her four boxes. */
export async function closePracticeAnswer(
  answerId: string,
  mark: "fine" | "repeat",
  selfCheck: SelfCheck,
): Promise<void> {
  const response = await authFetch(
    `${API_BASE_URL}/api/practice/answers/${encodeURIComponent(answerId)}/close`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ mark, self_check: selfCheck }),
      timeoutMs: PRACTICE_TIMEOUT_MS,
    },
  );

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Your mark for that question was not recorded (HTTP ${response.status}${detail}).`,
    );
  }
}

/** She opened the stronger-answer drawer; Chuck's sheet says so. */
export async function markHelpOpened(answerId: string): Promise<void> {
  const response = await authFetch(
    `${API_BASE_URL}/api/practice/answers/${encodeURIComponent(answerId)}/help`,
    { method: "POST", timeoutMs: PRACTICE_TIMEOUT_MS },
  );

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Opening the stronger answer was not recorded (HTTP ${response.status}${detail}).`,
    );
  }
}

/** Close the session and get Chuck's sheet. */
export async function endPracticeSession(
  sessionId: string,
): Promise<PracticeSheet> {
  const response = await authFetch(
    `${API_BASE_URL}/api/practice/sessions/${encodeURIComponent(sessionId)}/end`,
    { method: "POST", timeoutMs: PRACTICE_TIMEOUT_MS },
  );

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `The session could not be closed (HTTP ${response.status}${detail}).`,
    );
  }
  const parsed = (await response.json()) as Partial<PracticeSheet>;
  if (!Array.isArray(parsed.rows) || typeof parsed.heading !== "string") {
    throw new Error(
      "Chuck's sheet came back without its rows or its heading — " +
        "backend/frontend contract mismatch.",
    );
  }
  return parsed as PracticeSheet;
}
