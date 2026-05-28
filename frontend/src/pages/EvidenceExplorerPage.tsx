import React, { useCallback, useEffect, useMemo, useState } from "react";
import { getAllegations, AllegationDto } from "../services/allegations";
import { getEvidenceChain, EvidenceChainResponse } from "../services/evidenceChain";
import { getAnalysis, AllegationStrength } from "../services/analysisApi";
import { LegalCountInfo } from "../services/caseSummary";
import { useCase } from "../context/CaseContext";
import { COLORS } from "../components/EvidenceChainParts";
import { CountGroup, CountSection } from "../components/EvidenceExplorerParts";
import InfoPopup from "../components/InfoPopup";
import { parseLeadingParagraph } from "../utils/paragraphSort";

/**
 * Sort comparator: order Allegations by their complaint paragraph number
 * ascending, falling back to original-index stability for ties and pushing
 * non-numeric paragraph values to the end.
 *
 * Used twice in groupByCount() — once per real Count, once for the
 * Unassigned bucket — so the inline closure stays at the call site and the
 * primitive parse lives in utils/paragraphSort.ts. Previous behavior sorted
 * by stable_entity_id (`a.id`) which produced essentially random order.
 */
function compareByParagraph(
  a: { paragraph?: string },
  b: { paragraph?: string },
): number {
  const pa = a.paragraph != null ? parseLeadingParagraph(a.paragraph) : null;
  const pb = b.paragraph != null ? parseLeadingParagraph(b.paragraph) : null;
  if (pa === null && pb === null) return 0;
  if (pa === null) return 1; // non-numeric → end
  if (pb === null) return -1;
  return pa - pb;
}

const UNASSIGNED_BUCKET_NAME = "Unassigned";

/**
 * Group allegations by legal count, seeded from `legal_count_details`
 * (so counts with zero allegations still render a card). The backend now
 * assigns each allegation a deterministic section via paragraph number
 * ranges: "common-allegations" for ¶7-71, real LegalCount IDs for the
 * four Counts. The "common-allegations" group is created dynamically
 * here since it isn't a real LegalCount node. Allegations with no
 * recognized section fall into "Unassigned" (should be rare/empty).
 */
function groupByCount(
  allegations: AllegationDto[],
  legalCountDetails: LegalCountInfo[],
): CountGroup[] {
  const byId = new Map<string, CountGroup>();
  for (const lc of legalCountDetails) {
    byId.set(lc.id, {
      countName: lc.name || `Count ${lc.count_number}`,
      countId: lc.id,
      countNumber: lc.count_number,
      allegations: [],
    });
  }

  const other: AllegationDto[] = [];
  for (const a of allegations) {
    const ids = a.legal_count_ids ?? [];
    let bucketed = false;
    for (const id of ids) {
      if (id === "common-allegations" && !byId.has(id)) {
        byId.set(id, {
          countName: "Common Allegations",
          countId: "common-allegations",
          countNumber: 0,
          allegations: [],
        });
      }
      const group = byId.get(id);
      if (group) {
        group.allegations.push(a);
        bucketed = true;
      }
    }
    if (!bucketed) {
      other.push(a);
    }
  }

  const groups: CountGroup[] = Array.from(byId.values())
    .map((g) => ({
      ...g,
      // Sort by complaint paragraph (numeric ascending) instead of by raw
      // stable_entity_id, which produced essentially random visual order.
      allegations: g.allegations.sort(compareByParagraph),
    }))
    .sort((a, b) => a.countNumber - b.countNumber);

  if (other.length > 0) {
    groups.push({
      countName: UNASSIGNED_BUCKET_NAME,
      countId: null,
      countNumber: 999,
      allegations: other.sort(compareByParagraph),
    });
  }

  return groups;
}

// ─── Main component ──────────────────────────────────────────────────────────

