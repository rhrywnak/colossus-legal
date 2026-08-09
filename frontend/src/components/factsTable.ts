// =============================================================================
// factsTable.ts — the working view's pure core (task 1.4)
// =============================================================================
//
// The Casefleet Facts-table pattern (study §1.4/§3) over a scenario's INCLUDED
// evidence. Two pure things live here; the component next door renders them:
//
//   1. `includedRows` — the row anatomy: which cards belong in the table and
//      what each row shows.
//   2. `filterRows` — the client-side search, over server-supplied strings.
//
// ## Why the search is client-side, and why that is not a shortcut
//
// Task 3.1 owns search. This filters rows already on screen — a narrowing of a
// list the human is looking at, not a query. It searches only strings the
// backend composed, so it can never surface something the payload did not say.
//
// ## The frontend still composes nothing
//
// A row's fields are payload fields. This module chooses which rows exist and
// which are visible; it never builds a sentence.

import type { FactTier, ScenarioCard } from "../services/scenarioCards";

/** One row of the working view — the Facts-table anatomy. */
export type WorkingRow = {
  /** `"C-14"`, or `null` for a candidate not yet numbered. */
  code: string | null;
  graphNodeId: string;
  /** The quote, verbatim from the payload. */
  text: string;
  /** Accusation chips: what this item bears on. */
  bearsOn: string[];
  /** The pinpoint chip's label and its viewer link. */
  pinpointLabel: string;
  pinpointHref: string;
  /** The ruling state, in plain language. */
  statusLabel: string;
  /**
   * Whether a HUMAN wrote this fact, rather than it being evidence they ruled in.
   *
   * Drives the row's coloured left-edge stripe (task 1.7D, item 6): green for
   * included evidence, blue for a human-added fact. Approved as an additive
   * descriptor field by ruling R6.
   *
   * ## Why the flag and not "does it have a pinpoint"
   *
   * A human fact has no citation BY DESIGN (§8), so "no pinpoint" correlates with
   * it today — but correlation is not the fact. Inferring provenance from a missing
   * field would break the day an evidence row legitimately lacks a page, and it
   * would be the browser deducing authorship rather than being told it.
   */
  isHuman: boolean;
  /** The interrogatory question this answer responds to, or `null` when there is
   *  none (documentary evidence has no question). Task 2.13: a bare "Yes" is
   *  noise; the same "Yes" under what was asked is a sworn admission. */
  question: string | null;
  /** The kind of statement, humanized, or `null` when the extraction recorded
   *  none. Rendered as served — the vocabulary is mixed across extraction
   *  generations, and normalizing it in the browser would be the frontend
   *  deciding what a document said. */
  statementKind: string | null;
  /** How much this fact carries the scenario. `null` for a human-authored fact,
   *  which is not evidence and carries no weight tier. */
  tier: FactTier | null;
  /** The stored position, or `null` when the human has never placed this fact.
   *  Drives the un-place control: a fact with no stored position has nothing to
   *  clear. NOT the sort key — see `displayOrdinal`. */
  sortOrdinal: number | null;
  /** The server's computed position, and the ONLY thing this list sorts on. */
  displayOrdinal: number | null;
  /** Who said it, or `null` for documentary evidence, which genuinely has no
   *  speaker. Part of the header row's fixed anatomy (task 2.13b §3). */
  speaker: string | null;
  /**
   * The WHOLE payload this row came from, or `null` for a human-authored fact.
   *
   * ## Why the row carries the card rather than more projected fields
   *
   * This is the ONE_CARD_GRAMMAR seam. Every field above is a projection of a
   * `ScenarioCard`, and the projection is what lost the elements, the count, the
   * anchor highlight, the context, the band and the scan's reason — so the fact
   * card could not answer "who said what, and why does it matter here?" while the
   * candidate card three inches higher could. Adding six more projected fields
   * would have made the loss smaller and kept the mechanism that caused it.
   *
   * Carrying the card means `FactRow` calls the SAME `evidenceCardView` the
   * candidate wrapper calls, on the same value. The two cannot show different
   * fields, because there is one builder and one payload.
   *
   * `null` for a human fact, which is not evidence: it has no anchor, no
   * pinpoint, no allegations and no scan verdict, and §8 keeps it uncited by
   * design. That row renders its text and its provenance line, as it always has.
   */
  card: ScenarioCard | null;
};

