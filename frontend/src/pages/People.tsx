import React, { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { getPersons, PersonDto } from "../services/persons";

const ROLE_COLORS: Record<string, { bg: string; text: string }> = {
  plaintiff: { bg: "#dcfce7", text: "#166534" },
  defendant: { bg: "#fee2e2", text: "#991b1b" },
  attorney: { bg: "#dbeafe", text: "#1e40af" },
  witness: { bg: "#f3f4f6", text: "#374151" },
  judge: { bg: "#f3e8ff", text: "#6b21a8" },
};

const DEFAULT_ROLE_COLOR = { bg: "#f3f4f6", text: "#374151" };

function getRoleStyle(role: string | undefined) {
  if (!role) return DEFAULT_ROLE_COLOR;
  return ROLE_COLORS[role.toLowerCase()] || DEFAULT_ROLE_COLOR;
}

const People: React.FC = () => {
  const [persons, setPersons] = useState<PersonDto[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

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
        if (active) {
          setLoading(false);
        }
      }
    };

    fetchPersons();

    return () => {
      active = false;
    };
  }, []);

  if (loading) {
    return (
      <div style={{ padding: "2rem", textAlign: "center", color: "#6b7280" }}>
        Loading persons...
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
      <h1 style={{ marginBottom: "1rem" }}>People ({total})</h1>

      {persons.length === 0 ? (
        <div style={{ color: "#6b7280", padding: "1rem" }}>
          No persons found in the database.
        </div>
      ) : (
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: "0.75rem",
          }}
        >
          {persons.map((person) => {
            const roleStyle = getRoleStyle(person.role);
            return (
              <div
                key={person.id}
                style={{
                  padding: "1rem",
                  backgroundColor: "#fff",
                  border: "1px solid #e5e7eb",
                  borderRadius: "8px",
                }}
              >
                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: "0.75rem",
                  }}
                >
                  <span style={{ fontWeight: "600", fontSize: "1.1rem" }}>
                    {person.name}
                  </span>
                  {person.role && (
                    <span
                      style={{
                        padding: "0.25rem 0.5rem",
                        backgroundColor: roleStyle.bg,
                        color: roleStyle.text,
                        borderRadius: "9999px",
                        fontSize: "0.75rem",
                        fontWeight: "500",
                        textTransform: "capitalize",
                      }}
                    >
                      {person.role}
                    </span>
                  )}
                </div>
                {person.description && (
                  <div
                    style={{
                      marginTop: "0.5rem",
                      color: "#6b7280",
                      fontSize: "0.9rem",
                    }}
                  >
                    {person.description}
                  </div>
                )}
                <div style={{ marginTop: "0.5rem" }}>
                  <Link
                    to={`/people/${person.id}`}
                    style={{
                      color: "#2563eb",
                      textDecoration: "none",
                      fontSize: "0.85rem",
                      fontWeight: 500,
                    }}
                  >
                    View Detail &rarr;
                  </Link>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};

export default People;
