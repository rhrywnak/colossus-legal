// =============================================================================
// CardQueue — the candidate list and its keyboard (tasks 1.3, 1.7E-a)
// =============================================================================
//
// The surface where a human clears a 148-candidate pool in one sitting. The proven
// pattern (Rayyan one-key screening, Relativity save-and-advance) in the Casey card
// layout: I include · E exclude · D defer · U undo, auto-advance, single-step undo,
// a running count, and no page navigation to rule.
//
// ## From a queue to a LIST (task 1.7E-a)
//
// It showed one card at a time until 1.7E. Roman's finding, twice stated: that is
// unusable for working a pool of 148 — no skip forward, no skip back, and no way to
// see where the ~24 rulable candidates sit. So the queue is now a scrollable,
// filterable list:
//
//   * `CandidateFilterBar` — the Status and Scan selects, and the counter line
//   * `CandidateList`      — every card in the filter, one of them selected
//   * `cardTriage`         — what a key does, and what a card's own buttons do
//   * `candidateFilters`   — which cards a filter leaves, and how many of each
//
// ## Task 1.7G — every card rules itself, and the filters are dropdowns
//
// Roman worked a real 148-candidate pool on beta.369 and found the list unusable
// in two ways this task fixes:
//
//   * Only the first card could be acted on. Two mechanisms together: buttons
//     rendered on the SELECTED card alone, and every ruling resolved its target
//     through one shared index — so a mouse ruling also fired the card's select
//     handler, destroyed the auto-advance, and left the human at the top of the
//     list. Both are gone: a ruling now names its card (`{type: "rule"}`), the
//     buttons stop their own clicks from selecting, and the highlight lands
//     relative to the card that was RULED (ruling R2).
//   * The filter was two anonymous rows of count-pills. They are replaced by the
//     Bias Analysis page's pattern — two labelled selects and a Clear filters
//     link — which is what Roman asked for twice before 1.7E built pills.
//
// ## The deferred TRAY is gone, and it was not dropped
//
// A "Deferred (n)" button used to swap the whole queue for a read-only tray of
// parked cards. That is now a Status facet beside the other five, which is strictly
// more: the same cards, in the same list, still selectable and still rulable — a
// parked card can be picked up again without leaving the view it is listed in.
// Keeping both would have meant two surfaces answering "what is deferred?", and
// they would eventually disagree.
//
// ## This component fetches, wires, and renders
//
// Every string on screen comes from the 1.2 payload. Which cards a filter leaves
// and which state each is in are pure functions in `candidateFilters`; what a key
// does is a pure reducer in `cardTriage`. This file is the fetch, the keyboard
// boundary, and the JSX that puts the three together.
//
// ## No PDF renders here (task 1.7B, defect D2)
//
// A split-pane viewer used to sit beside the focused card, on the theory that
// verifying a quote against its page should not leave the queue. In practice it
// rendered every page from the cited one to the end of the document, stacked — and
// a zoomed legal page in half a column is unreadable anyway, which is the ruling
// that retired it (Roman, 2026-08-02: popup-only document viewing).
//
// The pinpoint stays a first-class element: `card.pinpoint.viewer_href` opens the
// DEDICATED viewer at the cited page, in a real sized WINDOW rather than a tab
// (task 1.7C, defect D5 — see `viewerWindow.ts`).

import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";

import {
  fetchScenarioCards,
  type ScenarioCard,
  type ScenarioCardsResponse,
} from "../services/scenarioCards";
import { chipStyle } from "./CandidateCard";
import CandidateFilterBar from "./CandidateFilterBar";
import CandidateList from "./CandidateList";
import { useReducerWithEffects } from "./useQueueReducer";
import { DEFER_QUICK_REASONS, progress } from "./cardTriage";
import {
  candidateCounts,
  defaultFilters,
  filterCandidates,
  hasAnyFilter,
  UNFILTERED,
  type CandidateFilters,
} from "./candidateFilters";
import { keyboardShouldRule, nextUpHint } from "./queueRegion";
import type { AllegationOptions } from "../services/evidenceLinks";
import { revertQuestionOverride, saveQuestionOverride } from "../services/evidenceSummary";

// ─── §2c visual language ────────────────────────────────────────────────────

const SURFACE = "var(--bg-surface)"; // #ffffff — pure white, per §2c
const HAIRLINE = "1px solid var(--border-default)";

const hintBarStyle: React.CSSProperties = {
  display: "flex",
  gap: "1rem",
  alignItems: "center",
  flexWrap: "wrap",
  padding: "0.25rem 0",
  fontSize: "0.8rem",
  color: "var(--text-muted)",
};

