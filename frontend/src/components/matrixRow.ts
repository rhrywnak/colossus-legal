// =============================================================================
// matrixRow.ts — the Proof Matrix row's pure display helpers (PROOF_MATRIX_v2)
// =============================================================================
//
// Everything a row decides that does not need React. CLAUDE.md §30: there is no
// component-test infrastructure in this repo, so anything worth asserting has to
// be reachable without rendering — and on this page the things worth asserting
// are exactly the ones that only show up on screen. Which items are behind
// "N more". Which are hidden. Whether a line says `machine` or `kept · Roman`.
//
// ## The frontend composes nothing
//
// Every function here either SUBSTITUTES a number into a served template or
// PICKS one of several served strings. The one real sentence on a row — the RFA
// line — is composed by the backend and arrives finished, for the reason the
// language law gives: a sentence assembled in two places reads two ways.
//
// ## Nothing here derives state
//
// `hidden_reason` is served. `ruling` is served. The order is served. Rule 19
// says action availability and state transitions live in the backend, and the
// split below (visible / behind-more / hidden) is a fold of a list the backend
// already ordered — not a second opinion about it.

import type { AllegationEvidence } from "../services/elementDetailService";
import type { MatrixWording } from "../services/causesOfAction";
import { fillCount } from "./matrixStrength";

/**
 * One accusation's items, split the three ways the paragraph renders them.
 *
 * `shown` and `behindMore` together are every VISIBLE item, in the backend's
 * order; `hidden` is everything the backend flagged. The three are disjoint and
 * their lengths sum to the input's — asserted, because a fold that dropped an
 * item would take a piece of proof off the page with nothing to say so.
 */
export type SplitEvidence = {
  /** The first `limit` visible items — what a reader sees without clicking. */
  shown: AllegationEvidence[];
  /** The rest of the visible items, behind the "N more" control. */
  behindMore: AllegationEvidence[];
  /** Removed or machine-excluded items, behind the "show hidden" toggle. */
  hidden: AllegationEvidence[];
};

/**
 * Split one leg into the short list, the overflow, and the hidden group.
 *
 * @param items the leg AS SERVED — already ordered and flagged by the backend
 * @param limit the served `visible_items`
 *
 * ## Why the order is not touched
 *
 * The backend applied §3's four sort keys, folded duplicates and partitioned
 * hidden from visible. Re-sorting here would be a second ranking free to
 * disagree with the one the Word export prints from.
 */
export function splitEvidence(
  items: AllegationEvidence[],
  limit: number,
): SplitEvidence {
  const visible = items.filter((item) => item.hidden_reason === null);
  const hidden = items.filter((item) => item.hidden_reason !== null);
  // A negative or non-finite limit would make `slice` behave surprisingly (a
  // negative index counts from the END, which would show the LAST few items).
  // Clamped rather than trusted: the value is served, and a store edited around
  // the API can hold anything.
  const safeLimit = Number.isFinite(limit) && limit > 0 ? Math.floor(limit) : 0;
  return {
    shown: visible.slice(0, safeLimit),
    behindMore: visible.slice(safeLimit),
    hidden,
  };
}

/**
 * The "N more" control's label, or `null` when nothing is behind it.
 *
 * `null` rather than a disabled control: a button that opens nothing is a button
 * a reader clicks once and then distrusts.
 */
export function moreLabel(
  behindMore: AllegationEvidence[],
  wording: MatrixWording,
): string | null {
  if (behindMore.length === 0) return null;
  return fillCount(wording.more_template, behindMore.length);
}

/**
 * The "show hidden (N)" toggle's label, or `null` when nothing is hidden.
 *
 * @param expanded whether the hidden group is currently showing — the toggle
 *                 then reads as the control that puts them back
 */
export function hiddenToggleLabel(
  hidden: AllegationEvidence[],
  expanded: boolean,
  wording: MatrixWording,
): string | null {
  if (hidden.length === 0) return null;
  return expanded
    ? wording.hide_hidden_label
    : fillCount(wording.show_hidden_template, hidden.length);
}

