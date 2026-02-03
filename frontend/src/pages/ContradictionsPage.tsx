import React, { useEffect, useState } from "react";
import {
  getContradictions,
  ContradictionDto,
} from "../services/contradictions";

const ContradictionsPage: React.FC = () => {
  const [contradictions, setContradictions] = useState<ContradictionDto[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;

    const fetchContradictions = async () => {
      try {
        const result = await getContradictions();
        if (!active) return;
        setContradictions(result.contradictions);
        setTotal(result.total);
        setError(null);
      } catch {
        if (!active) return;
        setContradictions([]);
        setError("Failed to load contradictions");
      } finally {
        if (active) {
          setLoading(false);
        }
      }
    };

    fetchContradictions();

    return () => {
      active = false;
    };
  }, []);

  if (loading) {
    return (
      <div style={{ padding: "2rem", textAlign: "center", color: "#6b7280" }}>
        Loading contradictions...
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

  return (
    <div>
      <h1 style={{ marginBottom: "0.5rem" }}>Contradictions ({total})</h1>

      <p
        style={{
          color: "#6b7280",
          marginBottom: "1.5rem",
          fontSize: "0.9rem",
        }}
      >
        Evidence that contradicts other evidence - useful for demonstrating
        inconsistencies.
      </p>

      {contradictions.length === 0 ? (
        <div style={{ color: "#6b7280", padding: "1rem" }}>
          No contradictions found.
        </div>
      ) : (
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: "1.5rem",
          }}
        >
          {contradictions.map((contradiction, index) => (
            <div
              key={index}
              style={{
                border: "1px solid #e5e7eb",
                borderRadius: "8px",
                overflow: "hidden",
              }}
            >
              {/* Header */}
              <div
                style={{
                  padding: "0.75rem 1rem",
                  backgroundColor: "#f3f4f6",
                  borderBottom: "1px solid #e5e7eb",
                  fontWeight: "600",
                  color: "#374151",
                  display: "flex",
                  alignItems: "center",
                  gap: "0.75rem",
                }}
              >
                <span>{contradiction.topic || "CONTRADICTION"}</span>
                {contradiction.impeachment_value && (
                  <span
                    style={{
                      padding: "0.25rem 0.5rem",
                      borderRadius: "4px",
                      fontSize: "0.75rem",
                      fontWeight: "600",
                      color: "#fff",
                      backgroundColor:
                        contradiction.impeachment_value === "HIGH"
                          ? "#dc2626"
                          : contradiction.impeachment_value === "MEDIUM"
                            ? "#ea580c"
                            : "#6b7280",
                    }}
                  >
                    {contradiction.impeachment_value}
                  </span>
                )}
                {contradiction.description && (
                  <span
                    style={{
                      fontWeight: "normal",
                      color: "#6b7280",
                    }}
                  >
                    — {contradiction.description}
                  </span>
                )}
              </div>

              {/* Side-by-side comparison */}
              <div
                style={{
                  display: "grid",
                  gridTemplateColumns: "1fr 1fr",
                }}
              >
                {/* Left side - Claim A (disputed) */}
                <div
                  style={{
                    padding: "1rem",
                    backgroundColor: "#fef2f2",
                    borderRight: "1px solid #e5e7eb",
                  }}
                >
                  {contradiction.earlier_claim && (
                    <>
                      <div
                        style={{
                          fontSize: "0.75rem",
                          fontWeight: "600",
                          color: "#991b1b",
                          marginBottom: "0.5rem",
                          textTransform: "uppercase",
                        }}
                      >
                        Claimed:
                      </div>
                      <blockquote
                        style={{
                          margin: "0 0 1rem 0",
                          padding: "0.5rem 0.75rem",
                          borderLeft: "3px solid #fca5a5",
                          backgroundColor: "#fff",
                          color: "#4b5563",
                          fontSize: "0.9rem",
                          fontStyle: "italic",
                          lineHeight: "1.5",
                        }}
                      >
                        "{contradiction.earlier_claim}"
                      </blockquote>
                    </>
                  )}
                  <div
                    style={{
                      fontSize: "0.75rem",
                      fontWeight: "600",
                      color: "#991b1b",
                      marginBottom: "0.5rem",
                      textTransform: "uppercase",
                    }}
                  >
                    Claim A
                  </div>

                  {contradiction.evidence_a.title && (
                    <div
                      style={{
                        fontWeight: "600",
                        marginBottom: "0.5rem",
                        color: "#1f2937",
                      }}
                    >
                      {contradiction.evidence_a.title}
                    </div>
                  )}

                  {contradiction.evidence_a.answer && (
                    <blockquote
                      style={{
                        margin: "0.5rem 0",
                        padding: "0.5rem 0.75rem",
                        borderLeft: "3px solid #fca5a5",
                        backgroundColor: "#fff",
                        color: "#4b5563",
                        fontSize: "0.9rem",
                        fontStyle: "italic",
                        lineHeight: "1.5",
                      }}
                    >
                      "{contradiction.evidence_a.answer}"
                    </blockquote>
                  )}

                  {contradiction.evidence_a.document_title && (
                    <div
                      style={{
                        fontSize: "0.8rem",
                        color: "#6b7280",
                        marginTop: "0.5rem",
                      }}
                    >
                      Source: {contradiction.evidence_a.document_title}
                    </div>
                  )}
                </div>

                {/* Right side - Contradicted By */}
                <div
                  style={{
                    padding: "1rem",
                    backgroundColor: "#f0fdf4",
                  }}
                >
                  {contradiction.later_admission && (
                    <>
                      <div
                        style={{
                          fontSize: "0.75rem",
                          fontWeight: "600",
                          color: "#166534",
                          marginBottom: "0.5rem",
                          textTransform: "uppercase",
                        }}
                      >
                        Actually admitted:
                      </div>
                      <blockquote
                        style={{
                          margin: "0 0 1rem 0",
                          padding: "0.5rem 0.75rem",
                          borderLeft: "3px solid #86efac",
                          backgroundColor: "#fff",
                          color: "#4b5563",
                          fontSize: "0.9rem",
                          fontStyle: "italic",
                          lineHeight: "1.5",
                        }}
                      >
                        "{contradiction.later_admission}"
                      </blockquote>
                    </>
                  )}
                  <div
                    style={{
                      fontSize: "0.75rem",
                      fontWeight: "600",
                      color: "#166534",
                      marginBottom: "0.5rem",
                      textTransform: "uppercase",
                    }}
                  >
                    Contradicted By
                  </div>

                  {contradiction.evidence_b.title && (
                    <div
                      style={{
                        fontWeight: "600",
                        marginBottom: "0.5rem",
                        color: "#1f2937",
                      }}
                    >
                      {contradiction.evidence_b.title}
                    </div>
                  )}

                  {contradiction.evidence_b.answer && (
                    <blockquote
                      style={{
                        margin: "0.5rem 0",
                        padding: "0.5rem 0.75rem",
                        borderLeft: "3px solid #86efac",
                        backgroundColor: "#fff",
                        color: "#4b5563",
                        fontSize: "0.9rem",
                        fontStyle: "italic",
                        lineHeight: "1.5",
                      }}
                    >
                      "{contradiction.evidence_b.answer}"
                    </blockquote>
                  )}

                  {contradiction.evidence_b.document_title && (
                    <div
                      style={{
                        fontSize: "0.8rem",
                        color: "#6b7280",
                        marginTop: "0.5rem",
                      }}
                    >
                      Source: {contradiction.evidence_b.document_title}
                    </div>
                  )}
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export default ContradictionsPage;