/**
 * The card's rows, in their FIXED order, with the absent ones dropped.
 *
 * ## Why this is a function and not just JSX
 *
 * Roman's 2026-08-05 note: "the Candidate numbers appear in different places
 * cards." They did. The C-code, the kind and the quote all shared one
 * `flex-wrap` row, so the code's position moved with the length of whatever sat
 * beside it, and a card with a question line pushed it somewhere else again.
 *
 * The ruling is that every card has the SAME anatomy and an absent element's row
 * collapses without anything reflowing into its place. That is an invariant, and
 * an invariant that lives only in JSX is one nobody can test. So the order is
 * declared here, once, as data — and `factsTable.test.ts` asserts that the
 * sequence is identical across cards of wildly different shapes.
 *
 * The renderer walks THIS array. It cannot emit a row this function did not
 * list, and it cannot emit them in another order.
 */
export type CardSection = "header" | "question" | "quote" | "allegations" | "source";

/** Every section a fact card can show, in the one order it may show them. */
export const CARD_ANATOMY: readonly CardSection[] = [
  "header",
  "question",
  "quote",
  "allegations",
  "source",
] as const;

/**
 * Which sections THIS row renders, in canonical order.
 *
 * `header` and `quote` are always present: every fact has a landmark line and
 * something to say. The other three are conditional, and each simply drops out —
 * the surviving sections never change their relative order, which is the whole
 * point.
 */
export function sectionsFor(row: WorkingRow): CardSection[] {
  return CARD_ANATOMY.filter((section) => {
    switch (section) {
      case "header":
      case "quote":
        return true;
      case "question":
        return row.question !== null;
      case "allegations":
        return row.bearsOn.length > 0;
      case "source":
        // A human fact has no citation by design (§8), so it has no source row —
        // but it still has scenario controls, which live in that row. The row
        // exists whenever there is either a pinpoint or a control to put in it.
        return true;
    }
  });
}

/** The weight order the list groups by — heaviest first. Mirrors `FactTier::rank`. */
const TIER_RANK: Record<FactTier, number> = {
  carries: 0,
  backup: 1,
  background: 2,
};

/**
 * The rank a row sorts under.
 *
 * A human fact has no tier, and sorts with `backup` — the middle. It is neither
 * promoted above evidence the human deliberately starred nor buried in the
 * background pile they folded away, and it keeps the position 1.7D gave it (in
 * the same table as the evidence, distinguished by its stripe).
 */
function rankOf(row: WorkingRow): number {
  return row.tier === null ? TIER_RANK.backup : TIER_RANK[row.tier];
}

/**
 * The included items, as rows.
 *
 * ## Domain note: filtered on the STATE, never the label
 *
 * `status === "included"` uses the machine-readable token the 1.4 payload added
 * beside `status_label`. Filtering on the label would mean inferring state from
 * prose — it would break silently the day the wording changed, and it is exactly
 * what that field pair exists to prevent.
 *
 * The working view shows what a human has PUT IN the scenario. Undecided items
 * are the queue's business; set-aside items are deliberately out.
 */
export function includedRows(cards: ScenarioCard[]): WorkingRow[] {
  return cards
    .filter((card) => card.status === "included")
    .map((card) => ({
      code: card.code,
      graphNodeId: card.graph_node_id,
      text: card.quote.text,
      // Both kinds of accusation chip, machine first (task 2.10). A human's link
      // is what put some of these rows in the table at all — the card could not
      // be included until somebody linked it — so a facts table that showed only
      // the extraction's accusations would leave those rows looking unattached to
      // anything, which is the state the human just fixed.
      //
      // Order, not merge: the payload keeps the two lists apart (they are
      // different claims), and this joins them for display only.
      bearsOn: [
        ...card.bears_on.map((b) => b.accusation),
        ...card.human_links.map((l) => l.label),
      ],
      pinpointLabel: card.pinpoint.label,
      pinpointHref: card.pinpoint.viewer_href,
      statusLabel: card.status_label,
      // Every row from the CARD payload is evidence a human ruled in — a card
      // exists because the graph produced it. Human facts enter through
      // `humanFactRows` below.
      isHuman: false,
      // Task 2.13. All four are payload fields read straight across; the browser
      // composes none of them. `?? null` normalizes the two the backend SKIPS
      // when absent (`tier`, `sort_ordinal`) so every row has the same shape and
      // no consumer has to handle `undefined` as well as `null`.
      // `|| null` folds an EMPTY question into the same absent state as a missing
      // one. The backend already normalizes `""` to `None`, so this is a second
      // fence rather than the only one — but the row type promises `string | null`
      // and an empty string is neither: it would render a `Q:` label introducing
      // nothing, asserting that a question exists and was lost.
      question: card.quote.question || null,
      statementKind: card.statement_kind,
      tier: card.tier ?? null,
      sortOrdinal: card.sort_ordinal ?? null,
      displayOrdinal: card.display_ordinal ?? null,
      // Documentary evidence genuinely has no speaker; `null` says so rather
      // than the header inventing "Unknown".
      speaker: card.speaker.name,
      // The payload itself, so the shared card body can be built from it.
      card,
    }));
}

