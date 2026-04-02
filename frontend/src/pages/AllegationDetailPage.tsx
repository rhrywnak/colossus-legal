import React, { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import {
  getAllegationDetail,
  AllegationDetailResponse,
} from "../services/decomposition";
import Breadcrumb from "../components/Breadcrumb";

// ---------------------------------------------------------------------------
// Color helpers (same palette as DecompositionPage)
// ---------------------------------------------------------------------------

const STATUS_COLORS: Record<string, { bg: string; text: string }> = {
  PROVEN: { bg: "#dcfce7", text: "#166534" },
  PARTIAL: { bg: "#fef3c7", text: "#92400e" },
  UNPROVEN: { bg: "#fee2e2", text: "#991b1b" },
};

const DEFAULT_STATUS_COLOR = { bg: "#f3f4f6", text: "#374151" };

function getStatusStyle(status: string) {
  return STATUS_COLORS[status.toUpperCase()] || DEFAULT_STATUS_COLOR;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

const AllegationDetailPage: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const allegationId = id ?? "";

  const [data, setData] = useState<AllegationDetailResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [notFound, setNotFound] = useState(false);

  useEffect(() => {
    if (!allegationId) return;
    let active = true;

    const fetchData = async () => {
      try {
        const result = await getAllegationDetail(allegationId);
        if (!active) return;
        setData(result);
        setError(null);
      } catch (err) {
        if (!active) return;
        const msg =
          err instanceof Error ? err.message : "Failed to load allegation";
        if (msg.includes("not found")) {
          setNotFound(true);
        } else {
          setError(msg);
        }
      } finally {
        if (active) setLoading(false);
      }
    };

    fetchData();
    return () => {
      active = false;
    };
  }, [allegationId]);

  // -- Loading state --------------------------------------------------------
  if (loading) {
    return (
      <div style={{ padding: "2rem", textAlign: "center", color: "#6b7280" }}>
        Loading allegation detail...
      </div>
    );
  }

  // -- Not found ------------------------------------------------------------
  if (notFound) {
    return (
      <div style={{ padding: "1rem" }}>
        <p style={{ color: "#6b7280" }}>Allegation not found.</p>
        <Link to="/decomposition" style={backLinkStyle}>
          Back to Decomposition
        </Link>
      </div>
    );
  }

  // -- Error state ----------------------------------------------------------
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
        <div style={{ marginTop: "0.5rem" }}>
          <Link to="/decomposition" style={backLinkStyle}>
            Back to Decomposition
          </Link>
        </div>
      </div>
    );
  }

  if (!data) {
    return <div>No data available.</div>;
  }

  const { allegation, characterizations, proof_claims } = data;
  const statusStyle = getStatusStyle(allegation.status);

  // -- Render ---------------------------------------------------------------
  return (
    <div style={{ maxWidth: "960px" }}>
      <Breadcrumb items={[
        { label: "Dashboard", to: "/" },
        { label: "Allegations", to: "/allegations" },
        { label: allegationId },
      ]} />

      {/* Header */}
      <div style={{ marginBottom: "1.5rem" }}>
        <div
          style={{
            display: "flex",
            flexWrap: "wrap",
            alignItems: "center",
            gap: "0.5rem",
            marginBottom: "0.5rem",
          }}
        >
          <span
            style={{
              padding: "0.2rem 0.5rem",
              backgroundColor: "#e5e7eb",
              color: "#374151",
              borderRadius: "4px",
              fontSize: "0.75rem",
              fontFamily: "monospace",
            }}
          >
            {allegation.id}
          </span>
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
            {allegation.status}
          </span>
          {allegation.legal_counts.map((count) => (
            <span key={count} style={legalCountTagStyle}>
              {count}
            </span>
          ))}
        </div>

        <h1 style={{ margin: 0, fontSize: "1.5rem" }}>{allegation.title}</h1>

        {allegation.description && (
          <p style={{ color: "#4b5563", marginTop: "0.5rem", lineHeight: 1.5 }}>
            {allegation.description}
          </p>
        )}
      </div>

      {/* ── Section: Characterizations ────────────────────────────────── */}
      <h2 style={sectionHeaderStyle}>
        George Phillips' Characterizations
        <span style={countBadgeStyle}>{characterizations.length}</span>
      </h2>

      {characterizations.length === 0 ? (
        <p style={{ color: "#6b7280", fontStyle: "italic" }}>
          No characterizations found for this allegation.
        </p>
      ) : (
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: "1rem",
            marginBottom: "2rem",
          }}
        >
          {characterizations.map((char) => (
            <div key={char.evidence_id} style={cardStyle}>
              {/* Characterization header: label + source */}
              <div
                style={{
                  display: "flex",
                  flexWrap: "wrap",
                  alignItems: "center",
                  gap: "0.5rem",
                  marginBottom: "0.5rem",
                }}
              >
                <span style={charLabelStyle}>{char.label}</span>
                {char.stated_by && (
                  <span style={{ fontSize: "0.8rem", color: "#6b7280" }}>
                    &mdash; {char.stated_by}
                  </span>
                )}
              </div>

              {/* Source document + page number */}
              {/* TODO: Link to /documents/:id when document_id is added to CharacterizationDetail API type */}
              {(char.document || char.page_number) && (
                <div style={sourceLineStyle}>
                  {char.document && <span>{char.document}</span>}
                  {char.page_number && (
                    <span style={pageNumberStyle}>p. {char.page_number}</span>
                  )}
                </div>
              )}

              {/* Verbatim quote */}
              {char.verbatim_quote && (
                <blockquote style={quoteStyle}>{char.verbatim_quote}</blockquote>
              )}

              {/* Nested rebuttals */}
              {char.rebuttals.length > 0 && (
                <div style={{ marginTop: "0.75rem" }}>
                  <div
                    style={{
                      fontSize: "0.8rem",
                      fontWeight: 600,
                      color: "#1e40af",
                      marginBottom: "0.5rem",
                      textTransform: "uppercase",
                      letterSpacing: "0.05em",
                    }}
                  >
                    Rebuttals ({char.rebuttals.length})
                  </div>
                  <div
                    style={{
                      display: "flex",
                      flexDirection: "column",
                      gap: "0.75rem",
                      paddingLeft: "1rem",
                      borderLeft: "3px solid #bfdbfe",
                    }}
                  >
                    {char.rebuttals.map((reb) => (
                      <div key={reb.evidence_id}>
                        {/* Rebuttal source */}
                        {/* TODO: Link to /documents/:id when document_id is added to RebuttalDetail API type */}
                        <div style={sourceLineStyle}>
                          {reb.document && <span>{reb.document}</span>}
                          {reb.page_number && (
                            <span style={pageNumberStyle}>
                              p. {reb.page_number}
                            </span>
                          )}
                        </div>

                        {/* Rebuttal quote */}
                        {reb.verbatim_quote && (
                          <blockquote style={rebuttalQuoteStyle}>
                            {reb.verbatim_quote}
                          </blockquote>
                        )}

                        {/* Stated by */}
                        {reb.stated_by && (
                          <div
                            style={{
                              fontSize: "0.8rem",
                              color: "#6b7280",
                              marginTop: "0.25rem",
                            }}
                          >
                            &mdash; {reb.stated_by}
                            {reb.topic && (
                              <span
                                style={{ fontStyle: "italic", marginLeft: "0.5rem" }}
                              >
                                (re: {reb.topic})
                              </span>
                            )}
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {/* ── Section: Proof Claims ──────────────────────────────────────── */}
      <h2 style={sectionHeaderStyle}>
        Proof Claims
        <span style={countBadgeStyle}>{proof_claims.length}</span>
      </h2>

      {proof_claims.length === 0 ? (
        <p style={{ color: "#6b7280", fontStyle: "italic" }}>
          No proof claims found for this allegation.
        </p>
      ) : (
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: "0.75rem",
            marginBottom: "2rem",
          }}
        >
          {proof_claims.map((claim) => (
            <div key={claim.id} style={cardStyle}>
              <div
                style={{
                  display: "flex",
                  flexWrap: "wrap",
                  alignItems: "center",
                  gap: "0.5rem",
                }}
              >
                <span style={{ fontWeight: 600, color: "#1f2937" }}>
                  {claim.title}
                </span>
                {claim.category && (
                  <span
                    style={{
                      padding: "0.15rem 0.4rem",
                      backgroundColor: "#f3f4f6",
                      color: "#6b7280",
                      borderRadius: "3px",
                      fontSize: "0.7rem",
                    }}
                  >
                    {claim.category}
                  </span>
                )}
                <span
                  style={{
                    fontSize: "0.8rem",
                    color: "#6b7280",
                  }}
                >
                  {claim.evidence_count} evidence item{claim.evidence_count !== 1 ? "s" : ""}
                </span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

// ---------------------------------------------------------------------------
// Styles
// ---------------------------------------------------------------------------

const backLinkStyle: React.CSSProperties = {
  color: "#2563eb",
  textDecoration: "none",
  fontSize: "0.9rem",
};

const sectionHeaderStyle: React.CSSProperties = {
  fontSize: "1.1rem",
  fontWeight: 600,
  color: "#1f2937",
  borderBottom: "1px solid #e5e7eb",
  paddingBottom: "0.5rem",
  marginBottom: "1rem",
  display: "flex",
  alignItems: "center",
  gap: "0.5rem",
};

const countBadgeStyle: React.CSSProperties = {
  padding: "0.1rem 0.4rem",
  backgroundColor: "#e5e7eb",
  color: "#374151",
  borderRadius: "10px",
  fontSize: "0.8rem",
  fontWeight: 500,
};

const legalCountTagStyle: React.CSSProperties = {
  padding: "0.15rem 0.4rem",
  backgroundColor: "#ede9fe",
  color: "#5b21b6",
  borderRadius: "3px",
  fontSize: "0.7rem",
  fontWeight: 500,
};

const cardStyle: React.CSSProperties = {
  padding: "1rem",
  backgroundColor: "#ffffff",
  border: "1px solid #e5e7eb",
  borderRadius: "8px",
};

const charLabelStyle: React.CSSProperties = {
  padding: "0.2rem 0.5rem",
  backgroundColor: "#fee2e2",
  color: "#991b1b",
  borderRadius: "4px",
  fontSize: "0.8rem",
  fontWeight: 600,
  textTransform: "capitalize",
};

const sourceLineStyle: React.CSSProperties = {
  fontSize: "0.85rem",
  color: "#4b5563",
  marginBottom: "0.4rem",
  display: "flex",
  flexWrap: "wrap",
  alignItems: "center",
  gap: "0.5rem",
};

const pageNumberStyle: React.CSSProperties = {
  padding: "0.1rem 0.4rem",
  backgroundColor: "#dbeafe",
  color: "#1e40af",
  borderRadius: "3px",
  fontSize: "0.75rem",
  fontWeight: 600,
};

const quoteStyle: React.CSSProperties = {
  margin: "0.4rem 0 0 0",
  padding: "0.5rem 0.75rem",
  borderLeft: "3px solid #d1d5db",
  color: "#374151",
  fontStyle: "italic",
  fontSize: "0.9rem",
  lineHeight: 1.6,
  backgroundColor: "#f9fafb",
  borderRadius: "0 4px 4px 0",
};

const rebuttalQuoteStyle: React.CSSProperties = {
  ...quoteStyle,
  borderLeftColor: "#93c5fd",
  backgroundColor: "#eff6ff",
};

export default AllegationDetailPage;
