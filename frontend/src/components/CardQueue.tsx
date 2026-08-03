// =============================================================================
// CardQueue — keyboard triage over the §7 candidate cards (task 1.3)
// =============================================================================
//
// The surface where a human clears a 148-candidate pool in one sitting. The
// proven pattern (Rayyan one-key screening, Relativity save-and-advance) in the
// Casey card layout: I include · E exclude · D defer · U undo, auto-advance,
// single-step undo, a defer queue with visible reasons, a running count, and no
// page navigation to rule.
//
// ## This component renders and does nothing else
//
// Every string on screen comes from the 1.2 payload. All the logic — which rows
// a card shows, what a key does, when the defer prompt opens — lives in the pure
// `cardTriage` module and is tested there. This file is the JSX that walks the
// descriptor and the wiring that performs the reducer's effects.
//
// ## Visual language (§2c, binding)
//
// Pure white surfaces, hairline borders, regular weight with bold reserved for
// the pinpoint page, one accent. Born compliant, and as of 1.7A the app shell
// they sit in is white too.
//
// ## No PDF renders here (task 1.7B, defect D2)
//
// A split-pane viewer used to sit beside the focused card, on the theory that
// verifying a quote against its page should not leave the queue. In practice it
// rendered every page from the cited one to the end of the document, stacked —
// and a zoomed legal page in half a column is unreadable anyway, which is the
// ruling that retired it (Roman, 2026-08-02: popup-only document viewing).
//
// The pinpoint stays a first-class element: `card.pinpoint.viewer_href` opens the
// DEDICATED viewer at the cited page, which is where a page is actually readable.
// The queue keeps the whole width for the card.
//
// ## The viewer is a WINDOW, not a tab (task 1.7C, defect D5)
//
// It was `<a target="_blank">` in 1.7B. Roman's ruling: a pinpoint opens a real
// separate, sized window, because reading a quote against its page means having
// both visible at once and a tab hides the page. See `viewerWindow.ts` for the
// geometry, the named-window reuse, and why `noopener` is deliberately absent.
//
// ## The orphan strip LEFT this component (task 1.7C, defect D9)
//
// It used to hang off the bottom of the queue, derived in the browser by
// set-differencing the saved facts against the card pool. Design §2.8 moves it to
// the bottom of the PAGE as a humane, grouped disclosure fed by its own endpoint
// (`/facts/orphans`), which can resolve document titles the browser cannot see.
//
// The orphan guarantee is unchanged — a confirmed fact must never silently
// disappear — it is simply upheld somewhere better. The second `listScenarioFacts`
// read that existed only to support the strip is gone with it.

import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";

import {
  fetchScenarioCards,
  type ScenarioCard,
  type ScenarioCardsResponse,
} from "../services/scenarioCards";
import { CandidateCard, cardStyle, chipStyle } from "./CandidateCard";
import { useReducerWithEffects } from "./useQueueReducer";
import { DEFER_QUICK_REASONS, progress, type QueueState } from "./cardTriage";
import { keyboardShouldRule, nextUpHint } from "./queueRegion";

// ─── §2c visual language ────────────────────────────────────────────────────

const SURFACE = "var(--bg-surface)"; // #ffffff — pure white, per §2c
const HAIRLINE = "1px solid var(--border-default)";

// One column since task 1.7B. It was a two-column split with a PDF pane on the
// right; see the header for why that is gone.
const shellStyle: React.CSSProperties = {
  background: SURFACE,
  padding: "1rem",
};






// The anchor quote, highlighted inside its context (§2c's mockup, task 1.7C).
// `mark` rather than a bold span: the quote is the thing being ruled on, and a
// highlight is what a reader's eye finds without the weight bold would add — §2c
// reserves bold for true emphasis and this is a different job.

const hintBarStyle: React.CSSProperties = {
  display: "flex",
  gap: "1rem",
  alignItems: "center",
  flexWrap: "wrap",
  padding: "0.5rem 0",
  fontSize: "0.8rem",
  color: "var(--text-muted)",
};

// ─── The queue ──────────────────────────────────────────────────────────────

