import React, { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useCase } from "../context/CaseContext";
import { HarmDto, getHarms } from "../services/harms";

// ─── Static data ─────────────────────────────────────────────────────────────

const ROMAN_NUMERALS: Record<number, string> = {
  1: "I", 2: "II", 3: "III", 4: "IV", 5: "V", 6: "VI", 7: "VII", 8: "VIII",
};

const toCountLabel = (countNumber: number): string => {
  const numeral = ROMAN_NUMERALS[countNumber] || String(countNumber);
  return `COUNT ${numeral}`;
};

// TODO: Fetch descriptions from LegalCount.description in Neo4j once the field
// is populated. For now these are hardcoded summaries from the complaint.
const COUNT_DESCRIPTIONS: Record<string, string> = {
  "count-breach-fiduciary-duty":
    "CFS and Phillips violated duties of loyalty and care owed to Marie as estate beneficiary.",
  "count-fraud":
    "Defendants made material misrepresentations to the court about Marie's cooperation and estate assets.",
  "count-declaratory-ultra-vires":
    "Request for court determination regarding the rights and duties of parties under the estate.",
  "count-abuse-of-process":
    "Phillips used court proceedings for improper purposes including sanctions motions and character attacks.",
};

const formatCurrency = (amount: number): string =>
  new Intl.NumberFormat("en-US", { style: "currency", currency: "USD" }).format(amount);

// ─── Component ───────────────────────────────────────────────────────────────

