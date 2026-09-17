// =============================================================================
// PracticeReviewBar.tsx — "{n} new since you last reviewed" · [Done reviewing]
// =============================================================================
//
// CC_TASK_REVIEW_LOOP_v1 §2, placed by CC_GO_REVIEW_LOOP_v2: on the DECK page,
// directly under `PracticeTitleRow` — not in `PracticeTopBar`, which is the
// sitting's exit bar and never renders on this page.
//
// ## Domain note: a read cursor, per person
//
// The number is answers by SOMEONE ELSE since the signed-in viewer last pressed
// Done reviewing (Slack's read-mark pattern). Pressing it moves ONE timestamp for
// this viewer on this deck; the page then re-reads, so the zero it shows is the
// server's and not a local guess. Marie pressing it would clear only her own mark.
//
// ## Hidden at zero (GO v2)
//
// A first-time viewer of a deck nobody has answered also sees nothing: a bar
// saying "0 new" over an empty deck is a control with nothing to do.
//
// Every string is from the store (`wordingOf` throws by name on a missing key).

import React from "react";

import { wordingOf, type PracticeWording } from "../../services/practice";
import { markDeckReviewed } from "../../services/practiceReviewLoop";
import * as r from "./practiceReviewLoopStyles";
import * as s from "./practiceStyles";

interface Props {
  slug: string;
  scenarioId: string;
  /** The deck payload's `new_since_you_reviewed`. */
  count: number;
  wording: PracticeWording;
  /** Re-read the deck after the mark moved. */
  onReviewed: () => void;
}

const PracticeReviewBar: React.FC<Props> = ({ slug, scenarioId, count, wording, onReviewed }) => {
  const [busy, setBusy] = React.useState(false);
  const [failed, setFailed] = React.useState(false);
  const w = (key: string) => wordingOf(wording, key);

  if (count === 0) return null;

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
      <span style={r.reviewCount}>
        {w("deck_review_new_template").replace("{count}", String(count))}
      </span>
      <button type="button" style={s.buttonPrimary} disabled={busy} onClick={onDone}>
        {w("deck_review_done_label")}
      </button>
      {failed && (
        <div style={r.reviewError} role="alert">
          {w("deck_review_failed")}
        </div>
      )}
    </div>
  );
};

export default PracticeReviewBar;
