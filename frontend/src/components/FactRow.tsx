// =============================================================================
// FactRow — one row of the Facts table (extracted from WorkingView, task 2.13)
// =============================================================================
//
// EXTRACTION, then addition. `WorkingView.tsx` was 412 lines before this task —
// already past the 300-line limit (Rule 17) — and slice 1 adds a weight control,
// a drag handle and a question line to every row. Growing that file further was
// not an option, so the row moved here first, unchanged, and the new parts were
// built on top. The move and the additions are separate concerns and are called
// out separately in the commit message.
//
// ## What this renders, and what it refuses to
//
// Every string on screen is either a payload field or a stored setting. This
// component composes no sentence: it places the question the extraction captured
// above the answer, prints the kind the extraction recorded, and labels its own
// controls from the words the server served. The one substitution it performs is
// dropping a count into a stored template, which `fillCount` does with a single
// named slot and no expression language.

import React, { useState } from "react";

import { openViewerWindow } from "./viewerWindow";
import RemoveControl from "./FactRemoveControl";
import type { WorkingRow } from "./factsTable";
import type { LinkPanelWording } from "../services/evidenceLinks";
import type { FactTier } from "../services/scenarioCards";

const HAIRLINE = "1px solid var(--border-default)";

const rowStyle: React.CSSProperties = {
  display: "flex",
  gap: "12px",
  padding: "12px 16px 12px 12px",
  borderBottom: HAIRLINE,
  fontWeight: 400,
  alignItems: "stretch",
};

const arrivedRowStyle: React.CSSProperties = {
  ...rowStyle,
  background: "var(--state-warning-bg-soft)",
  transition: "background 600ms ease-out",
};

/**
 * The row being dragged over, so the human can see where it would land.
 *
 * Purely presentational and deliberately NOT a stored setting (ruling R5's line):
 * it changes how a drop target looks, never what the list contains or means.
 */
const dropTargetRowStyle: React.CSSProperties = {
  ...rowStyle,
  borderTop: "2px solid var(--accent-primary)",
};

const stripeStyle = (isHuman: boolean): React.CSSProperties => ({
  width: "4px",
  borderRadius: "2px",
  flexShrink: 0,
  alignSelf: "stretch",
  minHeight: "44px",
  background: isHuman ? "var(--accent-primary)" : "var(--state-success-strong)",
});

const chipStyle: React.CSSProperties = {
  border: HAIRLINE,
  borderRadius: "999px",
  padding: "0.1rem 0.55rem",
  fontSize: "0.75rem",
  color: "var(--text-muted)",
  whiteSpace: "nowrap",
};

const accusationChipStyle: React.CSSProperties = {
  ...chipStyle,
  whiteSpace: "normal",
  overflowWrap: "anywhere",
  textAlign: "left",
};

/**
 * The §7.1 question, above the answer it makes interpretable.
 *
 * ## Domain note: why this line is the whole point of slice 1
 *
 * A discovery answer reading "Yes" is noise on its own. Under the interrogatory
 * it responds to, the same word is a sworn admission — which is why the label is
 * a stored setting rather than a hardcoded "Q:", and why the line renders only
 * when the extraction actually captured a question. Documentary evidence has
 * none, and shows none: an empty `Q:` would assert that a question exists and
 * was lost.
 */
const QuestionLine: React.FC<{ question: string; label: string }> = ({ question, label }) => (
  <div style={{ display: "flex", gap: "0.4rem", alignItems: "baseline" }}>
    <span style={{ ...chipStyle, border: "none", padding: 0, fontWeight: 600 }}>{label}</span>
    <span
      style={{
        fontSize: "0.85rem",
        lineHeight: 1.5,
        color: "var(--text-muted)",
        minWidth: 0,
        flex: 1,
        overflowWrap: "anywhere",
      }}
    >
      {question}
    </span>
  </div>
);

/** The three weights, in the order they read as a scale. */
const TIERS: FactTier[] = ["carries", "backup", "background"];

/** The glyph for each weight — filled, half, hollow. */
const TIER_GLYPH: Record<FactTier, string> = {
  carries: "★",
  backup: "☆",
  background: "·",
};

