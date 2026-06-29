// =============================================================================
// ScenarioDetailPage.tsx — /cases/:slug/trial-prep/:scenarioId
// -----------------------------------------------------------------------------
// Stage 1: full-page view of one scenario's exchange, from the PLACEHOLDER
// payload. Renders the attack, the chronological exchange timeline (grounded
// turns with a source-PDF link; anticipated turns visually distinct with NO
// citation — the hard rule), Marie's rehearsable responses, and the pattern
// summary. Thin renderer over TrialPrepViews + the tested helpers; display-only
// (no editing) in Stage 1.
// =============================================================================

import React, { useEffect, useState } from "react";
import { useParams } from "react-router-dom";

import Breadcrumb from "../components/Breadcrumb";
import {
  EmptyState,
  ResponseCard,
  TimelineTurn,
} from "../components/TrialPrepViews";
import { DEFAULT_CASE_SLUG } from "../services/caseHeader";
import { getScenarioDetailLive } from "../services/trialPrep";
import type { ScenarioDetail } from "./trialPrepData";
import { sortTimelineByDate, statusMeta } from "./trialPrepHelpers";

const containerStyle: React.CSSProperties = {
  paddingTop: "32px",
  paddingBottom: "4rem",
};
const sectionLabel: React.CSSProperties = {
  fontSize: "0.74rem",
  fontWeight: 600,
  letterSpacing: "0.05em",
  textTransform: "uppercase",
  color: "var(--text-muted)",
  margin: "1.5rem 0 0.75rem",
};
const attackBox: React.CSSProperties = {
  border: "1px solid var(--border-default)",
  backgroundColor: "var(--bg-surface)",
  borderRadius: "8px",
  padding: "16px 18px",
  fontSize: "1.05rem",
  color: "var(--text-primary)",
  fontWeight: 500,
};
const binderStyle: React.CSSProperties = {
  border: "1px dashed var(--border-default)",
  borderRadius: "6px",
  padding: "0.35rem 0.7rem",
  fontSize: "0.78rem",
  color: "var(--text-disabled)",
  cursor: "not-allowed",
};
const patternHeadline: React.CSSProperties = {
  marginTop: "0.75rem",
  padding: "0.6rem 0.9rem",
  borderLeft: "3px solid var(--state-danger-strong)",
  backgroundColor: "var(--state-danger-bg-soft)",
  color: "var(--state-danger-strong)",
  borderRadius: "6px",
  fontSize: "0.86rem",
  fontWeight: 600,
};
// Gating styles mirror TrialPrepDashboardPage (tokens only — Rule 2).
const messageStyle: React.CSSProperties = {
  padding: "2rem",
  textAlign: "center",
  color: "var(--text-muted)",
  fontSize: "14px",
};
const errorStyle: React.CSSProperties = {
  margin: "1rem 0",
  padding: "1rem",
  backgroundColor: "var(--state-danger-bg-soft)",
  border: "1px solid var(--state-danger-border)",
  borderRadius: "6px",
  color: "var(--state-danger-strong)",
};

const ScenarioDetailPage: React.FC = () => {
  const { slug: slugParam, scenarioId } = useParams<{
    slug: string;
    scenarioId: string;
  }>();
  const slug = slugParam ?? DEFAULT_CASE_SLUG;
  const backCrumb = { label: "Trial Prep", to: `/cases/${slug}/trial-prep` };

  // Gating fetch (mirrors TrialPrepDashboardPage). `null` after load = a real
  // 404, which renders the "Scenario not found" empty state — distinct from a
  // fetch error (banner) and from still-loading.
  const [scenario, setScenario] = useState<ScenarioDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!scenarioId) {
      setLoading(false);
      return;
    }
    let cancelled = false;
    setLoading(true);
    setError(null);
    getScenarioDetailLive(slug, scenarioId)
      .then((data) => {
        if (cancelled) return;
        setScenario(data);
        setLoading(false);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        setError(
          err instanceof Error
            ? err.message
            : "Failed to load the scenario. Try reloading the page.",
        );
        setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [slug, scenarioId]);

  // Breadcrumb shown on every gating state (loading / error / not-found).
  const gatingCrumb = (
    <Breadcrumb
      items={[{ label: "Dashboard", to: "/" }, backCrumb, { label: "Scenario" }]}
    />
  );

  if (loading) {
    return (
      <div style={containerStyle}>
        {gatingCrumb}
        <div style={messageStyle}>Loading scenario…</div>
      </div>
    );
  }
  if (error) {
    return (
      <div style={containerStyle}>
        {gatingCrumb}
        <div style={errorStyle}>{error}</div>
      </div>
    );
  }
  if (!scenario) {
    return (
      <div style={containerStyle}>
        {gatingCrumb}
        <EmptyState message="Scenario not found." />
      </div>
    );
  }

  const status = statusMeta(scenario.status);
  const timeline = sortTimelineByDate(scenario.timeline);

  return (
    <div style={containerStyle}>
      <Breadcrumb
        items={[{ label: "Dashboard", to: "/" }, backCrumb, { label: scenario.attack }]}
      />

      <div style={{ display: "flex", alignItems: "center", gap: "1rem", marginBottom: "0.5rem" }}>
        <h1 className="count-header" style={{ margin: 0 }}>
          {scenario.attack}
        </h1>
        {/* Deferred "Binder" affordance — inert/greyed in Stage 1. */}
        <span style={binderStyle} aria-disabled="true" title="Coming soon">
          Binder
        </span>
      </div>
      <div style={{ fontSize: "0.8rem", color: "var(--text-muted)", marginBottom: "1rem" }}>
        Status: <span style={{ color: status.color, fontWeight: 600 }}>{status.label}</span>
      </div>

      {scenario.pattern_summary && (
        <div style={patternHeadline}>Pattern: {scenario.pattern_summary}</div>
      )}

      <div style={sectionLabel}>The attack</div>
      <div style={attackBox}>{scenario.attack}</div>

      <div style={sectionLabel}>Exchange timeline</div>
      {timeline.length === 0 ? (
        <EmptyState message="No exchange turns recorded for this scenario." />
      ) : (
        timeline.map((turn, i) => <TimelineTurn key={i} turn={turn} />)
      )}

      <div style={sectionLabel}>Marie's responses</div>
      {scenario.responses.length === 0 ? (
        <EmptyState message="No response drafted yet." />
      ) : (
        scenario.responses.map((r) => <ResponseCard key={r.id} response={r} />)
      )}

      {scenario.notes && (
        <>
          <div style={sectionLabel}>Notes</div>
          <div style={{ fontSize: "0.86rem", color: "var(--text-secondary)", lineHeight: 1.5 }}>
            {scenario.notes}
          </div>
        </>
      )}
    </div>
  );
};

export default ScenarioDetailPage;
