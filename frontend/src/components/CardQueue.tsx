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
// DEDICATED viewer at the cited page in a new tab, which is where a page is
// actually readable. The queue keeps the whole width for the card.

import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";

import {
  fetchScenarioCards,
  type ScenarioCard,
  type ScenarioCardsResponse,
} from "../services/scenarioCards";
import { applyFactAction } from "../services/scenarioGather";
import { listScenarioFacts, type ScenarioFactDto } from "../services/scenarioFacts";
import {
  cardRows,
  DEFER_QUICK_REASONS,
  initialQueueState,
  progress,
  queueReducer,
  type CardRow,
  type QueueEvent,
  type QueueState,
} from "./cardTriage";

// ─── §2c visual language ────────────────────────────────────────────────────

const SURFACE = "var(--bg-surface)"; // #ffffff — pure white, per §2c
const HAIRLINE = "1px solid var(--border-default)";

// One column since task 1.7B. It was a two-column split with a PDF pane on the
// right; see the header for why that is gone.
const shellStyle: React.CSSProperties = {
  background: SURFACE,
  padding: "1rem",
};

const cardStyle: React.CSSProperties = {
  background: SURFACE,
  border: HAIRLINE,
  borderRadius: "8px",
  padding: "1rem 1.15rem",
  display: "flex",
  flexDirection: "column",
  gap: "0.6rem",
  fontWeight: 400, // §2c: regular weight; bold is reserved
};

const focusedCardStyle: React.CSSProperties = {
  ...cardStyle,
  borderColor: "var(--accent-primary)",
};

const chipStyle: React.CSSProperties = {
  border: HAIRLINE,
  borderRadius: "999px",
  padding: "0.1rem 0.55rem",
  fontSize: "0.75rem",
  color: "var(--text-muted)",
  whiteSpace: "nowrap",
};

const quoteStyle: React.CSSProperties = {
  fontSize: "1rem",
  lineHeight: 1.7, // §2c: generous line height
  color: "var(--text-primary)",
};

const contextStyle: React.CSSProperties = {
  color: "var(--text-muted)",
  fontSize: "0.85rem",
  lineHeight: 1.7,
};

const hintBarStyle: React.CSSProperties = {
  display: "flex",
  gap: "1rem",
  alignItems: "center",
  flexWrap: "wrap",
  padding: "0.5rem 0",
  fontSize: "0.8rem",
  color: "var(--text-muted)",
};

// ─── Row rendering ──────────────────────────────────────────────────────────

/**
 * Render one descriptor row.
 *
 * The `element` decides the presentation; the `value` is displayed verbatim.
 * This function chooses no words — a `switch` that composed prose here would be
 * the frontend inventing vocabulary, which the language law forbids.
 */
const Row: React.FC<{ row: CardRow }> = ({ row }) => {
  if (row.element === "pinpoint") {
    return (
      <div>
        <a
          href={row.href}
          target="_blank"
          rel="noreferrer"
          style={{ ...chipStyle, color: "var(--accent-primary)", textDecoration: "none" }}
        >
          {row.value} ↗
        </a>
      </div>
    );
  }

  return (
    <div style={{ display: "flex", gap: "0.4rem", alignItems: "baseline", flexWrap: "wrap" }}>
      <span
        style={
          row.element === "quote"
            ? quoteStyle
            : { fontSize: "0.85rem", color: "var(--text-primary)" }
        }
      >
        {row.value}
      </span>
      {row.chips?.map((chip) => (
        <span key={chip} style={chipStyle}>
          {chip}
        </span>
      ))}
    </div>
  );
};

