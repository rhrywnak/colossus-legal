import React, { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useCase } from "../context/CaseContext";
import { CaseSummaryResponse, getCaseSummary } from "../services/caseSummary";

// Quick-action card definitions
const QUICK_ACTIONS = [
  {
    label: "George's Claims vs. Reality",
    route: "/decomposition",
    subtitle: (s: CaseSummaryResponse) =>
      `${s.characterizations_total} characterizations exposed`,
  },
  {
    label: "Damages Breakdown",
    route: "/damages",
    subtitle: (s: CaseSummaryResponse) =>
      `${formatCurrency(s.damages_total)} proven`,
  },
  {
    label: "Evidence Library",
    route: "/evidence",
    subtitle: (s: CaseSummaryResponse) =>
      `${s.evidence_grounded} grounded exhibits`,
  },
  {
    label: "Contradictions",
    route: "/contradictions",
    subtitle: () => "Statements vs. admissions",
  },
  {
    label: "All Allegations",
    route: "/allegations",
    subtitle: (s: CaseSummaryResponse) =>
      `${s.allegations_proven} proven allegations`,
  },
];

function formatCurrency(amount: number): string {
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: 0,
    maximumFractionDigits: 0,
  }).format(amount);
}

// ─── Reusable styles ─────────────────────────────────────────────────────────

const pageStyle: React.CSSProperties = {
  backgroundColor: "#f9fafb",
  minHeight: "calc(100vh - 100px)",
  margin: "-1.5rem",
  padding: "1.5rem",
};

const cardStyle: React.CSSProperties = {
  backgroundColor: "#ffffff",
  border: "1px solid #e5e7eb",
  borderRadius: "8px",
  padding: "1.5rem",
  marginBottom: "1rem",
};

const sectionTitleStyle: React.CSSProperties = {
  fontSize: "0.75rem",
  fontWeight: 600,
  color: "#6b7280",
  textTransform: "uppercase",
  letterSpacing: "0.05em",
  marginBottom: "0.75rem",
};

// ─── Component ───────────────────────────────────────────────────────────────

