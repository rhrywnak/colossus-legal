// =============================================================================
// ProofReviewPage.tsx — GET /api/cases/:slug/proof-review (PR2)
// -----------------------------------------------------------------------------
// Read-only page behind the four Proof-Review sub-views (Summary · Proof edges ·
// Excluded · Borderline). ONE fetch per (slug, document filter); the four
// sub-tabs switch client-side over that payload (no refetch). Changing the
// document filter DOES refetch (?document_id=). All shaping is delegated to the
// tested helpers in `proofReviewHelpers`; presentation to `ProofReviewViews`.
// This component is a thin orchestrator — no business logic, no client-side
// counts (Standing Rule: the backend owns all numbers).
// =============================================================================

import React, { useEffect, useState } from "react";
import { useParams, useSearchParams } from "react-router-dom";

import Breadcrumb from "../components/Breadcrumb";
import {
  ExcludedCard,
  EmptyState,
  ProofEdgeCard,
  SummarySection,
} from "../components/ProofReviewViews";
import { DEFAULT_CASE_SLUG } from "../services/caseHeader";
import {
  getProofReview,
  type ProofReviewResponse,
} from "../services/proofReview";
import {
  distinctSourceDocuments,
  distinctStatementTypes,
  filterEdges,
  sectionEmptyStates,
  subTabBadgeCounts,
  EDGE_FILTER_ALL,
} from "./proofReviewHelpers";

// ─── Tab model ───────────────────────────────────────────────────────────────

type TabId = "summary" | "proof_edges" | "excluded" | "borderline";
const TABS: { id: TabId; label: string }[] = [
  { id: "summary", label: "Summary" },
  { id: "proof_edges", label: "Proof edges" },
  { id: "excluded", label: "Excluded" },
  { id: "borderline", label: "Borderline" },
];
const DEFAULT_TAB: TabId = "summary";

// ─── Styles (design tokens only; mirrors ProofMatrixPage / AnalysisPage) ──────

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
  color: "var(--state-danger-strong)",
  borderRadius: "8px",
  fontSize: "14px",
};
const controlsRowStyle: React.CSSProperties = {
  display: "flex",
  flexWrap: "wrap",
  gap: "1rem",
  alignItems: "center",
  marginBottom: "1rem",
};
const selectStyle: React.CSSProperties = {
  padding: "0.35rem 0.5rem",
  fontSize: "0.82rem",
  fontFamily: "inherit",
  border: "1px solid var(--border-default)",
  borderRadius: "6px",
  backgroundColor: "var(--bg-surface)",
  color: "var(--text-secondary)",
  cursor: "pointer",
};
const tabBarStyle: React.CSSProperties = {
  display: "flex",
  borderBottom: "1px solid var(--border-default)",
  marginBottom: "1.5rem",
  gap: "0.5rem",
};
const tabButtonBaseStyle: React.CSSProperties = {
  padding: "0.6rem 0.9rem",
  border: "none",
  borderBottom: "2px solid transparent",
  backgroundColor: "transparent",
  cursor: "pointer",
  fontSize: "0.88rem",
  fontWeight: 500,
  color: "var(--text-muted)",
  display: "flex",
  alignItems: "center",
  gap: "0.4rem",
};
const tabBadgeStyle: React.CSSProperties = {
  padding: "0.1rem 0.45rem",
  borderRadius: "9999px",
  fontSize: "0.72rem",
  fontWeight: 600,
  backgroundColor: "var(--bg-page)",
  color: "var(--text-secondary)",
};

// ─── Sub-tab button (mirrors AnalysisPage TabButton) ─────────────────────────

const SubTab: React.FC<{
  label: string;
  active: boolean;
  badge?: number;
  onClick: () => void;
}> = ({ label, active, badge, onClick }) => (
  <button
    style={{
      ...tabButtonBaseStyle,
      color: active ? "var(--accent-primary)" : "var(--text-muted)",
      borderBottomColor: active ? "var(--accent-primary)" : "transparent",
      fontWeight: active ? 600 : 500,
    }}
    onClick={onClick}
  >
    {label}
    {badge !== undefined && <span style={tabBadgeStyle}>{badge}</span>}
  </button>
);

// ─── Page ────────────────────────────────────────────────────────────────────

