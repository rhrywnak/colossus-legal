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

import type { ScenarioCard } from "../services/scenarioCards";

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
};

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
      bearsOn: card.bears_on.map((b) => b.accusation),
      pinpointLabel: card.pinpoint.label,
      pinpointHref: card.pinpoint.viewer_href,
      statusLabel: card.status_label,
      // Every row from the CARD payload is evidence a human ruled in — a card
      // exists because the graph produced it. Human facts enter through
      // `humanFactRows` below.
      isHuman: false,
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
