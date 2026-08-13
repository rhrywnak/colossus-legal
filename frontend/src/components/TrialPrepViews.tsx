// =============================================================================
// TrialPrepViews.tsx — presentational pieces for the Trial Prep ("War Room") pages
// -----------------------------------------------------------------------------
// Pure presentational components (props in → JSX out): no fetch, no state, no
// business logic. They render the placeholder payload, labeled. Kept in their
// own file so the page components stay thin orchestrators over these + the
// tested helpers, and no module exceeds the size limit.
//
// Grounded vs anticipated is the hard visual rule: anticipated turns get a
// dashed, muted treatment with an explicit "anticipated — not in record" marker
// and NO source link. Nullable fields fall back to an em-dash (Charter §8).
// =============================================================================

import React from "react";

import { pdfHref } from "./ElementAllegationList";
import { pillStyle } from "./trialPrepCardStyles";
import type {
  ExchangeTurn,
  MarieResponse,
  WarRoomWording,
} from "../pages/trialPrepData";
import { isAnticipated, showsRepeatFlag } from "../pages/trialPrepHelpers";

const EMDASH = "—";

// ─── Styles (design tokens only) ─────────────────────────────────────────────

const cardRow: React.CSSProperties = {
  display: "flex",
  gap: "1rem",
  marginBottom: "1.5rem",
  flexWrap: "wrap",
};
const metricCard: React.CSSProperties = {
  flex: "1 1 140px",
  padding: "0.75rem 1rem",
  backgroundColor: "var(--bg-surface)",
  borderRadius: "8px",
  border: "1px solid var(--border-default)",
};
const metricValue: React.CSSProperties = {
  fontSize: "1.5rem",
  fontWeight: 700,
  color: "var(--text-primary)",
};
const metricLabel: React.CSSProperties = {
  fontSize: "0.76rem",
  color: "var(--text-muted)",
  marginTop: "0.1rem",
};
const alertRow: React.CSSProperties = {
  padding: "0.6rem 0.9rem",
  marginBottom: "0.5rem",
  borderLeft: "3px solid var(--state-warning-strong)",
  backgroundColor: "var(--bg-surface)",
  borderRadius: "6px",
  fontSize: "0.84rem",
  color: "var(--text-secondary)",
};
const turnCardBase: React.CSSProperties = {
  borderRadius: "8px",
  padding: "12px 14px",
  marginBottom: "10px",
};
const labelStyle: React.CSSProperties = {
  fontSize: "0.72rem",
  fontWeight: 600,
  letterSpacing: "0.04em",
  textTransform: "uppercase",
  color: "var(--text-muted)",
};
const locatorLink: React.CSSProperties = {
  fontSize: "0.76rem",
  color: "var(--accent-primary)",
  textDecoration: "none",
  fontFamily: "var(--font-mono, monospace)",
};
const emptyStyle: React.CSSProperties = {
  padding: "1.5rem",
  textAlign: "center",
  color: "var(--text-muted)",
  fontSize: "0.88rem",
  border: "1px dashed var(--border-default)",
  borderRadius: "8px",
};

// ─── Shared ──────────────────────────────────────────────────────────────────

/** Explicit empty-state panel — never a blank region (Charter §8). */
export const EmptyState: React.FC<{ message: string }> = ({ message }) => (
  <div style={emptyStyle}>{message}</div>
);

// ─── Metrics band ────────────────────────────────────────────────────────────

const MetricCard: React.FC<{
  value: number;
  label: string;
  emphasized?: boolean;
  hint?: string;
}> = ({ value, label, emphasized, hint }) => (
  <div
    style={{
      ...metricCard,
      ...(emphasized
        ? { backgroundColor: "var(--state-info-bg-soft)", borderColor: "var(--accent-primary)" }
        : {}),
    }}
  >
    <div style={metricValue}>{value}</div>
    <div style={metricLabel}>{label}</div>
    {hint ? (
      <div style={{ fontSize: "0.7rem", color: "var(--accent-primary)", marginTop: "0.15rem" }}>
        {hint}
      </div>
    ) : null}
  </div>
);

/**
 * The metrics band.
 *
 * Two cards were removed on 2026-07-27 — "Baseless-repeat patterns" (the Count IV
 * signal) and "No response yet". Both were derived from card fields that are
 * hardcoded stubs, so one was structurally always 0 and the other always equalled
 * the scenario count: constants rendered as measurements, indistinguishable on
 * screen from real results. They come back when pattern analysis and responses
 * have real sources.
 *
 * "Instances" went on 2026-08-07, for a different reason. It was a REAL
 * measurement — a live REBUTS count across each scenario's anchor allegations —
 * of something nobody could act on, and its name collided with task 2.11's
 * unrelated "accusation instances". Roman ruled it earns nothing. Unlike the
 * other two it is not coming back when a source appears; it had one.
 */