const ProofReviewPage: React.FC = () => {
  const { slug: slugParam } = useParams<{ slug: string }>();
  const slug = slugParam ?? DEFAULT_CASE_SLUG;

  // Active sub-tab persisted in the URL (?tab=) so a reload / shared link keeps
  // the view — same pattern as DocumentWorkspaceTabs.
  const [searchParams, setSearchParams] = useSearchParams();
  const activeTab = (searchParams.get("tab") as TabId) || DEFAULT_TAB;
  const setActiveTab = (id: TabId) =>
    setSearchParams({ tab: id }, { replace: true });

  // Document filter ("all" = no filter → no ?document_id=, full payload). The
  // option list is captured from the unfiltered load so it never collapses to
  // the single selected document.
  const [docFilter, setDocFilter] = useState<string>(EDGE_FILTER_ALL);
  const [docOptions, setDocOptions] = useState<string[]>([]);

  // Proof-edges tab client-side statement_type filter (narrows visible rows;
  // does NOT change any count — the Summary/badges always reflect the full
  // payload).
  const [edgeStatementType, setEdgeStatementType] =
    useState<string>(EDGE_FILTER_ALL);

  const [payload, setPayload] = useState<ProofReviewResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // One fetch per (slug, docFilter). Changing the sub-tab does NOT refetch.
  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    const documentId = docFilter === EDGE_FILTER_ALL ? undefined : docFilter;
    getProofReview(slug, documentId)
      .then((data) => {
        if (cancelled) return;
        setPayload(data);
        // Capture the full document list only from the unfiltered load.
        if (documentId === undefined) {
          setDocOptions(distinctSourceDocuments(data));
        }
        setLoading(false);
      })
      .catch((e: unknown) => {
        if (cancelled) return;
        setError(e instanceof Error ? e.message : String(e));
        setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [slug, docFilter]);

  if (loading) return <div style={messageStyle}>Loading Proof Review…</div>;
  if (error) return <div style={errorStyle}>{error}</div>;
  if (!payload) return <div style={messageStyle}>No proof-review data.</div>;

  const badges = subTabBadgeCounts(payload);
  const empties = sectionEmptyStates(payload);
  const visibleEdges = filterEdges(payload.proof_edges, {
    statementType: edgeStatementType,
    sourceDocument: EDGE_FILTER_ALL,
  });
  const statementTypeOptions = distinctStatementTypes(payload.proof_edges);

  return (
    <div style={containerStyle}>
      <Breadcrumb
        items={[
          { label: "Dashboard", to: "/" },
          { label: "Proof Matrix", to: `/cases/${slug}/proof-matrix` },
          { label: "Proof Review" },
        ]}
      />
      <div style={{ marginBottom: "1.25rem" }}>
        <h1 className="count-header" style={{ margin: 0 }}>
          Proof Review
        </h1>
        <div style={subtitleStyle}>
          Discovery answers corroborating complaint allegations — and the
          non-answers the bar excluded. Read-only.
        </div>
      </div>

      {/* Document filter — change refetches with ?document_id= */}
      <div style={controlsRowStyle}>
        <label style={{ fontSize: "0.82rem", color: "var(--text-muted)" }}>
          Document:&nbsp;
          <select
            style={selectStyle}
            value={docFilter}
            onChange={(e) => setDocFilter(e.target.value)}
          >
            <option value={EDGE_FILTER_ALL}>All documents</option>
            {docOptions.map((doc) => (
              <option key={doc} value={doc}>
                {doc}
              </option>
            ))}
          </select>
        </label>
      </div>

      {/* Sub-tab bar */}
      <div style={tabBarStyle}>
        {TABS.map((tab) => {
          const badge =
            tab.id === "proof_edges"
              ? badges.proofEdges
              : tab.id === "excluded"
                ? badges.excluded
                : tab.id === "borderline"
                  ? badges.borderline
                  : undefined;
          return (
            <SubTab
              key={tab.id}
              label={tab.label}
              active={activeTab === tab.id}
              badge={badge}
              onClick={() => setActiveTab(tab.id)}
            />
          );
        })}
      </div>

      {/* Tab content — rendered client-side over the one payload */}
      {activeTab === "summary" &&
        (empties.summaryEmpty ? (
          <EmptyState message="No corroboration or excluded data for this scope." />
        ) : (
          <SummarySection summary={payload.summary} />
        ))}

      {activeTab === "proof_edges" &&
        (empties.proofEdgesEmpty ? (
          <EmptyState message="No proof edges for this scope." />
        ) : (
          <>
            <div style={controlsRowStyle}>
              <label style={{ fontSize: "0.82rem", color: "var(--text-muted)" }}>
                Statement type:&nbsp;
                <select
                  style={selectStyle}
                  value={edgeStatementType}
                  onChange={(e) => setEdgeStatementType(e.target.value)}
                >
                  <option value={EDGE_FILTER_ALL}>All</option>
                  {statementTypeOptions.map((t) => (
                    <option key={t} value={t}>
                      {t}
                    </option>
                  ))}
                </select>
              </label>
              <span style={{ fontSize: "0.78rem", color: "var(--text-muted)" }}>
                {visibleEdges.length} of {payload.proof_edges.length}
              </span>
            </div>
            {visibleEdges.length === 0 ? (
              <EmptyState message="No proof edges match this filter." />
            ) : (
              visibleEdges.map((edge, i) => (
                <ProofEdgeCard key={edge.allegation_id ?? i} edge={edge} />
              ))
            )}
          </>
        ))}

      {activeTab === "excluded" &&
        (empties.excludedEmpty ? (
          <EmptyState message="No excluded non-answers for this scope." />
        ) : (
          payload.excluded.map((row, i) => <ExcludedCard key={i} row={row} />)
        ))}

      {activeTab === "borderline" &&
        (empties.borderlineEmpty ? (
          <EmptyState message="No borderline (partial-admission) edges for this scope." />
        ) : (
          payload.borderline.map((edge, i) => (
            <ProofEdgeCard key={edge.allegation_id ?? i} edge={edge} />
          ))
        ))}
    </div>
  );
};

export default ProofReviewPage;
