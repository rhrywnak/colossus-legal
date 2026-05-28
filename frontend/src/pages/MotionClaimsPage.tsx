import React, { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { getMotionClaims, MotionClaimDto } from "../services/motionClaims";

const CATEGORY_COLORS: Record<string, { bg: string; text: string }> = {
  admission: { bg: "var(--state-success-bg-soft)", text: "var(--status-active-text)" },
  factual_allegation: { bg: "var(--accent-bg-soft)", text: "var(--accent-primary-hover)" },
  argument: { bg: "var(--bg-page)", text: "var(--text-secondary)" },
  evidence_summary: { bg: "var(--bias-purple-bg-soft)", text: "var(--bias-purple-text)" },
};

const DEFAULT_CATEGORY_COLOR = { bg: "var(--bg-page)", text: "var(--text-secondary)" };

function getCategoryStyle(category: string | undefined) {
  if (!category) return DEFAULT_CATEGORY_COLOR;
  return CATEGORY_COLORS[category.toLowerCase()] || DEFAULT_CATEGORY_COLOR;
}

const MotionClaimsPage: React.FC = () => {
  const [claims, setClaims] = useState<MotionClaimDto[]>([]);
  const [total, setTotal] = useState(0);
  const [byCategory, setByCategory] = useState<Record<string, number>>({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;

    const fetchClaims = async () => {
      try {
        const result = await getMotionClaims();
        if (!active) return;
        setClaims(result.motion_claims);
        setTotal(result.total);
        setByCategory(result.by_category);
        setError(null);
      } catch {
        if (!active) return;
        setClaims([]);
        setError("Failed to load motion claims");
      } finally {
        if (active) {
          setLoading(false);
        }
      }
    };

    fetchClaims();

    return () => {
      active = false;
    };
  }, []);

  if (loading) {
    return (
      <div style={{ padding: "2rem", textAlign: "center", color: "var(--text-muted)" }}>
        Loading motion claims...
      </div>
    );
  }

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

  return (
    <div>
      <h1 style={{ marginBottom: "0.5rem" }}>Motion Claims</h1>

      {/* Summary */}
      <div
        style={{
          padding: "0.5rem 1rem",
          backgroundColor: "var(--bg-page)",
          borderRadius: "6px",
          marginBottom: "1rem",
          color: "var(--text-secondary)",
        }}
      >
        <strong>{total}</strong> Motion Claims
      </div>

      {/* Category breakdown */}
      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          gap: "1rem",
          marginBottom: "1.5rem",
        }}
      >
        {Object.entries(byCategory).map(([category, count]) => {
          const style = getCategoryStyle(category);
          return (
            <div
              key={category}
              style={{
                padding: "0.75rem 1rem",
                backgroundColor: style.bg,
                borderRadius: "6px",
                minWidth: "120px",
              }}
            >
              <div
                style={{
                  fontSize: "0.75rem",
                  color: style.text,
                  textTransform: "capitalize",
                }}
              >
                {category.replace(/_/g, " ")}
              </div>
              <div
                style={{
                  fontSize: "1.25rem",
                  fontWeight: "600",
                  color: style.text,
                }}
              >
                {count}
              </div>
            </div>
          );
        })}
      </div>

      {/* Claims list */}
      {claims.length === 0 ? (
        <div style={{ color: "var(--text-muted)", padding: "1rem" }}>
          No motion claims found.
        </div>
      ) : (
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: "1rem",
          }}
        >
          {claims.map((claim) => {
            const categoryStyle = getCategoryStyle(claim.category);

            return (
              <div
                key={claim.id}
                style={{
                  padding: "1rem",
                  backgroundColor: "var(--bg-surface)",
                  border: "1px solid var(--border-default)",
                  borderRadius: "8px",
                  borderLeft: `4px solid ${categoryStyle.text}`,
                }}
              >
                {/* Header with category badge */}
                <div
                  style={{
                    display: "flex",
                    flexWrap: "wrap",
                    alignItems: "center",
                    gap: "0.5rem",
                    marginBottom: "0.5rem",
                  }}
                >
                  {claim.category && (
                    <span
                      style={{
                        padding: "0.2rem 0.5rem",
                        backgroundColor: categoryStyle.bg,
                        color: categoryStyle.text,
                        borderRadius: "4px",
                        fontSize: "0.75rem",
                        fontWeight: "500",
                        textTransform: "capitalize",
                      }}
                    >
                      {claim.category.replace(/_/g, " ")}
                    </span>
                  )}
                  <span
                    style={{
                      fontSize: "0.75rem",
                      color: "var(--text-disabled)",
                    }}
                  >
                    {claim.id}
                  </span>
                </div>

                {/* Title */}
                <div
                  style={{
                    fontWeight: "600",
                    fontSize: "1rem",
                    marginBottom: "0.5rem",
                  }}
                >
                  {claim.title}
                </div>

                {/* Claim text */}
                {claim.claim_text && (
                  <div
                    style={{
                      color: "var(--text-secondary)",
                      fontSize: "0.9rem",
                      lineHeight: "1.5",
                      marginBottom: "0.5rem",
                      padding: "0.5rem",
                      backgroundColor: "var(--bg-page)",
                      borderRadius: "4px",
                    }}
                  >
                    {claim.claim_text}
                  </div>
                )}

                {/* Significance */}
                {claim.significance && (
                  <div
                    style={{
                      fontSize: "0.85rem",
                      color: "var(--status-active-text)",
                      backgroundColor: "var(--state-success-bg-soft)",
                      padding: "0.4rem 0.6rem",
                      borderRadius: "4px",
                      marginBottom: "0.5rem",
                    }}
                  >
                    <strong>Significance:</strong> {claim.significance}
                  </div>
                )}

                {/* Source document */}
                {claim.source_document_id && (
                  <div
                    style={{
                      fontSize: "0.8rem",
                      color: "var(--text-muted)",
                      marginBottom: "0.5rem",
                    }}
                  >
                    <span style={{ fontWeight: "500" }}>Source: </span>
                    <Link
                      to={`/documents/${claim.source_document_id}`}
                      style={{
                        color: "var(--accent-primary)",
                        textDecoration: "none",
                      }}
                    >
                      {claim.source_document_title || claim.source_document_id}
                    </Link>
                  </div>
                )}

                {/* Related allegations and evidence */}
                <div
                  style={{
                    display: "flex",
                    flexWrap: "wrap",
                    gap: "1rem",
                    fontSize: "0.8rem",
                    color: "var(--text-muted)",
                    marginTop: "0.5rem",
                  }}
                >
                  {claim.proves_allegations.length > 0 && (
                    <div>
                      <span style={{ fontWeight: "500" }}>Proves: </span>
                      {claim.proves_allegations.join(", ")}
                    </div>
                  )}
                  {claim.relies_on_evidence.length > 0 && (
                    <div>
                      <span style={{ fontWeight: "500" }}>Relies on: </span>
                      {claim.relies_on_evidence.join(", ")}
                    </div>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};

export default MotionClaimsPage;
