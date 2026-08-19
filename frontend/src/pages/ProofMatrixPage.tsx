// =============================================================================
// ProofMatrixPage.tsx — the Proof Matrix page (PM4), Part 2: shell + selector +
// live structural columns
// -----------------------------------------------------------------------------
// Route: /cases/:slug/proof-matrix  (top-level page; the user switches Counts in
// place via local state, NOT by navigating per Count).
//
// Structurally parallel to PM3's Count-detail drill-down, but as a standing page:
// a Count selector on top, and below it the selected Count's Elements rendered as
// rows (the shared ElementRow from Part 1) showing the LIVE structural columns —
// Element name/number + per-Element mapped-allegation count.
//
// Data path (Home's parallel-fetch + graceful-degrade pattern, here factored into
// the `useProofMatrixData` hook):
//   - getCausesOfAction(slug) GATES the page: it supplies the Counts, their
//     Elements, and the per-Element `allegation_count` (correct per-Element —
//     used directly in each row).
//   - getProofMatrixRollup(slug) is NON-BLOCKING supplementary: it supplies the
//     selector's DEDUPED per-Count total. We must NOT sum per-Element counts for
//     a Count total (an allegation bearing on several Elements of one Count would
//     be double-counted — the exact dedup the rollup endpoint fixes). On a failed
//     or pending rollup the selector degrades to a muted `—`.
//
// Part 3 added the Supporting / Opposing / Status columns (honest "discovery
// pending" empties — no evidence data exists yet) and row-expand: clicking an
// Element reveals its live mapped-allegation detail via ElementDetailContent.
//
// The component is split into a data hook + small presentational pieces so every
// function stays within the 50-line limit (CLAUDE.md Rule 18).
// =============================================================================

import React, { useEffect, useMemo, useState } from "react";
import { useParams, useSearchParams } from "react-router-dom";
import Breadcrumb from "../components/Breadcrumb";
import PageTabs, { activeTab } from "../components/PageTabs";
import { PROOF_TABS } from "./proofTabs";
import ProofReviewPage from "./ProofReviewPage";
import CountSelector from "../components/CountSelector";
import MatrixRowWithDetail from "../components/MatrixRowWithDetail";
import {
  PROOF_MATRIX_GRID_TEMPLATE,
  proofMatrixColumnLabels,
} from "../components/proofMatrixColumns";
import { sortElements } from "../components/CountCard";
import {
  CountDetail,
  getCausesOfAction,
  type MatrixWording,
} from "../services/causesOfAction";
import {
  getProofMatrixRollup,
  indexAllegationTotals,
} from "../services/proofMatrix";
import { DEFAULT_CASE_SLUG } from "../services/caseHeader";

/** What the page needs from its two reads, after shaping. */
interface ProofMatrixData {
  /** Counts sorted ascending by number; `[]` until the gating fetch resolves. */
  sortedCounts: CountDetail[];
  loading: boolean;
  error: string | null;
  /** Deduped per-Count totals keyed by count_number (supplementary). */
  allegationTotals: Record<number, number>;
  /**
   * The matrix's served words. `null` until the gating fetch resolves — there is
   * deliberately no fallback vocabulary to draw a column header from (the
   * language law; the same `null`-until-loaded shape `CandidateCard` uses).
   */
  matrixWording: MatrixWording | null;
}

/**
 * Gating read: the Counts + their Elements + per-Element counts. A failure here
 * blanks the page with a visible message, so it is surfaced as `error`. The
 * `cancelled` flag stops a navigate-away mid-flight from setting state on an
 * unmounted component.
 */
