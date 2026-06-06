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
import { Link } from "react-router-dom";

import { pdfHref } from "./ElementAllegationList";
import type {
  ExchangeTurn,
  MarieResponse,
  ScenarioSummary,
} from "../pages/trialPrepData";
import {
  isAnticipated,
  patternFlagText,
  scenarioMetaLine,
  showsRepeatFlag,
  statusMeta,
} from "../pages/trialPrepHelpers";

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
const scenarioCardStyle: React.CSSProperties = {
  border: "1px solid var(--border-default)",
  backgroundColor: "var(--bg-surface)",
  borderRadius: "8px",
  padding: "14px 16px",
  display: "flex",
  flexDirection: "column",
  gap: "8px",
};
const pillStyle: React.CSSProperties = {
  display: "inline-block",
  padding: "0.12rem 0.5rem",
  borderRadius: "9999px",
  fontSize: "0.72rem",
  fontWeight: 600,
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

/** The metrics band. `baseless_repeat_patterns` is emphasized (Count IV signal). */
export const MetricsBand: React.FC<{
  metrics: {
    scenarios: number;
    ready: number;
    drafted_or_review: number;
    instances: number;
    baseless_repeat_patterns: number;
    no_response_yet: number;
  };
}> = ({ metrics }) => (
  <div style={cardRow}>
    <MetricCard value={metrics.scenarios} label="Scenarios" />
    <MetricCard value={metrics.ready} label="Ready" />
    <MetricCard value={metrics.drafted_or_review} label="Drafted / in review" />
    <MetricCard value={metrics.instances} label="Instances" />
    <MetricCard
      value={metrics.baseless_repeat_patterns}
      label="Baseless-repeat patterns"
      emphasized
      hint="Count IV signal"
    />
    <MetricCard value={metrics.no_response_yet} label="No response yet" />
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

// ─── Scenario card (dashboard grid) ──────────────────────────────────────────

export const ScenarioCard: React.FC<{
  scenario: ScenarioSummary;
  slug: string;
}> = ({ scenario, slug }) => {
  const status = statusMeta(scenario.status);
  const flag = patternFlagText(scenario.baseless_repeat_count);
  return (
    // The WHOLE card is the navigation target (matches the app's clickable-card
    // pattern). `<Link>` renders an <a>, so we reset its default underline/blue
    // to keep the card's visual styling — children carry their own colors; only
    // the trailing "Open scenario →" hint is accent-colored.
    <Link
      to={`/cases/${slug}/trial-prep/${scenario.id}`}
      style={{ ...scenarioCardStyle, textDecoration: "none", color: "var(--text-primary)" }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
        <span
          style={{
            width: "9px",
            height: "9px",
            borderRadius: "50%",
            backgroundColor: status.color,
            flexShrink: 0,
          }}
        />
        <span style={{ fontSize: "0.72rem", color: "var(--text-muted)" }}>{status.label}</span>
      </div>
      <div style={{ fontSize: "0.95rem", fontWeight: 600, color: "var(--text-primary)" }}>
        {scenario.attack}
      </div>
      <span
        style={{
          ...pillStyle,
          alignSelf: "flex-start",
          backgroundColor: flag.muted ? "var(--bg-page)" : "var(--state-info-bg-soft)",
          color: flag.muted ? "var(--text-muted)" : "var(--accent-primary)",
        }}
      >
        {flag.text}
      </span>
      <div style={{ fontSize: "0.78rem", color: "var(--text-secondary)" }}>
        {scenarioMetaLine(scenario)}
      </div>
      {/* Visual affordance only — the whole card navigates, so this is plain
          text, not a separate link. */}
      <span style={{ fontSize: "0.82rem", color: "var(--accent-primary)", marginTop: "auto" }}>
        Open scenario →
      </span>
    </Link>
  );
};

/** The dashed "Generate a scenario" affordance — visual only in Stage 1. */
export const GenerateScenarioCard: React.FC<{ onClick?: () => void }> = ({ onClick }) => (
  <button
    type="button"
    onClick={onClick}
    style={{
      border: "1px dashed var(--border-default)",
      backgroundColor: "transparent",
      borderRadius: "8px",
      padding: "14px 16px",
      cursor: "pointer",
      color: "var(--text-muted)",
      fontFamily: "inherit",
      fontSize: "0.9rem",
      minHeight: "120px",
    }}
  >
    + Generate a scenario
  </button>
);

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
