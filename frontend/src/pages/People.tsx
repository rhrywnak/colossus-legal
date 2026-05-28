import React, { useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { getPersons, PersonDto } from "../services/persons";
import { useEffect } from "react";

const ROLE_COLORS: Record<string, { bg: string; text: string }> = {
  plaintiff: { bg: "var(--state-success-bg-soft)", text: "var(--status-active-text)" },
  defendant: { bg: "var(--state-danger-bg-soft)", text: "var(--status-dropped-text)" },
  attorney: { bg: "var(--accent-bg-soft)", text: "var(--accent-primary-hover)" },
  witness: { bg: "var(--bg-page)", text: "var(--text-secondary)" },
  judge: { bg: "var(--bias-purple-bg-soft)", text: "var(--bias-purple-text)" },
};

const DEFAULT_ROLE_COLOR = { bg: "var(--bg-page)", text: "var(--text-secondary)" };

function getRoleStyle(role: string | undefined) {
  if (!role) return DEFAULT_ROLE_COLOR;
  return ROLE_COLORS[role.toLowerCase()] || DEFAULT_ROLE_COLOR;
}

const ROLE_ORDER: Record<string, number> = {
  defendant: 0,
  plaintiff: 1,
  attorney: 2,
  witness: 3,
  judge: 4,
};

const INFO_TEXT = `This page lists everyone involved in the case — parties, attorneys, witnesses, and the court.

Click "View Detail" on any person to see what they said across all case documents, including exact quotes with page references.

For defendants, the detail view also shows how they characterized the plaintiff's claims and the evidence that disproves those characterizations.`;

type RoleGroup = { role: string; persons: PersonDto[] };

function groupByRole(persons: PersonDto[]): RoleGroup[] {
  const groups = new Map<string, PersonDto[]>();
  for (const p of persons) {
    const role = (p.role ?? "other").toLowerCase();
    if (!groups.has(role)) groups.set(role, []);
    groups.get(role)!.push(p);
  }
  // Sort groups by ROLE_ORDER, unknowns go last
  const entries = Array.from(groups.entries()).sort(([a], [b]) => {
    return (ROLE_ORDER[a] ?? 99) - (ROLE_ORDER[b] ?? 99);
  });
  // Sort persons within each group alphabetically
  return entries.map(([role, list]) => ({
    role,
    persons: list.sort((a, b) => a.name.localeCompare(b.name)),
  }));
}

function pluralRole(role: string, count: number): string {
  if (count === 1) return role.charAt(0).toUpperCase() + role.slice(1);
  // Simple plurals
  if (role === "witness") return "Witnesses";
  if (role === "attorney") return "Attorneys";
  return role.charAt(0).toUpperCase() + role.slice(1) + "s";
}

// ─── Info Modal ──────────────────────────────────────────────────────────────

const InfoModal: React.FC<{ onClose: () => void }> = ({ onClose }) => (
  <div
    onClick={onClose}
    style={{
      position: "fixed", inset: 0, backgroundColor: "rgba(0,0,0,0.4)",
      display: "flex", alignItems: "center", justifyContent: "center", zIndex: 1000,
    }}
  >
    <div
      onClick={(e) => e.stopPropagation()}
      style={{
        backgroundColor: "var(--bg-surface)", borderRadius: "12px", padding: "2rem",
        maxWidth: "520px", width: "90%", position: "relative", boxShadow: "0 20px 60px rgba(0,0,0,0.15)",
      }}
    >
      <button
        onClick={onClose}
        style={{
          position: "absolute", top: "0.75rem", right: "0.75rem",
          background: "none", border: "none", fontSize: "1.25rem",
          cursor: "pointer", color: "var(--text-muted)", lineHeight: 1,
        }}
      >
        &times;
      </button>
      <h2 style={{ margin: "0 0 1rem 0", fontSize: "1.15rem", color: "var(--text-primary)" }}>
        About This Page
      </h2>
      <div style={{ color: "var(--text-secondary)", fontSize: "0.9rem", lineHeight: 1.7, whiteSpace: "pre-line" }}>
        {INFO_TEXT}
      </div>
    </div>
  </div>
);

// ─── Main component ──────────────────────────────────────────────────────────

const People: React.FC = () => {
  const [persons, setPersons] = useState<PersonDto[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showInfo, setShowInfo] = useState(false);

  useEffect(() => {
    let active = true;
    const fetchPersons = async () => {
      try {
        const result = await getPersons();
        if (!active) return;
        setPersons(result.persons);
        setTotal(result.total);
        setError(null);
      } catch {
        if (!active) return;
        setPersons([]);
        setTotal(0);
        setError("Failed to load persons");
      } finally {
        if (active) setLoading(false);
      }
    };
    fetchPersons();
    return () => { active = false; };
  }, []);

  const roleGroups = useMemo(() => groupByRole(persons), [persons]);

  if (loading) {
    return <div style={{ padding: "2rem", textAlign: "center", color: "var(--text-muted)" }}>Loading persons...</div>;
  }
  if (error) {
    return (
      <div style={{ padding: "1rem", backgroundColor: "var(--state-danger-bg-soft)", border: "1px solid var(--state-danger-border)", borderRadius: "6px", color: "var(--state-danger-strong)" }}>
        {error}
      </div>
    );
  }

  return (
    <div>
      <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", marginBottom: "1rem" }}>
        <h1 style={{ margin: 0 }}>Case Parties &amp; Witnesses ({total})</h1>
        <button
          onClick={() => setShowInfo(true)}
          title="About this page"
          style={{
            background: "none", border: "none", cursor: "pointer",
            fontSize: "1.1rem", color: "var(--text-disabled)", lineHeight: 1, padding: "0.25rem",
          }}
        >
          &#9432;
        </button>
      </div>

      {showInfo && <InfoModal onClose={() => setShowInfo(false)} />}

      {persons.length === 0 ? (
        <div style={{ color: "var(--text-muted)", padding: "1rem" }}>No persons found in the database.</div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: "1.25rem" }}>
          {roleGroups.map((group, gi) => (
            <div key={group.role}>
              {gi > 0 && <div style={{ borderTop: "1px solid var(--border-default)", marginBottom: "0.75rem" }} />}
              <div style={{
                fontSize: "0.7rem", fontWeight: 700, color: "var(--text-disabled)",
                textTransform: "uppercase", letterSpacing: "0.06em", marginBottom: "0.5rem",
              }}>
                {pluralRole(group.role, group.persons.length)} ({group.persons.length})
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
                {group.persons.map((person) => {
                  const roleStyle = getRoleStyle(person.role);
                  return (
                    <div
                      key={person.id}
                      style={{
                        padding: "1rem", backgroundColor: "var(--bg-surface)",
                        border: "1px solid var(--border-default)", borderRadius: "8px",
                      }}
                    >
                      <div style={{ display: "flex", alignItems: "center", gap: "0.75rem" }}>
                        <span style={{ fontWeight: "600", fontSize: "1.1rem" }}>{person.name}</span>
                        {person.role && (
                          <span style={{
                            padding: "0.25rem 0.5rem", backgroundColor: roleStyle.bg,
                            color: roleStyle.text, borderRadius: "9999px",
                            fontSize: "0.75rem", fontWeight: "500", textTransform: "capitalize",
                          }}>
                            {person.role}
                          </span>
                        )}
                      </div>
                      {person.description && (
                        <div style={{ marginTop: "0.5rem", color: "var(--text-muted)", fontSize: "0.9rem" }}>
                          {person.description}
                        </div>
                      )}
                      <div style={{ marginTop: "0.5rem" }}>
                        <Link
                          to={`/people/${person.id}`}
                          style={{ color: "var(--accent-primary)", textDecoration: "none", fontSize: "0.85rem", fontWeight: 500 }}
                        >
                          View Detail &rarr;
                        </Link>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export default People;
