import React, { useEffect, useMemo, useState } from "react";
import { getAllegations, AllegationDto } from "../services/allegations";
import { getEvidenceChain, EvidenceChainResponse } from "../services/evidenceChain";
import { useCase } from "../context/CaseContext";
import { COLORS } from "../components/EvidenceChainParts";
import { CountGroup, CountSection } from "../components/EvidenceExplorerParts";

// ─── Count metadata (hardcoded — not in API yet) ────────────────────────────

const COUNT_METADATA: Record<string, { numeral: string; legal_basis: string; paragraphs: string }> = {
  "Breach of Fiduciary Duty": { numeral: "I", legal_basis: "MCL 700.1212", paragraphs: "72-85" },
  "Fraud": { numeral: "II", legal_basis: "Common Law", paragraphs: "86-100" },
  "Declaratory Relief": { numeral: "III", legal_basis: "MCL 450.178 / MCL 700.1212", paragraphs: "101-114" },
  "Abuse of Process": { numeral: "IV", legal_basis: "Common Law", paragraphs: "115-126" },
};

const NUMERAL_ORDER: Record<string, number> = { I: 1, II: 2, III: 3, IV: 4 };

/** Group allegations by legal count. Allegations in multiple counts appear in each. */
function groupByCount(allegations: AllegationDto[]): CountGroup[] {
  const map = new Map<string, AllegationDto[]>();
  const other: AllegationDto[] = [];

  for (const a of allegations) {
    const counts = a.legal_counts ?? [];
    if (counts.length === 0) {
      other.push(a);
    } else {
      for (const c of counts) {
        if (!map.has(c)) map.set(c, []);
        map.get(c)!.push(a);
      }
    }
  }

  const groups: CountGroup[] = Array.from(map.entries())
    .map(([name, list]) => {
      const meta = COUNT_METADATA[name] ?? { numeral: "?", legal_basis: "", paragraphs: "" };
      const sorted = list.sort((a, b) => a.id.localeCompare(b.id));
      return {
        countName: name,
        numeral: meta.numeral,
        legalBasis: meta.legal_basis,
        paragraphs: meta.paragraphs,
        allegations: sorted,
        provenCount: sorted.filter((a) => a.evidence_status?.toUpperCase() === "PROVEN").length,
      };
    })
    .sort((a, b) => (NUMERAL_ORDER[a.numeral] ?? 99) - (NUMERAL_ORDER[b.numeral] ?? 99));

  if (other.length > 0) {
    groups.push({
      countName: "Other Allegations",
      numeral: "",
      legalBasis: "",
      paragraphs: "",
      allegations: other.sort((a, b) => a.id.localeCompare(b.id)),
      provenCount: other.filter((a) => a.evidence_status?.toUpperCase() === "PROVEN").length,
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

  useEffect(() => {
    let active = true;
    getAllegations()
      .then((result) => { if (active) { setAllegations(result.allegations); setError(null); } })
      .catch(() => { if (active) { setAllegations([]); setError("Failed to load allegations"); } })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, []);

  // caseData accessed to ensure CaseContext is connected
  void caseData;

  const countGroups = useMemo(() => groupByCount(allegations), [allegations]);

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
          Evidence Explorer
        </h1>
        <p style={{ fontSize: "0.95rem", color: COLORS.textSecondary, marginTop: "0.25rem", marginBottom: 0 }}>
          Trace every allegation in the complaint back to its supporting evidence and source documents.
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
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default EvidenceExplorerPage;
