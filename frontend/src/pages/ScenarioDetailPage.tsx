// =============================================================================
// ScenarioDetailPage.tsx — /cases/:slug/trial-prep/:scenarioId
// =============================================================================
//
// The FINAL page structure (task 1.7C, SCENARIO_PAGE_DESIGN_v2 §2). Every §2
// component now has exactly one home here, and the future-phase ones have their
// placement reserved and render NOTHING until their task lands — absent, not fake.
//
// SEVEN SECTIONS, TOP TO BOTTOM
//
//   1. Two-tier header + eyebrow      §2.1 (D7) — `ScenarioHeaderTiers`
//   2. Identity block, read-only      §2.2 (D8) — `ScenarioIdentityBlock`
//   3. Scan & candidates              §2.3 (D10) — `ScanSection`
//   4. Scenario facts                 §2.4 — `ScenarioFactsSection`
//   5. Marie's talking points         §2.5 (C5) — `TalkingPointsSection`
//   6. Watch-list                     §2.6 (C6) — `WatchListSection`
//   7. Orphan strip, collapsed        §2.8 (D9) — `ScenarioOrphanStrip`
//
// ## This file is composition and fetching, nothing else
//
// Each section owns its own rendering and its own writes; this page owns the three
// reads they share and the two modals. That is what keeps it inside the 300-line
// limit with seven sections on it — the 1.7B version put the header's markup, its
// styles, and the delete flow inline, and there was no room left.
//
// ## WHAT DIED HERE IN 1.7C
//
//   * `pattern_summary` and `notes`. Both were hardcoded `None` in
//     `scenario_dashboard::build_detail` and neither has a column or a jsonb key —
//     measured on DEV, so they could never render. §2's seven sections are the
//     complete page and neither has a home in it (ruling R5).
//   * The bare `Delete` button in the header row. Destructive actions live in the
//     kebab and nowhere else (D7).
//   * "Marie's responses" / `ResponseCard`. §2.5's talking-points section is what
//     that data is for, and it now has a real one.
//
// ## The augmentation read became unconditional
//
// It used to fire only after a human clicked "Add", because the panel it fed was
// hidden until then. Three sections need it now — the identity block reads the C1
// texts from it, and §2.5/§2.6 are always present — so it loads with the page.

