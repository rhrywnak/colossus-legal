// =============================================================================
// ElementAllegationList.tsx — one labeled section of mapped Allegations, each
// with the evidence under it (PROOF_MATRIX_v2 §3)
// -----------------------------------------------------------------------------
// The paragraph, then its two stance lists: the top few, a "N more" control, and
// a "show hidden (N)" toggle at the foot. Each row is a `MatrixEvidenceRow`,
// which owns everything about one item; this file owns the LIST — the folds, the
// optimistic update, and the error line when a write does not land.
//
// ## What §3 removed, and why
//
// The machine `summary` line above the paragraph is gone: it was a model's
// précis of words the complaint states exactly, printed above those very words.
// The Strong/Hedged/Other tier chip is gone with it — the reader-facing page
// leads with what the linking pass and a human said, not with a claim about how
// hard an item is to dispute. What is left above the quote is `¶n` and the
// complaint's own sentence.
//
// ## Why the update is optimistic
//
// A reader goes through a paragraph of five items in a few seconds. A round trip
// between each click would make the page feel broken, so the row moves at once
// and the write follows. That makes a silent failure the worst outcome available
// — so a failed write puts the served refusal sentence on screen AND rolls the
// row back, which is the only pair of behaviours that leaves the screen agreeing
// with the database.
// =============================================================================

import React, { useCallback, useMemo, useState } from "react";
import {
  AllegationSummary,
  AllegationEvidence,
} from "../services/elementDetailService";
import type { MatrixWording } from "../services/causesOfAction";
import {
  saveRuling,
  withdrawRuling,
  type MatrixRulingToken,
} from "../services/matrixRulings";
import MatrixEvidenceRow from "./MatrixEvidenceRow";
import {
  hiddenToggleLabel,
  moreLabel,
  splitEvidence,
} from "./matrixRow";

/** How a ruling changes one item, applied locally before the write returns. */
type LocalRuling = {
  ruling: string | null;
  ruled_by: string | null;
  hidden_reason: string | null;
};

/**
 * The item as it will look once the write lands.
 *
 * ## Why the frontend computes this at all, given Rule 19
 *
 * It is not deriving state — it is PREDICTING the state the backend will return,
 * for the moment between the click and the reply, and it is thrown away the
 * instant the parent re-fetches. The rules it predicts are the backend's own,
 * stated once here: a `keep` shows the item (overruling a machine retraction), a
 * `remove` hides it, and an undo returns it to whatever the machine said.
 */
function optimistic(
  item: AllegationEvidence,
  ruling: MatrixRulingToken | null,
  actor: string,
): LocalRuling {
  if (ruling === null) {
    // Withdrawn: back to the machine's own claim, whatever that was.
    return {
      ruling: null,
      ruled_by: null,
      hidden_reason: item.role === "does_not_belong" ? "does_not_belong" : null,
    };
  }
  return {
    ruling,
    ruled_by: actor,
    hidden_reason: ruling === "remove" ? "removed" : null,
  };
}

interface EvidenceLegProps {
  caseSlug: string;
  allegationId: string;
  evidence: AllegationEvidence[];
  /** The served `visible_items` — how many show before "N more". */
  limit: number;
  /** Empty-state wording; the two legs mean different things when empty. */
  emptyLabel: string;
  /** Left rule colour, so a rebuttal can never read as corroboration. */
  accent: string;
  wording: MatrixWording;
}

/**
 * One stance list under one accusation: the short list, the overflow, the hidden
 * group, and the two controls that open them.
 */
