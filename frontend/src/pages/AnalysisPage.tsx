import React, { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import {
  getAnalysis,
  AnalysisResponse,
  AllegationStrength,
  DocumentCoverage,
} from "../services/analysisApi";
import { getContradictions, ContradictionDto } from "../services/contradictions";
import ImpeachmentCard from "../components/ImpeachmentCard";

// ============================================================================
// Types
// ============================================================================

type TabId = "gap-analysis" | "contradictions" | "evidence-coverage";

// ============================================================================
// Color Configuration
// ============================================================================

const STRENGTH_COLORS: Record<string, { bg: string; text: string; bar: string }> = {
  strong: { bg: "#dcfce7", text: "#166534", bar: "#22c55e" },
  moderate: { bg: "#dbeafe", text: "#1e40af", bar: "#3b82f6" },
  weak: { bg: "#fef3c7", text: "#92400e", bar: "#f59e0b" },
  gap: { bg: "#fee2e2", text: "#991b1b", bar: "#ef4444" },
};

const DEFAULT_STRENGTH_COLOR = { bg: "#f3f4f6", text: "#374151", bar: "#9ca3af" };

function getStrengthStyle(category: string) {
  return STRENGTH_COLORS[category] || DEFAULT_STRENGTH_COLOR;
}

// ============================================================================
// Styles
// ============================================================================

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
  marginBottom: "1.5rem",
};

const sectionTitleStyle: React.CSSProperties = {
  fontSize: "0.75rem",
  fontWeight: 600,
  color: "#6b7280",
  textTransform: "uppercase",
  letterSpacing: "0.05em",
  marginBottom: "1rem",
};


const tabBarStyle: React.CSSProperties = {
  display: "flex",
  borderBottom: "1px solid #e5e7eb",
  marginBottom: "1.5rem",
  gap: "0.5rem",
};

const tabButtonBaseStyle: React.CSSProperties = {
  padding: "0.75rem 1rem",
  border: "none",
  borderBottom: "2px solid transparent",
  backgroundColor: "transparent",
  cursor: "pointer",
  fontSize: "0.9rem",
  fontWeight: 500,
  color: "#6b7280",
  display: "flex",
  alignItems: "center",
  gap: "0.5rem",
  transition: "color 0.2s ease, border-color 0.2s ease",
};

const tabButtonActiveStyle: React.CSSProperties = {
  ...tabButtonBaseStyle,
  color: "#2563eb",
  borderBottomColor: "#2563eb",
};

const tabBadgeStyle: React.CSSProperties = {
  padding: "0.125rem 0.5rem",
  borderRadius: "9999px",
  fontSize: "0.75rem",
  fontWeight: 600,
};

const tableStyle: React.CSSProperties = {
  width: "100%",
  borderCollapse: "collapse",
  fontSize: "0.9rem",
};

const thStyle: React.CSSProperties = {
  textAlign: "left",
  padding: "0.75rem 1rem",
  borderBottom: "2px solid #e5e7eb",
  color: "#374151",
  fontWeight: 600,
  fontSize: "0.8rem",
  textTransform: "uppercase",
};

const tdStyle: React.CSSProperties = {
  padding: "0.75rem 1rem",
  borderBottom: "1px solid #f3f4f6",
  color: "#374151",
  verticalAlign: "top",
};

const badgeStyle: React.CSSProperties = {
  display: "inline-block",
  padding: "0.25rem 0.5rem",
  borderRadius: "4px",
  fontSize: "0.75rem",
  fontWeight: 600,
  textTransform: "uppercase",
};

const progressBarContainerStyle: React.CSSProperties = {
  width: "100%",
  height: "8px",
  backgroundColor: "#e5e7eb",
  borderRadius: "4px",
  overflow: "hidden",
};

const linkStyle: React.CSSProperties = {
  color: "#2563eb",
  textDecoration: "none",
  fontWeight: 500,
};

const expandButtonStyle: React.CSSProperties = {
  backgroundColor: "transparent",
  border: "none",
  color: "#6b7280",
  cursor: "pointer",
  padding: "0.25rem 0.5rem",
  fontSize: "0.8rem",
};

// ============================================================================
// Components
// ============================================================================

