// =============================================================================
// PracticeReviewBar.tsx — "{n} answers awaiting {reviewer}'s review" · [Done reviewing]
// =============================================================================
//
// CC_TASK_REVIEW_LOOP_v1 §2, placed by CC_GO_REVIEW_LOOP_v2 on the DECK page under
// `PracticeTitleRow`; made global by CC_TASK_SIMPLE_COUNTS_v1.
//
// ## Domain note: one queue, owned by the reviewer
//
// The number is the REVIEWER's backlog on this deck — answers newer than the
// reviewer's last Done reviewing — and every viewer sees the same number. Only the
// reviewer is offered the button (`review.can_mark_reviewed`, decided on the
// server — the page never compares a username). The reviewer's press moves the
// queue for everyone; the page then re-reads, so the zero it shows is the
// server's and not a local guess.
//
// ## Hidden at zero (GO v2)
//
// A bar saying "0 awaiting" over a reviewed deck is a control with nothing to do.
//
// Every string is from the store (`wordingOf` throws by name on a missing key).

import React from "react";

import { wordingOf, type DeckReview, type PracticeWording } from "../../services/practice";
import { markDeckReviewed } from "../../services/practiceReviewLoop";
import { pickByCount } from "../../utils/countWording";
import * as r from "./practiceReviewLoopStyles";
import * as s from "./practiceStyles";

interface Props {
  slug: string;
  scenarioId: string;
  /** The deck payload's `review` block, decided on the server. */
  review: DeckReview;
  wording: PracticeWording;
  /** Re-read the deck after the mark moved. */
  onReviewed: () => void;
}

/**
 * The bar's sentence: the singular row at exactly 1, the plural otherwise (GO v3).
 * Exported so the pick is testable without rendering the bar.
 */
export function reviewBarLine(review: DeckReview, wording: PracticeWording): string {
  const template = pickByCount(
    review.awaiting,
    wordingOf(wording, "deck_review_awaiting_one"),
    wordingOf(wording, "deck_review_awaiting_template"),
  );
  return template
    .replace("{count}", String(review.awaiting))
    .replace("{reviewer}", review.reviewer_display_name);
}

const PracticeReviewBar: React.FC<Props> = ({ slug, scenarioId, review, wording, onReviewed }) => {
  const [busy, setBusy] = React.useState(false);
  const [failed, setFailed] = React.useState(false);
  const w = (key: string) => wordingOf(wording, key);

  if (review.awaiting === 0) return null;

  const onDone = () => {
    setBusy(true);
    setFailed(false);
    markDeckReviewed(slug, scenarioId)
      .then(() => onReviewed())
      .catch((cause: unknown) => {
        // Standing Rule 1: the mark did not move, so the count stays and the
        // stored sentence says why; the cause goes to the console for a reader.
        // eslint-disable-next-line no-console
        console.error("practice: Done reviewing failed", cause);
        setFailed(true);
      })
      .finally(() => setBusy(false));
  };

  return (
    <div style={r.reviewBar} data-review-bar>
      <span style={r.reviewCount}>{reviewBarLine(review, wording)}</span>
      {/* Only the reviewer is offered the button — the server decided (rule 12).
          Everyone else sees the same count and nothing to press. */}
      {review.can_mark_reviewed && (
        <button type="button" style={s.buttonPrimary} disabled={busy} onClick={onDone}>
          {w("deck_review_done_label")}
        </button>
      )}
      {failed && (
        <div style={r.reviewError} role="alert">
          {w("deck_review_failed")}
        </div>
      )}
    </div>
  );
};

export default PracticeReviewBar;