const Home: React.FC = () => {
  const { caseData, loading, error } = useCase();
  const [showDamages, setShowDamages] = useState(false);
  const [harms, setHarms] = useState<HarmDto[]>([]);
  const [harmsLoading, setHarmsLoading] = useState(false);
  const [timelinePhases, setTimelinePhases] = useState<{ id: string; label: string; date_range: string; color: string; eventCount: number }[]>([]);
  const [timelineEventCount, setTimelineEventCount] = useState(0);

  useEffect(() => {
    fetch("/data/timeline.json")
      .then((r) => r.json())
      .then((d) => {
        const phases = (d.phases ?? []).map((p: any) => ({
          id: p.id,
          label: p.label,
          date_range: p.date_range,
          color: p.color,
          eventCount: (d.events ?? []).filter((e: any) => e.phase === p.id).length,
        }));
        setTimelinePhases(phases);
        setTimelineEventCount((d.events ?? []).length);
      })
      .catch(() => {});
  }, []);

  if (loading) {
    return (
      <div style={{ padding: "2rem", textAlign: "center", color: "#64748b" }}>
        Loading case data...
      </div>
    );
  }

  if (error) {
    return (
      <div style={{ padding: "1rem", backgroundColor: "#fef2f2", border: "1px solid #fecaca", borderRadius: "6px", color: "#dc2626" }}>
        {error}
      </div>
    );
  }

  if (!caseData) {
    return (
      <div style={{ padding: "3rem", textAlign: "center" }}>
        <div style={{ fontSize: "1.1rem", fontWeight: 600, color: "#334155", marginBottom: "0.5rem" }}>
          No case data available
        </div>
        <div style={{ fontSize: "0.84rem", color: "#64748b", marginBottom: "1rem", lineHeight: 1.5 }}>
          Process documents through the Pipeline to populate the knowledge graph.
        </div>
        <Link
          to="/pipeline"
          style={{ fontSize: "0.84rem", color: "#2563eb", textDecoration: "none", fontWeight: 500 }}
        >
          Go to Pipeline Dashboard {"\u2192"}
        </Link>
      </div>
    );
  }

  // Strip any " - DocumentType" suffix from the case title (e.g., "Awad v. CFS - Complaint" → "Awad v. CFS")
  const rawTitle = caseData.case_title;
  const displayTitle = rawTitle.includes(" - ")
    ? rawTitle.substring(0, rawTitle.lastIndexOf(" - "))
    : rawTitle;

  const metaParts: string[] = [];
  if (caseData.court) metaParts.push(caseData.court);
  if (caseData.case_number) metaParts.push(`Case No. ${caseData.case_number}`);

  return (
    <div style={{ paddingTop: "2rem", paddingBottom: "4rem" }}>

      {/* 2A: Case Header */}
      <div style={{ marginBottom: "1.75rem" }}>
        <h1 style={{ fontSize: "1.55rem", fontWeight: 700, color: "#0f172a", letterSpacing: "-0.02em", margin: 0, marginBottom: "0.4rem" }}>
          {displayTitle}
        </h1>
        {metaParts.length > 0 && (
          <span style={{ fontSize: "0.84rem", color: "#64748b" }}>
            {metaParts.join(" \u00b7 ")}
          </span>
        )}
      </div>

      {/* 2B: Case Summary — rendered when available from API */}

      {/* 2B2: Compact Timeline Bar */}
      {timelinePhases.length > 0 && (
        <div style={{ marginBottom: "1.75rem" }}>
          <div style={{ fontSize: "0.72rem", fontWeight: 700, color: "#94a3b8", textTransform: "uppercase", letterSpacing: "0.05em", marginBottom: "0.65rem" }}>
            Case Timeline
          </div>
          <div style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap" }}>
            {timelinePhases.map((phase) => (
              <Link
                key={phase.id}
                to={`/timeline#phase-${phase.id}`}
                style={{
                  flex: 1, minWidth: "140px", padding: "0.75rem 1rem",
                  backgroundColor: phase.color + "0d", border: `1px solid ${phase.color}30`,
                  borderRadius: "8px", textDecoration: "none",
                  borderLeft: `3px solid ${phase.color}`,
                  transition: "all 0.15s ease",
                }}
                onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = phase.color + "18"; }}
                onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = phase.color + "0d"; }}
              >
                <div style={{ fontSize: "0.82rem", fontWeight: 600, color: "#0f172a", marginBottom: "0.15rem" }}>
                  {phase.label}
                </div>
                <div style={{ fontSize: "0.7rem", color: "#64748b" }}>
                  {phase.date_range} {"\u00b7"} {phase.eventCount} event{phase.eventCount !== 1 ? "s" : ""}
                </div>
              </Link>
            ))}
          </div>
          <Link
            to="/timeline"
            style={{ display: "inline-block", marginTop: "0.6rem", fontSize: "0.84rem", color: "#2563eb", textDecoration: "none", fontWeight: 500 }}
          >
            View Full Timeline {"\u2192"}
          </Link>
        </div>
      )}

      {/* 2C: Causes of Action */}
      {caseData.legal_count_details.length > 0 && (
        <section style={{ marginBottom: "1.75rem" }}>
          <div style={{ fontSize: "0.95rem", fontWeight: 700, color: "#0f172a", marginBottom: "0.85rem", letterSpacing: "-0.01em" }}>
            Causes of Action
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(2, 1fr)", gap: "0.65rem" }}>
            {caseData.legal_count_details.map((lc) => (
              <Link
                key={lc.id}
                to={`/allegations?count=${encodeURIComponent(lc.id)}`}
                style={{
                  backgroundColor: "#ffffff", border: "1px solid #e2e8f0", borderRadius: "10px",
                  padding: "1.15rem 1.25rem", textDecoration: "none", display: "flex",
                  justifyContent: "space-between", alignItems: "flex-start",
                  transition: "all 0.15s ease", cursor: "pointer",
                }}
                onMouseEnter={(e) => { e.currentTarget.style.borderColor = "#3b82f6"; e.currentTarget.style.boxShadow = "0 2px 8px rgba(37,99,235,0.08)"; }}
                onMouseLeave={(e) => { e.currentTarget.style.borderColor = "#e2e8f0"; e.currentTarget.style.boxShadow = "none"; }}
              >
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ fontSize: "0.68rem", fontWeight: 700, color: "#2563eb", textTransform: "uppercase", letterSpacing: "0.05em", marginBottom: "0.2rem" }}>
                    {toCountLabel(lc.count_number)}
                  </div>
                  <div style={{ fontSize: "0.92rem", fontWeight: 600, color: "#0f172a", marginBottom: "0.3rem", lineHeight: 1.3 }}>
                    {lc.name}
                  </div>
                  <div style={{ fontSize: "0.8rem", color: "#475569", lineHeight: 1.45, fontFamily: "'Georgia', serif" }}>
                    {COUNT_DESCRIPTIONS[lc.id] || ""}
                  </div>
                </div>
                {/* TODO: Replace hardcoded "Supported" with a status field from LegalCount nodes once available */}
                <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flexShrink: 0, marginLeft: "1rem" }}>
                  <span style={{
                    padding: "0.22rem 0.55rem", borderRadius: "5px", fontSize: "0.68rem",
                    fontWeight: 700, textTransform: "uppercase", letterSpacing: "0.03em",
                    backgroundColor: "#ecfdf5", color: "#047857", whiteSpace: "nowrap",
                  }}>
                    Supported
                  </span>
                  <span style={{ color: "#cbd5e1", fontSize: "0.9rem" }}>{"\u2192"}</span>
                </div>
              </Link>
            ))}
          </div>
        </section>
      )}

      {/* 2D: Explore the Case */}
      <section style={{ marginBottom: "1.75rem" }}>
        <div style={{ fontSize: "0.95rem", fontWeight: 700, color: "#0f172a", marginBottom: "0.85rem", letterSpacing: "-0.01em" }}>
          Explore the Case
        </div>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: "0.65rem" }}>
          {[
            { name: "Evidence Explorer", desc: "Browse proof chains with verbatim quotes and page numbers", stat: `${caseData.evidence_total} sworn statements and exhibits`, path: "/explorer" },
            { name: "Documents", desc: "Briefs, motions, discovery responses, and court orders", stat: `${caseData.documents_total} court filings`, path: "/documents" },
            { name: "Contradictions", desc: "Impeachment evidence from sworn testimony", stat: "Where Phillips contradicted himself", path: "/contradictions" },
            { name: "Damages", desc: "Financial and reputational damages with evidence links", stat: `${formatCurrency(caseData.damages_total)} in documented harm`, path: "/damages", hasInfo: true },
            { name: "Graph", desc: "Interactive graph from legal counts through evidence", stat: "Visual proof chains", path: "/graph" },
          ].map((card) => (
            <Link
              key={card.path}
              to={card.path}
              style={{
                backgroundColor: "#ffffff", border: "1px solid #e2e8f0", borderRadius: "10px",
                padding: "1.15rem 1.25rem", textDecoration: "none", color: "inherit",
                display: "flex", flexDirection: "column", justifyContent: "space-between",
                minHeight: "100px", transition: "all 0.15s ease", cursor: "pointer",
              }}
              onMouseEnter={(e) => { e.currentTarget.style.borderColor = "#3b82f6"; e.currentTarget.style.boxShadow = "0 2px 8px rgba(37,99,235,0.08)"; }}
              onMouseLeave={(e) => { e.currentTarget.style.borderColor = "#e2e8f0"; e.currentTarget.style.boxShadow = "none"; }}
            >
              <div>
                <div style={{ fontSize: "0.9rem", fontWeight: 600, color: "#0f172a", marginBottom: "0.25rem" }}>
                  {card.name}
                </div>
                <div style={{ fontSize: "0.78rem", color: "#475569", lineHeight: 1.4, fontFamily: "'Georgia', serif" }}>
                  {card.desc}
                </div>
              </div>
              <div style={{ marginTop: "0.6rem", fontSize: "0.72rem", fontWeight: 600, color: "#2563eb", display: "flex", alignItems: "center", gap: "0.4rem" }}>
                {card.stat}
                {card.hasInfo && (
                  <button
                    onClick={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      setShowDamages(true);
                      if (harms.length === 0) {
                        setHarmsLoading(true);
                        getHarms()
                          .then((res) => setHarms(res.harms))
                          .catch(() => {})
                          .finally(() => setHarmsLoading(false));
                      }
                    }}
                    style={{
                      background: "none", border: "none", padding: 0,
                      fontSize: "0.84rem", color: "#2563eb", cursor: "pointer",
                      fontWeight: 500, fontFamily: "inherit",
                    }}
                  >
                    View Breakdown {"\u2192"}
                  </button>
                )}
              </div>
            </Link>
          ))}
        </div>
      </section>

      {/* Damages Breakdown Popup */}
      {showDamages && (
        <div
          onClick={() => setShowDamages(false)}
          style={{
            position: "fixed", inset: 0, backgroundColor: "rgba(0,0,0,0.4)",
            display: "flex", alignItems: "center", justifyContent: "center", zIndex: 1000,
          }}
        >
          <div
            onClick={(e) => e.stopPropagation()}
            style={{
              backgroundColor: "#ffffff", borderRadius: "12px", padding: "1.75rem",
              maxWidth: "600px", width: "90%", maxHeight: "80vh", overflowY: "auto",
              boxShadow: "0 20px 60px rgba(0,0,0,0.15)",
            }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "1.25rem" }}>
              <h2 style={{ margin: 0, fontSize: "1.1rem", fontWeight: 700, color: "#0f172a" }}>Damages Breakdown</h2>
              <button
                onClick={() => setShowDamages(false)}
                style={{
                  background: "none", border: "none", fontSize: "1.25rem",
                  color: "#94a3b8", cursor: "pointer", padding: "0.25rem",
                }}
              >
                {"\u2715"}
              </button>
            </div>

            {harmsLoading ? (
              <div style={{ textAlign: "center", padding: "2rem", color: "#64748b" }}>Loading...</div>
            ) : (
              <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "0.84rem" }}>
                <thead>
                  <tr style={{ borderBottom: "2px solid #e2e8f0" }}>
                    <th style={{ textAlign: "left", padding: "0.5rem 0.5rem 0.5rem 0", color: "#64748b", fontWeight: 600 }}>Category</th>
                    <th style={{ textAlign: "left", padding: "0.5rem", color: "#64748b", fontWeight: 600 }}>Description</th>
                    <th style={{ textAlign: "right", padding: "0.5rem 0 0.5rem 0.5rem", color: "#64748b", fontWeight: 600 }}>Amount</th>
                  </tr>
                </thead>
                <tbody>
                  {harms.map((h) => (
                    <tr key={h.id} style={{ borderBottom: "1px solid #f1f5f9" }}>
                      <td style={{ padding: "0.5rem 0.5rem 0.5rem 0", color: "#334155", textTransform: "capitalize", whiteSpace: "nowrap" }}>
                        {h.category ?? "Other"}
                      </td>
                      <td style={{ padding: "0.5rem", color: "#334155" }}>{h.title}</td>
                      <td style={{ padding: "0.5rem 0 0.5rem 0.5rem", textAlign: "right", color: "#0f172a", fontWeight: 500, whiteSpace: "nowrap" }}>
                        {h.amount != null ? formatCurrency(h.amount) : "\u2014"}
                      </td>
                    </tr>
                  ))}
                </tbody>
                <tfoot>
                  <tr style={{ borderTop: "2px solid #e2e8f0" }}>
                    <td colSpan={2} style={{ padding: "0.65rem 0.5rem 0.5rem 0", fontWeight: 700, color: "#0f172a" }}>Total</td>
                    <td style={{ padding: "0.65rem 0 0.5rem 0.5rem", textAlign: "right", fontWeight: 700, color: "#0f172a" }}>
                      {formatCurrency(caseData.damages_total)}
                    </td>
                  </tr>
                </tfoot>
              </table>
            )}
          </div>
        </div>
      )}

    </div>
  );
};

export default Home;