/**
 * Human facts, as rows of the same table.
 *
 * ## Why they share the table rather than sitting under it
 *
 * 1.7C rendered them as a separate list beneath the evidence, because a fact with
 * no citation and a fact with a pinpoint are different kinds of thing and §8
 * requires the distinction be visible. That reasoning still holds — what changed is
 * HOW it is made visible. The v3 mockup puts both in one table and carries the
 * distinction in a coloured left-edge stripe (green evidence, blue human), which
 * says it at a glance without splitting the reader's attention across two lists
 * they have to mentally join.
 *
 * The row keeps no pinpoint, because there is none to keep.
 */
export function humanFactRows(
  facts: { id: string; text: string; authored_tag: string; date_label: string | null }[],
): WorkingRow[] {
  return facts.map((fact) => ({
    code: null,
    graphNodeId: `human:${fact.id}`,
    text: fact.text,
    bearsOn: [],
    pinpointLabel: "",
    pinpointHref: "",
    // The composed provenance line ("Added by Roman") stands where the ruling state
    // stands on an evidence row: it is what this row's authority IS.
    statusLabel: fact.date_label ? `${fact.authored_tag} · ${fact.date_label}` : fact.authored_tag,
    isHuman: true,
    // A human fact is not a discovery answer and has no question, no extraction
    // kind, and no weight tier — §8 keeps it uncited by design. `null` four times
    // is the honest shape, not a gap waiting to be filled.
    question: null,
    statementKind: null,
    tier: null,
    sortOrdinal: null,
    displayOrdinal: null,
    // The provenance line already says who wrote it ("Added by Roman"), and it
    // is not a SPEAKER — nobody said this under oath. `null` keeps the header's
    // speaker slot empty rather than restating authorship as testimony.
    speaker: null,
    // A human fact is not evidence and has no card payload behind it. `null` is
    // what makes the row render its own shape rather than an evidence card with
    // every field empty.
    card: null,
  }));
}

/**
 * Narrow the rows to those matching a search term.
 *
 * Case-insensitive substring across the row's visible text: the quote, the
 * accusations, the pinpoint, the code — and, since ONE_CARD_GRAMMAR Piece 7, the
 * SPEAKER and the KIND, because both are now chips on the card's face. Searching
 * only what is DISPLAYED means a human never gets a hit they cannot see the
 * reason for; the corollary is that anything newly displayed has to join the
 * haystack, or typing a name a human can read on screen finds nothing.
 *
 * An empty or whitespace-only term returns every row: "no filter" and "a filter
 * that matches nothing" are different states, and a blank box means the former.
 */
export function filterRows(rows: WorkingRow[], term: string): WorkingRow[] {
  const needle = term.trim().toLowerCase();
  if (!needle) return rows;

  return rows.filter((row) => {
    const haystack = [
      row.code ?? "",
      row.text,
      row.pinpointLabel,
      row.speaker ?? "",
      row.statementKind ?? "",
      ...row.bearsOn,
    ]
      .join(" ")
      .toLowerCase();
    return haystack.includes(needle);
  });
}

/**
 * Which rows are NEW since the last payload (task 1.7F Part A).
 *
 * ## Why "new" is defined against the PREVIOUS PAYLOAD and not against a ruling
 *
 * The facts list is drawn only from what the server returned (ruling R3 — no
 * optimistic rows), so the honest definition of "just arrived" is "present now,
 * absent last time". Defining it as "the card I just ruled" would tint a row on
 * the strength of a click rather than a stored fact, and would miss a row that
 * appeared because a merge or another person's ruling added it — which is
 * precisely when a reader most needs to see what changed under them.
 *
 * @param previous the ids from the last payload, or `null` on the first one.
 *        `null` yields NO arrivals: every row is new on a page that just loaded,
 *        and tinting the whole table says nothing.
 * @param current  the ids in the payload now on screen.
 */
export function arrivedIds(previous: Set<string> | null, current: string[]): string[] {
  if (previous === null) return [];
  return current.filter((id) => !previous.has(id));
}

