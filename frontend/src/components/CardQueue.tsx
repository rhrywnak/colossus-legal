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

import type { ScenarioCard } from "../services/scenarioCards";
import { useCardsPayload } from "./useCardsPayload";
import CandidateFilterBar from "./CandidateFilterBar";
import QueueNotices from "./QueueNotices";
import CandidateList from "./CandidateList";
import { useReducerWithEffects, type RulingOutcome } from "./useQueueReducer";
import {
  candidateCounts,
  candidateState,
  countForFacet,
  defaultFilters,
  facetLabel,
  filterCandidates,
  filterProgress,
  hasAnyFilter,
  matchesFilter,
  stateChip,
  UNFILTERED,
  type CandidateFilters,
  type StateFacet,
} from "./candidateFilters";
import { matchesChip, type ChipFilter } from "./evidenceCardModel";
import { rulingAcknowledgment, type RulingReceipt } from "./rulingAcknowledgment";
import { keyboardShouldRule } from "./queueRegion";
import type { AllegationOptions } from "../services/evidenceLinks";
import { revertQuestionOverride, saveQuestionOverride } from "../services/evidenceSummary";

// ─── §2c visual language ────────────────────────────────────────────────────

const SURFACE = "var(--bg-surface)"; // #ffffff — pure white, per §2c

/**
 * The receipt strip: what the last ruling did, when its card has left the list.
 *
 * Quiet by default — an acknowledgment is information, not a warning — and only
 * the failure variant raises its voice.
 */
// REMOVED (task R1): `receiptStyle`, which only its danger variant used.

// REMOVED (task R1): `receiptFailedStyle`, a danger variant of the receipt
// strip that no render site ever used. The failure surfaces this queue does
// have are `QueueNotices` and the card's own banners.

// REMOVED (task R2): `hintBarStyle` — the key-hint row it styled is gone.

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
   * Called when the SERVER confirms a ruling (task 1.7F Part A).
   *
   * The page re-reads its cards from this, so a newly included candidate shows up
   * in the facts section without a reload. Deliberately NOT called on the
   * optimistic advance: a fact row is a claim that something is stored, and the
   * row is drawn only once the server says it exists (ruling R3).
   */
  onRulingSaved?: () => void;
  /**
   * Report the queue heading's two values upward (task R4, P4).
   *
   * `ScanSection` draws the heading — it owns the region head the fold arrow
   * sits in — and this is where the active filter and the addressed count are
   * known. See the effect that calls it for why this is not the upward
   * reporting task 2.13c removed.
   */
  onFrameChanged: (frame: QueueFrame) => void;
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

/** What the queue heading needs, and nothing else. */
export type QueueFrame = {
  /** The active facet's token — the heading asks the store for its name. */
  facet: StateFacet;
  /** How many cards that facet holds. */
  count: number;
  /** Addressed over the proposed bucket, for the right-hand end of the line. */
  progress: { ruled: number; total: number };
};

