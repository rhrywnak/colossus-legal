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
import { useNavigate, useParams } from "react-router-dom";

import Breadcrumb from "../components/Breadcrumb";
import ScenarioCurationPanel from "../components/ScenarioCurationPanel";
import ScenarioDefinitionForm from "../components/ScenarioDefinitionForm";
import ThemeScanPanel from "../components/ThemeScanPanel";
import ScenarioDeleteConfirm from "../components/ScenarioDeleteConfirm";
import { EmptyState, ResponseCard } from "../components/TrialPrepViews";
import { DEFAULT_CASE_SLUG } from "../services/caseHeader";
import { deleteScenario, updateScenario } from "../services/scenarioCrud";
import { getScenarioDetailLive } from "../services/trialPrep";
import type { ScenarioDetail } from "./trialPrepData";
import { statusMeta } from "./trialPrepHelpers";

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
// Delete affordance sits at the far end of the header row (marginLeft:auto),
// visually separated from the title so it is not a mis-click target.
const deleteBtnStyle: React.CSSProperties = {
  marginLeft: "auto",
  border: "1px solid var(--state-danger-border)",
  borderRadius: "6px",
  padding: "0.35rem 0.8rem",
  fontSize: "0.8rem",
  fontWeight: 600,
  backgroundColor: "var(--state-danger-bg-soft)",
  color: "var(--state-danger-strong)",
  cursor: "pointer",
};
// Inline title editor (D1.6). The input is sized to read like the title it
// replaces; Save/Cancel are compact neutral/accent buttons.
const titleInputStyle: React.CSSProperties = {
  flex: "1 1 auto",
  minWidth: 0,
  fontSize: "1.5rem",
  fontWeight: 700,
  padding: "0.2rem 0.5rem",
  border: "1px solid var(--border-default)",
  borderRadius: "6px",
  backgroundColor: "var(--bg-surface)",
  color: "var(--text-primary)",
};
const titleSaveBtn: React.CSSProperties = {
  border: "1px solid var(--accent-primary)",
  borderRadius: "6px",
  padding: "0.35rem 0.8rem",
  fontSize: "0.8rem",
  fontWeight: 600,
  backgroundColor: "var(--accent-bg-soft)",
  color: "var(--accent-primary)",
  cursor: "pointer",
};
const titleSaveBtnDisabled: React.CSSProperties = {
  ...titleSaveBtn,
  opacity: 0.5,
  cursor: "not-allowed",
};
const titleCancelBtn: React.CSSProperties = {
  border: "1px solid var(--border-default)",
  borderRadius: "6px",
  padding: "0.35rem 0.8rem",
  fontSize: "0.8rem",
  fontWeight: 600,
  backgroundColor: "var(--bg-surface)",
  color: "var(--text-secondary)",
  cursor: "pointer",
};
// The title in view mode reads as a clickable affordance (hover cursor).
const titleViewStyle: React.CSSProperties = { margin: 0, cursor: "pointer" };
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
  const navigate = useNavigate();
  const backCrumb = { label: "Trial Prep", to: `/cases/${slug}/trial-prep` };

  // Gating fetch (mirrors TrialPrepDashboardPage). `null` after load = a real
  // 404, which renders the "Scenario not found" empty state — distinct from a
  // fetch error (banner) and from still-loading.
  const [scenario, setScenario] = useState<ScenarioDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  // Bumped after the definition form saves, to re-fetch the scenario (the same
  // idiom ScenarioCurationPanel uses). Keyed into the load effect below so the
  // page re-loads and the form re-fills from the persisted definition — a
  // re-fetch, not a hand-merged response, is the source of truth.
  const [refreshKey, setRefreshKey] = useState(0);

  // Delete flow: `showDelete` gates the confirm modal; `deleting` disables it
  // while the DELETE is in flight; `deleteError` keeps the modal open and shows
  // the failure (the modal closing is never treated as proof the delete worked).
  const [showDelete, setShowDelete] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);

  // ── Inline title-rename state machine (D1.6) ───────────────────────────────
  //
  // A tiny two-state UI: VIEW (the <h1>, click to edit) ↔ EDIT (an input +
  // Save/Cancel). `titleDraft` is LOCAL, editable text; it is deliberately kept
  // separate from the fetched `scenario.attack` so typing never mutates the
  // displayed source-of-truth until a save actually persists.
  //
  // Domain note: the field being edited is the scenario's top-level NAME. On this
  // payload the name arrives (confusingly) as `scenario.attack` — the backend
  // `build_detail` maps the `name` column onto the DTO's `attack` field. It is NOT
  // `attack_text` (the accusation), which lives in `scenario.definition`. So we
  // SEED the draft from `scenario.attack` but SEND `{ name }`.
  const [editingTitle, setEditingTitle] = useState(false);
  const [titleDraft, setTitleDraft] = useState("");
  const [savingTitle, setSavingTitle] = useState(false);
  const [titleError, setTitleError] = useState<string | null>(null);

  // Validation parity with the backend `validate_name`: a name must be
  // non-empty / non-whitespace. Save is disabled otherwise (and while in flight).
  const canSaveTitle =
    titleDraft.trim().length > 0 && !savingTitle;

  const beginEditTitle = () => {
    if (!scenario) return;
    setTitleDraft(scenario.attack); // seed from the persisted name (see note above)
    setTitleError(null);
    setEditingTitle(true);
  };

  const cancelEditTitle = () => {
    setEditingTitle(false);
    setTitleError(null);
  };

  const handleSaveTitle = () => {
    if (!scenarioId) return;
    const trimmed = titleDraft.trim();
    if (!trimmed) return; // guarded even though Save is disabled when empty

    setSavingTitle(true);
    setTitleError(null);
    // Send the TOP-LEVEL name (never attack_text) through the same PUT the define
    // form uses. On success we do NOT optimistically show `trimmed`: we bump
    // `refreshKey` and let the re-fetch supply the title. Why: the persisted value
    // can differ from the draft (the backend trims `name`), so the RELOAD is the
    // source of truth, not the local input — the same discipline the define form
    // follows via `onSaved`.
    updateScenario(slug, scenarioId, { name: trimmed })
      .then(() => {
        setEditingTitle(false);
        setSavingTitle(false);
        setRefreshKey((k) => k + 1);
      })
      .catch((err: unknown) => {
        // Standing Rule 1: a failed rename stays visible and the editor stays OPEN
        // with the user's text intact — we never silently revert, and never let
        // the title show a name that was not actually saved.
        setTitleError(
          err instanceof Error
            ? err.message
            : "Failed to rename the scenario. Try again.",
        );
        setSavingTitle(false);
      });
  };

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

  const status = statusMeta(scenario.status);

  return (
    <div style={containerStyle}>
      <Breadcrumb
        items={[{ label: "Dashboard", to: "/" }, backCrumb, { label: scenario.attack }]}
      />

      <div style={{ display: "flex", alignItems: "center", gap: "1rem", marginBottom: "0.5rem" }}>
        {editingTitle ? (
          // EDIT mode: an input over the local draft + Save/Cancel. Enter saves
          // (when valid), Escape cancels — keyboard parity with the buttons.
          <>
            <input
              autoFocus
              aria-label="Scenario name"
              style={titleInputStyle}
              value={titleDraft}
              disabled={savingTitle}
              onChange={(e) => setTitleDraft(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && canSaveTitle) handleSaveTitle();
                if (e.key === "Escape") cancelEditTitle();
              }}
            />
            <button
              type="button"
              style={canSaveTitle ? titleSaveBtn : titleSaveBtnDisabled}
              onClick={handleSaveTitle}
              disabled={!canSaveTitle}
            >
              {savingTitle ? "Saving…" : "Save"}
            </button>
            <button
              type="button"
              style={titleCancelBtn}
              onClick={cancelEditTitle}
              disabled={savingTitle}
            >
              Cancel
            </button>
          </>
        ) : (
          // VIEW mode: the title reads as a clickable affordance. `scenario.attack`
          // holds the top-level NAME (see the state-machine note above), so clicking
          // it opens the NAME editor — not the accusation/define surface.
          <>
            <h1
              className="count-header"
              style={titleViewStyle}
              title="Click to rename"
              onClick={beginEditTitle}
            >
              {scenario.attack}
            </h1>
            {/* Deferred "Binder" affordance — inert/greyed in Stage 1. */}
            <span style={binderStyle} aria-disabled="true" title="Coming soon">
              Binder
            </span>
            {scenarioId && (
              <button
                type="button"
                style={deleteBtnStyle}
                onClick={() => {
                  setDeleteError(null);
                  setShowDelete(true);
                }}
              >
                Delete scenario
              </button>
            )}
          </>
        )}
      </div>
      {/* A failed rename stays visible here; the editor above stays open with the
          user's text intact (never a silent revert / false success). */}
      {editingTitle && titleError && <div style={errorStyle}>{titleError}</div>}
      <div style={{ fontSize: "0.8rem", color: "var(--text-muted)", marginBottom: "1rem" }}>
        Status: <span style={{ color: status.color, fontWeight: 600 }}>{status.label}</span>
      </div>

      {scenario.pattern_summary && (
        <div style={patternHeadline}>Pattern: {scenario.pattern_summary}</div>
      )}

      <div style={sectionLabel}>The attack</div>
      <div style={attackBox}>{scenario.attack}</div>

      {/* B2a: author this scenario's definition (theme + seeds). Sits between the
          attack and the curated-facts binder — authoring, then seeding. On save
          it bumps `refreshKey` so the page re-fetches and the form re-fills from
          the persisted definition. */}
      {scenarioId && (
        <ScenarioDefinitionForm
          slug={slug}
          scenarioId={scenarioId}
          definition={scenario.definition}
          anchorAllegationIds={scenario.anchor_allegation_ids}
          onSaved={() => setRefreshKey((k) => k + 1)}
        />
      )}

      {/* Theme Scan driver: run the background LLM judge over every candidate
          quote (per-model benchmark), with live progress + results. */}
      {scenarioId && (
        <ThemeScanPanel slug={slug} scenarioId={scenarioId} scenarioTitle={scenario.attack} />
      )}

      {/* Phase A: the curated-facts binder replaces the old (broken)
          allegation-anchored timeline. `scenarioId` is defined here (the
          detail loaded via it), but the guard keeps the type honest. */}
      {scenarioId && (
        <ScenarioCurationPanel
          slug={slug}
          scenarioId={scenarioId}
          definition={scenario.definition}
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
