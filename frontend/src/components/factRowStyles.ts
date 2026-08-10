// =============================================================================
// factRowStyles — the fact card's type scale and shared styles (task 2.13c)
// =============================================================================
//
// Split from `FactRow` when 2.13c pushed it past the 300-line limit (Rule 17).
// Keeping the scale in ONE module is also what makes the "two text sizes, total"
// rule checkable by reading a file rather than auditing every span across three
// components — which is exactly how the card drifted to many sizes before.

import React from "react";

// ── The type scale, declared once ───────────────────────────────────────────
//
// All seven values below are PRESENTATIONAL, and therefore not Rule-2 tunables —
// the standing line R5 draws: they choose how the same content is SEEN, never
// what it is, which cards exist, or what any of them mean. Named rather than
// inlined so the study's "one body size, metadata one step down" rule is
// checkable by reading a few lines instead of auditing thirty spans.

// CONST: the ONE body size on a fact card — the quote, and nothing else.
// Study §2 binding: one size for quotes with generous line-height. A second
// body size anywhere on this card is the "no consistency within a card" defect.
export const BODY_SIZE = "0.95rem";

// CONST: everything that is not the exchange — header meta, allegation, source.
// One step down from the body and --text-secondary (6.00:1), NOT --text-muted.
// Task 2.13c: muted is reserved for genuinely tertiary chrome like a page footer.
// It is not card content, and Roman read the card as "text still very light".
export const META_SIZE = "0.8rem";

// CONST: line-height for the quote. ≈1.6 is the "generous" the ruling names;
// a legal quote runs long and tight leading is what makes a card a wall.
export const BODY_LINE_HEIGHT = 1.6;

// ── Spacing rhythm (ruling 2) ───────────────────────────────────────────────

// CONST: the gap between rows INSIDE one card. Must stay the largest such gap —
// `MAX_INTRA_GAP_PX` below is its numeric twin and the fence compares them.
export const INTRA_GAP = "6px";

// CONST: the card's own padding. Not part of the rhythm comparison: it is the
// distance from the border to the content, not between two pieces of content.
export const CARD_PADDING = "14px 16px";

// CONST: the gap BETWEEN cards. Ruling 2 — proximity does the separating and the
// border assists, so this must be decisively larger than any intra-card gap.
// Exported because `WorkingView` lays out the stack and
// `scenarioPageStructure.test.ts` asserts the ratio against the value below.
export const CARD_GAP_PX = 20;

// CONST: the largest intra-card gap, as a number, for that same fence.
// Kept beside `INTRA_GAP` so the two cannot drift: if a row gap grows past this,
// the ratio test fails rather than the rhythm quietly flattening.
export const MAX_INTRA_GAP_PX = 6;

// REMOVED (task R1): `cardStyle`, superseded by the shared card body
// (`evidenceCardView` and `EvidenceCardParts`) and unreferenced since.

/**
 * The coloured left spine, running the FULL height of the card (ruling 1).
 *
 * Green = evidence a human ruled in, blue = a fact a human wrote. `alignSelf:
 * stretch` inside a `stretch` flex row is what makes it run edge to edge; it
 * used to stop at a `minHeight`, which left a stub beside taller cards and read
 * as an alignment bug.
 *
 * A cue, never the only signal — the source row still says which in words, for
 * a colourblind reader and for greyscale print.
 */
// REMOVED (task R1): `spineStyle`, the human/machine spine cue. The source
// row states the same fact in words, which is what actually ships.

/**
 * Metadata: the SMALL of the card's two sizes, regular weight, secondary colour.
 *
 * Task 2.13c collapsed the card to exactly two text sizes. This is the smaller
 * one and it covers every non-exchange element; `bodyStyle` below is the other.
 * There is no third size and no muted grey on this card.
 */
export const metaStyle: React.CSSProperties = {
  fontSize: META_SIZE,
  fontWeight: 400,
  color: "var(--text-secondary)",
};

/**
 * The exchange: the question and the answer, at ONE size in the primary colour.
 *
 * Roman's ruling that they are EQUALS is the whole point. A question set smaller
 * or greyer than its answer reads as a caption on the answer; under oath they
 * carry the same weight, and a discovery answer is only interpretable next to
 * what was asked.
 */
export const bodyStyle: React.CSSProperties = {
  fontSize: BODY_SIZE,
  fontWeight: 400,
  lineHeight: BODY_LINE_HEIGHT,
  color: "var(--text-primary)",
  minWidth: 0,
  // A single unbroken token (a URL inside a quote) has nowhere to wrap and would
  // otherwise clip against the card's edge.
  overflowWrap: "anywhere",
};

export const chipStyle: React.CSSProperties = {
  ...metaStyle,
  border: "1px solid var(--border-card)",
  borderRadius: "999px",
  padding: "0.1rem 0.55rem",
  whiteSpace: "nowrap",
};

/**
 * An accusation chip, which unlike the others is a SENTENCE.
 *
 * ¶41's label runs to about two hundred characters, so this one wraps where the
 * short chips do not: `nowrap` on a chip that long pushes it past the container's
 * right edge, where the end — the part that distinguishes it from its neighbours
 * — is exactly what gets clipped.
 */
export const accusationChipStyle: React.CSSProperties = {
  ...chipStyle,
  whiteSpace: "normal",
  overflowWrap: "anywhere",
  textAlign: "left",
};
