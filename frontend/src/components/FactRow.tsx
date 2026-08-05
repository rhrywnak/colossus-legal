// =============================================================================
// FactRow — one fact CARD, with a fixed anatomy (tasks 2.13, 2.13b)
// =============================================================================
//
// Extracted from `WorkingView` in 2.13 (which was 412 lines, past Rule 17), then
// rebuilt in 2.13b against Roman's own screenshot of S-2 on beta.377:
//
//   "They are not human friendly. everything just blurs together. also no
//    consistency within a card. 1) it is difficult to see where one card ends
//    and the next starts. 2) the Candidate numbers appear in different places
//    cards. the text is light grey and very difficult to read. the tags and
//    buttons are also difficult to see."
//
// Every one of those is a decision this file used to get wrong:
//
// * **Where a card ends.** Rows shared one surface and were divided by a
//   near-invisible line (#eef0f3, 1.14:1). Each fact is now its own card with a
//   `--border-card` hairline, and the gap BETWEEN cards is more than three times
//   the largest gap inside one — proximity separates, the border assists.
// * **Where the number is.** The C-code, the kind and the quote shared one
//   `flex-wrap` row, so the code moved with the length of whatever sat beside it.
//   The header row is now its own line and the code is always its first element.
//   The order comes from `sectionsFor`, which is data and is tested.
// * **Light grey text.** `--text-muted` was 3.10:1 on this surface. It is now
//   4.97:1, and metadata recedes by SIZE rather than by fading (study §2).
//
// ## Typography: one stack, one body size, one bold
//
// There is no per-element font styling below. `--font-sans` at regular weight
// throughout; exactly one body size for the quote with a 1.6 line-height; every
// piece of metadata one step down at `--meta-size`. The ONLY bold on this
// surface is the C-code, which is the card's landmark. That is the study's §2
// binding, and the reason the ad-hoc `fontSize`/`fontWeight` values that used to
// live on individual spans are gone.

import React, { useState } from "react";

import { openViewerWindow } from "./viewerWindow";
import RemoveControl from "./FactRemoveControl";
import { sectionsFor, type CardSection, type WorkingRow } from "./factsTable";
import type { LinkPanelWording } from "../services/evidenceLinks";
import type { FactTier } from "../services/scenarioCards";

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
const BODY_SIZE = "0.95rem";

// CONST: every piece of metadata, one step down from the body.
// The study requires metadata be distinguished by SIZE, not by fading to grey —
// which is what --text-muted at 3.10:1 was doing before task 2.13b.
const META_SIZE = "0.8rem";

// CONST: line-height for the quote. ≈1.6 is the "generous" the ruling names;
// a legal quote runs long and tight leading is what makes a card a wall.
const BODY_LINE_HEIGHT = 1.6;

// ── Spacing rhythm (ruling 2) ───────────────────────────────────────────────

// CONST: the gap between rows INSIDE one card. Must stay the largest such gap —
// `MAX_INTRA_GAP_PX` below is its numeric twin and the fence compares them.
const INTRA_GAP = "6px";

// CONST: the card's own padding. Not part of the rhythm comparison: it is the
// distance from the border to the content, not between two pieces of content.
const CARD_PADDING = "14px 16px";

// CONST: the gap BETWEEN cards. Ruling 2 — proximity does the separating and the
// border assists, so this must be decisively larger than any intra-card gap.
// Exported because `WorkingView` lays out the stack and
// `scenarioPageStructure.test.ts` asserts the ratio against the value below.
export const CARD_GAP_PX = 20;

// CONST: the largest intra-card gap, as a number, for that same fence.
// Kept beside `INTRA_GAP` so the two cannot drift: if a row gap grows past this,
// the ratio test fails rather than the rhythm quietly flattening.
export const MAX_INTRA_GAP_PX = 6;

const cardStyle = (justArrived: boolean, isDropTarget: boolean): React.CSSProperties => ({
  display: "flex",
  gap: "12px",
  alignItems: "stretch",
  padding: CARD_PADDING,
  // Ruling 1: the hairline stays and is now visible. `--border-card`, not
  // `--border-default` — the divider token keeps its own job elsewhere.
  border: "1px solid var(--border-card)",
  borderRadius: "var(--radius-card)",
  // The drop indicator REPLACES the top border rather than adding to it, so a
  // card never changes height while being dragged over — a 1px reflow that
  // ripples down a list of forty-six is how a drop target ends up somewhere
  // other than where the human aimed.
  borderTop: isDropTarget ? "2px solid var(--accent-primary)" : "1px solid var(--border-card)",
  background: justArrived ? "var(--state-warning-bg-soft)" : "var(--bg-surface)",
  transition: "background 600ms ease-out",
});

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
const spineStyle = (isHuman: boolean): React.CSSProperties => ({
  width: "4px",
  borderRadius: "2px",
  flexShrink: 0,
  alignSelf: "stretch",
  background: isHuman ? "var(--accent-primary)" : "var(--state-success-strong)",
});

/** Metadata: one size step down, regular weight, and now AA-legible. */
const metaStyle: React.CSSProperties = {
  fontSize: META_SIZE,
  fontWeight: 400,
  color: "var(--text-muted)",
};

