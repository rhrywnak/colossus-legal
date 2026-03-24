import React, { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { getEvidence, EvidenceDto } from "../services/evidence";
import { API_BASE_URL } from "../services/api";

const KIND_COLORS: Record<string, { bg: string; text: string }> = {
  testimonial: { bg: "#dbeafe", text: "#1e40af" },
  sworn_testimony: { bg: "#dbeafe", text: "#1e40af" },
  documentary: { bg: "#dcfce7", text: "#166534" },
  physical: { bg: "#ffedd5", text: "#9a3412" },
};

const DEFAULT_KIND_COLOR = { bg: "#f3f4f6", text: "#374151" };

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
      <div style={{ padding: "2rem", textAlign: "center", color: "#6b7280" }}>
        Loading evidence...
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

  const kindSummary = Object.entries(byKind)
    .map(([kind, count]) => `${count} ${kind}`)
    .join(" • ");

  return (
    <div>
      <h1 style={{ marginBottom: "0.5rem" }}>Evidence</h1>

      <div
        style={{
          padding: "0.75rem 1rem",
          backgroundColor: "#f3f4f6",
          borderRadius: "6px",
          marginBottom: "1.5rem",
          color: "#374151",
        }}
      >
        <strong>{total} Evidence Items</strong>
        {kindSummary && (
          <span style={{ marginLeft: "1rem", color: "#6b7280" }}>
            ({kindSummary})
          </span>
        )}
      </div>

      {evidence.length === 0 ? (
        <div style={{ color: "#6b7280", padding: "1rem" }}>
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
                  backgroundColor: "#fff",
                  border: "1px solid #e5e7eb",
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
                        backgroundColor: "#e5e7eb",
                        color: "#374151",
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
                        color: "#6b7280",
                      }}
                    >
                      Weight: {item.weight}/10
                    </span>
                  )}
                  {item.page_number !== undefined && (
                    <span
                      style={{
                        padding: "0.1rem 0.4rem",
                        backgroundColor: "#dbeafe",
                        color: "#1e40af",
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
                        color: "#6b7280",
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
                      color: "#6b7280",
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
                      color: "#1f2937",
                      fontSize: "0.9rem",
                      lineHeight: "1.5",
                      marginBottom: "0.5rem",
                      paddingLeft: "0.75rem",
                      borderLeft: "3px solid #e5e7eb",
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
                      borderLeft: "3px solid #d1d5db",
                      backgroundColor: "#f9fafb",
                      color: "#374151",
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
                      backgroundColor: critical ? "#fef3c7" : "#f9fafb",
                      borderRadius: "4px",
                      fontSize: "0.85rem",
                      color: critical ? "#92400e" : "#4b5563",
                      borderLeft: critical ? "3px solid #f59e0b" : "none",
                    }}
                  >
                    {item.significance}
                  </div>
                )}

                {/* Source document link */}
                {item.document_id && item.document_title && (
                  <div
                    style={{
                      marginTop: "0.75rem",
                      fontSize: "0.8rem",
                    }}
                  >
                    <span style={{ color: "#6b7280" }}>Source: </span>
                    <a
                      href={`${API_BASE_URL}/api/documents/${encodeURIComponent(item.document_id!)}/file${
                        item.page_number !== undefined ? `#page=${item.page_number}` : ""
                      }`}
                      target="_blank"
                      rel="noopener noreferrer"
                      style={{ color: "#2563eb", textDecoration: "none" }}
                      onMouseEnter={(e) => { e.currentTarget.style.textDecoration = "underline"; }}
                      onMouseLeave={(e) => { e.currentTarget.style.textDecoration = "none"; }}
                    >
                      {item.document_title}
                      {item.page_number !== undefined && (
                        <span style={{ color: "#6b7280" }}> (p. {item.page_number})</span>
                      )}
                    </a>
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
