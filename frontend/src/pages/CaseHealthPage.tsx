// =============================================================================
// CaseHealthPage.tsx — /cases/:slug/case-health (Pane 1, "Graph Inventory")
// -----------------------------------------------------------------------------
// Thin renderer over the composed payload from
// `getCaseHealthInventory(slug)` (services/caseHealth) and the presentational
// pieces in CaseHealthViews. NO numbers are computed here — the rates, counts,
// ordering and edge-class column list all arrive already decided by the backend.
//
// Why this page exists: eight documents were processed and judged on grounding
// rate while the metric that actually matters — how much of the extracted
// Evidence is wired into an Allegation — lived nowhere in the product and had to
// be hand-queried. This page makes that blindness structurally impossible.
// =============================================================================

import React, { useEffect, useState } from "react";
import { useParams } from "react-router-dom";

import Breadcrumb from "../components/Breadcrumb";
import {
  CorpusHeadline,
  DocumentConnectionTable,
  EdgeTripleTable,
  NodeLabelTable,
} from "../components/CaseHealthViews";
import { DEFAULT_CASE_SLUG } from "../services/caseHeader";
import {
  getCaseHealthInventory,
  type CaseHealthInventoryResponse,
} from "../services/caseHealth";

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
const measuredAtStyle: React.CSSProperties = {
  marginTop: "4px",
  fontFamily: "var(--font-sans)",
  fontSize: "12px",
  color: "var(--text-muted)",
};
// Loading + error styling reuse the SAME design tokens the Proof Matrix and
// Trial Prep pages' gating states use — no bespoke colors (Rule 2).
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
 * Gating read: the inventory payload. A failure here blanks the page with a
 * visible message (Standing Rule 1 — every `authFetch` has an explicit `.catch()`
 * and explicit error UI; nothing degrades silently to an empty table, which
 * would read as "the graph is empty" and be the exact opposite of the truth).
 *
 * The `cancelled` flag stops a navigate-away mid-flight from setting state on an
 * unmounted component. Mirrors `useTrialPrepDashboard`.
 */
function useCaseHealthInventory(slug: string): {
  inventory: CaseHealthInventoryResponse | null;
  loading: boolean;
  error: string | null;
} {
  const [inventory, setInventory] =
    useState<CaseHealthInventoryResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    getCaseHealthInventory(slug)
      .then((data) => {
        if (cancelled) return;
        setInventory(data);
        setLoading(false);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        setError(
          err instanceof Error
            ? err.message
            : "Failed to load Case Health. Try reloading the page.",
        );
        setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [slug]);

  return { inventory, loading, error };
}

const CaseHealthPage: React.FC = () => {
  const { slug: slugParam } = useParams<{ slug: string }>();
  const slug = slugParam ?? DEFAULT_CASE_SLUG;

  const { inventory, loading, error } = useCaseHealthInventory(slug);

  if (loading) return <div style={messageStyle}>Loading Case Health…</div>;
  if (error) return <div style={errorStyle}>{error}</div>;
  // After loading with no error the payload is always set; this guard satisfies
  // the type narrowing and would surface a genuinely impossible state loudly
  // rather than rendering a blank page.
  if (!inventory)
    return (
      <div style={errorStyle}>
        Case Health returned no measurement. Try reloading the page.
      </div>
    );

  const { current, edge_classes: edgeClasses } = inventory;

  return (
    <div className="container" style={containerStyle}>
      <Breadcrumb items={[{ label: "Case Health" }]} />
      <h1>Case Health — Graph Inventory</h1>
      <p style={subtitleStyle}>
        How much of the extracted Evidence is actually wired into the case, and
        which documents are carrying it.
      </p>
      <p style={measuredAtStyle}>Measured {current.computed_at}</p>

      <CorpusHeadline corpus={current.corpus} />
      <DocumentConnectionTable
        documents={current.documents}
        edgeClasses={edgeClasses}
      />
      <NodeLabelTable
        labels={current.node_labels}
        unlabeledNodeCount={current.unlabeled_node_count}
      />
      <EdgeTripleTable triples={current.edge_triples} />
    </div>
  );
};

export default CaseHealthPage;
