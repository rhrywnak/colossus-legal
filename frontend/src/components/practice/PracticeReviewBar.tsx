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
// ## The button asks first (CC_TASK_REVIEW_COUNTS_HONEST_v1, ruled 2026-09-20)
//
// Done reviewing used to write on one click. The mark it moves is SHARED across
// the reviewer bench and there is no undo — the answers it sweeps behind it are
// simply no longer waiting for anybody — so it now opens a question naming the
// count and the deck. The decision of whether anything may be written lives in
// `deckReviewConfirmModel`, a pure reducer, for the reason that module's header
// gives: the property worth proving is a negative, and a negative over a
// component is only as good as the clicks somebody remembered to simulate.
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
import {
  deckReviewConfirmSentence,
  deckReviewConfirmStep,
  idleDeckReviewConfirm,
} from "./deckReviewConfirmModel";
import type {
  DeckReviewConfirmAction,
  DeckReviewConfirmState,
} from "./deckReviewConfirmModel";
import * as r from "./practiceReviewLoopStyles";
import * as s from "./practiceStyles";

interface Props {
  slug: string;
  scenarioId: string;
  /** `S-13` — named in the confirmation, because the press cannot be undone and
   *  a question that does not say WHICH deck is not a question worth asking. */
  code: string;
  /** The deck payload's `review` block, decided on the server. */
  review: DeckReview;
  wording: PracticeWording;
  /** Re-read the deck after the mark moved. */
  onReviewed: () => void;
}

/**
 * The bar's sentence: the singular row at exactly 1, the plural otherwise (GO v3),
 * with the oldest-waiting clause appended when the server sent a date.
 * Exported so the pick is testable without rendering the bar.
 *
 * ## Why the clause is WITHHELD rather than emptied
 *
 * `oldest` is `undefined` when nothing is waiting, and on a deck where the read
 * returned no date. Filling `{date}` with an empty string would print
 * `· oldest waiting since ` — a sentence that trails off, which reads as a
 * rendering fault rather than as an absent fact.
 *
 * The date arrives ALREADY FORMATTED, in the case's own timezone: the browser
 * holds no date format, so it fills a placeholder and nothing else.
 */
export function reviewBarLine(review: DeckReview, wording: PracticeWording): string {
  const template = pickByCount(
    review.awaiting,
    wordingOf(wording, "deck_review_awaiting_one"),
    wordingOf(wording, "deck_review_awaiting_template"),
  );
  const line = template
    .replace("{count}", String(review.awaiting))
    .replace("{reviewer}", review.reviewer_display_name);
  if (review.oldest === undefined || review.oldest === null) return line;
  const oldest = wordingOf(wording, "deck_review_oldest_template").replace(
    "{date}",
    review.oldest,
  );
  return `${line} ${oldest}`;
}

const PracticeReviewBar: React.FC<Props> = ({
  slug,
  scenarioId,
  code,
  review,
  wording,
  onReviewed,
}) => {
  const [busy, setBusy] = React.useState(false);
  const [failed, setFailed] = React.useState(false);
  const [confirm, setConfirm] = React.useState<DeckReviewConfirmState>(idleDeckReviewConfirm);
  const w = (key: string) => wordingOf(wording, key);

  // Every transition goes through the machine, including this one. The page
  // re-reads the deck while the question may be open — a note arrives, Marie
  // answers another — and the machine's `refresh` arm is what decides that the
  // question SURVIVES holding the number it asked about. Routed rather than
  // ignored so the closed-union proof covers a real path and not a fiction.
  const awaiting = review.awaiting;
  React.useEffect(() => {
    setConfirm(
      (open) => deckReviewConfirmStep(open, { type: "refresh", count: awaiting }, awaiting).state,
    );
  }, [awaiting]);

  // Escape closes the question wherever focus is — the button, a link, the bar.
  // Bound to `document` and removed on unmount, the pattern `Modal.tsx` sets: a
  // listener left behind would answer for a bar that is no longer on screen.
  // Never while the write is in flight: the request has gone and closing the
  // question would suggest it had not.
  const open = confirm.phase === "confirming";
  React.useEffect(() => {
    if (!open || busy) return undefined;
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      setConfirm((was) => deckReviewConfirmStep(was, { type: "dismiss" }, awaiting).state);
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [open, busy, awaiting]);

  if (review.awaiting === 0) return null;

  /** Dispatch one action; write only if the machine says to. */
  const act = (action: DeckReviewConfirmAction) => {
    const step = deckReviewConfirmStep(confirm, action, review.awaiting);
    setConfirm(step.state);
    // The ONE call site in the application that moves the review mark, and it
    // is reachable only through `step.send`. A control added to this bar later
    // cannot write by being wired to `markDeckReviewed` directly — there is
    // nothing here to wire it to.
    if (!step.send) return;
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

  const question = deckReviewConfirmSentence({
    state: confirm,
    one: w("deck_review_confirm_one"),
    many: w("deck_review_confirm_template"),
    code,
  });

  return (
    <div style={r.reviewBar} data-review-bar>
      <span style={r.reviewCount}>{reviewBarLine(review, wording)}</span>
      {/* Only the reviewer is offered the button — the server decided (rule 12).
          Everyone else sees the same count and nothing to press.
          It OPENS the question; it does not write. See `deckReviewConfirmModel`. */}
      {review.can_mark_reviewed && (
        <button
          type="button"
          style={s.buttonPrimary}
          disabled={busy || question !== null}
          onClick={() => act({ type: "open" })}
        >
          {w("deck_review_done_label")}
        </button>
      )}
      {question !== null && (
        // `role="alertdialog"`: this interrupts to ask a question with
        // consequences, and a screen reader reaching the bar should hear the
        // question rather than find two unexplained buttons. Not a modal —
        // nothing is trapped, and the deck behind it is where somebody checks
        // what they are about to sweep.
        <div style={r.reviewConfirm} role="alertdialog" aria-label={question} data-review-confirm>
          <span style={r.reviewConfirmSentence}>{question}</span>
          <button
            type="button"
            style={r.reviewConfirmYes}
            disabled={busy}
            onClick={() => act({ type: "confirm" })}
          >
            {w("deck_review_confirm_yes_label")}
          </button>
          <button
            type="button"
            style={r.reviewConfirmCancel}
            disabled={busy}
            onClick={() => act({ type: "cancel" })}
          >
            {w("deck_review_confirm_cancel_label")}
          </button>
        </div>
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
