import React from "react";
import { Link } from "react-router-dom";
import { useCase } from "../context/CaseContext";
import { CaseResponse, PartyDto } from "../services/case";

// Stat card configuration
type StatCard = {
  label: string;
  route: string;
  description: string;
};

const STAT_CARDS: StatCard[] = [
  { label: "Allegations", route: "/allegations", description: "From original complaint" },
  { label: "Evidence", route: "/evidence", description: "Supporting exhibits & testimony" },
  { label: "Documents", route: "/documents", description: "Court filings & records" },
  { label: "Damages", route: "/damages", description: "Total claimed harm" },
];

// Build dynamic summary from case data
const buildDynamicSummary = (caseData: CaseResponse): string => {
  const { parties, stats } = caseData;
  const plaintiffNames = parties.plaintiffs.map((p) => p.name).join(", ");
  const defendantNames = parties.defendants.map((p) => p.name).join(" and ");
  const causesOfAction = stats.legal_count_details.map((lc) => lc.name).join(", ");
  const damages = stats.damages_total.toLocaleString("en-US", {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: 0,
  });

  return `${plaintiffNames} brings ${stats.legal_counts} cause${stats.legal_counts !== 1 ? "s" : ""} of action against ${defendantNames}: ${causesOfAction}. The complaint contains ${stats.allegations_total} allegations, of which ${stats.allegations_proven} are supported by evidence, with claimed damages of ${damages}.`;
};

// Styles
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

const titleStyle: React.CSSProperties = {
  fontSize: "1.75rem",
  fontWeight: 700,
  color: "#1f2937",
  margin: 0,
  marginBottom: "0.5rem",
};

const metaLineStyle: React.CSSProperties = {
  color: "#6b7280",
  fontSize: "0.95rem",
  marginBottom: "0.75rem",
};

const statusBadgeStyle: React.CSSProperties = {
  display: "inline-block",
  padding: "0.25rem 0.75rem",
  backgroundColor: "#dbeafe",
  color: "#2563eb",
  borderRadius: "9999px",
  fontSize: "0.875rem",
  fontWeight: 500,
};

const sectionTitleStyle: React.CSSProperties = {
  fontSize: "0.75rem",
  fontWeight: 600,
  color: "#6b7280",
  textTransform: "uppercase",
  letterSpacing: "0.05em",
  marginBottom: "0.75rem",
};

const twoColumnStyle: React.CSSProperties = {
  display: "grid",
  gridTemplateColumns: "1fr 1.5fr",
  gap: "1rem",
};

const partyGroupStyle: React.CSSProperties = {
  marginBottom: "1rem",
};

const partyLabelStyle: React.CSSProperties = {
  fontSize: "0.8rem",
  fontWeight: 600,
  color: "#374151",
  marginBottom: "0.25rem",
};

const partyItemStyle: React.CSSProperties = {
  color: "#1f2937",
  fontSize: "0.95rem",
  padding: "0.125rem 0",
};

const orgIndicatorStyle: React.CSSProperties = {
  fontSize: "0.75rem",
  color: "#6b7280",
  marginLeft: "0.5rem",
};

const statsGridStyle: React.CSSProperties = {
  display: "grid",
  gridTemplateColumns: "repeat(4, 1fr)",
  gap: "0.75rem",
  marginBottom: "1rem",
};

const statCardStyle: React.CSSProperties = {
  backgroundColor: "#f9fafb",
  border: "1px solid #e5e7eb",
  borderRadius: "8px",
  padding: "1rem",
  textAlign: "center",
  textDecoration: "none",
  transition: "box-shadow 0.2s ease, border-color 0.2s ease",
};

const statValueStyle: React.CSSProperties = {
  fontSize: "1.5rem",
  fontWeight: 700,
  color: "#2563eb",
  marginBottom: "0.25rem",
};

const statLabelStyle: React.CSSProperties = {
  fontSize: "0.75rem",
  fontWeight: 600,
  color: "#6b7280",
  textTransform: "uppercase",
};

const statDescriptionStyle: React.CSSProperties = {
  fontSize: "0.7rem",
  color: "#9ca3af",
  marginTop: "0.25rem",
};

const provenContainerStyle: React.CSSProperties = {
  marginTop: "1rem",
  padding: "0.75rem 1rem",
  backgroundColor: "#f0fdf4",
  borderRadius: "6px",
  border: "1px solid #bbf7d0",
};

const provenTextStyle: React.CSSProperties = {
  color: "#059669",
  fontWeight: 600,
  fontSize: "0.95rem",
};