/**
 * The weight control: three states, on the row it is printed on.
 *
 * ## Why three buttons and not one cycling button
 *
 * A control that cycles hides its own vocabulary — a human has to click twice to
 * discover what the third state even is, and cannot go back without going
 * forward. Three labelled buttons say what the three weights ARE, let any one be
 * chosen in one click, and read correctly to a screen reader as a radio group.
 *
 * Every label comes from the store, so renaming "Carries the scenario" is a
 * Settings edit and no rebuild (the signed design left the names open on purpose).
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
    <div role="radiogroup" aria-label={wording.fact_tier_prompt} style={{ display: "flex", gap: "0.15rem" }}>
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
              fontSize: "0.95rem",
              lineHeight: 1,
              color: active ? "var(--accent-primary)" : "var(--text-muted)",
              opacity: active ? 1 : 0.55,
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

/** The row's second line: what it bears on, where it is, and its provenance. */
const RowMeta: React.FC<{
  row: WorkingRow;
  wording: LinkPanelWording | null;
  onBlocked: (m: string | null) => void;
  onRemove?: () => void;
  onSetTier?: (tier: FactTier) => void;
  confirm?: LinkPanelWording | null;
}> = ({ row, wording, onBlocked, onRemove, onSetTier, confirm = null }) => (
  <div style={{ display: "flex", gap: "0.4rem", flexWrap: "wrap", alignItems: "center" }}>
    {row.bearsOn.map((accusation) => (
      <span key={accusation} style={accusationChipStyle}>
        {accusation}
      </span>
    ))}
    <PinpointChip row={row} onBlocked={onBlocked} />
    <span style={{ ...chipStyle, marginLeft: "auto" }}>{row.statusLabel}</span>
    {/* The weight control is WITHHELD until the wording loads, exactly as the
        Remove control is (R4): an unlabelled control that changes a human's own
        curation judgment is worse than no control for the second it takes. */}
    {wording && onSetTier && <TierControl row={row} wording={wording} onSetTier={onSetTier} />}
    {onRemove && <RemoveControl row={row} onRemove={onRemove} confirm={confirm} />}
  </div>
);

/** One Facts-table row. */
const FactRow: React.FC<{
  row: WorkingRow;
  /** Tinted briefly because this row was not in the previous payload. */
  justArrived?: boolean;
  /** The stored words, or `null` until they load. */
  wording: LinkPanelWording | null;
  onRemove?: () => void;
  onSetTier?: (tier: FactTier) => void;
  /** Drag plumbing. Absent for a row that cannot be reordered (a human fact). */
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
  // row so the message appears where the human just clicked.
  const [blocked, setBlocked] = useState<string | null>(null);
  const [dragOver, setDragOver] = useState(false);

  const draggable = Boolean(onDragStart && onDropOn);

  return (
    <div
      style={dragOver ? dropTargetRowStyle : justArrived ? arrivedRowStyle : rowStyle}
      draggable={draggable}
      onDragStart={onDragStart}
      onDragOver={(event) => {
        if (!draggable) return;
        // Without `preventDefault` the browser refuses the drop outright — this
        // is what makes the row a legal target, not a styling concern.
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
      <span style={stripeStyle(row.isHuman)} aria-hidden="true" />
      {/* The handle is a separate affordance from the row body so a human can
          still select the quote's text without starting a drag. */}
      {draggable && wording && (
        <span
          aria-label={wording.fact_order_drag_hint}
          title={wording.fact_order_drag_hint}
          style={{ cursor: "grab", color: "var(--text-muted)", alignSelf: "center", fontSize: "0.9rem" }}
        >
          ⠿
        </span>
      )}
      <div style={{ display: "flex", flexDirection: "column", gap: "0.4rem", flex: 1 }}>
        {/* §7.1: the question, then the answer it makes interpretable. */}
        {row.question && wording && (
          <QuestionLine question={row.question} label={wording.fact_question_label} />
        )}

        <div style={{ display: "flex", gap: "0.5rem", alignItems: "baseline", flexWrap: "wrap" }}>
          {row.code && (
            <span style={{ ...chipStyle, color: "var(--text-primary)" }}>{row.code}</span>
          )}
          {/* The extraction's own word for what kind of statement this is. Served
              as stored — the vocabulary is mixed across extraction generations
              ("admission", "evasive", "attorney argument"), and Roman ruled it
              ships as-is: an admission is gold, and the browser normalizing it
              would be the frontend deciding what a document said. */}
          {row.statementKind && wording && (
            <span style={chipStyle}>
              {wording.fact_statement_kind_label} {row.statementKind}
            </span>
          )}
          <span
            style={{
              fontSize: "0.95rem",
              lineHeight: 1.6,
              color: "var(--text-primary)",
              minWidth: 0,
              flex: 1,
              overflowWrap: "anywhere",
            }}
          >
            {row.text}
          </span>
        </div>

        <RowMeta
          row={row}
          wording={wording}
          onBlocked={setBlocked}
          onRemove={onRemove}
          onSetTier={onSetTier}
          confirm={confirm}
        />

        {blocked && (
          <div role="alert" style={{ fontSize: "0.78rem", color: "var(--state-danger-strong)" }}>
            {blocked}
          </div>
        )}
      </div>
    </div>
  );
};

export default FactRow;