// Tab button
const TabButton: React.FC<{
  id: TabId;
  label: string;
  isActive: boolean;
  onClick: () => void;
  badge?: number;
  badgeColor?: { bg: string; text: string };
}> = ({ label, isActive, onClick, badge, badgeColor }) => {
  const [hovered, setHovered] = useState(false);

  return (
    <button
      style={{
        ...(isActive ? tabButtonActiveStyle : tabButtonBaseStyle),
        color: isActive ? "#2563eb" : hovered ? "#374151" : "#6b7280",
        borderBottomColor: isActive ? "#2563eb" : hovered ? "#d1d5db" : "transparent",
      }}
      onClick={onClick}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      {label}
      {badge !== undefined && badge > 0 && (
        <span
          style={{
            ...tabBadgeStyle,
            backgroundColor: badgeColor?.bg || "#f3f4f6",
            color: badgeColor?.text || "#374151",
          }}
        >
          {badge}
        </span>
      )}
    </button>
  );
};

// Strength progress bar
const StrengthBar: React.FC<{ percent: number; category: string }> = ({
  percent,
  category,
}) => {
  const colors = getStrengthStyle(category);
  return (
    <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
      <div style={progressBarContainerStyle}>
        <div
          style={{
            width: `${percent}%`,
            height: "100%",
            backgroundColor: colors.bar,
            borderRadius: "4px",
            transition: "width 0.3s ease",
          }}
        />
      </div>
      <span style={{ fontSize: "0.8rem", color: "#6b7280", minWidth: "3rem" }}>
        {percent}%
      </span>
    </div>
  );
};

// Strength badge
const StrengthBadge: React.FC<{ category: string }> = ({ category }) => {
  const colors = getStrengthStyle(category);
  return (
    <span
      style={{
        ...badgeStyle,
        backgroundColor: colors.bg,
        color: colors.text,
      }}
    >
      {category}
    </span>
  );
};

// Expandable allegation row
const AllegationRow: React.FC<{ allegation: AllegationStrength }> = ({
  allegation,
}) => {
  const [expanded, setExpanded] = useState(false);

  return (
    <>
      <tr>
        <td style={tdStyle}>
          <Link to="/allegations" style={linkStyle}>
            {allegation.id}
          </Link>
        </td>
        <td style={{ ...tdStyle, maxWidth: "300px" }}>
          <div
            style={{
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: expanded ? "normal" : "nowrap",
            }}
          >
            {allegation.allegation || "No description"}
          </div>
          {allegation.paragraph && (
            <div style={{ fontSize: "0.75rem", color: "#9ca3af" }}>
              {allegation.paragraph}
            </div>
          )}
        </td>
        <td style={tdStyle}>
          <StrengthBar
            percent={allegation.strength_percent}
            category={allegation.strength_category}
          />
        </td>
        <td style={{ ...tdStyle, textAlign: "center" }}>
          {allegation.supporting_evidence_count}
        </td>
        <td style={tdStyle}>
          <StrengthBadge category={allegation.strength_category} />
        </td>
        <td style={tdStyle}>
          <button
            style={expandButtonStyle}
            onClick={() => setExpanded(!expanded)}
          >
            {expanded ? "\u25B2 Less" : "\u25BC More"}
          </button>
        </td>
      </tr>
      {expanded && (
        <tr>
          <td colSpan={6} style={{ ...tdStyle, backgroundColor: "#f9fafb" }}>
            <div style={{ padding: "0.5rem" }}>
              {allegation.supporting_evidence &&
                allegation.supporting_evidence.length > 0 && (
                  <div style={{ marginBottom: "0.5rem" }}>
                    <strong style={{ fontSize: "0.8rem", color: "#374151" }}>
                      Supporting Evidence:
                    </strong>
                    <ul
                      style={{
                        margin: "0.25rem 0 0 1rem",
                        padding: 0,
                        fontSize: "0.85rem",
                        color: "#6b7280",
                      }}
                    >
                      {allegation.supporting_evidence.map((e, i) => (
                        <li key={i}>{e}</li>
                      ))}
                    </ul>
                  </div>
                )}
              {allegation.gap_notes && (
                <div
                  style={{
                    padding: "0.5rem",
                    backgroundColor:
                      allegation.strength_category === "gap"
                        ? "#fee2e2"
                        : "#fef3c7",
                    borderRadius: "4px",
                    fontSize: "0.85rem",
                    color:
                      allegation.strength_category === "gap"
                        ? "#991b1b"
                        : "#92400e",
                  }}
                >
                  <strong>Note:</strong> {allegation.gap_notes}
                </div>
              )}
            </div>
          </td>
        </tr>
      )}
    </>
  );
};