const EvidenceLeg: React.FC<EvidenceLegProps> = ({
  caseSlug,
  allegationId,
  evidence,
  limit,
  emptyLabel,
  accent,
  wording,
}) => {
  const [showAll, setShowAll] = useState(false);
  const [showHidden, setShowHidden] = useState(false);
  const [pendingId, setPendingId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Rulings this session has applied but not yet re-fetched, keyed by item id.
  const [local, setLocal] = useState<Record<string, LocalRuling>>({});

  // The served list with any local rulings laid over it. `useMemo` because the
  // overlay walks every item and the leg re-renders on every hover of a control.
  const items = useMemo(
    () =>
      evidence.map((item) =>
        local[item.id] ? { ...item, ...local[item.id] } : item,
      ),
    [evidence, local],
  );

  const write = useCallback(
    async (item: AllegationEvidence, ruling: MatrixRulingToken | null) => {
      setPendingId(item.id);
      setError(null);
      // "you" rather than a username: the browser does not know the
      // authenticated name, and the next read replaces this with the one the
      // backend recorded. Guessing a name would put a wrong attribution on a
      // proof surface, briefly, which is worse than a vague true one.
      setLocal((prev) => ({ ...prev, [item.id]: optimistic(item, ruling, "you") }));
      try {
        if (ruling === null) {
          await withdrawRuling(caseSlug, item.id, allegationId);
        } else {
          await saveRuling(caseSlug, item.id, allegationId, ruling);
        }
      } catch (err: unknown) {
        const detail = err instanceof Error ? err.message : "unknown error";
        setError(wording.ruling_failed_template.replace("{detail}", detail));
        // Roll the row back. The screen and the database disagree until this
        // line runs, and the message above is what says so.
        setLocal((prev) => {
          const next = { ...prev };
          delete next[item.id];
          return next;
        });
      } finally {
        setPendingId(null);
      }
    },
    [caseSlug, allegationId, wording.ruling_failed_template],
  );

  if (items.length === 0) {
    return <div style={NO_EVIDENCE_STYLE}>{emptyLabel}</div>;
  }

  const { shown, behindMore, hidden } = splitEvidence(items, limit);
  const visible = showAll ? [...shown, ...behindMore] : shown;
  const more = moreLabel(behindMore, wording);
  const hiddenLabel = hiddenToggleLabel(hidden, showHidden, wording);

  const row = (item: AllegationEvidence) => (
    <MatrixEvidenceRow
      key={item.id}
      item={item}
      accent={accent}
      wording={wording}
      pending={pendingId === item.id}
      onRule={(id, ruling) => {
        const target = items.find((e) => e.id === id);
        if (target) void write(target, ruling);
      }}
      onUndo={(id) => {
        const target = items.find((e) => e.id === id);
        if (target) void write(target, null);
      }}
    />
  );

  return (
    <div style={LEG_STYLE}>
      {visible.map(row)}
      {error && <div style={ERROR_LINE_STYLE}>{error}</div>}
      {more && !showAll && (
        <button type="button" style={FOLD_BUTTON_STYLE} onClick={() => setShowAll(true)}>
          {more}
        </button>
      )}
      {hiddenLabel && (
        <button
          type="button"
          style={FOLD_BUTTON_STYLE}
          onClick={() => setShowHidden((v) => !v)}
        >
          {hiddenLabel}
        </button>
      )}
      {showHidden && hidden.map(row)}
    </div>
  );
};

// ─── Allegation section ──────────────────────────────────────────────────────

export interface AllegationSectionProps {
  label: string;
  labelColor: string;
  labelBg: string;
  accentColor: string;
  allegations: AllegationSummary[];
  caseSlug: string;
  /** The served `visible_items`, threaded from the Element detail payload. */
  visibleItems: number;
  /** The Proof Matrix's served words. Every visible word on the page. */
  wording: MatrixWording;
}

/**
 * One labeled group of allegation cards (Common / Count-specific / Other).
 *
 * The card is now `¶n` + the complaint's own words, and nothing else above the
 * evidence — see the module header on what §3 removed.
 */
const AllegationSection: React.FC<AllegationSectionProps> = ({
  label,
  labelColor,
  labelBg,
  accentColor,
  allegations,
  caseSlug,
  visibleItems,
  wording,
}) => (
  <div>
    <div style={SECTION_DIVIDER_STYLE_BASE}>
      <span
        style={{
          color: labelColor,
          backgroundColor: labelBg,
          padding: "2px 8px",
          borderRadius: "12px",
        }}
      >
        {label}
      </span>
      <span style={SECTION_DIVIDER_RULE_STYLE} />
    </div>
    {allegations.map((a) => (
      <div key={a.allegation_id} style={ALLEGATION_CARD_STYLE}>
        <div>
          <span style={PARAGRAPH_LABEL_STYLE}>¶{a.paragraph_number}</span>
        </div>
        {a.verbatim_quote && (
          <div
            className="proof-text"
            style={{ ...QUOTE_TEXT_STYLE_BASE, borderLeft: `3px solid ${accentColor}` }}
          >
            {a.verbatim_quote}
          </div>
        )}
        <EvidenceLeg
          caseSlug={caseSlug}
          allegationId={a.allegation_id}
          evidence={a.supporting_evidence}
          limit={visibleItems}
          wording={wording}
          accent="var(--state-success-strong)"
          emptyLabel={wording.no_supporting_line}
        />
        <EvidenceLeg
          caseSlug={caseSlug}
          allegationId={a.allegation_id}
          evidence={a.disputing_evidence}
          limit={visibleItems}
          wording={wording}
          accent="var(--state-danger-strong)"
          emptyLabel={wording.no_disputing_line}
        />
      </div>
    ))}
  </div>
);

export default AllegationSection;

// ─── Styles (tokens only) ────────────────────────────────────────────────────

const SECTION_DIVIDER_STYLE_BASE: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "8px",
  margin: "12px 0 8px",
  fontFamily: "var(--font-sans)",
  fontSize: "11px",
  fontWeight: 700,
  letterSpacing: "0.05em",
  textTransform: "uppercase",
};

