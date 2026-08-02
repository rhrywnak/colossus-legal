// =============================================================================
// ScenarioDetailPage.tsx — /cases/:slug/trial-prep/:scenarioId
// =============================================================================
//
// The baseline page every Phase-2 feature lands on (task 1.7B). Restructured
// top-down so each future addition answers "did this hurt usability?" against a
// stable page, rather than a big-bang redo after ten features are wired in.
//
// TOP TO BOTTOM
//
//   1. A LEAN one-line header (study §1.7): code · name · direction chip ·
//      status chip · edit · rehearsal link. An issue's header is lean; all the
//      richness lives on the facts below it.
//   2. The scan control — one line, inside the Theme Scan panel.
//   3. The working view — what has been included, Facts-table style.
//   4. The ruling queue — §7 keyboard triage, behaviour untouched.
//
// WHAT DIED HERE
//
//   * The permanent full-width definition form with its scroll-box allegation
//     picker. Editing an identity is a modal now (`ScenarioIdentityModal`), and
//     nothing on this page opens a second page.
//   * The inline title-rename state machine. The name is one field of the
//     identity, and it is edited where the rest of the identity is — two editors
//     for one field is how one of them ends up forgotten.
//   * The "The attack" box. That text is THEIR framing, one of the three
//     identity texts, and it reads in the modal beside the other two rather than
//     as a banner that says nothing about what to do next.
//   * The split-pane PDF viewer (in `CardQueue`, defect D2).
//
// Display-only decisions stay server-side: chips, labels and the status
// vocabulary all arrive composed or come from tested pure helpers.
// =============================================================================

import React, { useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";

import Breadcrumb from "../components/Breadcrumb";
import ScenarioCurationPanel from "../components/ScenarioCurationPanel";
import ScenarioIdentityModal from "../components/ScenarioIdentityModal";
import ThemeScanPanel from "../components/ThemeScanPanel";
import ScenarioDeleteConfirm from "../components/ScenarioDeleteConfirm";
import { EmptyState, ResponseCard } from "../components/TrialPrepViews";
import { headerDescriptor } from "../components/scenarioHeader";
import { DEFAULT_CASE_SLUG } from "../services/caseHeader";
import { deleteScenario } from "../services/scenarioCrud";
import { getScenarioDetailLive } from "../services/trialPrep";
import type { ScenarioDetail } from "./trialPrepData";
import ReadyToggle from "../components/ReadyToggle";

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
  margin: "1.5rem 0 0.5rem",
};

// ── The lean header line (study §1.7) ───────────────────────────────────────
//
// One row, wrapping on a narrow window. Everything in it is either the
// scenario's identity or a way out of the page; nothing here is a form.
const headerRow: React.CSSProperties = {
  display: "flex",
  alignItems: "baseline",
  flexWrap: "wrap",
  gap: "0.6rem",
  marginBottom: "0.35rem",
};

const codeStyle: React.CSSProperties = {
  fontSize: "0.8rem",
  fontWeight: 600,
  color: "var(--text-muted)",
  letterSpacing: "0.04em",
};

// Regular weight, not bold: §2 reserves bold for true emphasis — a pinpoint
// page, a section head — and a page title is neither.
const nameStyle: React.CSSProperties = {
  margin: 0,
  fontSize: "1.15rem",
  fontWeight: 500,
  color: "var(--text-primary)",
};

const chipStyle: React.CSSProperties = {
  border: "1px solid var(--border-default)",
  borderRadius: "999px",
  padding: "0.1rem 0.6rem",
  fontSize: "0.74rem",
  whiteSpace: "nowrap",
};

const quietButton: React.CSSProperties = {
  border: "1px solid var(--border-default)",
  borderRadius: "6px",
  background: "var(--bg-surface)",
  color: "var(--text-secondary)",
  fontSize: "0.78rem",
  fontFamily: "inherit",
  padding: "0.15rem 0.55rem",
  cursor: "pointer",
};

const patternHeadline: React.CSSProperties = {
  fontSize: "0.82rem",
  color: "var(--text-muted)",
  marginBottom: "1rem",
};

const messageStyle: React.CSSProperties = {
  padding: "2rem 0",
  color: "var(--text-muted)",
};

const errorStyle: React.CSSProperties = {
  padding: "0.75rem 1rem",
  border: "1px solid var(--state-danger-border)",
  borderRadius: "6px",
  color: "var(--state-danger-strong)",
  background: "var(--state-danger-bg-soft)",
  fontSize: "0.85rem",
  marginBottom: "1rem",
};