const chipStyle: React.CSSProperties = {
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
const accusationChipStyle: React.CSSProperties = {
  ...chipStyle,
  whiteSpace: "normal",
  overflowWrap: "anywhere",
  textAlign: "left",
};

/** The three weights, in the order they read as a scale. */
const TIERS: FactTier[] = ["carries", "backup", "background"];
const TIER_GLYPH: Record<FactTier, string> = {
  carries: "★",
  backup: "☆",
  background: "·",
};

/**
 * The weight control: three states, on the card it is printed on.
 *
 * Three labelled buttons rather than one cycling button — a control that cycles
 * hides its own vocabulary, needs two clicks to reveal the third state, and
 * cannot go back without going forward. Every label is a stored setting, so
 * renaming "Carries the scenario" is a Settings edit and no rebuild.
 */
const TierControl: React.FC<{
  row: WorkingRow;
  wording: LinkPanelWording;
  onSetTier: (tier: FactTier) => void;
}> = ({ row, wording, onSetTier }) => {
  const label: Record<FactTier, string> = {
    carries: wording.fact_tier_carries_label,
    backup: wording.fact_tier_backup_label,
    background: wording.fact_tier_background_label,
  };

  return (
    <div
      role="radiogroup"
      aria-label={wording.fact_tier_prompt}
      style={{ display: "flex", gap: "0.15rem" }}
    >
      {TIERS.map((tier) => {
        const active = row.tier === tier;
        return (
          <button
            key={tier}
            type="button"
            role="radio"
            aria-checked={active}
            title={`${wording.fact_tier_prompt} — ${label[tier]}`}
            aria-label={label[tier]}
            onClick={() => onSetTier(tier)}
            style={{
              border: "none",
              background: "none",
              cursor: "pointer",
              padding: "0 0.15rem",
              fontSize: BODY_SIZE,
              lineHeight: 1,
              // Roman: "the tags and buttons are also difficult to see." An
              // inactive weight is now --text-secondary (6.00:1) rather than
              // muted-at-55%-opacity, which was illegible twice over.
              color: active ? "var(--accent-primary)" : "var(--text-secondary)",
            }}
          >
            {TIER_GLYPH[tier]}
          </button>
        );
      })}
    </div>
  );
};

/** The pinpoint chip, or nothing. A human fact has no pinpoint by design (§8). */
const PinpointChip: React.FC<{ row: WorkingRow; onBlocked: (m: string | null) => void }> = ({
  row,
  onBlocked,
}) => {
  if (!row.pinpointHref) return null;
  return (
    <a
      href={row.pinpointHref}
      onClick={(event) => {
        event.preventDefault();
        const result = openViewerWindow(row.pinpointHref);
        onBlocked(result.opened ? null : result.message);
      }}
      style={{ ...chipStyle, color: "var(--accent-primary)", textDecoration: "none" }}
    >
      {row.pinpointLabel} ↗
    </a>
  );
};

/**
 * The header row — the card's landmark line.
 *
 * C-code first, ALWAYS, then kind, then speaker; weight control and drag handle
 * to the right. The code is the one bold thing on the card and the one thing at
 * a fixed position: `flexShrink: 0` and its own element, so nothing beside it can
 * push it anywhere. This is the direct answer to "the Candidate numbers appear in
 * different places cards".
 */
const HeaderRow: React.FC<{
  row: WorkingRow;
  wording: LinkPanelWording | null;
  onSetTier?: (tier: FactTier) => void;
  draggable: boolean;
}> = ({ row, wording, onSetTier, draggable }) => (
  <div
    style={{
      display: "flex",
      alignItems: "center",
      gap: "0.5rem",
      minHeight: "22px",
    }}
  >
    {/* The landmark. A human fact has no candidate number and shows none — an
        em-dash placeholder would imply a missing value where there is none. */}
    {row.code && (
      <span
        data-fact-code
        style={{
          fontSize: META_SIZE,
          fontWeight: 600,
          color: "var(--text-primary)",
          flexShrink: 0,
        }}
      >
        {row.code}
      </span>
    )}
    {row.statementKind && wording && (
      <span style={metaStyle}>
        {wording.fact_statement_kind_label} {row.statementKind}
      </span>
    )}
    {row.speaker && <span style={metaStyle}>{row.speaker}</span>}

    <span style={{ marginLeft: "auto", display: "flex", alignItems: "center", gap: "0.4rem" }}>
      {wording && onSetTier && (
        <TierControl row={row} wording={wording} onSetTier={onSetTier} />
      )}
      {draggable && wording && (
        <span
          aria-label={wording.fact_order_drag_hint}
          title={wording.fact_order_drag_hint}
          style={{ cursor: "grab", color: "var(--text-secondary)", fontSize: META_SIZE }}
        >
          ⠿
        </span>
      )}
    </span>
  </div>
);

/**
 * The §7.1 question, indented, above the answer it makes interpretable.
 *
 * A discovery answer reading "Yes" is noise alone; under the interrogatory it
 * responds to, the same word is a sworn admission. Rendered only when the
 * extraction captured one — documentary evidence has none, and an empty `Q:`
 * would assert that a question exists and was lost.
 */
const QuestionRow: React.FC<{ question: string; label: string }> = ({ question, label }) => (
  <div style={{ display: "flex", gap: "0.4rem", paddingLeft: "0.75rem" }}>
    <span style={{ ...metaStyle, fontWeight: 600, flexShrink: 0 }}>{label}</span>
    <span style={{ ...metaStyle, lineHeight: BODY_LINE_HEIGHT, minWidth: 0 }}>{question}</span>
  </div>
);

/** The quote — the one body size on the card. */
const QuoteRow: React.FC<{ text: string }> = ({ text }) => (
  <div
    style={{
      fontSize: BODY_SIZE,
      fontWeight: 400,
      lineHeight: BODY_LINE_HEIGHT,
      color: "var(--text-primary)",
      minWidth: 0,
      // A single unbroken token (a URL inside a quote) has nowhere to wrap and
      // would clip against the card's edge.
      overflowWrap: "anywhere",
    }}
  >
    {text}
  </div>
);

/** What this fact bears on. */
const AllegationRow: React.FC<{ bearsOn: string[] }> = ({ bearsOn }) => (
  <div style={{ display: "flex", gap: "0.4rem", flexWrap: "wrap" }}>
    {bearsOn.map((accusation) => (
      <span key={accusation} style={accusationChipStyle}>
        {accusation}
      </span>
    ))}
  </div>
);

/** Where it lives, and what may be done with it here. */
const SourceRow: React.FC<{
  row: WorkingRow;
  onBlocked: (m: string | null) => void;
  onRemove?: () => void;
  confirm?: LinkPanelWording | null;
}> = ({ row, onBlocked, onRemove, confirm = null }) => (
  <div style={{ display: "flex", gap: "0.4rem", flexWrap: "wrap", alignItems: "center" }}>
    <PinpointChip row={row} onBlocked={onBlocked} />
    {/* For evidence this is the ruling state; for a human fact the composed
        "Added by Roman · Around Mar 2010". Either way it is the row's AUTHORITY,
        and it is the words backing the coloured spine on the left. */}
    <span style={{ ...metaStyle, marginLeft: "auto" }}>{row.statusLabel}</span>
    {onRemove && <RemoveControl row={row} onRemove={onRemove} confirm={confirm} />}
  </div>
);

/** One fact card. */
const FactRow: React.FC<{
  row: WorkingRow;
  justArrived?: boolean;
  wording: LinkPanelWording | null;
  onRemove?: () => void;
  onSetTier?: (tier: FactTier) => void;
  onDragStart?: () => void;
  onDropOn?: () => void;
  confirm?: LinkPanelWording | null;
}> = ({
  row,
  justArrived = false,
  wording,
  onRemove,
  onSetTier,
  onDragStart,
  onDropOn,
  confirm = null,
}) => {
  // A refused popup has to be SAID, not swallowed (Standing Rule 1). Local to the
  // card so the message appears where the human just clicked.
  const [blocked, setBlocked] = useState<string | null>(null);
  const [dragOver, setDragOver] = useState(false);

  const draggable = Boolean(onDragStart && onDropOn);

  /**
   * The renderer for one section. Walking `sectionsFor` rather than writing the
   * rows out inline is what makes the anatomy testable: this switch cannot emit a
   * section the list did not include, and cannot emit them in another order.
   */
  const render = (section: CardSection) => {
    switch (section) {
      case "header":
        return (
          <HeaderRow
            key={section}
            row={row}
            wording={wording}
            onSetTier={onSetTier}
            draggable={draggable}
          />
        );
      case "question":
        return row.question && wording ? (
          <QuestionRow key={section} question={row.question} label={wording.fact_question_label} />
        ) : null;
      case "quote":
        return <QuoteRow key={section} text={row.text} />;
      case "allegations":
        return <AllegationRow key={section} bearsOn={row.bearsOn} />;
      case "source":
        return (
          <SourceRow
            key={section}
            row={row}
            onBlocked={setBlocked}
            onRemove={onRemove}
            confirm={confirm}
          />
        );
    }
  };

  return (
    <div
      style={cardStyle(justArrived, dragOver)}
      draggable={draggable}
      onDragStart={onDragStart}
      onDragOver={(event) => {
        if (!draggable) return;
        // Without `preventDefault` the browser refuses the drop outright — this
        // is what makes the card a legal target, not a styling concern.
        event.preventDefault();
        setDragOver(true);
      }}
      onDragLeave={() => setDragOver(false)}
      onDrop={(event) => {
        if (!draggable) return;
        event.preventDefault();
        setDragOver(false);
        onDropOn?.();
      }}
    >
      <span style={spineStyle(row.isHuman)} aria-hidden="true" />
      <div style={{ display: "flex", flexDirection: "column", gap: INTRA_GAP, flex: 1, minWidth: 0 }}>
        {sectionsFor(row).map(render)}
        {blocked && (
          <div role="alert" style={{ ...metaStyle, color: "var(--state-danger-strong)" }}>
            {blocked}
          </div>
        )}
      </div>
    </div>
  );
};

export default FactRow;
