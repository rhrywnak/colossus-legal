// =============================================================================
// TrialPrepDashboardPage.tsx — /cases/:slug/trial-prep ("War Room" launch point)
// -----------------------------------------------------------------------------
// Stage 2: renders the dashboard payload fetched LIVE from the backend (metrics
// band · alerts strip · scenario card grid). The data source moved from the
// Stage-1 placeholder to `getTrialPrepDashboard(slug)` (services/trialPrep) — the
// payload shape is identical (`TrialPrepDashboard`), so the rendering below is
// unchanged; only the source and the loading/error gating are new. Thin renderer
// over the presentational pieces in TrialPrepViews; no numbers computed here
// (Charter §8 — the metrics object IS what is shown).
// =============================================================================

import React, { useEffect, useState } from "react";
import { useParams } from "react-router-dom";

import Breadcrumb from "../components/Breadcrumb";
import {
  AlertsStrip,
  EmptyState,
  GenerateScenarioCard,
  MetricsBand,
  ScenarioCard,
} from "../components/TrialPrepViews";
import { DEFAULT_CASE_SLUG } from "../services/caseHeader";
import { getTrialPrepDashboard } from "../services/trialPrep";
import type { TrialPrepDashboard } from "./trialPrepData";

const containerStyle: React.CSSProperties = {
  paddingTop: "32px",
  paddingBottom: "4rem",
};
const subtitleStyle: React.CSSProperties = {
  marginTop: "6px",
  fontFamily: "var(--font-sans)",
  fontSize: "14px",
  color: "var(--text-secondary)",
};
const gridStyle: React.CSSProperties = {
  display: "grid",
  gridTemplateColumns: "repeat(auto-fit, minmax(210px, 1fr))",
  gap: "1rem",
};
// Loading + error styling reuse the SAME design tokens the Proof Matrix page's
// gating states use (`--text-muted` for the neutral message, the
// `--state-danger-*` family for the error banner) — no bespoke colors (Rule 2).
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

/**
 * Gating read: the dashboard payload. A failure here blanks the page with a
 * visible message (surfaced as `error`), mirroring the Proof Matrix page's
 * gating fetch. The `cancelled` flag stops a navigate-away mid-flight from
 * setting state on an unmounted component.
 */
function useTrialPrepDashboard(slug: string): {
  dashboard: TrialPrepDashboard | null;
  loading: boolean;
  error: string | null;
} {
  const [dashboard, setDashboard] = useState<TrialPrepDashboard | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    getTrialPrepDashboard(slug)
      .then((data) => {
        if (cancelled) return;
        setDashboard(data);
        setLoading(false);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        setError(
          err instanceof Error
            ? err.message
            : "Failed to load the Trial Prep dashboard. Try reloading the page.",
        );
        setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [slug]);

  return { dashboard, loading, error };
}

const TrialPrepDashboardPage: React.FC = () => {
  const { slug: slugParam } = useParams<{ slug: string }>();
  const slug = slugParam ?? DEFAULT_CASE_SLUG;

  const { dashboard, loading, error } = useTrialPrepDashboard(slug);

  if (loading)
    return <div style={messageStyle}>Loading Trial Prep dashboard…</div>;
  if (error) return <div style={errorStyle}>{error}</div>;
  // After loading with no error the dashboard is always set; this guard is the
  // type-narrowing for `dashboard: T | null` (never expected to render).
  if (!dashboard)
    return <div style={errorStyle}>No dashboard data available.</div>;

  return (
    <div style={containerStyle}>
      <Breadcrumb items={[{ label: "Dashboard", to: "/" }, { label: "Trial Prep" }]} />
      <div style={{ marginBottom: "1.25rem" }}>
        <h1 className="count-header" style={{ margin: 0 }}>
          Trial Prep — War Room
        </h1>
        <div style={subtitleStyle}>
          System-generated cross-examination scenarios — the attacks, their
          recorded instances, and Marie's rehearsable responses.
        </div>
      </div>

      <MetricsBand metrics={dashboard.metrics} />

      {dashboard.alerts.length > 0 && <AlertsStrip alerts={dashboard.alerts} />}

      {dashboard.scenarios.length === 0 ? (
        <EmptyState message="No scenarios generated yet." />
      ) : (
        <div style={gridStyle}>
          {dashboard.scenarios.map((s) => (
            <ScenarioCard key={s.id} scenario={s} slug={slug} />
          ))}
          {/* On-demand entry point — visual affordance only in Stage 1. */}
          <GenerateScenarioCard />
        </div>
      )}
    </div>
  );
};

export default TrialPrepDashboardPage;