function useCausesOfAction(slug: string): {
  counts: CountDetail[] | null;
  matrixWording: MatrixWording | null;
  loading: boolean;
  error: string | null;
} {
  const [counts, setCounts] = useState<CountDetail[] | null>(null);
  const [matrixWording, setMatrixWording] = useState<MatrixWording | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    getCausesOfAction(slug)
      .then((data) => {
        if (cancelled) return;
        setCounts(data.counts);
        setMatrixWording(data.matrix_wording);
        setLoading(false);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        setError(
          err instanceof Error
            ? err.message
            : "Failed to load the Proof Matrix. Try reloading the page.",
        );
        setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [slug]);

  return { counts, matrixWording, loading, error };
}

/**
 * Supplementary read: the deduped per-Count totals. Best-effort — a failure must
 * NOT blank the page; the selector degrades to a muted `—` instead.
 */
function useRollupTotals(slug: string): Record<number, number> {
  const [allegationTotals, setAllegationTotals] = useState<
    Record<number, number>
  >({});

  useEffect(() => {
    let cancelled = false;
    getProofMatrixRollup(slug)
      .then((rollup) => {
        if (!cancelled) setAllegationTotals(indexAllegationTotals(rollup.counts));
      })
      // best-effort: the muted `—` placeholder IS the user-visible degradation;
      // we log an observable, contextual message (Rule 1) but do not block the
      // page or surface a second error banner (mirrors Home's rollup fetch).
      .catch((err: unknown) => {
        const message = err instanceof Error ? err.message : "unknown error";
        console.error(
          `ProofMatrixPage: could not load allegation totals — ${message}`,
        );
      });
    return () => {
      cancelled = true;
    };
  }, [slug]);

  return allegationTotals;
}

/**
 * Compose the two reads and shape them for the page.
 *
 * ## React Learning: a custom hook is just a function that calls hooks
 * Splitting the gating and supplementary reads into their own hooks keeps each
 * within the 50-line limit (Rule 18) and lets the page treat "data" as one value.
 */
function useProofMatrixData(slug: string): ProofMatrixData {
  const { counts, matrixWording, loading, error } = useCausesOfAction(slug);
  const allegationTotals = useRollupTotals(slug);
  const sortedCounts = useMemo(
    () =>
      counts ? [...counts].sort((a, b) => a.count_number - b.count_number) : [],
    [counts],
  );
  return { sortedCounts, loading, error, allegationTotals, matrixWording };
}

/**
 * Column header + the selected Count's Element rows, as the five-column matrix.
 * Header and rows share `PROOF_MATRIX_GRID_TEMPLATE`, so they stay aligned. The
 * three evidence columns render honest "discovery pending" empties (no data
 * exists yet); Mapped Allegations is the real per-Element count.
 *
 * Owns the single-open-accordion `expandedElementId` state. The page keys this
 * table by Count, so switching Counts remounts it and collapses any open row.
 */
const ElementTable: React.FC<{
  count: CountDetail;
  caseSlug: string;
  matrixWording: MatrixWording;
}> = ({ count, caseSlug, matrixWording }) => {
  const elements = sortElements(count.elements);
  const [expandedElementId, setExpandedElementId] = useState<string | null>(null);
  const toggleExpand = (elementId: string) =>
    setExpandedElementId((prev) => (prev === elementId ? null : elementId));

  return (
    <div style={{ ...CARD_STYLE, marginTop: "20px" }}>
      <div style={COLUMN_HEADER_STYLE}>
        {proofMatrixColumnLabels(matrixWording.strong_column_label).map((label) => (
          <span key={label}>{label}</span>
        ))}
      </div>
      {elements.length === 0 ? (
        <div style={MESSAGE_STYLE}>No Elements loaded for this Count.</div>
      ) : (
        <div>
          {elements.map((el, i) => (
            <MatrixRowWithDetail
              key={el.element_id}
              element={el}
              countNumber={count.count_number}
              index={i}
              caseSlug={caseSlug}
              expanded={expandedElementId === el.element_id}
              onToggleExpand={toggleExpand}
              matrixWording={matrixWording}
            />
          ))}
        </div>
      )}
    </div>
  );
};

/**
 * The happy-path body: the Count selector + the selected Count's Element table.
 * Owns the in-place `selectedCountNumber` state (defaulting to the first Count).
 * Rendered only when there is at least one Count, so `sortedCounts[0]` is safe.
 */
const ProofMatrixContent: React.FC<{
  sortedCounts: CountDetail[];
  allegationTotals: Record<number, number>;
  caseSlug: string;
  matrixWording: MatrixWording;
}> = ({ sortedCounts, allegationTotals, caseSlug, matrixWording }) => {
  const [selectedCountNumber, setSelectedCountNumber] = useState<number>(
    sortedCounts[0].count_number,
  );

  // Re-default if the selected number is no longer present (defensive; the
  // Counts are static once loaded).
  useEffect(() => {
    if (!sortedCounts.some((c) => c.count_number === selectedCountNumber)) {
      setSelectedCountNumber(sortedCounts[0].count_number);
    }
  }, [sortedCounts, selectedCountNumber]);

  const selected =
    sortedCounts.find((c) => c.count_number === selectedCountNumber) ??
    sortedCounts[0];

  return (
    <>
      <CountSelector
        counts={sortedCounts}
        selectedCountNumber={selected.count_number}
        allegationTotals={allegationTotals}
        onSelect={setSelectedCountNumber}
      />
      {/* Key by Count: switching Counts remounts the table, collapsing any
          expanded row (the single-open accordion resets per Count). */}
      <ElementTable
        key={selected.count_number}
        count={selected}
        caseSlug={caseSlug}
        matrixWording={matrixWording}
      />
    </>
  );
};

/**
 * Page shell: resolve the slug, run the data hook, handle loading/error/empty,
 * and render the header + content.
 */
const ProofMatrixPage: React.FC = () => {
  const { slug: slugParam } = useParams<{ slug: string }>();
  const slug = slugParam ?? DEFAULT_CASE_SLUG;
  const [search] = useSearchParams();
  const tab = activeTab(PROOF_TABS, search);
  const { sortedCounts, loading, error, allegationTotals, matrixWording } =
    useProofMatrixData(slug);

  // The review half is a whole page of its own and reads its own data. It is
  // returned BEFORE the matrix's loading and error gates: those describe the
  // matrix's fetch, and making the review tab wait behind them would show
  // "Loading Proof Matrix..." over a panel that is not the matrix.
  // Rendered BARE. `ProofReviewPage` already carries its own container,
  // breadcrumb, heading and tab bar — wrapping it in this page's chrome would
  // draw two breadcrumbs and two containers, one nested in the other.
  if (tab === "review") return <ProofReviewPage />;

  if (loading) return <div style={MESSAGE_STYLE}>Loading Proof Matrix...</div>;
  if (error) return <div style={ERROR_STYLE}>{error}</div>;
  // After a successful load the wording is always set (the service refuses a
  // payload without it). This narrows `MatrixWording | null` for the tree below,
  // and says out loud what would otherwise be a silent blank header.
  if (!matrixWording) {
    return (
      <div style={ERROR_STYLE}>
        The Proof Matrix loaded without its column wording. Reload the page; if
        this persists, report it to the site administrator.
      </div>
    );
  }

  return (
    <div style={{ maxWidth: "1000px", paddingTop: "32px", paddingBottom: "4rem" }}>
      <Breadcrumb
        items={[{ label: "Dashboard", to: "/" }, { label: "Proof Matrix" }]}
      />
      <PageTabs tabs={PROOF_TABS} />
      <div style={{ marginBottom: "1.25rem" }}>
        <h1 className="count-header" style={{ margin: 0 }}>
          Proof Matrix
        </h1>
        <div style={SUBTITLE_STYLE}>
          Select a Count to see its Elements and the allegations mapped to each.
        </div>
      </div>
      {sortedCounts.length === 0 ? (
        <div style={MESSAGE_STYLE}>
          No Counts loaded for this case. Run the canonical Element loader.
        </div>
      ) : (
        <ProofMatrixContent
          sortedCounts={sortedCounts}
          allegationTotals={allegationTotals}
          caseSlug={slug}
          matrixWording={matrixWording}
        />
      )}
    </div>
  );
};

// ─── Styles (tokens only) ────────────────────────────────────────────────────

const CARD_STYLE: React.CSSProperties = {
  border: "1px solid var(--border-default)",
  backgroundColor: "var(--bg-surface)",
  borderRadius: "8px",
  padding: "20px 24px",
};

const SUBTITLE_STYLE: React.CSSProperties = {
  marginTop: "6px",
  fontFamily: "var(--font-sans)",
  fontSize: "14px",
  color: "var(--text-secondary)",
};

// Column header row. Uses the SAME grid template as the matrix ElementRow (and a
// matching 3px transparent left border + 12px side padding) so every header
// label sits exactly over its column.
const COLUMN_HEADER_STYLE: React.CSSProperties = {
  display: "grid",
  gridTemplateColumns: PROOF_MATRIX_GRID_TEMPLATE,
  alignItems: "center",
  gap: "12px",
  padding: "0 12px 8px",
  borderLeft: "3px solid transparent",
  borderBottom: "1px solid var(--border-default)",
  marginBottom: "8px",
  fontFamily: "var(--font-sans)",
  fontSize: "12px",
  fontWeight: 600,
  letterSpacing: "0.05em",
  textTransform: "uppercase",
  color: "var(--text-secondary)",
};

const MESSAGE_STYLE: React.CSSProperties = {
  padding: "2rem",
  textAlign: "center",
  color: "var(--text-muted)",
  fontSize: "14px",
};

const ERROR_STYLE: React.CSSProperties = {
  margin: "1rem 0",
  padding: "1rem",
  backgroundColor: "var(--state-danger-bg-soft)",
  border: "1px solid var(--state-danger-border)",
  borderRadius: "6px",
  color: "var(--state-danger-strong)",
};

export default ProofMatrixPage;