const SECTION_DIVIDER_RULE_STYLE: React.CSSProperties = {
  flex: 1,
  height: "1px",
  backgroundColor: "var(--border-default)",
};

const ALLEGATION_CARD_STYLE: React.CSSProperties = {
  padding: "10px 12px",
  marginBottom: "8px",
  border: "1px solid var(--border-default)",
  borderRadius: "8px",
  backgroundColor: "var(--bg-surface)",
};

const PARAGRAPH_LABEL_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: "15px",
  fontWeight: 700,
  color: "var(--text-primary)",
};

// Layout-only; typography comes from the `.proof-text` utility class in
// tokens.css (the canonical proof/body treatment).
const QUOTE_TEXT_STYLE_BASE: React.CSSProperties = {
  marginTop: "6px",
  paddingLeft: "8px",
  lineHeight: 1.45,
};

// Evidence nests under the allegation, set off by a top rule so it reads as
// "what backs this allegation" rather than part of the allegation text.
const LEG_STYLE: React.CSSProperties = {
  marginTop: "8px",
  paddingTop: "8px",
  borderTop: "1px dashed var(--border-default)",
  display: "flex",
  flexDirection: "column",
  gap: "8px",
  alignItems: "flex-start",
};

// The explicit per-allegation gap — muted but obviously present, never blank.
const NO_EVIDENCE_STYLE: React.CSSProperties = {
  marginTop: "8px",
  paddingTop: "8px",
  borderTop: "1px dashed var(--border-default)",
  fontFamily: "var(--font-sans)",
  fontSize: "12px",
  fontStyle: "italic",
  color: "var(--text-muted)",
};

// A text control, not a button-shaped one: these open more of the same list, and
// a filled button beside five quotes would read as the action on the paragraph.
const FOLD_BUTTON_STYLE: React.CSSProperties = {
  padding: "2px 0",
  border: "none",
  background: "none",
  color: "var(--accent-primary)",
  fontFamily: "var(--font-sans)",
  fontSize: "12px",
  fontWeight: 600,
  cursor: "pointer",
};

// A failed write is a banner, not a console line: the row already moved, and
// this sentence is the only thing that says the screen and the database
// disagreed (Standing Rule 1).
const ERROR_LINE_STYLE: React.CSSProperties = {
  padding: "6px 10px",
  border: "1px solid var(--state-danger-border)",
  backgroundColor: "var(--state-danger-bg-soft)",
  borderRadius: "6px",
  color: "var(--state-danger-strong)",
  fontFamily: "var(--font-sans)",
  fontSize: "12px",
};
