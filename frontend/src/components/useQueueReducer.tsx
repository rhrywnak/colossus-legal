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

      if (effect.kind !== "rule") return;

      applyFactAction(slug, scenarioId, effect.graphNodeId, effect.action, effect.reason).catch(
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
    [slug, scenarioId, onRulingFailed],
  );

  return [state, dispatch];
}