import React, { useCallback, useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";

import Breadcrumb from "../components/Breadcrumb";
import ScanSection from "../components/ScanSection";
import ScenarioDeleteConfirm from "../components/ScenarioDeleteConfirm";
import ScenarioFactsSection from "../components/ScenarioFactsSection";
import ScenarioHeaderTiers from "../components/ScenarioHeaderTiers";
import ScenarioIdentityBlock from "../components/ScenarioIdentityBlock";
import ScenarioIdentityModal from "../components/ScenarioIdentityModal";
import ScenarioOrphanStrip from "../components/ScenarioOrphanStrip";
import TalkingPointsSection from "../components/TalkingPointsSection";
import WatchListSection from "../components/WatchListSection";
import { EmptyState } from "../components/TrialPrepViews";
import { getAllegations, type AllegationDto } from "../services/allegations";
import { DEFAULT_CASE_SLUG } from "../services/caseHeader";
import { fetchScenarioCards, type ScenarioCard } from "../services/scenarioCards";
import {
  fetchAugmentationPanel,
  type AugmentationPanelDto,
} from "../services/scenarioAugmentation";
import { deleteScenario } from "../services/scenarioCrud";
import { getScenarioDetailLive } from "../services/trialPrep";
import type { ScenarioDetail } from "./trialPrepData";

/**
 * The page container, and the v3 SURFACE SCOPE.
 *
 * `data-surface="v3"` is what makes the mockup's palette apply here and nowhere
 * unreviewed (ruling R1). Every token the v3 `:root` redefines is re-declared under
 * that selector in tokens.css, and custom properties inherit — so the components
 * below keep saying `var(--text-primary)` and get the mockup's #1a202c, while the
 * same components on other screens get the app's #101828. The cascade does the
 * scoping; app-wide adoption is task 3.14.
 *
 * Padding transcribed from the mockup's `body`: 28px top, 40px sides, 80px bottom.
 * Scoped rather than applied to the app shell (ruling R5) — the shell's 32px inset
 * governs every other screen and moving it globally was not this task's to do.
 */
const containerStyle: React.CSSProperties = {
  padding: "28px 40px 80px",
  // The negative side margin ESCAPES the app shell's own 2rem inset before applying
  // the mockup's 40px. Without it the two insets add up: the shell's 1080px box less
  // 64px less 80px left 936px of content where the mockup has exactly 1000px, and
  // the 64px shortfall wrapped the status control onto its own line — which the
  // screenshot comparison caught and a checklist would not have.
  marginLeft: "-2rem",
  marginRight: "-2rem",
};
const messageStyle: React.CSSProperties = { padding: "2rem 0", color: "var(--text-muted)" };
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
  const { slug: slugParam, scenarioId } = useParams<{ slug: string; scenarioId: string }>();
  const slug = slugParam ?? DEFAULT_CASE_SLUG;
  const navigate = useNavigate();
  const backCrumb = { label: "Trial Prep", to: `/cases/${slug}/trial-prep` };

  // Gating fetch. `null` after load = a real 404, which renders the "not found"
  // empty state — distinct from a fetch error (banner) and from still-loading.
  const [scenario, setScenario] = useState<ScenarioDetail | null>(null);
  const [augmentation, setAugmentation] = useState<AugmentationPanelDto | null>(null);
  const [allegations, setAllegations] = useState<AllegationDto[]>([]);
  const [cards, setCards] = useState<ScenarioCard[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Bumped after any write; a re-fetch (never a hand-merged response) is the source
  // of truth, because the backend trims and normalises what it stores.
  const [refreshKey, setRefreshKey] = useState(0);
  const refresh = useCallback(() => setRefreshKey((k) => k + 1), []);

  const [showDelete, setShowDelete] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const [editingIdentity, setEditingIdentity] = useState(false);

  const handleDelete = () => {
    if (!scenarioId) return;
    setDeleting(true);
    setDeleteError(null);
    deleteScenario(slug, scenarioId)
      .then(() => navigate(`/cases/${slug}/trial-prep`))
      .catch((err: unknown) => {
        // Standing Rule 1: a failed DELETE stays visible IN the modal. We do NOT
        // navigate away or close, which would imply a success that did not happen.
        setDeleteError(
          err instanceof Error ? err.message : "Failed to delete the scenario. Try again.",
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

    // Four reads, one gate. `Promise.all` rather than a waterfall: they are
    // independent, and a page that paints its header a beat before its identity
    // block reads as a page that is still broken.
    Promise.all([
      getScenarioDetailLive(slug, scenarioId),
      fetchAugmentationPanel(slug, scenarioId),
      getAllegations(),
      fetchScenarioCards(slug, scenarioId),
    ])
      .then(([detail, panel, allegationList, cardPayload]) => {
        if (cancelled) return;
        setScenario(detail);
        setAugmentation(panel);
        setAllegations(allegationList.allegations);
        setCards([...cardPayload.pool, ...cardPayload.set_aside]);
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

  const gatingCrumb = (
    <Breadcrumb
      items={[{ label: "Dashboard", to: "/" }, backCrumb, { label: "Scenario" }]}
    />
  );

  if (loading) {
    return (
      <div style={containerStyle} data-surface="v3">
        {gatingCrumb}
        <div style={messageStyle}>Loading scenario…</div>
      </div>
    );
  }
  if (error) {
    return (
      <div style={containerStyle} data-surface="v3">
        {gatingCrumb}
        <div style={errorStyle}>{error}</div>
      </div>
    );
  }
  if (!scenario || !scenarioId) {
    return (
      <div style={containerStyle} data-surface="v3">
        {gatingCrumb}
        <EmptyState message="Scenario not found." />
      </div>
    );
  }

  return (
    <div style={containerStyle} data-surface="v3">
      <Breadcrumb
        items={[{ label: "Dashboard", to: "/" }, backCrumb, { label: scenario.attack }]}
      />

      {/* 1 — §2.1 */}
      <ScenarioHeaderTiers
        slug={slug}
        scenarioId={scenarioId}
        code={scenario.code}
        name={scenario.attack}
        direction={scenario.direction}
        status={scenario.status}
        onEdit={() => setEditingIdentity(true)}
        onDelete={() => {
          setDeleteError(null);
          setShowDelete(true);
        }}
        onReadyChanged={refresh}
      />

      {/* 2 — §2.2. The three texts come from the augmentation identity, which is
          where the backend already serves them composed; `attack_text` also lives
          inside the definition jsonb and the two agree. */}
      <ScenarioIdentityBlock
        attackText={augmentation?.identity.attack_text ?? null}
        themeStatement={augmentation?.identity.theme_statement ?? null}
        motivation={augmentation?.identity.motivation ?? null}
        anchorAllegationIds={scenario.anchor_allegation_ids}
        allegations={allegations}
        onEdit={() => setEditingIdentity(true)}
      />

      {/* 3 — §2.3. A merge inside the scan writes the candidate facts every other
          section below reads, so a merge refreshes the page's whole payload. */}
      <ScanSection
        slug={slug}
        scenarioId={scenarioId}
        scenarioTitle={scenario.attack}
        externalRefresh={refreshKey}
        onFactsChanged={refresh}
      />

      {/* 4 — §2.4 */}
      <ScenarioFactsSection
        slug={slug}
        scenarioId={scenarioId}
        cards={cards}
        humanFacts={augmentation?.human_facts ?? []}
        onChanged={refresh}
      />

      {/* 5 — §2.5 */}
      <TalkingPointsSection
        slug={slug}
        scenarioId={scenarioId}
        points={augmentation?.talking_points ?? []}
        cap={augmentation?.talking_points_cap ?? 0}
        onChanged={refresh}
      />

      {/* 6 — §2.6. The backend splits watch-list notes from human facts; a client
          that re-filtered them would eventually show a note as a fact. */}
      <WatchListSection
        slug={slug}
        scenarioId={scenarioId}
        notes={augmentation?.watch_list ?? []}
        onChanged={refresh}
      />

      {/* 7 — §2.8 */}
      <ScenarioOrphanStrip
        slug={slug}
        scenarioId={scenarioId}
        externalRefresh={refreshKey}
      />

      {/* Mounted only while open so it re-reads on every open — a dialog holding a
          draft from ten minutes ago would let a human overwrite an edit made since,
          and this is the coldest path here. */}
      {editingIdentity && (
        <ScenarioIdentityModal
          slug={slug}
          scenarioId={scenarioId}
          definition={scenario.definition}
          anchorAllegationIds={scenario.anchor_allegation_ids}
          onSaved={refresh}
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