// Document coverage row
const DocumentCoverageRow: React.FC<{ doc: DocumentCoverage }> = ({ doc }) => {
  const linkedPercent =
    doc.evidence_count > 0
      ? Math.round((doc.linked_count / doc.evidence_count) * 100)
      : 0;

  return (
    <tr>
      <td style={tdStyle}>
        <Link to={`/documents/${doc.document_id}`} style={linkStyle}>
          {doc.document_title || doc.document_id}
        </Link>
      </td>
      <td style={{ ...tdStyle, textAlign: "center" }}>{doc.evidence_count}</td>
      <td style={{ ...tdStyle, textAlign: "center" }}>{doc.linked_count}</td>
      <td style={tdStyle}>
        <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
          <div style={{ ...progressBarContainerStyle, width: "100px" }}>
            <div
              style={{
                width: `${linkedPercent}%`,
                height: "100%",
                backgroundColor:
                  linkedPercent >= 80
                    ? "#22c55e"
                    : linkedPercent >= 50
                    ? "#3b82f6"
                    : "#f59e0b",
                borderRadius: "4px",
              }}
            />
          </div>
          <span style={{ fontSize: "0.8rem", color: "#6b7280" }}>
            {linkedPercent}%
          </span>
        </div>
      </td>
    </tr>
  );
};

// ============================================================================
// Tab Content Components
// ============================================================================

const GapAnalysisContent: React.FC<{
  gap_analysis: AnalysisResponse["gap_analysis"];
}> = ({ gap_analysis }) => (
  <div style={cardStyle}>
    <div
      style={{
        marginBottom: "1rem",
        padding: "0.75rem 1rem",
        backgroundColor: "#f3f4f6",
        borderRadius: "6px",
        display: "flex",
        gap: "1.5rem",
        fontSize: "0.85rem",
      }}
    >
      <span>
        <strong style={{ color: "#22c55e" }}>{gap_analysis.strong_evidence}</strong>{" "}
        Strong
      </span>
      <span>
        <strong style={{ color: "#3b82f6" }}>{gap_analysis.moderate_evidence}</strong>{" "}
        Moderate
      </span>
      <span>
        <strong style={{ color: "#f59e0b" }}>{gap_analysis.weak_evidence}</strong>{" "}
        Weak
      </span>
      <span>
        <strong style={{ color: "#ef4444" }}>{gap_analysis.gaps}</strong> Gaps
      </span>
    </div>

    {gap_analysis.allegations.length === 0 ? (
      <div style={{ color: "#6b7280", padding: "1rem" }}>
        No allegations found.
      </div>
    ) : (
      <div style={{ overflowX: "auto" }}>
        <table style={tableStyle}>
          <thead>
            <tr>
              <th style={{ ...thStyle, width: "100px" }}>ID</th>
              <th style={thStyle}>Allegation</th>
              <th style={{ ...thStyle, width: "200px" }}>Strength</th>
              <th style={{ ...thStyle, width: "80px", textAlign: "center" }}>
                Evidence
              </th>
              <th style={{ ...thStyle, width: "100px" }}>Status</th>
              <th style={{ ...thStyle, width: "80px" }}></th>
            </tr>
          </thead>
          <tbody>
            {gap_analysis.allegations.map((allegation) => (
              <AllegationRow key={allegation.id} allegation={allegation} />
            ))}
          </tbody>
        </table>
      </div>
    )}
  </div>
);

const ImpeachmentContent: React.FC<{
  total: number;
  fullContradictions: ContradictionDto[];
}> = ({ total, fullContradictions }) => (
  <div style={cardStyle}>
    {fullContradictions.length === 0 ? (
      <div style={{ padding: "2rem", textAlign: "center", color: "#059669", backgroundColor: "#dcfce7", borderRadius: "6px" }}>
        No impeachment evidence found.
      </div>
    ) : (
      <>
        <div style={{ color: "#6b7280", fontSize: "0.9rem", marginBottom: "1rem" }}>
          Found <strong style={{ color: "#f59e0b" }}>{total}</strong> instances of impeachment evidence
        </div>
        {fullContradictions.map((c, i) => (
          <ImpeachmentCard key={i} contradiction={c} />
        ))}
      </>
    )}
  </div>
);

