import React, { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import Breadcrumb from "../components/Breadcrumb";
import { getPersonDetail, PersonDetailResponse } from "../services/personDetail";
import StatementCard from "../components/StatementCard";

const ROLE_COLORS: Record<string, { bg: string; text: string }> = {
  plaintiff: { bg: "var(--state-success-bg-soft)", text: "var(--status-active-text)" },
  defendant: { bg: "var(--state-danger-bg-soft)", text: "var(--status-dropped-text)" },
  attorney: { bg: "var(--accent-bg-soft)", text: "var(--accent-primary-hover)" },
  witness: { bg: "var(--bg-page)", text: "var(--text-secondary)" },
  judge: { bg: "var(--bias-purple-bg-soft)", text: "var(--bias-purple-text)" },
};

const DEFAULT_ROLE = { bg: "var(--bg-page)", text: "var(--text-secondary)" };

function getRoleStyle(role: string | undefined) {
  if (!role) return DEFAULT_ROLE;
  return ROLE_COLORS[role.toLowerCase()] || DEFAULT_ROLE;
}

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
    return <div style={{ padding: "2rem", textAlign: "center", color: "var(--text-muted)" }}>Loading person detail...</div>;
  }
  if (notFound) {
    return (
      <div style={{ padding: "1rem" }}>
        <p style={{ color: "var(--text-muted)" }}>Person not found.</p>
        <Link to="/people" style={{ color: "var(--accent-primary)", textDecoration: "none" }}>Back to People</Link>
      </div>
    );
  }
  if (error) {
    return (
      <div style={{ padding: "1rem", backgroundColor: "var(--state-danger-bg-soft)", border: "1px solid var(--state-danger-border)", borderRadius: "6px", color: "var(--state-danger-strong)" }}>
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
        { label: "Case Overview", to: "/" },
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
        padding: "0.75rem 1rem", backgroundColor: "var(--bg-page)", borderRadius: "6px",
        marginBottom: "1.5rem", color: "var(--text-secondary)", fontSize: "0.9rem",
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

      {/* Document Appearances */}
      {documents.length > 0 && (
        <div style={{ marginBottom: "1.5rem" }}>
          <h2 style={{ fontSize: "1.1rem", fontWeight: 600, color: "var(--text-primary)", marginBottom: "0.5rem" }}>
            Document Appearances
          </h2>
          <div style={{
            display: "flex", flexDirection: "column", gap: "0.4rem",
            padding: "0.75rem 1rem", backgroundColor: "var(--bg-page)", borderRadius: "8px",
            border: "1px solid var(--border-default)",
          }}>
            {documents.map((doc) => (
              <div key={doc.document_id} style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                <Link
                  to={`/documents/${doc.document_id}`}
                  style={{ color: "var(--accent-primary)", textDecoration: "none", fontSize: "0.9rem", fontWeight: 500 }}
                >
                  {doc.document_title}
                </Link>
                <span style={{ fontSize: "0.8rem", color: "var(--text-muted)" }}>
                  {doc.statement_count} statement{doc.statement_count !== 1 ? "s" : ""}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Contradictions — placeholder for future cross-document analysis */}
      <div style={{
        marginBottom: "1.5rem", padding: "1rem", borderRadius: "8px",
        border: "1px dashed var(--border-default)", backgroundColor: "var(--bg-surface)",
      }}>
        <h2 style={{ fontSize: "1.1rem", fontWeight: 600, color: "var(--text-disabled)", marginBottom: "0.25rem" }}>
          Contradictions
        </h2>
        <p style={{ fontSize: "0.85rem", color: "var(--text-disabled)", fontStyle: "italic", margin: 0 }}>
          Cross-document contradiction analysis will appear here once the v2 knowledge graph supports cross-reference detection.
        </p>
      </div>

      {/* Statements by document */}
      {summary.total_statements === 0 ? (
        <div style={{
          padding: "2rem", textAlign: "center", color: "var(--text-muted)", fontSize: "0.9rem",
          backgroundColor: "var(--bg-page)", borderRadius: "8px", border: "1px solid var(--border-default)",
        }}>
          No statements have been extracted for this person yet.
          As more documents are processed, their statements will appear here.
        </div>
      ) : (
        documents.map((doc) => (
          <div key={doc.document_id} style={{
            marginBottom: "1.5rem", paddingLeft: "1rem", borderLeft: "3px solid var(--accent-primary)",
          }}>
            <div style={{
              display: "flex", alignItems: "center", gap: "0.5rem",
              marginBottom: "0.75rem", paddingBottom: "0.5rem", borderBottom: "1px solid var(--border-default)",
            }}>
              <Link
                to={`/documents/${doc.document_id}`}
                style={{ fontWeight: 600, fontSize: "1.05rem", color: "var(--accent-primary)", textDecoration: "none" }}
              >
                {doc.document_title}
              </Link>
              <span style={{ fontSize: "0.8rem", color: "var(--text-muted)" }}>
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