const CardQueue: React.FC<Props> = ({
  slug,
  scenarioId,
  externalRefresh,
  linkOptions,
  keyboardActive = true,
  onRulingSaved,
  onFrameChanged,
}) => {
  /** A LINK write's failure. Rulings report through `receipt` instead — they know
   *  which card they were about, and say so on it. */
  const [linkError, setLinkError] = useState<string | null>(null);
  /**
   * What the last ruling did, named on the card it was about.
   *
   * Every ruling produces one, landed or refused (the reframed rider). It is
   * REPLACED rather than accumulated: the human rules one card at a time, and a
   * stack of receipts would bury the queue it is meant to annotate.
   */
  const [receipt, setReceipt] = useState<RulingReceipt | null>(null);
  /**
   * A chip the human clicked, narrowing the queue to it (Piece 7).
   *
   * Held BESIDE the facet filter rather than folded into it: the two answer
   * different questions ("what state is this in" and "who said it"), and a human
   * working the Proposed list who clicks George Phillips means both — not a
   * replacement of the first by the second.
   */
  const [chipFilter, setChipFilter] = useState<ChipFilter | null>(null);
  // `null` until the first pool arrives: the default view is computed from the
  // counts (rulable if any exist), and choosing it before the counts exist would
  // pick "Not ruled" every time and then not correct itself.
  const [filters, setFilters] = useState<CandidateFilters | null>(null);
  const deferInputRef = useRef<HTMLInputElement | null>(null);

  // `load` is defined below and the failure handler needs it, so the handler
  // reaches it through a ref — a plain closure would capture the first `load`.
  const loadRef = useRef<() => Promise<void>>(async () => {});

  /**
   * Say what a ruling did, and reconcile if it failed.
   *
   * The sentence is composed by a pure helper from the STORED templates, so this
   * callback stays the wiring and the words stay testable. A failure also
   * re-reads the pool: the screen must show what the database holds, not what
   * the optimistic patch assumed.
   */
  const onRulingOutcome = useCallback(
    (outcome: RulingOutcome) => {
      const card = cardsRef.current.find((c) => c.graph_node_id === outcome.graphNodeId);
      setReceipt(
        rulingAcknowledgment({
          outcome,
          card,
          state: card ? candidateState(card) : null,
          stateLabel: card ? stateChip(candidateState(card)).label : null,
          leftTheList: card ? !matchesFilter(card, filtersRef.current) : false,
          filterLabel: linkOptionsRef.current
            ? facetLabel(filtersRef.current.state, linkOptionsRef.current.card_grammar)
            : null,
          wording: linkOptionsRef.current?.wording ?? null,
        }),
      );
      if (outcome.failure !== null) void loadRef.current();
    },
    [],
  );

  const onLinkFailed = useCallback((message: string) => {
    setLinkError(message);
    // RECONCILE: re-read the pool so the screen shows what the database holds.
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
    onRulingOutcome,
    onSaved,
    onLinksChanged,
    onLinkFailed,
    linkOptions?.wording ?? null,
  );

  // The outcome callback fires from a promise, long after the render that made
  // it. Refs keep it reading TODAY's cards, filter and wording rather than the
  // ones captured when it was created — the same reason `loadRef` exists.
  const cardsRef = useRef(state.cards);
  cardsRef.current = state.cards;
  const filtersRef = useRef<CandidateFilters>(UNFILTERED);
  const linkOptionsRef = useRef(linkOptions);
  linkOptionsRef.current = linkOptions;

  // The read, and the four pieces of payload state it lands in, live in
  // `useCardsPayload` — see that module for why the two lists arrive as one.
  const onCards = useCallback(
    (cards: ScenarioCard[]) => dispatch({ type: "cards_loaded", cards }),
    [dispatch],
  );
  const { loading, error, noTargetNotice, proposalSource, load } = useCardsPayload(
    slug,
    scenarioId,
    onCards,
  );

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
  filtersRef.current = active;
  const byFacet = useMemo(
    () => filterCandidates(state.cards, active),
    [state.cards, active],
  );
  // Piece 7: the chip narrows what the facet left. Matched on the RAW payload
  // value rather than the chip's displayed text, so a filter can never surface a
  // card whose chip says something else.
  const visible = useMemo(
    () => (chipFilter ? byFacet.filter((c) => matchesChip(c, chipFilter)) : byFacet),
    [byFacet, chipFilter],
  );

  // The default view is computed once, from the first pool that arrives:
  // "Proposed" while a scan is proposing anything, else the 1.7E computed default
  // (architect ruling R8). Recomputing it on every load would yank the human's
  // chosen filter away whenever a ruling triggered a reload — and it would also
  // move them off Proposed the moment they ruled the last proposal, mid-session.
  const chosen = filters !== null;
  useEffect(() => {
    if (!chosen && !loading && state.cards.length > 0) setFilters(defaultFilters(counts));
  }, [chosen, loading, state.cards.length, counts]);

  // THE HEADING'S TWO VALUES, reported upward (task R4, P4).
  //
  // The heading line — "Included — 21 … 22 of 22 addressed" — is drawn by
  // `ScanSection`, because it sits in the region head beside the fold arrow. The
  // active filter and the addressed count are known HERE, where the filtering
  // and the pool live.
  //
  // ## Why this is not the upward reporting task 2.13c removed
  //
  // That was the queue reporting its COUNTS, and the hazard was specific: the
  // region's open/closed state was derived from them, so a collapsed queue —
  // which reports nothing, because it is not mounted — computed "all candidates
  // ruled" and latched itself shut. Neither value below gates anything. They are
  // read by one line of text and nothing else, so a frame that has not heard yet
  // renders its served zero-state sentence and then the real one, with no state
  // machine in between.
  useEffect(() => {
    onFrameChanged({
      facet: active.state,
      count: countForFacet(active.state, counts),
      progress: filterProgress(state.cards),
    });
  }, [onFrameChanged, active.state, counts, state.cards]);

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
      {/* The bar is withheld until the stored words load — there is no fallback
          vocabulary for five filter names (R4), and a chip row reading
          "undefined (8)" would be worse than a moment with no chips. */}
      {linkOptions && (
        <CandidateFilterBar
          counts={counts}
          filters={active}
          wording={linkOptions.card_grammar}
          onChange={setFilters}
        />
      )}

      {/* THE HINT BAR IS GONE (Roman's cleanup ruling, 2026-08-10).
          Three lines died here and none of them were carrying their weight:

          "Move: ↑ ↓ or J K — moving never rules" and the "Keys: I E D U" line
          one level up were key hints for keys whose letters are already printed
          on the buttons that do the same thing (Include I · Exclude E · Defer D ·
          ↩ U). A legend for a control that labels itself is a line a reader has
          to skip on every scroll. THE KEYS THEMSELVES STILL WORK — only the
          teaching text went.

          "Next up: C-nn" named a card that was already the next row on screen.

          Five lines became two: the heading, then the chips with the progress
          count beside them. */}

      <QueueNotices
        linkError={linkError}
        onDismissLinkError={() => setLinkError(null)}
        // The receipt rides on the CARD when the card is still on the list;
        // this strip is the fallback for when the ruling took it away.
        orphanedReceipt={
          receipt && !visible.some((c) => c.graph_node_id === receipt.graphNodeId)
            ? receipt
            : null
        }
        chipFilter={chipFilter}
        onClearChipFilter={() => setChipFilter(null)}
        options={linkOptions}
      />

      <CandidateList
        cards={visible}
        onFilterChip={setChipFilter}
        selectedId={selectedId}
        // WHY the selection is where it is (task R4, P2). The list scrolls to
        // follow a keyboard move and stays put for a ruling's own advance; the
        // reducer is the only thing that knows which just happened.
        follow={state.follow}
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
        // ONE sentence for the whole list: only the latest completed run projects,
        // so every proposed card shares it. `null` when nothing is proposed, and
        // each card checks its own `proposed` field before rendering it.
        // The receipt for a card still ON the list rides on that card, where the
        // human's eye already is. `CandidateList` gives it to the one card it
        // names and to no other.
        receipt={receipt}
        proposedAttribution={
          proposalSource && linkOptions
            ? linkOptions.wording.card_proposed_attribution_template.replace(
                "{when}",
                formatProposedDate(proposalSource.started_at),
              )
            : null
        }
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
        // R1 (architect, 2026-08-08): the reason input renders ON the card being
        // deferred, under its action row — not at the bottom of the queue, below
        // the then-70vh scroll window, where the previous prompt could open entirely
        // outside the human's view. §7's contract is that a card is rulable from
        // the card alone, and collecting the reason anywhere else broke it.
        deferring={state.mode.kind === "deferring" ? state.mode : null}
        deferInputRef={deferInputRef}
        onDeferDraft={(draft) => dispatch({ type: "defer_draft", draft })}
      />

    </div>
  );
};

// The defer prompt that used to live here is GONE (architect ruling R1,
// 2026-08-08). It rendered after the card list — i.e. below the then-`70vh`
// scroll window — so pressing Defer on a card near the top of that window opened
// a prompt the human could be a full viewport away from. §7 says a card is
// rulable from the card alone, and the one ruling that needs a word from the
// human was collecting it somewhere else entirely. It is now `DeferReasonForm`
// in `CandidateCard`, under the action row of the card being deferred.

/**
 * The proposing run's date, as the attribution says it: "Aug 7".
 *
 * The BROWSER formats it because the server does not know the reader's locale —
 * the same split the scan-history delete confirmation makes with `{run}`, so both
 * dates on this page read alike. An unparseable timestamp falls back to the raw
 * string rather than "Invalid Date": a card naming an ugly date still says which
 * scan proposed it, which is the whole job of the line.
 *
 * Kept identical to `ScanSection`'s copy on purpose — the two sentences sit within
 * a few inches of each other, and a shared helper would be a third module for one
 * `toLocaleDateString` call. If a third caller appears, extract it then.
 */
function formatProposedDate(iso: string): string {
  const at = new Date(iso);
  if (Number.isNaN(at.getTime())) return iso;
  return at.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

export default CardQueue;