// Party list component
const PartyList: React.FC<{ label: string; parties: PartyDto[] }> = ({
  label,
  parties,
}) => {
  if (parties.length === 0) return null;

  return (
    <div style={partyGroupStyle}>
      <div style={partyLabelStyle}>{label}:</div>
      {parties.map((party) => (
        <div key={party.id} style={partyItemStyle}>
          {"\u2022"} {party.name}
          {party.type === "organization" && (
            <span style={orgIndicatorStyle}>(Org)</span>
          )}
        </div>
      ))}
    </div>
  );
};

const Home: React.FC = () => {
  const { caseData, loading, error } = useCase();

  if (loading) {
    return (
      <div style={{ padding: "2rem", textAlign: "center", color: "#6b7280" }}>
        Loading case data...
      </div>
    );
  }

  if (error) {
    return (
      <div
        style={{
          padding: "1rem",
          backgroundColor: "#fef2f2",
          border: "1px solid #fecaca",
          borderRadius: "6px",
          color: "#dc2626",
        }}
      >
        {error}
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

  const { case: caseInfo, parties, stats } = caseData;

  // Build metadata line from optional fields
  const metaParts: string[] = [];
  if (caseInfo.court) metaParts.push(caseInfo.court);
  if (caseInfo.case_number) metaParts.push(`Case No. ${caseInfo.case_number}`);
  if (caseInfo.filing_date) metaParts.push(`Filed ${caseInfo.filing_date}`);
  const metaLine = metaParts.join(" \u2022 ");

  // Calculate proven percentage
  const provenPercent =
    stats.allegations_total > 0
      ? Math.round((stats.allegations_proven / stats.allegations_total) * 100)
      : 0;

  // Get stat values for cards
  const statValues: Record<string, string> = {
    Allegations: String(stats.allegations_total),
    Evidence: String(stats.evidence_count),
    Documents: String(stats.document_count),
    Damages: stats.damages_total.toLocaleString("en-US", {
      style: "currency",
      currency: "USD",
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    }),
  };

  return (
    <div style={pageStyle}>
      {/* Case Header */}
      <div style={cardStyle}>
        <h1 style={titleStyle}>{caseInfo.title}</h1>
        {metaLine && <div style={metaLineStyle}>{metaLine}</div>}
        {caseInfo.status && <span style={statusBadgeStyle}>{caseInfo.status}</span>}
      </div>

      {/* Case Summary */}
      <div style={cardStyle}>
        <div style={sectionTitleStyle}>Case Summary</div>
        <p style={{ color: "#374151", lineHeight: 1.6, margin: 0 }}>
          {buildDynamicSummary(caseData)}
        </p>
      </div>

      {/* Parties and Statistics */}
      <div style={twoColumnStyle}>
        {/* Parties Column */}
        <div style={cardStyle}>
          <div style={sectionTitleStyle}>Parties</div>
          <PartyList label="Plaintiff" parties={parties.plaintiffs} />
          <PartyList label="Defendants" parties={parties.defendants} />
          <PartyList label="Other" parties={parties.other} />
        </div>

        {/* Statistics Column */}
        <div style={cardStyle}>
          <div style={sectionTitleStyle}>Key Statistics</div>
          <div style={statsGridStyle}>
            {STAT_CARDS.map((card) => (
              <Link
                key={card.label}
                to={card.route}
                style={statCardStyle}
                onMouseEnter={(e) => {
                  e.currentTarget.style.boxShadow = "0 4px 12px rgba(0,0,0,0.1)";
                  e.currentTarget.style.borderColor = "#2563eb";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.boxShadow = "none";
                  e.currentTarget.style.borderColor = "#e5e7eb";
                }}
              >
                <div
                  style={{
                    ...statValueStyle,
                    color: card.label === "Damages" ? "#059669" : "#2563eb",
                  }}
                >
                  {statValues[card.label]}
                </div>
                <div style={statLabelStyle}>{card.label}</div>
                <div style={statDescriptionStyle}>{card.description}</div>
              </Link>
            ))}
          </div>

          {/* Proven indicator */}
          <div style={provenContainerStyle}>
            <div style={provenTextStyle}>
              {stats.allegations_proven} of {stats.allegations_total} allegations
              PROVEN ({provenPercent}%)
            </div>
          </div>

          {/* Causes of Action */}
          {stats.legal_count_details.length > 0 && (
            <div style={{ marginTop: "1rem", fontSize: "0.875rem", color: "#374151" }}>
              <span style={{ fontWeight: 600 }}>Causes of Action: </span>
              {stats.legal_count_details.map((lc, i) => (
                <span key={lc.id}>
                  {lc.name}
                  {i < stats.legal_count_details.length - 1 ? ", " : ""}
                </span>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default Home;