export const MetricsBand: React.FC<{
  metrics: {
    scenarios: number;
    ready: number;
    drafted_or_review: number;
  };
  /**
   * The three tile labels, from the store (R2 §3, built .396). The third one
   * used to read "Drafted / in review" — two words for one number, which invited
   * a reader to look for a second figure that was never there.
   */
  wording: WarRoomWording;
}> = ({ metrics, wording }) => (
  <div style={cardRow}>
    <MetricCard value={metrics.scenarios} label={wording.metric_scenarios_label} />
    <MetricCard value={metrics.ready} label={wording.metric_ready_label} />
    <MetricCard value={metrics.drafted_or_review} label={wording.metric_draft_label} />
  </div>
);

/** The alerts strip (living-binder notices). Caller omits it when empty. */
export const AlertsStrip: React.FC<{ alerts: { message: string }[] }> = ({ alerts }) => (
  <div style={{ marginBottom: "1.5rem" }}>
    {alerts.map((a, i) => (
      <div key={i} style={alertRow}>
        ⚠︎ {a.message}
      </div>
    ))}
  </div>
);

// REMOVED in task R1 Piece 10d: `GenerateScenarioCard`.
//
// A dashed "+ Generate a scenario" tile that had been "visual only in Stage 1"
// since it was written — it took an optional `onClick` and the dashboard mounted
// it with none, so every click did nothing at all. A control that never worked is
// worse than a missing feature: it teaches a human that this page's buttons are
// unreliable, on the page where they are about to be asked to trust a delete
// confirmation.
//
// Creation has ONE control now, the dashboard's "New scenario" button. Whether
// generation ever arrives is a design question (D1-D6); a dead tile was not
// holding its place, it was just sitting there.

// ─── Exchange timeline turn ──────────────────────────────────────────────────

export const TimelineTurn: React.FC<{ turn: ExchangeTurn }> = ({ turn }) => {
  const anticipated = isAnticipated(turn);
  return (
    <div
      style={{
        ...turnCardBase,
        backgroundColor: anticipated ? "transparent" : "var(--bg-surface)",
        border: anticipated
          ? "1px dashed var(--state-warning-strong)"
          : "1px solid var(--border-default)",
      }}
    >
      <div style={{ display: "flex", gap: "0.5rem", alignItems: "center", flexWrap: "wrap" }}>
        <span style={labelStyle}>{turn.kind.replace(/_/g, " ")}</span>
        {anticipated ? (
          <span style={{ ...pillStyle, backgroundColor: "var(--state-warning-strong)", color: "var(--bg-surface)" }}>
            anticipated — not in record
          </span>
        ) : null}
        {showsRepeatFlag(turn) ? (
          <span style={{ ...pillStyle, backgroundColor: "var(--state-danger-bg-soft)", color: "var(--state-danger-strong)" }}>
            repeated after rebuttal
          </span>
        ) : null}
        {turn.relationship_type ? (
          <span style={{ ...pillStyle, backgroundColor: "var(--bg-page)", color: "var(--text-secondary)" }}>
            {turn.relationship_type}
          </span>
        ) : null}
      </div>
      <div style={{ fontSize: "0.9rem", color: "var(--text-primary)", margin: "6px 0", lineHeight: 1.5 }}>
        {turn.text}
      </div>
      <div style={{ fontSize: "0.76rem", color: "var(--text-muted)" }}>
        {turn.speaker ?? EMDASH}
        {turn.date ? `  ·  ${turn.date}` : ""}
      </div>
      {anticipated ? null : (
        // Grounded turns carry a source-PDF click-through (reused pdfHref). No
        // link is rendered for anticipated turns — there is nothing in the record.
        <div style={{ marginTop: "4px" }}>
          {turn.source_document ? (
            <a
              href={pdfHref(turn.source_document, turn.page_number)}
              target="_blank"
              rel="noopener noreferrer"
              style={locatorLink}
            >
              {turn.source_document}
              {turn.paragraph ? `  ·  ${turn.paragraph}` : ""}
              {turn.page_number !== null ? `  ·  p.${turn.page_number}` : ""}
            </a>
          ) : (
            <span style={{ ...locatorLink, color: "var(--text-muted)" }}>{EMDASH}</span>
          )}
        </div>
      )}
    </div>
  );
};

// ─── Marie response card ─────────────────────────────────────────────────────

export const ResponseCard: React.FC<{ response: MarieResponse }> = ({ response }) => (
  <div
    style={{
      border: "1px solid var(--border-default)",
      backgroundColor: "var(--bg-surface)",
      borderRadius: "8px",
      padding: "12px 14px",
      marginBottom: "10px",
    }}
  >
    <div style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
      <span style={{ ...pillStyle, backgroundColor: "var(--accent-bg-soft)", color: "var(--accent-primary)" }}>
        {response.label}
      </span>
      <span style={{ fontSize: "0.72rem", color: "var(--text-muted)" }}>
        {response.authored_by === "marie" ? "Marie's wording" : "system draft"}
      </span>
    </div>
    <div style={{ fontSize: "0.9rem", color: "var(--text-primary)", marginTop: "6px", lineHeight: 1.5 }}>
      {response.text}
    </div>
  </div>
);