const EvidenceCoverageContent: React.FC<{
  evidence_coverage: AnalysisResponse["evidence_coverage"];
}> = ({ evidence_coverage }) => (
  <div style={cardStyle}>
    <div
      style={{
        marginBottom: "1rem",
        padding: "0.75rem 1rem",
        backgroundColor: "#f3f4f6",
        borderRadius: "6px",
        display: "flex",
        gap: "1.5rem",
        fontSize: "0.85rem",
      }}
    >
      <span>
        <strong>{evidence_coverage.total_evidence_nodes}</strong> Total Evidence
      </span>
      <span>
        <strong style={{ color: "#22c55e" }}>
          {evidence_coverage.linked_to_allegations}
        </strong>{" "}
        Linked
      </span>
      <span>
        <strong style={{ color: "#f59e0b" }}>{evidence_coverage.unlinked}</strong>{" "}
        Unlinked
      </span>
    </div>

    {evidence_coverage.by_document.length === 0 ? (
      <div style={{ color: "#6b7280", padding: "1rem" }}>
        No documents found.
      </div>
    ) : (
      <table style={tableStyle}>
        <thead>
          <tr>
            <th style={thStyle}>Document</th>
            <th style={{ ...thStyle, textAlign: "center", width: "80px" }}>
              Evidence
            </th>
            <th style={{ ...thStyle, textAlign: "center", width: "80px" }}>
              Linked
            </th>
            <th style={{ ...thStyle, width: "150px" }}>Coverage</th>
          </tr>
        </thead>
        <tbody>
          {evidence_coverage.by_document.map((doc) => (
            <DocumentCoverageRow key={doc.document_id} doc={doc} />
          ))}
        </tbody>
      </table>
    )}
  </div>
);

// ============================================================================
// Main Page Component
// ============================================================================

const AnalysisPage: React.FC = () => {
  const [data, setData] = useState<AnalysisResponse | null>(null);
  const [fullContradictions, setFullContradictions] = useState<ContradictionDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<TabId>("gap-analysis");

  // Handle URL hash routing on mount
  useEffect(() => {
    const hash = window.location.hash.replace("#", "");
    if (
      hash === "gap-analysis" ||
      hash === "contradictions" ||
      hash === "evidence-coverage"
    ) {
      setActiveTab(hash as TabId);
    }
  }, []);

  // Update URL hash when tab changes
  const handleTabChange = (tab: TabId) => {
    setActiveTab(tab);
    window.history.replaceState(null, "", `#${tab}`);
  };

  // Fetch data
  useEffect(() => {
    let active = true;

    const fetchData = async () => {
      try {
        const [analysisResult, contradictionsResult] = await Promise.all([
          getAnalysis(),
          getContradictions(),
        ]);
        if (!active) return;
        setData(analysisResult);
        setFullContradictions(contradictionsResult.contradictions);
        setError(null);
      } catch {
        if (!active) return;
        setData(null);
        setError("Failed to load analysis data");
      } finally {
        if (active) {
          setLoading(false);
        }
      }
    };

    fetchData();

    return () => {
      active = false;
    };
  }, []);

  if (loading) {
    return (
      <div style={{ padding: "2rem", textAlign: "center", color: "#6b7280" }}>
        Loading analysis data...
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

  if (!data) {
    return (
      <div style={{ padding: "2rem", textAlign: "center", color: "#6b7280" }}>
        No analysis data available
      </div>
    );
  }

  const { gap_analysis, contradictions_summary, evidence_coverage } = data;

  // Tab configuration
  const tabs: Array<{
    id: TabId;
    label: string;
    badge?: number;
    badgeColor?: { bg: string; text: string };
  }> = [
    {
      id: "gap-analysis",
      label: "Gap Analysis",
      badge: gap_analysis.gaps,
      badgeColor: gap_analysis.gaps > 0 ? { bg: "#fee2e2", text: "#991b1b" } : undefined,
    },
    {
      id: "contradictions",
      label: "Impeachment Evidence",
      badge: contradictions_summary.total,
      badgeColor:
        contradictions_summary.total > 0 ? { bg: "#fef3c7", text: "#92400e" } : undefined,
    },
    {
      id: "evidence-coverage",
      label: "Evidence Coverage",
      badge: evidence_coverage.unlinked,
      badgeColor:
        evidence_coverage.unlinked > 0 ? { bg: "#fef3c7", text: "#92400e" } : undefined,
    },
  ];

  return (
    <div style={pageStyle}>
      <h1 style={titleStyle}>Case Analysis</h1>

      {/* Tab Bar */}
      <div style={tabBarStyle}>
        {tabs.map((tab) => (
          <TabButton
            key={tab.id}
            id={tab.id}
            label={tab.label}
            isActive={activeTab === tab.id}
            onClick={() => handleTabChange(tab.id)}
            badge={tab.badge}
            badgeColor={tab.badgeColor}
          />
        ))}
      </div>

      {/* Tab Content */}
      {activeTab === "gap-analysis" && (
        <GapAnalysisContent gap_analysis={gap_analysis} />
      )}

      {activeTab === "contradictions" && (
        <ImpeachmentContent total={contradictions_summary.total} fullContradictions={fullContradictions} />
      )}

      {activeTab === "evidence-coverage" && (
        <EvidenceCoverageContent evidence_coverage={evidence_coverage} />
      )}
    </div>
  );
};

export default AnalysisPage;