const Home: React.FC = () => {
  const { caseData, loading, error } = useCase();
  const [summary, setSummary] = useState<CaseSummaryResponse | null>(null);
  const [summaryError, setSummaryError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    getCaseSummary()
      .then((data) => {
        if (active) setSummary(data);
      })
      .catch(() => {
        if (active) setSummaryError("Failed to load case summary");
      });
    return () => { active = false; };
  }, []);

  if (loading) {
    return (
      <div style={{ padding: "2rem", textAlign: "center", color: "#6b7280" }}>
        Loading case data...
      </div>
    );
  }

  if (error || summaryError) {
    return (
      <div style={{
        padding: "1rem", backgroundColor: "#fef2f2",
        border: "1px solid #fecaca", borderRadius: "6px", color: "#dc2626",
      }}>
        {error || summaryError}
      </div>
    );
  }

  if (!caseData) {
    return (
      <div style={{ padding: "2rem", textAlign: "center", color: "#6b7280" }}>
        No case data available
      </div>
    );
  }

  const { case: caseInfo, parties } = caseData;
  const metaParts: string[] = [];
  if (caseInfo.court) metaParts.push(caseInfo.court);
  if (caseInfo.case_number) metaParts.push(`Case No. ${caseInfo.case_number}`);
  if (caseInfo.filing_date) metaParts.push(`Filed ${caseInfo.filing_date}`);

  return (
    <div style={pageStyle}>
      {/* 1. Case Header */}
      <div style={cardStyle}>
        <h1 style={{ fontSize: "1.75rem", fontWeight: 700, color: "#1f2937", margin: 0, marginBottom: "0.5rem" }}>
          {caseInfo.title}
        </h1>
        {metaParts.length > 0 && (
          <div style={{ color: "#6b7280", fontSize: "0.95rem" }}>
            {metaParts.join(" \u2022 ")}
          </div>
        )}
      </div>

      {summary && (
        <>
          {/* 2. Case Strength Banner */}
          <div style={{
            padding: "1.25rem 1.5rem", backgroundColor: "#059669",
            borderRadius: "8px", marginBottom: "1rem", color: "#ffffff",
          }}>
            <div style={{ fontSize: "1.25rem", fontWeight: 700, lineHeight: 1.4 }}>
              {summary.allegations_proven} of {summary.allegations_total} Allegations
              Proven {"\u2022"} All {summary.legal_counts} Legal Counts
              Supported {"\u2022"} {formatCurrency(summary.damages_total)} in Damages
            </div>
          </div>

          {/* 3. Key Finding Callout */}
          <div style={{
            padding: "1.25rem 1.5rem", backgroundColor: "#fffbeb",
            border: "2px solid #92400e", borderRadius: "8px", marginBottom: "1rem",
          }}>
            <div style={{ color: "#78350f", fontSize: "1rem", lineHeight: 1.6, marginBottom: "0.75rem" }}>
              George Phillips characterized Marie's claims as "frivolous," "false,"
              and "scattershot" in his 2011 Court of Appeals brief. Every single
              allegation he attacked has been proven by sworn testimony and
              documentary evidence.
            </div>
            <div style={{ color: "#92400e", fontSize: "0.875rem", fontWeight: 600 }}>
              {summary.characterizations_total} characterizations
              challenged {"\u2022"} {summary.rebuttals_total} directly
              rebutted {"\u2022"} {summary.allegations_proven}/{summary.allegations_total} proven
            </div>
          </div>

          {/* 4. Quick Actions */}
          <div style={{ ...cardStyle, padding: "1rem 1.5rem" }}>
            <div style={sectionTitleStyle}>Explore the Evidence</div>
            <div style={{
              display: "grid", gridTemplateColumns: "repeat(5, 1fr)", gap: "0.75rem",
            }}>
              {QUICK_ACTIONS.map((action) => (
                <Link
                  key={action.route}
                  to={action.route}
                  style={{
                    padding: "1rem", backgroundColor: "#f9fafb",
                    border: "1px solid #e5e7eb", borderRadius: "8px",
                    textDecoration: "none", textAlign: "center",
                    transition: "box-shadow 0.2s ease, border-color 0.2s ease",
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.boxShadow = "0 4px 12px rgba(0,0,0,0.1)";
                    e.currentTarget.style.borderColor = "#2563eb";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.boxShadow = "none";
                    e.currentTarget.style.borderColor = "#e5e7eb";
                  }}
                >
                  <div style={{ fontWeight: 600, color: "#1f2937", fontSize: "0.875rem", marginBottom: "0.25rem" }}>
                    {action.label}
                  </div>
                  <div style={{ fontSize: "0.75rem", color: "#6b7280" }}>
                    {action.subtitle(summary)}
                  </div>
                </Link>
              ))}
            </div>
          </div>

          {/* 5. Legal Counts Summary */}
          <div style={{ ...cardStyle, padding: "1rem 1.5rem" }}>
            <div style={sectionTitleStyle}>Causes of Action</div>
            <div style={{
              display: "grid",
              gridTemplateColumns: `repeat(${summary.legal_count_details.length}, 1fr)`,
              gap: "0.75rem",
            }}>
              {summary.legal_count_details.map((lc) => (
                <Link
                  key={lc.id}
                  to={`/allegations?count=${encodeURIComponent(lc.name)}`}
                  style={{
                    padding: "1rem", backgroundColor: "#eff6ff",
                    border: "1px solid #bfdbfe", borderRadius: "8px",
                    textDecoration: "none", textAlign: "center",
                    transition: "box-shadow 0.2s ease, border-color 0.2s ease",
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.boxShadow = "0 4px 12px rgba(0,0,0,0.1)";
                    e.currentTarget.style.borderColor = "#2563eb";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.boxShadow = "none";
                    e.currentTarget.style.borderColor = "#bfdbfe";
                  }}
                >
                  <div style={{ fontWeight: 600, color: "#1e40af", fontSize: "0.9rem", marginBottom: "0.25rem" }}>
                    {lc.name}
                  </div>
                  <div style={{ fontSize: "0.75rem", color: "#6b7280" }}>
                    {lc.allegation_count} allegation{lc.allegation_count !== 1 ? "s" : ""}
                  </div>
                </Link>
              ))}
            </div>
          </div>
        </>
      )}

      {/* 6. Parties */}
      <div style={cardStyle}>
        <div style={sectionTitleStyle}>Parties</div>
        <div style={{ display: "flex", gap: "3rem" }}>
          <div>
            <div style={{ fontSize: "0.8rem", fontWeight: 600, color: "#374151", marginBottom: "0.25rem" }}>
              Plaintiff
            </div>
            {parties.plaintiffs.map((p) => (
              <div key={p.id} style={{ color: "#1f2937", fontSize: "0.95rem" }}>
                {p.name}
              </div>
            ))}
          </div>
          <div>
            <div style={{ fontSize: "0.8rem", fontWeight: 600, color: "#374151", marginBottom: "0.25rem" }}>
              Defendants
            </div>
            {parties.defendants.map((p) => (
              <div key={p.id} style={{ color: "#1f2937", fontSize: "0.95rem" }}>
                {p.name}
                {p.type === "organization" && (
                  <span style={{ fontSize: "0.75rem", color: "#6b7280", marginLeft: "0.5rem" }}>
                    (Org)
                  </span>
                )}
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
};

export default Home;