/**
 * The rows in display order: by weight, then by the human's placement.
 *
 * ## The property this exists to guarantee
 *
 * A scenario nobody has dragged in must render EXACTLY as it did before task
 * 2.13 shipped. Rows with no stored position keep their incoming order (which is
 * the server's C-ordinal sort), and they follow every row that does have one. So
 * the untouched case is a no-op, and the touched case shows the human's own
 * sequence — the sequence IS the argument.
 *
 * ## Why this is a stable sort over the incoming order
 *
 * `Array.prototype.sort` is required to be stable in every engine we target, so
 * two rows that compare equal keep the order the server sent. That is what lets
 * the unplaced tail inherit the C-ordinal sort without this function needing to
 * know anything about C-numbers — the server already decided that, and
 * re-deriving it here would be a second opinion that can disagree.
 *
 * ## Task 2.13c: the browser no longer computes a position at all
 *
 * This used to derive where an unplaced fact went relative to a placed one, which
 * meant the same arithmetic lived here AND in `services::scenario_fact_order`.
 * The two disagreed about what "end of list" meant and a re-included fact came
 * back fourth. The server now sends `display_ordinal` — one number per card,
 * computed once — and this sorts on it and nothing else.
 */
export function orderedRows(rows: WorkingRow[]): WorkingRow[] {
  return [...rows].sort((a, b) => {
    const byTier = rankOf(a) - rankOf(b);
    if (byTier !== 0) return byTier;

    // Both positions come from the server. A row without one (a human fact, or a
    // card the server did not place) keeps its incoming order behind the rest:
    // `Array.prototype.sort` is stable in every engine we target, so equal keys
    // preserve the payload's own sequence.
    const aPos = a.displayOrdinal;
    const bPos = b.displayOrdinal;
    if (aPos === null && bPos === null) return 0;
    if (aPos === null) return 1;
    if (bPos === null) return -1;
    return aPos - bPos;
  });
}

/**
 * Split the ordered rows into the ones always shown and the folded background.
 *
 * ## Why this is a split and never a filter
 *
 * The background tier is FOLDED, not hidden. The count travels with the pile so
 * the list can always say how much is down there, and one click opens it — a
 * curated fact that silently disappears is the exact failure the tier exists to
 * avoid (Standing Rule 1, on a surface where the missing thing is evidence).
 *
 * Returning both halves means the caller cannot render the pile without being
 * handed its size.
 *
 * ## `pending` is the no-scroll-jump mechanism (Piece 5a, the C-91 defect)
 *
 * Weighing a fact into the background moves it out of `shown` and into the fold,
 * so the row LEAVES the list under the cursor and every card below it slides up
 * — on a forty-six-fact list, the thing the human was reading jumps somewhere
 * else at the moment they acted. That is the defect on record.
 *
 * A row whose id is in `pending` stays in `shown` even though its stored tier
 * says background. The caller holds an id there from the click until the human
 * dismisses the acknowledgment (or acts again), so the card stays exactly where
 * it was long enough to read the sentence and press undo. The STORED state is
 * untouched and the pile's count is unaffected — this delays a row's departure
 * from the visible half, it does not hide one.
 *
 * Testable as a pure function, which is the point: "never scroll-jumps" is
 * otherwise a claim about layout that nothing in this repo can assert.
 */
export function splitBackground(
  rows: WorkingRow[],
  pending: ReadonlySet<string> = new Set(),
): {
  shown: WorkingRow[];
  background: WorkingRow[];
} {
  const folded = (row: WorkingRow) =>
    row.tier === "background" && !pending.has(row.graphNodeId);
  return {
    shown: rows.filter((row) => !folded(row)),
    background: rows.filter(folded),
  };
}

/**
 * The two neighbours a dropped row lands between.
 *
 * Dropping row X ONTO row Y means "put X where Y is", i.e. immediately above Y.
 * So `before` is Y and `after` is whatever precedes Y once X has been lifted out
 * of the list. Lifting X out first is what makes a drop onto the row directly
 * below X behave as a human expects rather than as a no-op.
 *
 * Returns `null` when the drop cannot name a position — X and Y are the same
 * row, or Y is not in the list. The caller does nothing in that case rather than
 * sending a request the server would have to refuse.
 *
 * The browser computes NEIGHBOURS, never an ordinal: the number is the server's,
 * derived from what is stored (Rule 12).
 */
export function neighboursForDrop(
  rows: WorkingRow[],
  draggedId: string,
  targetId: string,
): { after: string | null; before: string | null } | null {
  if (draggedId === targetId) return null;

  const withoutDragged = rows.filter((row) => row.graphNodeId !== draggedId);
  const targetIndex = withoutDragged.findIndex((row) => row.graphNodeId === targetId);
  if (targetIndex === -1) return null;

  const previous = targetIndex > 0 ? withoutDragged[targetIndex - 1] : null;
  return {
    after: previous ? previous.graphNodeId : null,
    before: withoutDragged[targetIndex].graphNodeId,
  };
}