const EvidenceExplorerPage: React.FC = () => {
  const { caseData } = useCase();
  const [allegations, setAllegations] = useState<AllegationDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [expandedIds, setExpandedIds] = useState<Set<string>>(new Set());
  const [chainCache, setChainCache] = useState<Map<string, EvidenceChainResponse>>(new Map());
  const [loadingIds, setLoadingIds] = useState<Set<string>>(new Set());
  // Per-allegation chain-fetch error messages. Distinct from `loadingIds`
  // (still in flight) and `chainCache` (succeeded) so the three states are
  // mutually exclusive observables (Standing Rule 1). Populated by
  // fetchChain's catch arm; rendered as an inline "Failed to load —
  // Retry" block inside the expanded row.
  const [errorIds, setErrorIds] = useState<Map<string, string>>(new Map());
  const [collapsedCounts, setCollapsedCounts] = useState<Set<string>>(new Set());
  const [strengthMap, setStrengthMap] = useState<Map<string, AllegationStrength>>(new Map());

  useEffect(() => {
    let active = true;
    Promise.all([getAllegations(), getAnalysis()])
      .then(([allegationsResult, analysisResult]) => {
        if (!active) return;
        setAllegations(allegationsResult.allegations);
        const map = new Map<string, AllegationStrength>();
        for (const a of analysisResult.gap_analysis.allegations) {
          map.set(a.id, a);
        }
        setStrengthMap(map);
        setError(null);
      })
      .catch(() => {
        if (!active) return;
        setAllegations([]);
        setError("Failed to load allegations");
      })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, []);

  const legalCountDetails = caseData?.legal_count_details ?? [];

  const countGroups = useMemo(
    () => groupByCount(allegations, legalCountDetails),
    [allegations, legalCountDetails],
  );

  // Seed every count as collapsed the first time we learn what the groups
  // are. Leaving the set empty meant every card was open on load, which
  // pushed the evidence below the fold — users had to scroll past four
  // fully-expanded cards before they could pick one. We only seed once
  // (`size === 0`) so the user's expand/collapse choices survive any
  // subsequent re-renders of `countGroups`.
  useEffect(() => {
    if (countGroups.length > 0 && collapsedCounts.size === 0) {
      setCollapsedCounts(new Set(countGroups.map((g) => g.countName)));
    }
  }, [countGroups, collapsedCounts.size]);

  /**
   * Fetch the evidence chain for one Allegation. Pulled out of
   * `handleToggle` so the Retry button (rendered inside an already-expanded
   * row that errored) can re-run the fetch without going through the toggle
   * state machine.
   *
   * The row is always set to `expanded` after this call — on success so
   * the chain becomes visible, on failure so the inline error + Retry
   * affordance can render. This keeps the user from having to re-click an
   * already-expanded row to retry.
   */
  const fetchChain = useCallback(async (allegationId: string) => {
    setLoadingIds((prev) => new Set(prev).add(allegationId));
    // Clear any prior error before the new attempt — a successful retry
    // must remove the stale "Failed to load" indicator.
    setErrorIds((prev) => {
      if (!prev.has(allegationId)) return prev;
      const n = new Map(prev);
      n.delete(allegationId);
      return n;
    });
    try {
      const chain = await getEvidenceChain(allegationId);
      setChainCache((prev) => new Map(prev).set(allegationId, chain));
      setExpandedIds((prev) => new Set(prev).add(allegationId));
    } catch (err) {
      // Distinct observable per Standing Rule 1: log for the operator AND
      // surface to the user (the previous behavior swallowed the failure
      // visually, leaving the row collapsed with no indication of why).
      const message =
        err instanceof Error
          ? err.message
          : "Failed to load evidence chain";
      console.error("Failed to fetch evidence chain:", err);
      setErrorIds((prev) => new Map(prev).set(allegationId, message));
      // Expand the row so the inline error message is visible — otherwise
      // the user clicked but the row stayed collapsed silently.
      setExpandedIds((prev) => new Set(prev).add(allegationId));
    } finally {
      setLoadingIds((prev) => { const n = new Set(prev); n.delete(allegationId); return n; });
    }
  }, []);

  const handleToggle = (allegationId: string) => {
    if (expandedIds.has(allegationId)) {
      setExpandedIds((prev) => { const n = new Set(prev); n.delete(allegationId); return n; });
      // Collapsing clears any error so the next open attempt is fresh.
      setErrorIds((prev) => {
        if (!prev.has(allegationId)) return prev;
        const n = new Map(prev);
        n.delete(allegationId);
        return n;
      });
      return;
    }
    if (chainCache.has(allegationId)) {
      setExpandedIds((prev) => new Set(prev).add(allegationId));
      return;
    }
    fetchChain(allegationId);
  };

  const handleRetry = useCallback(
    (allegationId: string) => fetchChain(allegationId),
    [fetchChain],
  );

  const toggleCount = (countName: string) => {
    setCollapsedCounts((prev) => {
      const n = new Set(prev);
      if (n.has(countName)) n.delete(countName); else n.add(countName);
      return n;
    });
  };

  if (loading) {
    return <div style={{ padding: "3rem", textAlign: "center", color: COLORS.textSecondary, backgroundColor: COLORS.bgPage, minHeight: "100vh" }}>Loading allegations...</div>;
  }
  if (error) {
    return (
      <div style={{ padding: "2rem", backgroundColor: COLORS.bgPage, minHeight: "100vh" }}>
        <div style={{ padding: "1rem 1.25rem", backgroundColor: COLORS.badgeUnprovenBg, border: `1px solid ${COLORS.border}`, borderRadius: "8px", color: COLORS.badgeUnprovenText, maxWidth: "1200px", margin: "0 auto" }}>
          {error}
        </div>
      </div>
    );
  }

  return (
    <div style={{ backgroundColor: COLORS.bgPage, minHeight: "100vh", padding: "2rem" }}>
      <div style={{ maxWidth: "1200px", margin: "0 auto" }}>
        <h1 style={{ fontSize: "1.75rem", fontWeight: 600, color: COLORS.textPrimary, margin: 0 }}>
          Case Evidence &amp; Analysis
          <InfoPopup>
            <strong style={{ display: "block", marginBottom: "0.5rem" }}>How evidence strength is calculated</strong>
            <p style={{ margin: "0 0 0.5rem" }}>
              Strength measures how many independent evidence items support each allegation
              through the proof chain (MotionClaim &rarr; Evidence). This is a measure of
              evidentiary coverage, not legal sufficiency.
            </p>
            <ul style={{ margin: 0, paddingLeft: "1.25rem" }}>
              <li>0 items = Gap (25%) &mdash; No evidence linked</li>
              <li>1 item = Weak (60%) &mdash; Single source</li>
              <li>2 items = Moderate (80%) &mdash; Multiple sources</li>
              <li>3+ items = Strong (90%+) &mdash; Well-supported</li>
            </ul>
            <p style={{ margin: "0.5rem 0 0", fontSize: "0.8rem", color: "var(--text-muted)" }}>
              Evidence is counted as distinct items linked via: Evidence &larr; RELIES_ON &larr; MotionClaim &rarr; PROVES &rarr; Allegation
            </p>
          </InfoPopup>
        </h1>
        <p style={{ fontSize: "0.95rem", color: COLORS.textSecondary, marginTop: "0.25rem", marginBottom: 0 }}>
          Allegations organized by legal Count with evidence strength analysis
        </p>
        <div style={{ borderBottom: `1px solid ${COLORS.border}`, margin: "1.5rem 0" }} />

        {countGroups.length === 0 ? (
          <div style={{ color: COLORS.textMuted, padding: "2rem", textAlign: "center", fontStyle: "italic" }}>
            No allegations found.
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: "1.5rem" }}>
            {countGroups.map((group) => (
              <CountSection
                key={group.countName}
                group={group}
                collapsed={collapsedCounts.has(group.countName)}
                onToggleCollapse={() => toggleCount(group.countName)}
                expandedIds={expandedIds}
                loadingIds={loadingIds}
                chainCache={chainCache}
                errorMap={errorIds}
                onToggleAllegation={handleToggle}
                onRetryAllegation={handleRetry}
                strengthMap={strengthMap}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default EvidenceExplorerPage;
