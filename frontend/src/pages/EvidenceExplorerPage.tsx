import React, { useEffect, useMemo, useState } from "react";
import { getAllegations, AllegationDto } from "../services/allegations";
import { getEvidenceChain, EvidenceChainResponse } from "../services/evidenceChain";
import { getAnalysis, AllegationStrength } from "../services/analysisApi";
import { LegalCountInfo } from "../services/caseSummary";
import { useCase } from "../context/CaseContext";
import { COLORS } from "../components/EvidenceChainParts";
import { CountGroup, CountSection } from "../components/EvidenceExplorerParts";
import InfoPopup from "../components/InfoPopup";

const OTHER_BUCKET_NAME = "Other Allegations";

/**
 * Group allegations by legal count, seeded from the full `legal_count_details`
 * list (so counts with zero SUPPORTS-linked allegations still render a card)
 * and bucketed on stable `legal_count_ids` (LegalCount.id — immune to LLM
 * title drift, unlike the previous title-based map). Allegations without a
 * matching count id fall through to the "Other Allegations" bucket.
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
      allegations: g.allegations.sort((a, b) => a.id.localeCompare(b.id)),
    }))
    .sort((a, b) => a.countNumber - b.countNumber);

  if (other.length > 0) {
    groups.push({
      countName: OTHER_BUCKET_NAME,
      countId: null,
      countNumber: 0,
      allegations: other.sort((a, b) => a.id.localeCompare(b.id)),
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

  const handleToggle = async (allegationId: string) => {
    if (expandedIds.has(allegationId)) {
      setExpandedIds((prev) => { const n = new Set(prev); n.delete(allegationId); return n; });
      return;
    }
    if (chainCache.has(allegationId)) {
      setExpandedIds((prev) => new Set(prev).add(allegationId));
      return;
    }
    setLoadingIds((prev) => new Set(prev).add(allegationId));
    try {
      const chain = await getEvidenceChain(allegationId);
      setChainCache((prev) => new Map(prev).set(allegationId, chain));
      setExpandedIds((prev) => new Set(prev).add(allegationId));
    } catch (err) {
      console.error("Failed to fetch evidence chain:", err);
    } finally {
      setLoadingIds((prev) => { const n = new Set(prev); n.delete(allegationId); return n; });
    }
  };

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
            <p style={{ margin: "0.5rem 0 0", fontSize: "0.8rem", color: "#6b7280" }}>
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
                onToggleAllegation={handleToggle}
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
