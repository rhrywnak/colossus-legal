// =============================================================================
// FactRowParts — the pieces one fact card is built from (task 2.13c)
// =============================================================================
//
// Split out of `FactRow` when 2.13c pushed that file past the 300-line limit
// (Rule 17), following the `EvidenceChainParts` / `EvidenceExplorerParts`
// precedent already in this directory.
//
// Everything here is presentation over a `WorkingRow` and the stored wording.
// None of it decides anything: the card's ANATOMY — which parts appear and in
// what order — is data in `factsTable.sectionsFor`, and `FactRow` walks it.

import React from "react";

import { openViewerWindow } from "./viewerWindow";
import RemoveControl from "./FactRemoveControl";
import type { WorkingRow } from "./factsTable";
import type { LinkPanelWording } from "../services/evidenceLinks";
import type { FactTier } from "../services/scenarioCards";
import {
  accusationChipStyle,
  bodyStyle,
  chipStyle,
  META_SIZE,
  metaStyle,
} from "./factRowStyles";

/** The three weights, in the order clicking cycles them. */
const TIERS: FactTier[] = ["carries", "backup", "background"];

/**
 * One glyph, three states — filled, outline, dimmed.
 *
 * Task 2.13c item 4 replaced a row of three radio glyphs with this. Roman's
 * words: the three-icon pile read as an icon pile, and the dot in particular
 * looked like decoration rather than a control that would make the card vanish.
 */
const TIER_GLYPH: Record<FactTier, string> = {
  carries: "★",
  backup: "☆",
  background: "☆",
};

/** What the next click produces — carries → backup → background → carries. */
export function nextTier(current: FactTier | null): FactTier {
  const at = TIERS.indexOf(current ?? "backup");
  return TIERS[(at + 1) % TIERS.length];
}

/**
 * The weight control: ONE star, and the current weight spelled out beside it.
 *
 * ## Why the name is on screen and not in a tooltip
 *
 * The previous control carried its vocabulary in `title` and `aria-label` only,
 * so a sighted human using a mouse could not learn what the three glyphs MEANT
 * without hovering each one. Roman's ruling: a feature must disclose itself on
 * screen. The label is a settings-served string, so renaming a tier renames it
 * here with no rebuild.
 *
 * ## Why cycling is safe here when it usually is not
 *
 * A cycling control normally hides its own vocabulary — the objection that put
 * three buttons here in 2.13. What changed is that the current state is now
 * WRITTEN OUT beside the glyph, and the hint under the section header names all
 * three weights, so nothing is hidden and one click is enough.
 */
export const TierControl: React.FC<{
  row: WorkingRow;
  wording: LinkPanelWording;
  onSetTier: (tier: FactTier) => void;
}> = ({ row, wording, onSetTier }) => {
  const label: Record<FactTier, string> = {
    carries: wording.fact_tier_carries_label,
    backup: wording.fact_tier_backup_label,
    background: wording.fact_tier_background_label,
  };
  const current = row.tier ?? "backup";
  const next = nextTier(current);

  return (
    <button
      type="button"
      onClick={() => onSetTier(next)}
      title={`${wording.fact_tier_prompt} — ${label[current]}`}
      aria-label={`${wording.fact_tier_prompt} ${label[current]}`}
      style={{
        display: "flex",
        alignItems: "center",
        gap: "0.3rem",
        border: "none",
        background: "none",
        cursor: "pointer",
        padding: 0,
        fontFamily: "inherit",
        fontSize: META_SIZE,
        // --text-secondary for EVERY state, including background.
        //
        // This read `--text-muted` for the background tier until the 2.13c gate
        // caught it: 3.10:1, on an interactive label a human must read to know
        // which state they are cycling away from — and in the same commit that
        // declares muted "not card content". The switched-off reading is carried
        // by the tier NAME beside the glyph, which is text, not by dimming the
        // one thing that says what the control currently is.
        color: "var(--text-secondary)",
      }}
    >
      <span aria-hidden="true" style={{ color: current === "carries" ? "var(--accent-primary)" : "inherit" }}>
        {TIER_GLYPH[current]}
      </span>
      {label[current]}
    </button>
  );
};