// CONST: the keys the list acts on. Listed once so the preventDefault guard and
// the reducer cannot disagree about which keystrokes the queue owns.
//
// These are UI affordances rather than configuration, on the same reasoning as
// `DEFER_QUICK_REASONS`: I/E/D/U is the one-key screening convention the study's
// tools ship (Rayyan, Relativity), and the letters are baked into the button
// labels, their `aria-label`s and the `<kbd>` chips beside them. A remapping
// setting would have to rewrite all of those to stay honest, so its only real
// effect would be to let a human make the screen lie about its own controls.
//
// They are also case-agnostic: nothing here names a party, a document or a
// claim, so another Colossus case runs them unchanged (Standing Rule 2).
const RULING_KEYS = ["i", "e", "d", "u"];
const NAVIGATION_KEYS = ["arrowup", "arrowdown", "j", "k"];

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
   * The queue lives inside a collapsible region (§2.3), and a `<details>` body
   * stays in the DOM when closed — so without this the one-key rulings would keep
   * firing on a card nobody can see. Defaults to `true` so a queue mounted outside
   * a region behaves exactly as it did before.
   *
   * The guard is HERE and not in `queueReducer`: the reducer is a pure state
   * machine that knows nothing about chrome.
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
  /**
   * Called when the SERVER confirms a ruling (task 1.7F Part A).
   *
   * The page re-reads its cards from this, so a newly included candidate shows up
   * in the facts section without a reload. Deliberately NOT called on the
   * optimistic advance: a fact row is a claim that something is stored, and the
   * row is drawn only once the server says it exists (ruling R3).
   */
  onRulingSaved?: () => void;
  /**
   * The accusations every stuck card's panel offers, and its words (task 2.10).
   *
   * ## Why the PAGE fetches this and not the queue (task 2.12)
   *
   * The facts section needs the same wording for its Remove control, and two
   * fetches of one catalogue on one screen is two copies that can disagree
   * mid-session — the exact class of defect `onProgress`'s doc warns about one
   * field up. `ScenarioDetailPage` already owns the reads its sections share, so
   * it owns this one too and hands it down.
   *
   * `null` until it lands, or if it failed: no panel is rendered then, because
   * there is no fallback wording to render one with (R4).
   */
  linkOptions: AllegationOptions | null;
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
  linkOptions,
  keyboardActive = true,
  onProgress,
  onRulingSaved,
}) => {
  const [error, setError] = useState<string | null>(null);
  const [rulingError, setRulingError] = useState<string | null>(null);
  /** The stuck pile's progress sentence, or `null` when nothing is stuck. */
  const [linkProgress, setLinkProgress] = useState<string | null>(null);
  /** The stored sentence a target-less scenario shows INSTEAD of a queue. */
  const [noTargetNotice, setNoTargetNotice] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  // `null` until the first pool arrives: the default view is computed from the
  // counts (rulable if any exist), and choosing it before the counts exist would
  // pick "Not ruled" every time and then not correct itself.
  const [filters, setFilters] = useState<CandidateFilters | null>(null);
  const deferInputRef = useRef<HTMLInputElement | null>(null);

  // `load` is defined below and the failure handler needs it, so the handler
  // reaches it through a ref — a plain closure would capture the first `load`.
  const loadRef = useRef<() => Promise<void>>(async () => {});

  const onRulingFailed = useCallback((message: string) => {
    setRulingError(message);
    // RECONCILE: re-read the pool so the screen shows what the database holds,
    // not what the optimistic ruling assumed.
    void loadRef.current();
  }, []);

  // Wrapped rather than passed straight through: the prop is optional (a queue
  // mounted outside the scenario page has nothing to tell), and the hook's
  // dependency list wants a stable identity either way.
  const onSaved = useCallback(() => onRulingSaved?.(), [onRulingSaved]);

  // A link or unlink re-reads the pool: everything a linked card shows — the
  // sentence, the chips, the unlocked buttons, the progress line — is composed
  // server-side, and rebuilding any of it here would be the browser inventing
  // vocabulary (ruling R2, and the language law).
  const onLinksChanged = useCallback(() => {
    void loadRef.current();
  }, []);

  const [state, dispatch] = useReducerWithEffects(
    slug,
    scenarioId,
    onRulingFailed,
    onSaved,
    onLinksChanged,
    linkOptions?.wording ?? null,
  );

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const cards: ScenarioCardsResponse = await fetchScenarioCards(slug, scenarioId);
      // Both lists, in one pool. The backend partitions the DROPPED candidates
      // into `set_aside` (they are out of the working queue), but the list has an
      // "Excluded (n)" option and it has to count something real — and "All" must
      // not shrink every time Roman excludes a card, or the denominator would move
      // under the counts (§9).
      //
      // Concatenated rather than merged: each list arrives sorted by C-ordinal,
      // and re-sorting the union here would be the browser re-deriving an order
      // the backend owns and warns about (`sort_by_code`).
      dispatch({ type: "cards_loaded", cards: [...cards.pool, ...cards.set_aside] });
      // "38 of 94 linked." — composed server-side and counted from the POOL, not
      // from this session's clicks (the 1.7E-a ruling). Held beside the cards it
      // describes and replaced on every read, so it can never be stale relative to
      // the chips beneath it.
      setLinkProgress(cards.link_progress);
      // Present only when the scenario names nobody, in which case both lists
      // above were empty. Held rather than derived: an empty queue is not by
      // itself this state (see the field's own note).
      setNoTargetNotice(cards.no_target_notice ?? null);
      setError(null);
    } catch (e: unknown) {
      // Name WHAT failed, WHERE, and WHY. The scenario is in scope here and the
      // human may have several open, so a bare "failed to load" leaves them
      // guessing which one broke — and a non-`Error` throw used to lose the cause
      // entirely (Standing Rule 1: a failure a reader cannot diagnose is
      // incomplete error handling).
      const cause = e instanceof Error ? e.message : String(e);
      setError(`Could not load the candidates for scenario ${scenarioId} (${slug}): ${cause}`);
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
      // The collapsed-region guard (ruling R7). A collapsed region's body stays in
      // the DOM, so without this the one-key rulings would keep firing on a card
      // nobody can see. Checked BEFORE `preventDefault` so a collapsed queue does
      // not even swallow the key.
      if (!keyboardShouldRule(keyboardActive)) return;

      const typing = isTyping(e.target);
      const key = e.key.toLowerCase();
      // Only swallow the browser's default for keys we act on, and never while
      // typing — Escape and Enter belong to the field then, and the arrow keys
      // belong to the text cursor.
      if (!typing && (RULING_KEYS.includes(key) || NAVIGATION_KEYS.includes(key))) {
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

  // ONE derivation of the counts and ONE of the visible list (ruling R1): a second
  // pass anywhere here is a number that can disagree with the filter row.
  const counts = useMemo(() => candidateCounts(state.cards), [state.cards]);
  const active = filters ?? UNFILTERED;
  const visible = useMemo(() => filterCandidates(state.cards, active), [state.cards, active]);

  // The default view is computed once, from the first pool that arrives: "Rulable
  // now" while any exist, else "Not ruled". Recomputing it on every load would
  // yank the human's chosen filter away whenever a ruling triggered a reload.
  const chosen = filters !== null;
  useEffect(() => {
    if (!chosen && !loading && state.cards.length > 0) setFilters(defaultFilters(counts));
  }, [chosen, loading, state.cards.length, counts]);

  // Tell the reducer what is on screen, so navigation and auto-advance walk the
  // filtered order rather than the whole pool.
  const visibleIds = useMemo(() => visible.map((c) => c.graph_node_id), [visible]);
  useEffect(() => {
    dispatch({ type: "visible", ids: visibleIds });
  }, [dispatch, visibleIds]);

  // ─── Correcting a machine-written question (task 1.7F Part B) ─────────────
  //
  // Both writes RE-READ the pool on success rather than patching the card in
  // place. The card's question is composed server-side from two sources (the
  // graph's sentence and the override table), and rebuilding that composition in
  // the browser would be the client deciding how authorship reads — which is the
  // language law's line. One read is also what proves the write landed.
  //
  // A failure is re-thrown so the editor can keep the human's text on screen
  // beside the message; swallowing it here would close the editor over a
  // correction that was never stored.
  const correctQuestion = useCallback(
    async (graphNodeId: string, text: string) => {
      await saveQuestionOverride(slug, graphNodeId, text);
      await load();
    },
    [slug, load],
  );

  const revertQuestion = useCallback(
    async (graphNodeId: string) => {
      await revertQuestionOverride(slug, graphNodeId);
      await load();
    },
    [slug, load],
  );

  const selected = state.cards[state.index];
  const selectedId = selected?.graph_node_id ?? null;
  const { ruled, total } = progress(state);
  // The next card in the VISIBLE order, which is what the human will land on.
  const nextCard = visible[visible.findIndex((c) => c.graph_node_id === selectedId) + 1];

  // Report the counts to the section above, which draws the progress bar and words
  // the region's summary. In an effect rather than inline: calling a parent's
  // setState during render is the React warning that turns into an infinite loop.
  useEffect(() => {
    onProgress?.({ ruled, total });
  }, [onProgress, ruled, total]);

  if (loading) return <div style={{ padding: "1rem" }}>Loading the candidate queue…</div>;

  // A scenario that names nobody has no queue to show, and the sentence saying
  // so replaces it entirely — before this, the same state rendered 148 cards
  // gathered over a subject the human never chose, indistinguishable from the
  // scenario beside it (CC_REPORT_SCENARIO_COPY_DIAGNOSTIC.md).
  //
  // Placed above the error branch because it is not an error: nothing failed,
  // and a red box would send someone looking for a fault instead of to the
  // Edit-identity control the sentence names.
  if (noTargetNotice) {
    return (
      <div style={{ padding: "1rem", color: "var(--text-secondary)" }}>
        {noTargetNotice}
      </div>
    );
  }

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

      <CandidateFilterBar
        counts={counts}
        filters={active}
        shown={visible.length}
        onChange={setFilters}
      />

      <div style={hintBarStyle}>
        <span>Move: ↑ ↓ or J K — moving never rules</span>
        {/* Rendered verbatim. The browser counts nothing here — both numbers were
            counted from the served pool (task 2.10). */}
        {linkProgress && <span>{linkProgress}</span>}
        {/* D10's next-up hint. Absent rather than "Next up: —" when the following
            card has no ordinal yet: a hint that names nothing is worse than none. */}
        {nextUpHint(nextCard?.code) && (
          <span style={{ marginLeft: "auto" }}>{nextUpHint(nextCard?.code)}</span>
        )}
      </div>

      <CandidateList
        cards={visible}
        selectedId={selectedId}
        notice={state.notice}
        // An empty list means two different things, and they need two different
        // sentences: a filter with nothing behind it, or a scenario nobody has
        // scanned yet.
        filtered={hasAnyFilter(active)}
        onSelect={(graphNodeId) => dispatch({ type: "select", graphNodeId })}
        // A ruling names its card (1.7G, ruling R1). The id comes up from the
        // card's own render scope, so this queue never has to guess which card a
        // button meant — and there is no "current index" here for it to guess
        // with. The keyboard's own path, above, is the one that reads the
        // selection, because the selection is what a keyboard is aimed at.
        onRule={(key, graphNodeId) => dispatch({ type: "rule", key, graphNodeId })}
        onCorrectQuestion={correctQuestion}
        onRevertQuestion={revertQuestion}
        linkOptions={linkOptions}
        // A link names its card, exactly as a ruling does (ruling R1). The id
        // comes up from the card's own render scope; this queue never guesses.
        onSaveLinks={async (graphNodeId, allegationIds, cut) =>
          dispatch({ type: "link", graphNodeId, allegationIds, cut })
        }
        onUnlink={(graphNodeId, allegationId) =>
          dispatch({ type: "unlink", graphNodeId, allegationId })
        }
      />

      {state.mode.kind === "deferring" && (
        <DeferPrompt
          inputRef={deferInputRef}
          draft={state.mode.draft}
          onDraft={(draft) => dispatch({ type: "defer_draft", draft })}
        />
      )}
    </div>
  );
};

/** The inline defer prompt: quick picks, free text, Enter commits, Esc cancels. */
const DeferPrompt: React.FC<{
  draft: string;
  onDraft: (draft: string) => void;
  inputRef: React.RefObject<HTMLInputElement>;
}> = ({ draft, onDraft, inputRef }) => (
  <div
    style={{
      background: SURFACE,
      borderRadius: "var(--radius-card)",
      boxShadow: "var(--shadow-raised)",
      padding: "14px 18px",
      marginTop: "0.75rem",
      display: "flex",
      flexDirection: "column",
      gap: "0.5rem",
    }}
  >
    <div style={{ fontSize: "0.8rem", color: "var(--text-muted)" }}>
      Why defer this? Enter commits · Esc cancels
    </div>
    <div style={{ display: "flex", gap: "0.4rem", flexWrap: "wrap" }}>
      {DEFER_QUICK_REASONS.map((reason, i) => (
        <button
          key={reason}
          type="button"
          onClick={() => onDraft(reason)}
          style={{ ...chipStyle, cursor: "pointer", background: "var(--v3-chrome)" }}
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

export default CardQueue;
