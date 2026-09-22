// =============================================================================
// useReadClear.ts — opening a question from the list clears it (ruling Q4)
// =============================================================================
//
// CC_TASK_FOR_YOU_v1 L1. A hook rather than thirty lines inside
// `PracticeQuestionPage.tsx`, which Rule 17 put at 306 the moment they were
// added there. The seam is also right: this is one complete behaviour with one
// piece of state, and the page only has to say where its sentence goes.
//
// ## What "opening" clears, and what it does NOT
//
// Everything on the question — her current answer, every note standing on it,
// what has changed about it — because all of it is in view. Otherwise a
// question carrying three notes would take three visits to go quiet.
//
// And ONLY when the address says the reader arrived from the list. Arriving
// from the deck, from a bookmark or from a discussion link must not silently
// mark somebody's notes as read; that is why the list composes its links with
// `?from=for-you` and why this reads the query rather than firing on mount.

import React from "react";

import { useForYou } from "../../context/ForYouContext";
import { markQuestionSeen } from "../../services/forYou";

/** The query value the For you list puts in its links. */
// STRUCTURAL: one half of a contract with `routePaths::practiceQuestionPath`,
// which writes the other. Not a deployment value — changing it means changing
// both, together.
const FROM_FOR_YOU = "for-you";

/**
 * Mark this question read when it was opened from the For you list.
 *
 * Returns whether that write FAILED, so the page can say so. It is a line on
 * the page and not a barrier: the question is on screen and readable either
 * way, and what the line says is the true consequence — the row is still
 * waiting on her list.
 */
export function useReadClear(questionId: string, search: string): boolean {
  const { refresh } = useForYou();
  const [failed, setFailed] = React.useState(false);
  const fromForYou = new URLSearchParams(search).get("from") === FROM_FOR_YOU;

  React.useEffect(() => {
    if (!fromForYou || questionId === "") return;
    markQuestionSeen(questionId)
      .then(() => {
        // The menu count is now stale by however many rows this cleared, and
        // the server is the only thing that knows the new number.
        refresh();
      })
      .catch((cause: unknown) => {
        // eslint-disable-next-line no-console
        console.error(`for you: question ${questionId} could not be marked as read`, cause);
        setFailed(true);
      });
  }, [fromForYou, questionId, refresh]);

  return failed;
}