interface Props {
  slug: string;
  scenarioId: string;
  /** Bumped by the parent when something OUTSIDE the queue changed the candidate
   *  set — today, a Merge applied in the Theme Scan panel. Any change re-fetches,
   *  so merged suggestions appear without a manual reload. Carried forward from
   *  the workbench this queue replaces. */
  externalRefresh?: number;
  /**
   * Whether the keyboard may rule (task 1.7C, ruling R7).
   *
   * The queue now lives inside a collapsible region (§2.3), and a `<details>` body
   * stays in the DOM when closed — so without this the one-key rulings would keep
   * firing on a card nobody can see. Defaults to `true` so a queue mounted outside
   * a region behaves exactly as it did before.
   *
   * The guard is HERE and not in `queueReducer`: the reducer is a pure state
   * machine that knows nothing about chrome, and its 31 tests stay byte-identical.
   */
  keyboardActive?: boolean;
  /**
   * Reports `{ruled, total}` upward so the section above can draw the progress bar
   * and word its summary line.
   *
   * The queue owns the fetch, so it owns the counts; the alternative was a second
   * read of the same pool in the parent, which is how two surfaces end up
   * disagreeing about how much work is left.
   */
  onProgress?: (progress: { ruled: number; total: number }) => void;
}

/**
 * Whether a keyboard event originated inside a text field.
 *
 * The typing guard: without it, writing a defer reason would rule half the pool.
 * Checked here (at the DOM boundary) and passed into the reducer as a plain
 * boolean, so the state machine stays testable without a browser.
 */
function isTyping(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  if (!el) return false;
  const tag = el.tagName?.toLowerCase();
  return tag === "input" || tag === "textarea" || el.isContentEditable === true;
}

const CardQueue: React.FC<Props> = ({
  slug,
  scenarioId,
  externalRefresh,
  keyboardActive = true,
  onProgress,
}) => {
  const [setAside, setSetAside] = useState<ScenarioCard[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [rulingError, setRulingError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [showDeferred, setShowDeferred] = useState(false);
  const deferInputRef = useRef<HTMLInputElement | null>(null);

  // `load` is defined below and the failure handler needs it, so the handler
  // reaches it through a ref — a plain closure would capture the first `load`.
  const loadRef = useRef<() => Promise<void>>(async () => {});

  const onRulingFailed = useCallback((message: string) => {
    setRulingError(message);
    // RECONCILE: re-read the pool so the screen shows what the database holds,
    // not what the optimistic advance assumed.
    void loadRef.current();
  }, []);

  const [state, dispatch] = useReducerWithEffects(slug, scenarioId, onRulingFailed);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const cards: ScenarioCardsResponse = await fetchScenarioCards(slug, scenarioId);
      dispatch({ type: "cards_loaded", cards: cards.pool });
      setSetAside(cards.set_aside);

      // The ORPHAN GUARANTEE moved to the page-bottom strip in task 1.7C (§2.8):
      // it is served by `/facts/orphans`, which groups the losses by document and
      // can resolve titles this component could not see. The second
      // `listScenarioFacts` read that lived here only to support the old strip is
      // gone with it — the guarantee is upheld, in a better place.
      setError(null);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Failed to load the candidate queue.");
    } finally {
      setLoading(false);
    }
  }, [slug, scenarioId, dispatch]);

  useEffect(() => {
    loadRef.current = load;
  }, [load]);

  useEffect(() => {
    void load();
    // `externalRefresh` is in the dependency list precisely so a Theme Scan merge
    // re-reads the pool; its VALUE is never used, only its change.
  }, [load, externalRefresh]);

  // One document-level listener while the queue is mounted, removed on unmount —
  // the house pattern (see `AuthorityPopover`).
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      // The collapsed-region guard (ruling R7). A `<details>` body stays in the
      // DOM when closed, so without this the one-key rulings would keep firing on
      // a card nobody can see — and the human would have no way to tell that from
      // a stray keypress. Checked BEFORE `preventDefault` so a collapsed queue does
      // not even swallow the key.
      if (!keyboardShouldRule(keyboardActive)) return;

      const typing = isTyping(e.target);
      // Only swallow the browser's default for keys we actually act on, and
      // never while typing — Escape and Enter belong to the field then.
      if (!typing && ["i", "e", "d", "u"].includes(e.key.toLowerCase())) {
        e.preventDefault();
      }
      dispatch({ type: "key", key: e.key, typing });
    }
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [dispatch, keyboardActive]);

  // Focus the defer field the moment the prompt opens, so the human never
  // reaches for the mouse mid-triage.
  useEffect(() => {
    if (state.mode.kind === "deferring") deferInputRef.current?.focus();
  }, [state.mode.kind]);

  const card = state.cards[state.index];
  const { ruled, total } = progress(state);
  const nextCard = state.cards[state.index + 1];

  // Report the counts to the section above, which draws the progress bar and words
  // the region's summary. In an effect rather than inline: calling a parent's
  // setState during render is the React warning that turns into an infinite loop.
  useEffect(() => {
    onProgress?.({ ruled, total });
  }, [onProgress, ruled, total]);

  if (loading) return <div style={{ padding: "1rem" }}>Loading the candidate queue…</div>;

  if (error) {
    // Explicit error UI, never a silent empty queue (Standing Rule 1).
    return (
      <div style={{ padding: "1rem", color: "var(--state-danger-strong)" }}>
        <div>{error}</div>
        <button type="button" onClick={() => void load()} style={{ marginTop: "0.5rem" }}>
          Retry
        </button>
      </div>
    );
  }

  return (
    <div style={{ background: SURFACE }}>
      {rulingError && (
        <div
          role="alert"
          style={{
            border: HAIRLINE,
            borderColor: "var(--state-danger-strong)",
            borderRadius: "8px",
            padding: "0.6rem 0.8rem",
            margin: "0.5rem 0",
            color: "var(--state-danger-strong)",
            fontSize: "0.85rem",
          }}
        >
          {rulingError}
          <button
            type="button"
            onClick={() => setRulingError(null)}
            style={{ ...chipStyle, marginLeft: "0.6rem", cursor: "pointer", background: SURFACE }}
          >
            Dismiss
          </button>
        </div>
      )}
      {/* The running count and the I/E/D/U legend moved UP into the region's
          summary line (§2.3) so they are readable while the queue is collapsed.
          What belongs beside the card is the deferred tray and what is coming
          next. */}
      <div style={hintBarStyle}>
        <button
          type="button"
          onClick={() => setShowDeferred((v) => !v)}
          style={{ ...chipStyle, cursor: "pointer", background: SURFACE }}
        >
          {showDeferred ? "Back to queue" : `Deferred (${deferredOf(state).length})`}
        </button>
        {/* D10's next-up hint. Absent rather than "Next up: —" when the following
            card has no ordinal yet: a hint that names nothing is worse than none. */}
        {!showDeferred && nextUpHint(nextCard?.code) && (
          <span style={{ marginLeft: "auto", color: "var(--text-muted)" }}>
            {nextUpHint(nextCard?.code)}
          </span>
        )}
      </div>

      {showDeferred ? (
        <DeferQueue cards={deferredOf(state)} />
      ) : (
        <div style={shellStyle}>
          <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
            {card ? (
              <>
                <CandidateCard card={card} focused />
                {state.mode.kind === "deferring" && (
                  <DeferPrompt
                    inputRef={deferInputRef}
                    draft={state.mode.draft}
                    onDraft={(draft) => dispatch({ type: "defer_draft", draft })}
                  />
                )}
              </>
            ) : (
              <div style={cardStyle}>Nothing left in this queue.</div>
            )}
          </div>
        </div>
      )}

      {setAside.length > 0 && !showDeferred && (
        <div style={{ padding: "0.75rem 1rem", fontSize: "0.8rem", color: "var(--text-muted)" }}>
          {setAside.length} set aside.
        </div>
      )}

    </div>
  );
};

