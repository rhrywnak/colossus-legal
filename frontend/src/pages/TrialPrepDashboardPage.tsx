// =============================================================================
// TrialPrepDashboardPage.tsx — /cases/:slug/trial-prep ("War Room" launch point)
// -----------------------------------------------------------------------------
// Stage 1: renders the PLACEHOLDER dashboard payload (metrics band · alerts strip
// · scenario card grid). No fetch — `getTrialPrepDashboard()` returns the
// placeholder shaped exactly like the eventual backend payload, so Stage 2 swaps
// the data source, not the component. Thin renderer over the presentational
// pieces in TrialPrepViews + the tested helpers; no numbers computed here
// (Charter §8 — the metrics object IS what is shown).
// =============================================================================

import React from "react";
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
import { getTrialPrepDashboard } from "./trialPrepPlaceholder";

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

const TrialPrepDashboardPage: React.FC = () => {
  const { slug: slugParam } = useParams<{ slug: string }>();
  const slug = slugParam ?? DEFAULT_CASE_SLUG;

  // Placeholder payload (Stage 2: a backend fetch returning the same shape).
  const dashboard = getTrialPrepDashboard();

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