/** The pinpoint chip, or nothing. A human fact has no pinpoint by design (§8). */
export const PinpointChip: React.FC<{ row: WorkingRow; onBlocked: (m: string | null) => void }> = ({
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
export const HeaderRow: React.FC<{
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
 * One line of the exchange — the `Q:` or `A:` prefix and its text, aligned.
 *
 * ## The hanging indent is the point
 *
 * Both prefixes are bold and reserve the SAME width, so the question text and the
 * answer text start at one column and wrap back to that column rather than under
 * the prefix. The two form aligned blocks a reader's eye can run down, instead of
 * two paragraphs that happen to be stacked. Same size, same colour, same column —
 * Roman's ruling that they are equals.
 *
 * Before task 2.13c the question was set at metadata size in a muted grey while
 * the answer was body text: a caption on its own answer, when under oath the two
 * carry the same weight and the answer means nothing without the question.
 *
 * Both prefixes are settings-served, so "Q:" / "A:" can become "Question:" /
 * "Answer:" with no rebuild — the shared column simply widens.
 */
export const ExchangeLine: React.FC<{ prefix: string; text: string }> = ({ prefix, text }) => (
  <div style={{ display: "flex", gap: "0.45rem", alignItems: "baseline" }}>
    <span style={{ ...bodyStyle, fontWeight: 600, flexShrink: 0, minWidth: "1.6em" }}>
      {prefix}
    </span>
    <span style={bodyStyle}>{text}</span>
  </div>
);

/**
 * The quote when there is NO question — plain body text, no prefix.
 *
 * A lone `A:` on a letter or a court ruling would imply a question that was lost.
 * Documentary evidence simply speaks.
 */
export const QuoteRow: React.FC<{ text: string }> = ({ text }) => <div style={bodyStyle}>{text}</div>;

/** What this fact bears on. */
export const AllegationRow: React.FC<{ bearsOn: string[] }> = ({ bearsOn }) => (
  <div style={{ display: "flex", gap: "0.4rem", flexWrap: "wrap" }}>
    {bearsOn.map((accusation) => (
      <span key={accusation} style={accusationChipStyle}>
        {accusation}
      </span>
    ))}
  </div>
);

/** Where it lives, and what may be done with it here. */
export const SourceRow: React.FC<{
  row: WorkingRow;
  wording: LinkPanelWording | null;
  onBlocked: (m: string | null) => void;
  onRemove?: () => void;
  onUnplace?: () => void;
  confirm?: LinkPanelWording | null;
}> = ({ row, wording, onBlocked, onRemove, onUnplace, confirm = null }) => (
  <div style={{ display: "flex", gap: "0.4rem", flexWrap: "wrap", alignItems: "center" }}>
    <PinpointChip row={row} onBlocked={onBlocked} />
    {/* A HUMAN fact's provenance line ("Added by Roman · Around Mar 2010") stays:
        it is that row's authority and the words backing its coloured spine.

        The EVIDENCE status tag does not. Task 2.13c item 6: every fact on this
        list is in the scenario by definition, so "In the scenario" on all
        forty-six carried zero bits — it was queue vocabulary that followed the
        card here and made the row longer without making it say more. */}
    {row.isHuman && <span style={metaStyle}>{row.statusLabel}</span>}

    <span style={{ marginLeft: "auto", display: "flex", alignItems: "center", gap: "0.5rem" }}>
      {/* Only a fact somebody has actually dragged can be un-placed; offering it
          on a card in its natural position would be a control that does nothing. */}
      {onUnplace && wording && row.sortOrdinal !== null && (
        <button
          type="button"
          onClick={onUnplace}
          style={{
            ...metaStyle,
            border: "none",
            background: "none",
            padding: 0,
            cursor: "pointer",
            fontFamily: "inherit",
            textDecoration: "underline",
          }}
        >
          {wording.fact_unplace_label}
        </button>
      )}
      {onRemove && <RemoveControl row={row} onRemove={onRemove} confirm={confirm} />}
    </span>
  </div>
);