/**
 * The words on a row, and whether they are a human's.
 *
 * ## Domain note: the most load-bearing line on the page
 *
 * An unruled item says `machine`. That single word is what keeps a ranking
 * nobody has read from being mistaken for reviewed work — which is the one false
 * claim this page must never make. It is never omitted, never abbreviated away,
 * and never inferred: an item is a human's only when the backend served a
 * `ruled_by` for it.
 */
export type AuthorMark = {
  label: string;
  /** `true` only for an item a human kept — the green treatment. */
  confirmed: boolean;
};

export function authorMark(
  item: AllegationEvidence,
  wording: MatrixWording,
): AuthorMark {
  if (item.ruling === "keep" && item.ruled_by !== null) {
    return {
      label: wording.kept_template.replace("{actor}", item.ruled_by),
      confirmed: true,
    };
  }
  // Everything else — unruled, removed, or a `keep` with no attribution the
  // backend could not have written — reads as the machine's own claim. A `keep`
  // without a `ruled_by` is unreachable through the API; if it ever arrived, the
  // honest rendering is "nobody is recorded as having said this".
  return { label: wording.machine_label, confirmed: false };
}

/**
 * The confidence word for one item.
 *
 * Returns the stored `unrated` label for an item with NO confidence, because 286
 * edges predate the linking pass and "no mark at all" would make them read as
 * items the pass had considered. An unknown token returns `null` — this build
 * has no word it is entitled to print, and printing the raw token would put the
 * database's vocabulary in front of Chuck.
 */
export function confidenceLabel(
  confidence: string | null,
  wording: MatrixWording,
): string | null {
  switch (confidence) {
    case "high":
      return wording.confidence_high_label;
    case "medium":
      return wording.confidence_medium_label;
    case "low":
      // No edge carries `low` today — the 2026-09-06 pass emitted only high and
      // medium. The case exists BEFORE a pass emits one, because the alternative
      // is that the first low-confidence item renders with no mark at all, which
      // is exactly how an UNRATED item reads: a reader would be told nothing
      // instead of being told "low".
      return wording.confidence_low_label;
    case null:
      return wording.confidence_unrated_label;
    default:
      // Not the Rule 1 storage carve-out — this is neither storage nor a
      // preference. It degrades for its own reason: a confidence this build
      // cannot name renders with no mark, which is a missing LABEL and not a
      // missing item. Nothing is hidden and no count moves. The console warning
      // carries the token so an operator can see the frontend is behind the
      // backend's vocabulary.
      console.warn(
        `Proof Matrix: unknown link confidence "${confidence}" — the row renders ` +
          `without a confidence mark. This build knows high/medium/low.`,
      );
      return null;
  }
}

/**
 * The grey line under a quote: the pass's `rank_reason`, or its older `why`.
 *
 * §3 says "`rank_reason` (or `why` if no rank)". Implemented as a fallback on the
 * REASON rather than on the rank, because 36 edges carry a `why` and no rank at
 * all, and an item with a reason nobody reads is worse than an untidy rule.
 * Returns `null` when there is neither — the row then carries no grey line,
 * rather than an empty one that reads as a rendering fault.
 */
export function reasonLine(item: AllegationEvidence): string | null {
  const reason = item.rank_reason ?? item.why;
  if (reason === null || reason.trim() === "") return null;
  return reason;
}

/**
 * The words a row prints: the backend-composed RFA line when there is one, the
 * verbatim quote otherwise.
 *
 * A Q&A card's `verbatim_quote` is the ANSWER alone — "Admitted", "No." — and a
 * column of those under an accusation says nothing about what was admitted.
 */
export function quoteText(item: AllegationEvidence): string | null {
  const text = item.rfa_line ?? item.verbatim_quote;
  if (text === null || text.trim() === "") return null;
  return text;
}
