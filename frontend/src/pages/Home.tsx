import React from "react";
import { Link } from "react-router-dom";
import { useCase } from "../context/CaseContext";

// ─── Static data ─────────────────────────────────────────────────────────────

const COUNT_LABELS: Record<number, string> = {
  0: "COUNT I", 1: "COUNT II", 2: "COUNT III", 3: "COUNT IV",
};

const COUNT_DESCRIPTIONS: Record<string, string> = {
  "count-breach-of-fiduciary-duty":
    "CFS and Phillips violated duties of loyalty and care owed to Marie as estate beneficiary.",
  "count-fraud":
    "Defendants made material misrepresentations to the court about Marie's cooperation and estate assets.",
  "count-declaratory-relief":
    "Request for court determination regarding the rights and duties of parties under the estate.",
  "count-abuse-of-process":
    "Phillips used court proceedings for improper purposes including sanctions motions and character attacks.",
};

const EXPLORE_CARDS = [
  { name: "Evidence Explorer", desc: "Browse proof chains with verbatim quotes and page numbers", stat: "102 evidence items", path: "/explorer" },
  { name: "Graph", desc: "Visual proof chain from legal counts down through evidence", stat: "18 allegation hierarchies", path: "/graph" },
  { name: "Contradictions", desc: "Where Phillips contradicted his own prior statements under oath", stat: "5 impeachment pairs", path: "/contradictions" },
  { name: "Court Documents", desc: "Briefs, motions, discovery responses, and court orders", stat: "17 filings", path: "/documents" },
  { name: "Damages", desc: "Documented financial and reputational harms with evidence links", stat: "12 harms \u00b7 $46,258.61", path: "/damages" },
  { name: "Case Analysis", desc: "Gap analysis, allegation strength review, and evidence coverage", stat: "18 allegations analyzed", path: "/analysis" },
];

// ─── Component ───────────────────────────────────────────────────────────────

const Home: React.FC = () => {
  const { caseData, loading, error } = useCase();
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
      <div style={{ padding: "2rem", textAlign: "center", color: "#64748b" }}>
        No case data available
      </div>
    );
  }

  const { case: caseInfo, stats } = caseData;

  const metaParts: string[] = [];
  if (caseInfo.court) metaParts.push(caseInfo.court);
  if (caseInfo.case_number) metaParts.push(`Case No. ${caseInfo.case_number}`);
  if (caseInfo.filing_date) metaParts.push(`Filed ${caseInfo.filing_date}`);

  return (
    <div style={{ paddingTop: "2rem", paddingBottom: "4rem" }}>

      {/* 2A: Case Header */}
      <div style={{ marginBottom: "1.75rem" }}>
        <h1 style={{ fontSize: "1.55rem", fontWeight: 700, color: "#0f172a", letterSpacing: "-0.02em", margin: 0, marginBottom: "0.4rem" }}>
          {caseInfo.title}
        </h1>
        <div style={{ display: "flex", alignItems: "center", gap: "0.6rem", flexWrap: "wrap" }}>
          {metaParts.length > 0 && (
            <span style={{ fontSize: "0.84rem", color: "#64748b" }}>
              {metaParts.join(" \u00b7 ")}
            </span>
          )}
          {caseInfo.status && (
            <span style={{
              display: "inline-block", padding: "0.2rem 0.6rem", borderRadius: "5px",
              fontSize: "0.7rem", fontWeight: 700, textTransform: "uppercase",
              letterSpacing: "0.03em", backgroundColor: "#ecfdf5", color: "#047857",
            }}>
              {caseInfo.status}
            </span>
          )}
        </div>
      </div>

      {/* 2B: Case Summary */}
      {caseInfo.summary && (
        <div style={{
          backgroundColor: "#ffffff", border: "1px solid #e2e8f0", borderRadius: "10px",
          padding: "1.5rem 1.75rem", marginBottom: "1.75rem",
        }}>
          <div style={{ fontSize: "0.72rem", fontWeight: 700, color: "#94a3b8", textTransform: "uppercase", letterSpacing: "0.05em", marginBottom: "0.65rem" }}>
            Case Summary
          </div>
          <div style={{ fontSize: "0.9rem", color: "#334155", lineHeight: 1.65 }}>
            {caseInfo.summary}
          </div>
        </div>
      )}

      {/* 2C: Causes of Action */}
      {stats.legal_count_details.length > 0 && (
        <section style={{ marginBottom: "1.75rem" }}>
          <div style={{ fontSize: "0.95rem", fontWeight: 700, color: "#0f172a", marginBottom: "0.85rem", letterSpacing: "-0.01em" }}>
            Causes of Action
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(2, 1fr)", gap: "0.65rem" }}>
            {stats.legal_count_details.map((lc, idx) => (
              <Link
                key={lc.id}
                to={`/explorer?count=${encodeURIComponent(lc.id)}`}
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
                    {COUNT_LABELS[idx] || `COUNT ${idx + 1}`}
                  </div>
                  <div style={{ fontSize: "0.92rem", fontWeight: 600, color: "#0f172a", marginBottom: "0.3rem", lineHeight: 1.3 }}>
                    {lc.name}
                  </div>
                  <div style={{ fontSize: "0.8rem", color: "#64748b", lineHeight: 1.45 }}>
                    {COUNT_DESCRIPTIONS[lc.id] || ""}
                  </div>
                </div>
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

      {/* 2D: Explore the Evidence */}
      <section style={{ marginBottom: "1.75rem" }}>
        <div style={{ fontSize: "0.95rem", fontWeight: 700, color: "#0f172a", marginBottom: "0.85rem", letterSpacing: "-0.01em" }}>
          Explore the Evidence
        </div>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: "0.65rem" }}>
          {EXPLORE_CARDS.map((card) => (
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
                <div style={{ fontSize: "0.78rem", color: "#64748b", lineHeight: 1.4 }}>
                  {card.desc}
                </div>
              </div>
              <div style={{ marginTop: "0.6rem", fontSize: "0.72rem", fontWeight: 600, color: "#2563eb" }}>
                {card.stat}
              </div>
            </Link>
          ))}
        </div>
      </section>

    </div>
  );
};

export default Home;