/** One candidate card in the Casey layout. */
const Card: React.FC<{ card: ScenarioCard; focused: boolean }> = ({ card, focused }) => {
  const rows = useMemo(() => cardRows(card), [card]);
  const code = rows.find((r) => r.element === "code");
  const status = rows.find((r) => r.element === "status");
  const body = rows.filter((r) => r.element !== "code" && r.element !== "status");

  return (
    <div style={focused ? focusedCardStyle : cardStyle}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline" }}>
        <span style={{ ...chipStyle, color: "var(--text-primary)" }}>{code?.value ?? "—"}</span>
        <span style={{ fontSize: "0.75rem", color: "var(--text-muted)" }}>{status?.value}</span>
      </div>

      {/* §7.1 — the question that makes a bare answer interpretable, then the
          quote in its surrounding text. */}
      {card.quote.question && <div style={contextStyle}>{card.quote.question}</div>}
      <div>
        {card.quote.context_before && (
          <span style={contextStyle}>{card.quote.context_before}</span>
        )}
        <span style={quoteStyle}>{card.quote.text}</span>
        {card.quote.context_after && <span style={contextStyle}>{card.quote.context_after}</span>}
      </div>

      {body
        .filter((r) => r.element !== "quote")
        .map((row, i) => (
          <Row key={`${row.element}-${i}`} row={row} />
        ))}

      {/* A reason a HUMAN gave, distinct from the system's defer notice above. */}
      {card.defer_reason && (
        <div style={{ ...contextStyle, fontStyle: "italic" }}>Parked: {card.defer_reason}</div>
      )}
    </div>
  );
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

const CardQueue: React.FC<Props> = ({ slug, scenarioId, externalRefresh }) => {
  const [setAside, setSetAside] = useState<ScenarioCard[]>([]);
  const [orphans, setOrphans] = useState<ScenarioFactDto[]>([]);
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

      // The ORPHAN GUARANTEE, carried forward from the workbench: the card
      // endpoint is pool-driven, so a saved ref whose graph node has vanished is
      // simply absent from it. Left unhandled a confirmed fact would silently
      // disappear — the ratified policy this second read exists to uphold.
      const saved = await listScenarioFacts(slug, scenarioId);
      const known = new Set([
        ...cards.pool.map((c) => c.graph_node_id),
        ...cards.set_aside.map((c) => c.graph_node_id),
      ]);
      setOrphans(saved.filter((f) => !known.has(f.graph_node_id)));
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
  }, [dispatch]);

  // Focus the defer field the moment the prompt opens, so the human never
  // reaches for the mouse mid-triage.
  useEffect(() => {
    if (state.mode.kind === "deferring") deferInputRef.current?.focus();
  }, [state.mode.kind]);

  const card = state.cards[state.index];
  const { ruled, total } = progress(state);

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
      <div style={hintBarStyle}>
        <strong style={{ fontWeight: 600 }}>
          {ruled} of {total} ruled
        </strong>
        <span>I include</span>
        <span>E exclude</span>
        <span>D defer</span>
        <span>U undo</span>
        <button
          type="button"
          onClick={() => setShowDeferred((v) => !v)}
          style={{ ...chipStyle, cursor: "pointer", background: SURFACE }}
        >
          {showDeferred ? "Back to queue" : `Deferred (${deferredOf(state).length})`}
        </button>
      </div>

      {showDeferred ? (
        <DeferQueue cards={deferredOf(state)} />
      ) : (
        <div style={shellStyle}>
          <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
            {card ? (
              <>
                <Card card={card} focused />
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

      {orphans.length > 0 && <OrphanStrip orphans={orphans} />}
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
        <Card key={card.graph_node_id} card={card} focused={false} />
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

/**
 * Saved facts the pool no longer knows about — the orphan guarantee.
 *
 * They cannot be ruled on (the graph node is gone), so they sit outside the
 * triage flow rather than in it. Showing them is the point: a confirmed fact
 * must never silently disappear.
 */
const OrphanStrip: React.FC<{ orphans: ScenarioFactDto[] }> = ({ orphans }) => (
  <div style={{ padding: "1rem", borderTop: HAIRLINE }}>
    <div style={{ fontSize: "0.8rem", color: "var(--state-warning-strong)" }}>
      {orphans.length} saved fact{orphans.length === 1 ? "" : "s"} no longer in the graph — the
      content is unavailable, but the reference is kept so nothing is lost silently.
    </div>
    <ul style={{ margin: "0.4rem 0 0", paddingLeft: "1.1rem", fontSize: "0.8rem" }}>
      {orphans.map((o) => (
        <li key={o.graph_node_id} style={{ color: "var(--text-muted)" }}>
          {o.graph_node_id}
        </li>
      ))}
    </ul>
  </div>
);

/** The cards a human has parked, read off the payload. */
function deferredOf(state: QueueState): ScenarioCard[] {
  return state.cards.filter((c) => c.defer_reason != null);
}

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
function useReducerWithEffects(
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

export default CardQueue;
