// =============================================================================
// useQueueReducer — wiring the pure §7 reducer to the backend (extracted 1.7C)
// =============================================================================
//
// `cardTriage.queueReducer` is pure: it takes the queue state and an event and
// returns the next state plus a DESCRIPTION of the call to make. This hook is the
// half that performs those calls, and it is deliberately the only impure part of
// the triage path.
//
// ## Why it is its own file
//
// `CardQueue.tsx` was 420 non-comment lines before task 1.7C and 318 after, against
// a 300-line limit (Rule 17). The seam is real rather than arithmetic: everything
// here is about REACHING THE BACKEND and reconciling when it refuses, while what
// remains in `CardQueue` is about what is on screen. Two different reasons to
// change, two files.
//
// The reducer's own 31 tests live with the reducer and are untouched by this move.

import { useCallback, useRef, useState } from "react";

import { applyFactAction } from "../services/scenarioGather";
import { fillDetail, saveLinks, unlink } from "../services/evidenceLinks";
import type { LinkPanelWording } from "../services/evidenceLinks";
import {
  initialQueueState,
  queueReducer,
  type QueueEvent,
  type QueueState,
} from "./cardTriage";

/**
 * The reducer, wired so its effects reach the backend.
 *
 * ## Why the effect is performed OUTSIDE the state updater
 *
 * `queueReducer` is pure and returns a description of the call to make. Firing
 * that call from inside a `setState` updater would be a side effect in a function
 * React may invoke more than once (it does, in StrictMode) — the same ruling would
 * post twice. So the reducer is run against a ref, the state is set, and the
 * effect is performed after, exactly once.
 *
 * ## Why a failed ruling must do TWO things
 *
 * The UI advances optimistically, which is what makes triage fast. That is only
 * honest if a refusal is caught: the human has already moved on, so a failure has
 * to (1) SAY so, naming the card, and (2) RECONCILE by re-reading the pool, so
 * what is on screen is what the database holds. Logging to the console satisfies
 * neither — the audience that must act is the person ruling, not a developer with
 * devtools open (Standing Rule 1).
 */
export function useReducerWithEffects(
  slug: string,
  scenarioId: string,
  onRulingFailed: (message: string) => void,
  /**
   * Called once the SERVER has confirmed a ruling (task 1.7F Part A).
   *
   * The facts section below the queue re-reads itself from this, so an included
   * card appears there without a page reload — and only after the write is
   * durable. No optimistic row (ruling R3): the queue may advance on a promise,
   * because a refusal reconciles it, but a row in the facts list is a claim that
   * something IS stored, and the 1.3 gate found what that costs when it is not.
   */
  onRulingSaved: () => void,
  /**
   * Called after a link or unlink has been attempted (task 2.10).
   *
   * Re-reads the pool, on success AND on failure. On success because everything a
   * linked card shows is composed server-side; on failure because the panel has
   * already cleared itself and the screen must go back to matching the database.
   */
  onLinksChanged: () => void,
  /**
   * The stored words this hook needs to report a link write (task 2.10, R4).
   *
   * `null` until the scenario's panel wording has loaded — and no link effect can
   * fire before then, because the control that produces one is not rendered.
   */
  linkWording: LinkPanelWording | null,
): [QueueState, (event: QueueEvent) => void] {
  const [state, setState] = useState<QueueState>(() => initialQueueState([]));

  // The reducer needs the CURRENT state, and `setState` is async — a ref keeps
  // the two in step without making `dispatch` depend on `state` (which would
  // rebuild the keydown listener on every keystroke).
  const stateRef = useRef(state);
  stateRef.current = state;

  const dispatch = useCallback(
    (event: QueueEvent) => {
      const { state: next, effect } = queueReducer(stateRef.current, event);
      stateRef.current = next;
      setState(next);

      // Task 2.10: a link is not a ruling — it changes what MAY be ruled — so it
      // has its own performer and its own sentence when it fails. It also does
      // NOT report through `onRulingSaved`: that callback tells the facts section
      // a ruling landed, and a link has not put anything in the facts list. What a
      // link needs is a full re-read of the pool, because the card's sentence, its
      // chips, its unlocked buttons and the progress line are all composed
      // server-side from the new state (ruling R2 forbids composing them here).
      if (effect.kind === "link" || effect.kind === "unlink") {
        // Every sentence below comes from the settings store (R4). `linkWording`
        // is non-null by construction whenever a link effect can fire: the panel
        // that produces one is not rendered until the wording has loaded. The
        // guard is a type narrowing, not a fallback — there is no literal here to
        // fall back TO.
        const said = linkWording;
        const failed = (e: unknown) => {
          const detail = e instanceof Error ? e.message : String(e);
          if (said) onRulingFailed(fillDetail(said.save_failed_template, detail));
          // Reconcile as well as say so. Without this the panel would clear on a
          // save that never happened, and the card would look linked until
          // something else reloaded it.
          onLinksChanged();
        };

        if (effect.kind === "unlink") {
          unlink(slug, effect.graphNodeId, effect.allegationId).then((result) => {
            // `removed: false` means the pair was NOT linked — the view was stale.
            // Silently reloading would make "I removed your link" and "there was
            // nothing there" the same observable, which is the distinction the
            // backend goes to the trouble of reporting (Standing Rule 1).
            if (!result.removed && said) onRulingFailed(said.unlink_found_nothing);
            onLinksChanged();
          }, failed);
          return;
        }

        saveLinks(slug, effect.graphNodeId, effect.allegationIds, effect.cut).then(
          () => onLinksChanged(),
          failed,
        );
        return;
      }

      if (effect.kind !== "rule") return;

      // Two-argument `then` rather than `.then().catch()`, deliberately: with a
      // trailing `.catch` an exception thrown INSIDE the success callback would
      // be reported to the human as "that ruling did not save", about a ruling
      // that saved perfectly. The rejection handler here sees only the request's
      // own failure.
      applyFactAction(slug, scenarioId, effect.graphNodeId, effect.action, effect.reason).then(
        () => onRulingSaved(),
        (e: unknown) => {
          const detail = e instanceof Error ? e.message : String(e);
          onRulingFailed(
            `That ruling did not save (${effect.action} on ${effect.graphNodeId}): ` +
              `${detail} The queue has been reloaded from the server, so what you ` +
              `see now is what is stored.`,
          );
        },
      );
    },
    [slug, scenarioId, onRulingFailed, onRulingSaved, onLinksChanged, linkWording],
  );

  return [state, dispatch];
}

