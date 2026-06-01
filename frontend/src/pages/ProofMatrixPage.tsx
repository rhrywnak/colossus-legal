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
// Part 3 (not here) adds the Supporting / Opposing / Status columns and the
// row-expand (ElementDetailContent). This part renders no evidence columns, no
// placeholder cells, and wires no row expansion.
//
// The component is split into a data hook + small presentational pieces so every
// function stays within the 50-line limit (CLAUDE.md Rule 18).
// =============================================================================

import React, { useEffect, useMemo, useState } from "react";
import { useParams } from "react-router-dom";
import Breadcrumb from "../components/Breadcrumb";
import CountSelector from "../components/CountSelector";
import ElementRow from "../components/ElementRow";
import { sortElements } from "../components/CountCard";
import { CountDetail, getCausesOfAction } from "../services/causesOfAction";
import {
  getProofMatrixRollup,
  indexAllegationTotals,
} from "../services/proofMatrix";
import { DEFAULT_CASE_SLUG } from "../services/caseHeader";

/**
 * Element rows are non-interactive in Part 2: there is no per-Element selection
 * or expand until Part 3 wires `ElementDetailContent`. ElementRow requires an
 * `onSelect` prop, so we pass this shared no-op — a single stable reference (not
 * a fresh closure per render). This is a deliberate inert state, not a swallowed
 * action (Rule 1): Part 3 replaces it with the real expand handler. (Consuming
 * ElementRow this way needs no change to ElementRow itself.)
 */
const NOOP_SELECT = (): void => {};

/** What the page needs from its two reads, after shaping. */
interface ProofMatrixData {
  /** Counts sorted ascending by number; `[]` until the gating fetch resolves. */
  sortedCounts: CountDetail[];
  loading: boolean;
  error: string | null;
  /** Deduped per-Count totals keyed by count_number (supplementary). */
  allegationTotals: Record<number, number>;
}

/**
 * Gating read: the Counts + their Elements + per-Element counts. A failure here
 * blanks the page with a visible message, so it is surfaced as `error`. The
 * `cancelled` flag stops a navigate-away mid-flight from setting state on an
 * unmounted component.
 */
function useCausesOfAction(slug: string): {
  counts: CountDetail[] | null;
  loading: boolean;
  error: string | null;
} {
  const [counts, setCounts] = useState<CountDetail[] | null>(null);
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

  return { counts, loading, error };
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
  const { counts, loading, error } = useCausesOfAction(slug);
  const allegationTotals = useRollupTotals(slug);
  const sortedCounts = useMemo(
    () =>
      counts ? [...counts].sort((a, b) => a.count_number - b.count_number) : [],
    [counts],
  );
  return { sortedCounts, loading, error, allegationTotals };
}

/**
 * Column header + the selected Count's Element rows. The header labels the two
 * LIVE columns only ("Element", "Mapped Allegations"); Part 3 extends ElementRow
 * and this header together with Supporting / Opposing / Status — no placeholder
 * cells are pre-rendered here.
 */
const ElementTable: React.FC<{ count: CountDetail }> = ({ count }) => {
  const elements = sortElements(count.elements);
  return (
    <div style={{ ...CARD_STYLE, marginTop: "20px" }}>
      <div style={COLUMN_HEADER_STYLE}>
        <span>Element</span>
        <span>Mapped Allegations</span>
      </div>
      {elements.length === 0 ? (
        <div style={MESSAGE_STYLE}>No Elements loaded for this Count.</div>
      ) : (
        <div>
          {elements.map((el, i) => (
            <ElementRow
              key={el.element_id}
              element={el}
              countNumber={count.count_number}
              index={i}
              selected={false}
              onSelect={NOOP_SELECT}
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
}> = ({ sortedCounts, allegationTotals }) => {
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
      <ElementTable count={selected} />
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
  const { sortedCounts, loading, error, allegationTotals } =
    useProofMatrixData(slug);

  if (loading) return <div style={MESSAGE_STYLE}>Loading Proof Matrix...</div>;
  if (error) return <div style={ERROR_STYLE}>{error}</div>;

  return (
    <div style={{ maxWidth: "1000px", paddingTop: "32px", paddingBottom: "4rem" }}>
      <Breadcrumb
        items={[{ label: "Dashboard", to: "/" }, { label: "Proof Matrix" }]}
      />
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

// Column header row. "Element" sits over the number+name columns (left),
// "Mapped Allegations" over the badge (right) — `space-between` aligns them to
// the row's outer edges, matching ElementRow's horizontal padding.
const COLUMN_HEADER_STYLE: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  padding: "0 12px 8px",
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
