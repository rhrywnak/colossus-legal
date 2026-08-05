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
  /** The stored position, or `null` when the human has never placed this fact. */
  sortOrdinal: number | null;
};

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
  }));
}

/**
 * Narrow the rows to those matching a search term.
 *
 * Case-insensitive substring across the row's visible text: the quote, the
 * accusations, the pinpoint and the code. Searching only what is DISPLAYED means
 * a human never gets a hit they cannot see the reason for — a match on a hidden
 * field would look like the filter was broken.
 *
 * An empty or whitespace-only term returns every row: "no filter" and "a filter
 * that matches nothing" are different states, and a blank box means the former.
 */
export function filterRows(rows: WorkingRow[], term: string): WorkingRow[] {
  const needle = term.trim().toLowerCase();
  if (!needle) return rows;

  return rows.filter((row) => {
    const haystack = [row.code ?? "", row.text, row.pinpointLabel, ...row.bearsOn]
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
 * Not a re-implementation of the backend's `plan_move`: that computes the NUMBER
 * to store for one dragged row, this renders the numbers already stored. The
 * browser never invents a position.
 */
export function orderedRows(rows: WorkingRow[]): WorkingRow[] {
  return [...rows].sort((a, b) => {
    const byTier = rankOf(a) - rankOf(b);
    if (byTier !== 0) return byTier;

    // A placed row always precedes an unplaced one within the same tier.
    const aPlaced = a.sortOrdinal !== null;
    const bPlaced = b.sortOrdinal !== null;
    if (aPlaced !== bPlaced) return aPlaced ? -1 : 1;

    // Both placed: the human's own numbers decide.
    if (aPlaced && bPlaced) return (a.sortOrdinal as number) - (b.sortOrdinal as number);

    // Neither placed: equal, so the stable sort keeps the incoming order.
    return 0;
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
 * Returning both halves plus the count means the caller cannot render the pile
 * without being handed its size.
 */
export function splitBackground(rows: WorkingRow[]): {
  shown: WorkingRow[];
  background: WorkingRow[];
} {
  return {
    shown: rows.filter((row) => row.tier !== "background"),
    background: rows.filter((row) => row.tier === "background"),
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
