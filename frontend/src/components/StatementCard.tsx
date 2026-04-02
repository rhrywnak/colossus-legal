/**
 * StatementCard — Displays a single evidence statement with characterizations and rebuttals.
 *
 * Used on the PersonDetailPage to render each statement within a document group.
 */
import React from "react";
import type { StatementDetail } from "../services/personDetail";

/** Group characterizations by label, sort allegation IDs numerically within each group */
function groupCharacterizations(chars: { allegation_id: string; characterization_label?: string }[]) {
  const map = new Map<string, string[]>();
  for (const ch of chars) {
    const label = ch.characterization_label ?? "unknown";
    if (!map.has(label)) map.set(label, []);
    map.get(label)!.push(ch.allegation_id);
  }
  return Array.from(map.entries()).map(([label, ids]) => ({
    label,
    ids: ids.sort((a, b) => {
      const na = parseInt(a.replace(/\D/g, ""), 10) || 0;
      const nb = parseInt(b.replace(/\D/g, ""), 10) || 0;
      return na - nb;
    }),
  }));
}

const StatementCard: React.FC<{ stmt: StatementDetail }> = ({ stmt }) => (
  <div style={{
    padding: "1rem", backgroundColor: "#fff",
    border: "1px solid #e5e7eb", borderRadius: "8px", marginBottom: "0.75rem",
  }}>
    {/* Title + page badge + kind badge */}
    <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: "0.5rem", marginBottom: "0.5rem" }}>
      <span style={{ fontWeight: 600, fontSize: "0.95rem", color: "#1f2937" }}>{stmt.title}</span>
      {stmt.page_number != null && (
        <span style={{
          padding: "0.1rem 0.4rem", backgroundColor: "#dbeafe", color: "#1e40af",
          borderRadius: "3px", fontSize: "0.75rem", fontWeight: 600,
        }}>
          p. {stmt.page_number}
        </span>
      )}
      {stmt.kind && (
        <span style={{
          padding: "0.1rem 0.4rem", backgroundColor: "#f3f4f6", color: "#6b7280",
          borderRadius: "3px", fontSize: "0.7rem",
        }}>
          {stmt.kind}
        </span>
      )}
    </div>

    {/* Verbatim quote */}
    {stmt.verbatim_quote && (
      <blockquote style={{
        margin: "0 0 0.5rem 0", padding: "0.5rem 0.75rem",
        borderLeft: "3px solid #93c5fd", backgroundColor: "#eff6ff",
        color: "#374151", fontStyle: "italic", fontSize: "0.9rem", lineHeight: 1.6,
        borderRadius: "0 4px 4px 0",
      }}>
        {stmt.verbatim_quote}
      </blockquote>
    )}

    {/* Significance */}
    {stmt.significance && (
      <div style={{ fontSize: "0.85rem", color: "#4b5563", marginBottom: "0.5rem", lineHeight: 1.5 }}>
        {stmt.significance}
      </div>
    )}

    {/* Characterizations — grouped by label */}
    {stmt.characterizes.length > 0 && (
      <div style={{ display: "flex", flexDirection: "column", gap: "0.25rem", marginBottom: "0.5rem" }}>
        {groupCharacterizations(stmt.characterizes).map(({ label, ids }) => (
          <div key={label} style={{
            display: "flex", flexWrap: "wrap", alignItems: "center", gap: "0.5rem",
          }}>
            <span style={{
              padding: "0.15rem 0.4rem", backgroundColor: "#fef3c7", color: "#92400e",
              borderRadius: "4px", fontSize: "0.75rem", fontWeight: 600,
            }}>
              Characterized as &ldquo;{label}&rdquo;
            </span>
            <span style={{ fontSize: "0.8rem" }}>
              {ids.map((aid, i) => (
                <React.Fragment key={aid}>
                  {i > 0 && ", "}
                  <a
                    href={`/allegations/${aid}/detail`}
                    target="_blank"
                    rel="noopener noreferrer"
                    style={{ color: "#2563eb", textDecoration: "none" }}
                  >
                    {aid}
                  </a>
                </React.Fragment>
              ))}
            </span>
          </div>
        ))}
      </div>
    )}

    {/* Rebuttals */}
    {stmt.rebutted_by.length > 0 && (
      <div style={{ marginTop: "0.5rem" }}>
        <div style={{
          fontSize: "0.75rem", fontWeight: 600, color: "#059669",
          textTransform: "uppercase", letterSpacing: "0.05em", marginBottom: "0.35rem",
        }}>
          Rebutted ({stmt.rebutted_by.length})
        </div>
        <div style={{ paddingLeft: "0.75rem", borderLeft: "3px solid #bbf7d0" }}>
          {stmt.rebutted_by.map((reb) => (
            <div key={reb.evidence_id} style={{ marginBottom: "0.5rem" }}>
              {reb.verbatim_quote && (
                <blockquote style={{
                  margin: "0 0 0.25rem 0", padding: "0.4rem 0.6rem",
                  borderLeft: "3px solid #86efac", backgroundColor: "#f0fdf4",
                  color: "#374151", fontStyle: "italic", fontSize: "0.85rem",
                  lineHeight: 1.5, borderRadius: "0 4px 4px 0",
                }}>
                  {reb.verbatim_quote}
                </blockquote>
              )}
              <div style={{ fontSize: "0.8rem", color: "#6b7280" }}>
                {reb.stated_by && <span>&mdash; {reb.stated_by}</span>}
                {reb.document_title && (
                  <span style={{ marginLeft: "0.5rem", fontStyle: "italic" }}>
                    ({reb.document_title})
                  </span>
                )}
              </div>
            </div>
          ))}
        </div>
      </div>
    )}
  </div>
);

export default StatementCard;
