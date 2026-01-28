import React, { useEffect, useState } from "react";
import { getSchema, SchemaResponse } from "../services/schema";

type CardData = {
  label: string;
  key: string;
  color: string;
};

const CARDS: CardData[] = [
  { label: "Documents", key: "Document", color: "#3b82f6" },
  { label: "Evidence", key: "Evidence", color: "#10b981" },
  { label: "Persons", key: "Person", color: "#8b5cf6" },
  { label: "Allegations", key: "ComplaintAllegation", color: "#f59e0b" },
  { label: "Harms", key: "Harm", color: "#ef4444" },
  { label: "Legal Counts", key: "LegalCount", color: "#6366f1" },
];

const Home: React.FC = () => {
  const [schema, setSchema] = useState<SchemaResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;

    const fetchSchema = async () => {
      try {
        const result = await getSchema();
        if (!active) return;
        setSchema(result);
        setError(null);
      } catch {
        if (!active) return;
        setSchema(null);
        setError("Failed to load schema data");
      } finally {
        if (active) {
          setLoading(false);
        }
      }
    };

    fetchSchema();

    return () => {
      active = false;
    };
  }, []);

  if (loading) {
    return (
      <div style={{ padding: "2rem", textAlign: "center", color: "#6b7280" }}>
        Loading database schema...
      </div>
    );
  }

  if (error || !schema) {
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
        {error || "Unknown error"}
      </div>
    );
  }

  return (
    <div>
      <h1 style={{ marginBottom: "0.5rem" }}>Case Database Overview</h1>

      <div
        style={{
          padding: "0.75rem 1rem",
          backgroundColor: "#f3f4f6",
          borderRadius: "6px",
          marginBottom: "1.5rem",
          color: "#374151",
          fontSize: "1.1rem",
        }}
      >
        <strong>{schema.total_nodes}</strong> nodes &bull;{" "}
        <strong>{schema.total_relationships}</strong> relationships
      </div>

      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))",
          gap: "1rem",
        }}
      >
        {CARDS.map((card) => {
          const count = schema.node_counts[card.key] ?? 0;
          return (
            <div
              key={card.key}
              style={{
                padding: "1.25rem",
                backgroundColor: "#fff",
                border: "1px solid #e5e7eb",
                borderRadius: "8px",
                borderLeft: `4px solid ${card.color}`,
              }}
            >
              <div
                style={{
                  fontSize: "2rem",
                  fontWeight: "bold",
                  color: card.color,
                }}
              >
                {count}
              </div>
              <div style={{ color: "#6b7280", marginTop: "0.25rem" }}>
                {card.label}
              </div>
            </div>
          );
        })}
      </div>

      <div style={{ marginTop: "2rem" }}>
        <h2 style={{ marginBottom: "0.75rem", fontSize: "1.25rem" }}>
          Relationship Types
        </h2>
        <div
          style={{
            display: "flex",
            flexWrap: "wrap",
            gap: "0.5rem",
          }}
        >
          {Object.entries(schema.relationship_counts).map(([type, count]) => (
            <span
              key={type}
              style={{
                padding: "0.375rem 0.75rem",
                backgroundColor: "#e5e7eb",
                borderRadius: "9999px",
                fontSize: "0.875rem",
                color: "#374151",
              }}
            >
              {type}: {count}
            </span>
          ))}
        </div>
      </div>
    </div>
  );
};

export default Home;