const ScenarioDetailPage: React.FC = () => {
  const { slug: slugParam, scenarioId } = useParams<{
    slug: string;
    scenarioId: string;
  }>();
  const slug = slugParam ?? DEFAULT_CASE_SLUG;
  const navigate = useNavigate();
  const backCrumb = { label: "Trial Prep", to: `/cases/${slug}/trial-prep` };

  // Gating fetch (mirrors TrialPrepDashboardPage). `null` after load = a real
  // 404, which renders the "Scenario not found" empty state — distinct from a
  // fetch error (banner) and from still-loading.
  const [scenario, setScenario] = useState<ScenarioDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  // Bumped after the identity modal or the ready toggle saves, to re-fetch the
  // scenario. A re-fetch, not a hand-merged response, is the source of truth:
  // the backend trims and normalises what it stores, so the page must be told
  // what was actually persisted rather than assume the draft it sent.
  const [refreshKey, setRefreshKey] = useState(0);

  // Bumped when the Theme Scan panel merges picks into this scenario's candidate
  // facts. Kept SEPARATE from `refreshKey` (which re-loads the whole scenario
  // detail): a merge changes only the candidate facts, so re-fetching the
  // identity, responses and everything else would be needless work and would
  // flash the whole page. One signal, one consumer.
  const [factsRefresh, setFactsRefresh] = useState(0);

  // Delete flow: `showDelete` gates the confirm modal; `deleting` disables it
  // while the DELETE is in flight; `deleteError` keeps the modal open and shows
  // the failure (the modal closing is never treated as proof the delete worked).
  const [showDelete, setShowDelete] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);

  // The identity modal — the ONE place a scenario's identity is edited (1.7B).
  const [editingIdentity, setEditingIdentity] = useState(false);

  const handleDelete = () => {
    if (!scenarioId) return;
    setDeleting(true);
    setDeleteError(null);
    deleteScenario(slug, scenarioId)
      .then(() => {
        // The row is gone — leave the (now-dead) detail page for the dashboard.
        navigate(`/cases/${slug}/trial-prep`);
      })
      .catch((err: unknown) => {
        // Standing Rule 1: a failed DELETE stays visible IN the modal; we do NOT
        // navigate away or close, which would imply a success that did not happen.
        setDeleteError(
          err instanceof Error
            ? err.message
            : "Failed to delete the scenario. Try again.",
        );
        setDeleting(false);
      });
  };

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
  }, [slug, scenarioId, refreshKey]);

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

  const header = headerDescriptor({
    code: scenario.code,
    name: scenario.attack,
    direction: scenario.direction,
    status: scenario.status,
  });

  return (
    <div style={containerStyle}>
      <Breadcrumb
        items={[{ label: "Dashboard", to: "/" }, backCrumb, { label: scenario.attack }]}
      />

      {/* ── The lean header: one line, everything it states is identity ──────
          The code sits outside the name because it is the one part of this
          header that can never be renamed. */}
      <div style={headerRow}>
        <span style={codeStyle}>{header.code}</span>
        <h1 className="count-header" style={nameStyle}>
          {header.name}
        </h1>

        <span
          style={{ ...chipStyle, color: header.direction.color }}
          title={header.direction.title ?? undefined}
        >
          {header.direction.label}
        </span>
        <span style={{ ...chipStyle, color: header.status.color }}>
          {header.status.label}
        </span>

        {/* The readiness verdict's slot (§9). `header.readiness` is null until
            task 2.4 computes one, and nothing renders — not "Unknown", not a
            grey placeholder. A verdict is a claim about whether this scenario
            can be taken into a courtroom. */}
        {header.readiness && (
          <span style={{ ...chipStyle, color: header.readiness.color }}>
            {header.readiness.label}
          </span>
        )}

        {scenarioId && (
          <button
            type="button"
            style={quietButton}
            title="Edit this scenario's identity"
            aria-label="Edit scenario identity"
            onClick={() => setEditingIdentity(true)}
          >
            ✎ Edit
          </button>
        )}

        {/* The ready gate (task 1.5, v2 §5): the only path that changes status,
            because a readiness declaration is a human act with a name recorded
            against it. The generic update route refuses `status` outright. */}
        {scenarioId && (
          <ReadyToggle
            slug={slug}
            scenarioId={scenarioId}
            ready={scenario.status === "ready"}
            onChanged={() => setRefreshKey((k) => k + 1)}
          />
        )}

        {scenarioId && (
          <button
            type="button"
            style={quietButton}
            onClick={() => {
              setDeleteError(null);
              setShowDelete(true);
            }}
          >
            Delete
          </button>
        )}

        <Link
          to={`/cases/${encodeURIComponent(slug)}/rehearsal`}
          style={{ marginLeft: "auto", fontSize: "0.82rem" }}
        >
          Rehearsal mode →
        </Link>
      </div>

      {scenario.pattern_summary && (
        <div style={patternHeadline}>Pattern: {scenario.pattern_summary}</div>
      )}

      {/* Theme Scan driver: run the background LLM judge over every candidate
          quote, with live progress + results. A merge here writes candidate facts
          the curation panel below is displaying, so the two are coupled through
          `factsRefresh`: this page owns the signal because the panels are siblings
          with no shared store. */}
      {scenarioId && (
        <ThemeScanPanel
          slug={slug}
          scenarioId={scenarioId}
          scenarioTitle={scenario.attack}
          onFactsChanged={() => setFactsRefresh((k) => k + 1)}
        />
      )}

      {/* Phase A: the curated-facts binder replaces the old (broken)
          allegation-anchored timeline. `scenarioId` is defined here (the
          detail loaded via it), but the guard keeps the type honest. */}
      {scenarioId && (
        <ScenarioCurationPanel
          slug={slug}
          scenarioId={scenarioId}
          externalRefresh={factsRefresh}
        />
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

      {/* The identity modal. Mounted only while open so it re-reads on every
          open — a dialog holding a draft from ten minutes ago would let a human
          overwrite an edit made since, and this is the coldest path here. */}
      {editingIdentity && scenarioId && (
        <ScenarioIdentityModal
          slug={slug}
          scenarioId={scenarioId}
          definition={scenario.definition}
          anchorAllegationIds={scenario.anchor_allegation_ids}
          onSaved={() => setRefreshKey((k) => k + 1)}
          onClose={() => setEditingIdentity(false)}
        />
      )}

      {showDelete && (
        <ScenarioDeleteConfirm
          title="Delete this scenario?"
          message={
            `“${scenario.attack}” and its curated facts and responses will be ` +
            `permanently deleted. This cannot be undone. (The underlying evidence ` +
            `in the case graph is not affected.)`
          }
          confirmLabel="Delete scenario"
          busy={deleting}
          error={deleteError}
          onConfirm={handleDelete}
          onCancel={() => {
            setShowDelete(false);
            setDeleteError(null);
          }}
        />
      )}
    </div>
  );
};

export default ScenarioDetailPage;
