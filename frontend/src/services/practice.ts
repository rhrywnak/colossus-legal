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
  /**
   * `cross`, `direct` or `redirect` — what the question DOES.
   *
   * Not the same as `side`: Chuck asks both `direct` and `redirect`, and they
   * are dealt, tagged and judged differently. Nothing on this screen infers one
   * from the other.
   */
  kind: string;
  /** The stable handle the deck file uses (`g1`, `r2`), or `null`. */
  deck_key: string | null;
  /** The `deck_key` of the George question a redirect answers, or `null`. */
  follows_key: string | null;
  /**
   * `Answered on 22 Aug`, ALREADY COMPOSED by the server — or `null` when
   * nobody has answered this question.
   *
   * The ONE status a row carries. `null` renders NOTHING at all, not an empty
   * line: an empty status under a question reads as one that failed to load,
   * which is a different fact from "not answered yet". The stored footnote
   * under the list is what tells a reader which they are looking at.
   *
   * ## ⚑ This is not scoped to the person reading it
   *
   * The page is one page for two people. Chuck opens it to read Marie's
   * answers and to print them, so the answer belongs to the QUESTION rather
   * than to the requester. Its retired predecessor (`status`) was user-scoped,
   * because it reported what SHE did in a sitting.
   */
  answered_on: string | null;
  /**
   * True when this question has been deleted — the mechanism is a hide, so the
   * row is gone from every list while her answers keep pointing at it.
   */
  hidden: boolean;
  /** Who drafted it when nobody has reviewed it (`architect`), or `null`. */
  draft_by: string | null;
};

/** One note, as every panel renders it. */
export type PracticeNote = {
  id: string;
  question_id: string | null;
  answer_id: string | null;
  author: string;
  text: string;
  /** `Tue 18 Aug` — composed, so the browser holds no date format. */
  when: string;
  /** `struck Tue 19 Aug`, or `null` while it stands. Its presence strikes it. */
  struck: string | null;
};

/** What changed since her last sitting, composed. */
export type PracticeChanged = {
  heading: string;
  /** The plain-words list behind the fold. */
  items: string[];
};

/** One thing a new question can be attached to, in the add form's picker. */
export type PracticeAttachOption = {
  source_kind: string;
  source_index: number;
  label: string;
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

/** The sitting she walked out of, as the start card offers it back. */
export type OpenSession = {
  session_id: string;
  /** `· today 09:57 · George's side · 1 of 5 answered.` — composed server-side. */
  detail: string;
};

/** Everything the page needs, in one response. */
export type PracticeDeck = {
  scenario_id: string;
  code: string;
  title: string;
  /** EMPTY is a legitimate state: the page says "no practice deck yet". */
  questions: PracticeQuestion[];
  points: PracticePoint[];
  last_session_line: string;
  /**
   * When any question in this deck last changed — the deck's own date.
   *
   * ISO-8601, or `null` on a deck with no questions. Read by the print sheets'
   * "deck as of" line: paper outlives the deck it was taken from, and a reader
   * must be able to tell how stale the sheet in his hand is.
   */
  deck_as_of: string | null;
  /** What the "I'd point to…" picker offers, composed and de-duplicated. */
  receipts: string[];
  /** `null` withdraws the blue resume box entirely. */
  open_session: OpenSession | null;
  /** What the editor's add form may attach a new question to. */
  attach_options: PracticeAttachOption[];
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
  /** What she said she would point to. EMPTY withdraws the line. */
  points_to: string[];
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
  /** The deck changes made on the day of this sitting. EMPTY withdraws the block. */
  changes: string[];
  /** That block's heading. Empty when `changes` is. */
  changes_heading: string;
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
    !Array.isArray(parsed.receipts) ||
    !Array.isArray(parsed.attach_options) ||
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
/**
 * A refused answer, carrying the STATUS as a field.
 *
 * ## Why a class and not a message the page matches on
 *
 * The screen has to tell one refusal apart from the rest: 409, which means this
 * question is already answered in this sitting and this tab is simply behind
 * (two tabs on one sitting — hotfix §3.13). Every other status is "it did not
 * land, try again".
 *
 * Reading that out of `"… (HTTP 409)"` with a regex would work until somebody
 * edited the sentence, and the sentence is one the store may yet own. The
 * status is a number the server sent; carry the number.
 *
 * ## Rust Learning: this is the `thiserror` enum, in TypeScript's idiom
 *
 * The backend would model this as `#[derive(thiserror::Error)] enum AnswerError
 * { AlreadyAnswered, … }` and match on the variant. TypeScript has no enums
 * worth the name here, so the equivalent is a subclass with a discriminating
 * field and an `instanceof` check — same shape, same reason: the CALLER decides
 * what a particular failure means, and it cannot decide from prose.
 */
export class PracticeAnswerError extends Error {
  readonly status: number;

  constructor(status: number, message: string) {
    super(message);
    this.name = "PracticeAnswerError";
    this.status = status;
  }
}

export async function submitPracticeAnswer(input: {
  sessionId: string;
  questionId: string;
  answerText: string;
  dontRecall: boolean;
  /**
   * The receipts she picked, or `null` when she never opened the control.
   *
   * `null` and `[]` are DIFFERENT and are sent differently: an empty array says
   * she looked at the list and reached for nothing, which is a fact about the
   * answer; `null` says the question of what she would point to never came up.
   */
  pointsTo: string[] | null;
}): Promise<AnswerResult> {
  const response = await authFetch(`${API_BASE_URL}/api/practice/answers`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      session_id: input.sessionId,
      question_id: input.questionId,
      answer_text: input.answerText,
      dont_recall: input.dontRecall,
      points_to: input.pointsTo,
    }),
    timeoutMs: READ_TIMEOUT_MS,
  });

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new PracticeAnswerError(
      response.status,
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
