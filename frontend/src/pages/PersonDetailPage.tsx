import React, { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import Breadcrumb from "../components/Breadcrumb";
import {
  getPersonDetail,
  PersonDetailResponse,
  StatementDetail,
} from "../services/personDetail";

const ROLE_COLORS: Record<string, { bg: string; text: string }> = {
  plaintiff: { bg: "#dcfce7", text: "#166534" },
  defendant: { bg: "#fee2e2", text: "#991b1b" },
  attorney: { bg: "#dbeafe", text: "#1e40af" },
  witness: { bg: "#f3f4f6", text: "#374151" },
  judge: { bg: "#f3e8ff", text: "#6b21a8" },
};

const DEFAULT_ROLE = { bg: "#f3f4f6", text: "#374151" };

function getRoleStyle(role: string | undefined) {
  if (!role) return DEFAULT_ROLE;
  return ROLE_COLORS[role.toLowerCase()] || DEFAULT_ROLE;
}

// ─── Statement card (extracted to keep component under 300 lines) ────────────

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

    {/* Characterizations */}
    {stmt.characterizes.length > 0 && (
      <div style={{ display: "flex", flexDirection: "column", gap: "0.25rem", marginBottom: "0.5rem" }}>
        {stmt.characterizes.map((ch) => (
          <div key={`${ch.allegation_id}-${ch.characterization_label}`} style={{
            display: "flex", flexWrap: "wrap", alignItems: "center", gap: "0.5rem",
          }}>
            <span style={{
              padding: "0.15rem 0.4rem", backgroundColor: "#fef3c7", color: "#92400e",
              borderRadius: "4px", fontSize: "0.75rem", fontWeight: 600,
            }}>
              Characterized as "{ch.characterization_label}"
            </span>
            <Link
              to={`/allegations/${ch.allegation_id}/detail`}
              style={{ fontSize: "0.8rem", color: "#2563eb", textDecoration: "none" }}
            >
              {ch.allegation_id}
            </Link>
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

// ─── Main component ──────────────────────────────────────────────────────────

const PersonDetailPage: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const personId = id ?? "";

  const [data, setData] = useState<PersonDetailResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [notFound, setNotFound] = useState(false);

  useEffect(() => {
    if (!personId) return;
    let active = true;

    getPersonDetail(personId)
      .then((result) => { if (active) setData(result); })
      .catch((err) => {
        if (!active) return;
        const msg = err instanceof Error ? err.message : "Failed to load person";
        if (msg.includes("not found")) setNotFound(true);
        else setError(msg);
      })
      .finally(() => { if (active) setLoading(false); });

    return () => { active = false; };
  }, [personId]);

  if (loading) {
    return <div style={{ padding: "2rem", textAlign: "center", color: "#6b7280" }}>Loading person detail...</div>;
  }
  if (notFound) {
    return (
      <div style={{ padding: "1rem" }}>
        <p style={{ color: "#6b7280" }}>Person not found.</p>
        <Link to="/people" style={{ color: "#2563eb", textDecoration: "none" }}>Back to People</Link>
      </div>
    );
  }
  if (error) {
    return (
      <div style={{ padding: "1rem", backgroundColor: "#fef2f2", border: "1px solid #fecaca", borderRadius: "6px", color: "#dc2626" }}>
        {error}
      </div>
    );
  }
  if (!data) return <div>No data available.</div>;

  const { person, summary, documents } = data;
  const roleStyle = getRoleStyle(person.role);

  return (
    <div style={{ maxWidth: "960px" }}>
      <Breadcrumb items={[
        { label: "Dashboard", to: "/" },
        { label: "People", to: "/people" },
        { label: person.name },
      ]} />

      {/* Person header */}
      <div style={{ display: "flex", alignItems: "center", gap: "0.75rem", marginBottom: "1rem" }}>
        <h1 style={{ margin: 0, fontSize: "1.5rem" }}>{person.name}</h1>
        {person.role && (
          <span style={{
            padding: "0.25rem 0.75rem", backgroundColor: roleStyle.bg, color: roleStyle.text,
            borderRadius: "9999px", fontSize: "0.8rem", fontWeight: 500, textTransform: "capitalize",
          }}>
            {person.role}
          </span>
        )}
      </div>

      {/* Summary stats */}
      <div style={{
        padding: "0.75rem 1rem", backgroundColor: "#f3f4f6", borderRadius: "6px",
        marginBottom: "1.5rem", color: "#374151", fontSize: "0.9rem",
      }}>
        <strong>{summary.total_statements}</strong> statement{summary.total_statements !== 1 ? "s" : ""}
        {" \u2022 "}<strong>{summary.documents_count}</strong> document{summary.documents_count !== 1 ? "s" : ""}
        {summary.characterizations_count > 0 && (
          <>{" \u2022 "}<strong>{summary.characterizations_count}</strong> characterization{summary.characterizations_count !== 1 ? "s" : ""}</>
        )}
        {summary.rebuttals_received_count > 0 && (
          <>{" \u2022 "}<strong>{summary.rebuttals_received_count}</strong> rebutted</>
        )}
      </div>

      {/* Document sections */}
      {documents.length === 0 ? (
        <div style={{ color: "#6b7280", padding: "1rem" }}>No statements found for this person.</div>
      ) : (
        documents.map((doc) => (
          <div key={doc.document_id} style={{
            marginBottom: "1.5rem", paddingLeft: "1rem", borderLeft: "3px solid #2563eb",
          }}>
            <div style={{
              display: "flex", alignItems: "center", gap: "0.5rem",
              marginBottom: "0.75rem", paddingBottom: "0.5rem", borderBottom: "1px solid #e5e7eb",
            }}>
              <span style={{ fontWeight: 600, fontSize: "1.05rem", color: "#1f2937" }}>
                {doc.document_title}
              </span>
              <span style={{ fontSize: "0.8rem", color: "#6b7280" }}>
                ({doc.statement_count} statement{doc.statement_count !== 1 ? "s" : ""})
              </span>
            </div>
            {doc.statements.map((stmt) => (
              <StatementCard key={stmt.evidence_id} stmt={stmt} />
            ))}
          </div>
        ))
      )}
    </div>
  );
};

export default PersonDetailPage;