/** Cards a human has parked, with the reason they gave. */
const DeferQueue: React.FC<{ cards: ScenarioCard[] }> = ({ cards }) => {
  if (cards.length === 0) {
    return <div style={{ padding: "1rem", color: "var(--text-muted)" }}>Nothing deferred.</div>;
  }
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem", padding: "1rem" }}>
      {cards.map((card) => (
        <CandidateCard key={card.graph_node_id} card={card} focused={false} />
      ))}
    </div>
  );
};

/** The inline defer prompt: quick picks, free text, Enter commits, Esc cancels. */
const DeferPrompt: React.FC<{
  draft: string;
  onDraft: (draft: string) => void;
  inputRef: React.RefObject<HTMLInputElement>;
}> = ({ draft, onDraft, inputRef }) => (
  <div style={{ ...cardStyle, gap: "0.5rem" }}>
    <div style={{ fontSize: "0.8rem", color: "var(--text-muted)" }}>
      Why defer this? Enter commits · Esc cancels
    </div>
    <div style={{ display: "flex", gap: "0.4rem", flexWrap: "wrap" }}>
      {DEFER_QUICK_REASONS.map((reason, i) => (
        <button
          key={reason}
          type="button"
          onClick={() => onDraft(reason)}
          style={{ ...chipStyle, cursor: "pointer", background: SURFACE }}
        >
          {i + 1}. {reason}
        </button>
      ))}
    </div>
    <input
      ref={inputRef}
      value={draft}
      onChange={(e) => onDraft(e.target.value)}
      placeholder="or type a reason"
      style={{ border: HAIRLINE, borderRadius: "6px", padding: "0.4rem 0.6rem", fontWeight: 400 }}
    />
  </div>
);

/** The cards a human has parked, read off the payload. */
function deferredOf(state: QueueState): ScenarioCard[] {
  return state.cards.filter((c) => c.defer_reason != null);
}

export default CardQueue;
