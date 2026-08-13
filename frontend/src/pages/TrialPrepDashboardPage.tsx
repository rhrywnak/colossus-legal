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
  MetricsBand,
} from "../components/TrialPrepViews";
import ScenarioCard from "../components/ScenarioCard";
import ScenarioCreateForm from "../components/ScenarioCreateForm";
import ScenarioDeleteConfirm from "../components/ScenarioDeleteConfirm";
import { scenarioDeleteCopy } from "../components/scenarioDeleteCopy";
import { DEFAULT_CASE_SLUG } from "../services/caseHeader";
import { deleteScenario } from "../services/scenarioCrud";
import { getTrialPrepDashboard } from "../services/trialPrep";
import type { ScenarioSummary, TrialPrepDashboard } from "./trialPrepData";

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
const headerRowStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "flex-start",
  justifyContent: "space-between",
  gap: "1rem",
  marginBottom: "1.25rem",
};
const newScenarioButtonStyle: React.CSSProperties = {
  flexShrink: 0,
  padding: "8px 16px",
  fontFamily: "var(--font-sans)",
  fontSize: "14px",
  fontWeight: 600,
  color: "var(--accent-primary)",
  backgroundColor: "var(--accent-bg-soft)",
  border: "1px solid var(--accent-primary)",
  borderRadius: "6px",
  cursor: "pointer",
};

/**
 * Gating read: the dashboard payload. A failure here blanks the page with a
 * visible message (surfaced as `error`), mirroring the Proof Matrix page's
 * gating fetch. The `cancelled` flag stops a navigate-away mid-flight from
 * setting state on an unmounted component.
 */
function useTrialPrepDashboard(
  slug: string,
  refreshKey: number,
): {
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
    // `refreshKey` re-runs the fetch after a create so the new card appears.
  }, [slug, refreshKey]);

  return { dashboard, loading, error };
}

const TrialPrepDashboardPage: React.FC = () => {
  const { slug: slugParam } = useParams<{ slug: string }>();
  const slug = slugParam ?? DEFAULT_CASE_SLUG;

  // Bumping `refreshKey` re-runs the gating fetch (e.g. after a create or a
  // delete) — which is what makes a deleted card disappear AND the metrics band
  // above it drop by one, from one source, rather than two hand-patched states.
  const [refreshKey, setRefreshKey] = useState(0);
  const [showForm, setShowForm] = useState(false);

  // Delete lives on the PAGE, not on the card: one dialog serves N cards, and
  // the page is what owns the refresh that follows a successful delete.
  // `pendingDelete` is the scenario the dialog is asking about — `null` means
  // no dialog. Holding the scenario rather than a boolean is what lets the
  // dialog name it.
  const [pendingDelete, setPendingDelete] = useState<ScenarioSummary | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);

  const { dashboard, loading, error } = useTrialPrepDashboard(slug, refreshKey);

  const confirmDelete = () => {
    if (!pendingDelete) return;
    // Captured before the async call so the failure handler cannot read a
    // `pendingDelete` the human has since changed.
    const target = pendingDelete;
    setDeleting(true);
    setDeleteError(null);
    deleteScenario(slug, target.id)
      .then(() => {
        // Closed only AFTER the DELETE resolved. The dialog closing is not proof
        // the scenario is gone; the re-read is (Standing Rule 1).
        setDeleting(false);
        setPendingDelete(null);
        setRefreshKey((k) => k + 1);
      })
      .catch((err: unknown) => {
        // The dialog STAYS OPEN with the failure on it — the same contract the
        // scenario page's delete follows. Closing here would tell the human a
        // scenario was deleted that is still there, and the grid behind would
        // agree with them until the next reload.
        setDeleting(false);
        setDeleteError(
          err instanceof Error
            ? err.message
            : // A non-`Error` throw carries no message of its own, so this
              // branch composes the whole diagnosis. It NAMES the scenario:
              // `deleteScenario` bakes the id into the errors it throws, but a
              // throw from anywhere else does not, and on a grid of cards
              // "failed to delete the scenario" identifies none of them. The
              // dialog above shows the name; the error line must not depend on
              // the reader connecting the two.
              `Failed to delete “${target.attack}” (${target.code}). ` +
              `It has not been deleted — try again.`,
        );
      });
  };

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
      <div style={headerRowStyle}>
        <div>
          <h1 className="count-header" style={{ margin: 0 }}>
            Trial Prep — War Room
          </h1>
          {/* From the store since .396. The sentence it replaces credited the
              machine for work a human does — see `domain::wording_war_room`. */}
          <div style={subtitleStyle}>{dashboard.war_room_wording.subtitle}</div>
        </div>
        <button
          type="button"
          onClick={() => setShowForm((v) => !v)}
          style={newScenarioButtonStyle}
        >
          {showForm ? "Close" : "New scenario"}
        </button>
      </div>

      {showForm && (
        <ScenarioCreateForm
          slug={slug}
          wording={dashboard.create_wording}
          onCreated={() => {
            setShowForm(false);
            setRefreshKey((k) => k + 1);
          }}
          onCancel={() => setShowForm(false)}
        />
      )}

      <MetricsBand metrics={dashboard.metrics} wording={dashboard.war_room_wording} />

      {dashboard.alerts.length > 0 && <AlertsStrip alerts={dashboard.alerts} />}

      {dashboard.scenarios.length === 0 ? (
        <EmptyState message="No scenarios generated yet." />
      ) : (
        <div style={gridStyle}>
          {dashboard.scenarios.map((s) => (
            <ScenarioCard
              key={s.id}
              scenario={s}
              slug={slug}
              onRequestDelete={setPendingDelete}
            />
          ))}
          {/* On-demand entry point — visual affordance only in Stage 1. */}
        </div>
      )}

      {/* Mounted only while a scenario is pending, so the dialog can never carry
          a stale name from a card the human dismissed earlier. */}
      {pendingDelete && (
        <ScenarioDeleteConfirm
          // The same builder the scenario page uses — one question, asked in one
          // set of words wherever a scenario is deleted from.
          {...scenarioDeleteCopy(pendingDelete.attack)}
          busy={deleting}
          error={deleteError}
          onConfirm={confirmDelete}
          onCancel={() => {
            setPendingDelete(null);
            setDeleteError(null);
          }}
        />
      )}
    </div>
  );
};

export default TrialPrepDashboardPage;
