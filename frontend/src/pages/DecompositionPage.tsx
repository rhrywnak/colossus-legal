import React, { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import {
  getDecomposition,
  AllegationOverview,
  DecompositionSummary,
} from "../services/decomposition";

// ---------------------------------------------------------------------------
// Color helpers
// ---------------------------------------------------------------------------

const STATUS_COLORS: Record<string, { bg: string; text: string }> = {
  PROVEN: { bg: "var(--state-success-bg-soft)", text: "var(--status-active-text)" },
  PARTIAL: { bg: "var(--burden-warning-bg)", text: "var(--burden-warning-text)" },
  UNPROVEN: { bg: "var(--state-danger-bg-soft)", text: "var(--status-dropped-text)" },
};

const DEFAULT_STATUS_COLOR = { bg: "var(--bg-page)", text: "var(--text-secondary)" };

function getStatusStyle(status: string | undefined) {
  if (!status) return DEFAULT_STATUS_COLOR;
  return STATUS_COLORS[status.toUpperCase()] || DEFAULT_STATUS_COLOR;
}

// Characterization labels get warm/red tones — they are George's attacks
const CHAR_COLORS: Record<string, { bg: string; text: string }> = {
  frivolous: { bg: "var(--state-danger-bg-soft)", text: "var(--status-dropped-text)" },
  false: { bg: "var(--state-danger-bg-soft)", text: "var(--status-dropped-text)" },
  unfounded: { bg: "var(--state-danger-bg-soft)", text: "var(--status-dropped-text)" },
  "far fetched": { bg: "var(--burden-warning-bg)", text: "var(--burden-warning-text)" },
  "ill-conceived, unsupported": { bg: "var(--burden-warning-bg)", text: "var(--burden-warning-text)" },
  "scattershot, wholly ungrounded in fact": { bg: "var(--burden-warning-bg)", text: "var(--burden-warning-text)" },
  "not meritorious": { bg: "var(--burden-warning-bg)", text: "var(--burden-warning-text)" },
  "not relevant": { bg: "var(--burden-warning-bg)", text: "var(--burden-warning-text)" },
};

const DEFAULT_CHAR_COLOR = { bg: "var(--burden-warning-bg)", text: "var(--burden-warning-text)" };

function getCharColor(label: string) {
  return CHAR_COLORS[label.toLowerCase()] || DEFAULT_CHAR_COLOR;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

const DecompositionPage: React.FC = () => {
  const navigate = useNavigate();
  const [allegations, setAllegations] = useState<AllegationOverview[]>([]);
  const [summary, setSummary] = useState<DecompositionSummary>({
    total_allegations: 0,
    proven_count: 0,
    all_proven: false,
    total_characterizations: 0,
    total_rebuttals: 0,
  });
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;

    const fetchData = async () => {
      try {
        const result = await getDecomposition();
        if (!active) return;
        setAllegations(result.allegations);
        setSummary(result.summary);
        setError(null);
      } catch {
        if (!active) return;
        setAllegations([]);
        setError("Failed to load decomposition data");
      } finally {
        if (active) setLoading(false);
      }
    };

    fetchData();
    return () => {
      active = false;
    };
  }, []);

  // -- Loading state --------------------------------------------------------
  if (loading) {
    return (
      <div style={{ padding: "2rem", textAlign: "center", color: "var(--text-muted)" }}>
        Loading decomposition...
      </div>
    );
  }

  // -- Error state ----------------------------------------------------------
  if (error) {
    return (
      <div
        style={{
          padding: "1rem",
          backgroundColor: "var(--state-danger-bg-soft)",
          border: "1px solid var(--state-danger-border)",
          borderRadius: "6px",
          color: "var(--state-danger-strong)",
        }}
      >
        {error}
      </div>
    );
  }

  // -- Render ---------------------------------------------------------------
  return (
    <div>
      <h1 style={{ marginBottom: "0.5rem" }}>Complaint Decomposition</h1>

      {/* Summary stats row */}
      <div
        style={{
          padding: "0.75rem 1rem",
          backgroundColor: "var(--bg-page)",
          borderRadius: "6px",
          marginBottom: "1.5rem",
          color: "var(--text-secondary)",
          display: "flex",
          flexWrap: "wrap",
          gap: "0.75rem",
          alignItems: "center",
        }}
      >
        <strong>{summary.total_allegations} Allegations</strong>
        <span
          style={{
            padding: "0.25rem 0.5rem",
            backgroundColor: summary.all_proven ? "var(--state-success-bg-soft)" : "var(--burden-warning-bg)",
            color: summary.all_proven ? "var(--status-active-text)" : "var(--burden-warning-text)",
            borderRadius: "4px",
            fontSize: "0.875rem",
            fontWeight: 600,
          }}
        >
          All Proven: {summary.all_proven ? "\u2713" : "\u2717"}
        </span>
        <span style={{ color: "var(--text-disabled)" }}>&bull;</span>
        <span
          style={{
            padding: "0.25rem 0.5rem",
            backgroundColor: "var(--burden-warning-bg)",
            color: "var(--burden-warning-text)",
            borderRadius: "4px",
            fontSize: "0.875rem",
          }}
        >
          {summary.total_characterizations} Characterizations
        </span>
        <span style={{ color: "var(--text-disabled)" }}>&bull;</span>
        <span
          style={{
            padding: "0.25rem 0.5rem",
            backgroundColor: "var(--accent-bg-soft)",
            color: "var(--accent-primary-hover)",
            borderRadius: "4px",
            fontSize: "0.875rem",
          }}
        >
          {summary.total_rebuttals} Rebuttals
        </span>
      </div>

      {/* Table */}
      {allegations.length === 0 ? (
        <div style={{ color: "var(--text-muted)", padding: "1rem" }}>
          No allegations found.
        </div>
      ) : (
        <div style={{ overflowX: "auto" }}>
          <table
            style={{
              width: "100%",
              borderCollapse: "collapse",
              fontSize: "0.9rem",
            }}
          >
            <thead>
              <tr
                style={{
                  borderBottom: "2px solid var(--border-default)",
                  textAlign: "left",
                }}
              >
                <th style={thStyle}>Allegation</th>
                <th style={{ ...thStyle, width: "80px" }}>Status</th>
                <th style={thStyle}>George Called It</th>
                <th style={{ ...thStyle, width: "80px", textAlign: "center" }}>
                  Proofs
                </th>
                <th style={{ ...thStyle, width: "90px", textAlign: "center" }}>
                  Rebuttals
                </th>
              </tr>
            </thead>
            <tbody>
              {allegations.map((a) => {
                const statusStyle = getStatusStyle(a.status);
                return (
                  <tr
                    key={a.id}
                    onClick={() => navigate(`/allegations/${a.id}/detail`)}
                    style={{
                      borderBottom: "1px solid var(--border-default)",
                      cursor: "pointer",
                    }}
                    onMouseEnter={(e) => {
                      e.currentTarget.style.backgroundColor = "var(--bg-page)";
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.backgroundColor = "transparent";
                    }}
                  >
                    {/* Allegation column */}
                    <td style={tdStyle}>
                      <div
                        style={{
                          fontWeight: 600,
                          marginBottom: "0.2rem",
                          color: "var(--text-primary)",
                        }}
                      >
                        {a.title}
                      </div>
                      <div
                        style={{
                          fontSize: "0.8rem",
                          color: "var(--text-muted)",
                          fontFamily: "monospace",
                        }}
                      >
                        {a.id}
                      </div>
                    </td>

                    {/* Status column */}
                    <td style={tdStyle}>
                      {a.status && (
                        <span
                          style={{
                            padding: "0.2rem 0.5rem",
                            backgroundColor: statusStyle.bg,
                            color: statusStyle.text,
                            borderRadius: "4px",
                            fontSize: "0.75rem",
                            fontWeight: 600,
                          }}
                        >
                          {a.status}
                        </span>
                      )}
                    </td>

                    {/* Characterizations column */}
                    <td style={tdStyle}>
                      <div
                        style={{
                          display: "flex",
                          flexWrap: "wrap",
                          gap: "0.3rem",
                        }}
                      >
                        {a.characterizations.map((label, i) => {
                          const color = getCharColor(label);
                          return (
                            <span
                              key={i}
                              style={{
                                padding: "0.15rem 0.4rem",
                                backgroundColor: color.bg,
                                color: color.text,
                                borderRadius: "3px",
                                fontSize: "0.7rem",
                                fontWeight: 500,
                                whiteSpace: "nowrap",
                              }}
                            >
                              {label}
                            </span>
                          );
                        })}
                      </div>
                    </td>

                    {/* Proof count */}
                    <td style={{ ...tdStyle, textAlign: "center" }}>
                      {a.proof_count}
                    </td>

                    {/* Rebuttal count */}
                    <td style={{ ...tdStyle, textAlign: "center" }}>
                      {a.rebuttal_count > 0 ? (
                        <span style={{ fontWeight: 600, color: "var(--accent-primary-hover)" }}>
                          {a.rebuttal_count}
                        </span>
                      ) : (
                        <span style={{ color: "var(--text-disabled)" }}>0</span>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
};

// ---------------------------------------------------------------------------
// Table styles
// ---------------------------------------------------------------------------

const thStyle: React.CSSProperties = {
  padding: "0.75rem 0.5rem",
  fontWeight: 600,
  color: "var(--text-secondary)",
  fontSize: "0.8rem",
  textTransform: "uppercase",
  letterSpacing: "0.05em",
};

const tdStyle: React.CSSProperties = {
  padding: "0.75rem 0.5rem",
  verticalAlign: "top",
};

export default DecompositionPage;
