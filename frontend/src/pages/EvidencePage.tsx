import React, { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { getEvidence, EvidenceDto } from "../services/evidence";

const KIND_COLORS: Record<string, { bg: string; text: string }> = {
  testimonial: { bg: "var(--accent-bg-soft)", text: "var(--accent-primary-hover)" },
  sworn_testimony: { bg: "var(--accent-bg-soft)", text: "var(--accent-primary-hover)" },
  documentary: { bg: "var(--state-success-bg-soft)", text: "var(--status-active-text)" },
  physical: { bg: "var(--burden-warning-bg)", text: "var(--burden-warning-text)" },
};

const DEFAULT_KIND_COLOR = { bg: "var(--bg-page)", text: "var(--text-secondary)" };

function getKindStyle(kind: string | undefined) {
  if (!kind) return DEFAULT_KIND_COLOR;
  return KIND_COLORS[kind.toLowerCase()] || DEFAULT_KIND_COLOR;
}

function isCritical(significance: string | undefined): boolean {
  if (!significance) return false;
  return significance.toUpperCase().includes("CRITICAL");
}

const EvidencePage: React.FC = () => {
  const [evidence, setEvidence] = useState<EvidenceDto[]>([]);
  const [byKind, setByKind] = useState<Record<string, number>>({});
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;

    const fetchEvidence = async () => {
      try {
        const result = await getEvidence();
        if (!active) return;
        setEvidence(result.evidence);
        setByKind(result.by_kind);
        setTotal(result.total);
        setError(null);
      } catch {
        if (!active) return;
        setEvidence([]);
        setError("Failed to load evidence");
      } finally {
        if (active) {
          setLoading(false);
        }
      }
    };

    fetchEvidence();

    return () => {
      active = false;
    };
  }, []);

  if (loading) {
    return (
      <div style={{ padding: "2rem", textAlign: "center", color: "var(--text-muted)" }}>
        Loading evidence...
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

  const kindSummary = Object.entries(byKind)
    .map(([kind, count]) => `${count} ${kind}`)
    .join(" • ");

  return (
    <div>
      <h1 style={{ marginBottom: "0.5rem" }}>Evidence</h1>

      <div
        style={{
          padding: "0.75rem 1rem",
          backgroundColor: "var(--bg-page)",
          borderRadius: "6px",
          marginBottom: "1.5rem",
          color: "var(--text-secondary)",
        }}
      >
        <strong>{total} Evidence Items</strong>
        {kindSummary && (
          <span style={{ marginLeft: "1rem", color: "var(--text-muted)" }}>
            ({kindSummary})
          </span>
        )}
      </div>

      {evidence.length === 0 ? (
        <div style={{ color: "var(--text-muted)", padding: "1rem" }}>
          No evidence found.
        </div>
      ) : (
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: "1rem",
          }}
        >
          {evidence.map((item) => {
            const kindStyle = getKindStyle(item.kind);
            const critical = isCritical(item.significance);

            return (
              <div
                key={item.id}
                style={{
                  padding: "1rem",
                  backgroundColor: "var(--bg-surface)",
                  border: "1px solid var(--border-default)",
                  borderRadius: "8px",
                }}
              >
                {/* Header row with badges */}
                <div
                  style={{
                    display: "flex",
                    flexWrap: "wrap",
                    alignItems: "center",
                    gap: "0.5rem",
                    marginBottom: "0.5rem",
                  }}
                >
                  {item.exhibit_number && (
                    <span
                      style={{
                        padding: "0.2rem 0.5rem",
                        backgroundColor: "var(--border-default)",
                        color: "var(--text-secondary)",
                        borderRadius: "4px",
                        fontSize: "0.75rem",
                        fontFamily: "monospace",
                      }}
                    >
                      {item.exhibit_number}
                    </span>
                  )}
                  {item.kind && (
                    <span
                      style={{
                        padding: "0.2rem 0.5rem",
                        backgroundColor: kindStyle.bg,
                        color: kindStyle.text,
                        borderRadius: "4px",
                        fontSize: "0.75rem",
                        fontWeight: "500",
                      }}
                    >
                      {item.kind}
                    </span>
                  )}
                  {item.weight !== undefined && (
                    <span
                      style={{
                        fontSize: "0.75rem",
                        color: "var(--text-muted)",
                      }}
                    >
                      Weight: {item.weight}/10
                    </span>
                  )}
                  {item.page_number !== undefined && (
                    <span
                      style={{
                        padding: "0.1rem 0.4rem",
                        backgroundColor: "var(--accent-bg-soft)",
                        color: "var(--accent-primary-hover)",
                        borderRadius: "3px",
                        fontSize: "0.75rem",
                        fontWeight: 600,
                      }}
                    >
                      p. {item.page_number}
                    </span>
                  )}
                  {item.stated_by && (
                    <span
                      style={{
                        fontSize: "0.75rem",
                        color: "var(--text-muted)",
                      }}
                    >
                      &mdash; {item.stated_by}
                    </span>
                  )}
                </div>

                {/* Title */}
                {item.title && (
                  <div
                    style={{
                      fontWeight: "600",
                      fontSize: "1rem",
                      marginBottom: "0.5rem",
                    }}
                  >
                    {item.title}
                  </div>
                )}

                {/* Question */}
                {item.question && (
                  <div
                    style={{
                      fontStyle: "italic",
                      color: "var(--text-muted)",
                      fontSize: "0.9rem",
                      marginBottom: "0.5rem",
                    }}
                  >
                    Q: {item.question}
                  </div>
                )}

                {/* Answer */}
                {item.answer && (
                  <div
                    style={{
                      color: "var(--text-primary)",
                      fontSize: "0.9rem",
                      lineHeight: "1.5",
                      marginBottom: "0.5rem",
                      paddingLeft: "0.75rem",
                      borderLeft: "3px solid var(--border-default)",
                    }}
                  >
                    {item.answer}
                  </div>
                )}

                {/* Verbatim quote */}
                {item.verbatim_quote && (
                  <blockquote
                    style={{
                      margin: "0.5rem 0",
                      padding: "0.5rem 0.75rem",
                      borderLeft: "3px solid var(--border-default)",
                      backgroundColor: "var(--bg-page)",
                      color: "var(--text-secondary)",
                      fontStyle: "italic",
                      fontSize: "0.9rem",
                      lineHeight: 1.6,
                      borderRadius: "0 4px 4px 0",
                    }}
                  >
                    {item.verbatim_quote}
                  </blockquote>
                )}

                {/* Significance */}
                {item.significance && (
                  <div
                    style={{
                      marginTop: "0.5rem",
                      padding: "0.5rem",
                      backgroundColor: critical ? "var(--burden-warning-bg)" : "var(--bg-page)",
                      borderRadius: "4px",
                      fontSize: "0.85rem",
                      color: critical ? "var(--burden-warning-text)" : "var(--text-secondary)",
                      borderLeft: critical ? "3px solid var(--state-warning-strong)" : "none",
                    }}
                  >
                    {item.significance}
                  </div>
                )}

                {/* Source document link */}
                {item.document_id && item.document_title && (
                  <div style={{ marginTop: "0.75rem", fontSize: "0.8rem" }}>
                    <span style={{ color: "var(--text-muted)" }}>Source: </span>
                    <Link
                      to={`/documents/${item.document_id}`}
                      style={{ color: "var(--accent-primary)", textDecoration: "none" }}
                      onMouseEnter={(e) => { e.currentTarget.style.textDecoration = "underline"; }}
                      onMouseLeave={(e) => { e.currentTarget.style.textDecoration = "none"; }}
                    >
                      {item.document_title}
                      {item.page_number !== undefined && (
                        <span style={{ color: "var(--text-muted)" }}> (p. {item.page_number})</span>
                      )}
                    </Link>
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};

export default EvidencePage;
