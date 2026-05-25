import React from "react";
import { Link } from "react-router-dom";
import { useCase } from "../context/CaseContext";

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

      {/* 2B: Case Summary Stats */}
      <div style={{ display: "flex", gap: "2rem", marginBottom: "1.75rem", flexWrap: "wrap" }}>
        {caseData.plaintiffs.length > 0 && (
          <div>
            <div style={{ fontSize: "0.72rem", fontWeight: 700, color: "#94a3b8", textTransform: "uppercase", letterSpacing: "0.05em", marginBottom: "0.3rem" }}>
              Plaintiffs
            </div>
            <div style={{ fontSize: "0.9rem", color: "#0f172a" }}>{caseData.plaintiffs.join(", ")}</div>
          </div>
        )}
        {caseData.defendants.length > 0 && (
          <div>
            <div style={{ fontSize: "0.72rem", fontWeight: 700, color: "#94a3b8", textTransform: "uppercase", letterSpacing: "0.05em", marginBottom: "0.3rem" }}>
              Defendants
            </div>
            <div style={{ fontSize: "0.9rem", color: "#0f172a" }}>{caseData.defendants.join(", ")}</div>
          </div>
        )}
        {caseData.allegations_total > 0 && (
          <div>
            <div style={{ fontSize: "0.72rem", fontWeight: 700, color: "#94a3b8", textTransform: "uppercase", letterSpacing: "0.05em", marginBottom: "0.3rem" }}>
              Allegations
            </div>
            <div style={{ fontSize: "0.9rem", color: "#0f172a" }}>{caseData.allegations_total}</div>
          </div>
        )}
        {caseData.legal_counts > 0 && (
          <div>
            <div style={{ fontSize: "0.72rem", fontWeight: 700, color: "#94a3b8", textTransform: "uppercase", letterSpacing: "0.05em", marginBottom: "0.3rem" }}>
              Legal Counts
            </div>
            <div style={{ fontSize: "0.9rem", color: "#0f172a" }}>{caseData.legal_counts}</div>
          </div>
        )}
      </div>

      {/* 2C: Causes of Action \u2014 temporary placeholder.
          The old 2x2 CountCard grid and the "Explore the Case" nav cards were
          removed in Phase 2B. The full-width Count tables arrive in Phase 2C-E.
          This block also doubles as a smoke test for Phase 2A's tokens: if the
          heading/body render in the wrong color (e.g. plain black), the
          tokens.css import is broken \u2014 var(--text-secondary)/var(--text-muted)
          should resolve to the palette defined in styles/tokens.css. */}
      <div style={{ padding: '32px 0' }}>
        <h2 style={{
          fontSize: '14px',
          fontWeight: 600,
          textTransform: 'uppercase' as const,
          letterSpacing: '0.05em',
          color: 'var(--text-secondary)',
          marginBottom: '16px'
        }}>
          Causes of Action
        </h2>
        <p style={{
          fontSize: '14px',
          color: 'var(--text-muted)'
        }}>
          Count cards are being rebuilt \u2014 coming in the next update.
        </p>
      </div>

    </div>
  );
};

export default Home;
