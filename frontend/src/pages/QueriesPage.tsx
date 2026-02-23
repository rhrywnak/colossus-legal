import React, { useEffect, useState } from "react";
import Breadcrumb from "../components/Breadcrumb";
import {
  getQueries,
  runQuery,
  QueryCategory,
  QueryResultResponse,
} from "../services/queries";

// ─── Cell renderer: handles string, number, array, null ──────────────────────

function formatCell(value: unknown): string {
  if (value === null || value === undefined) return "\u2014";
  if (Array.isArray(value)) return value.join(", ");
  if (typeof value === "number") {
    return value % 1 === 0 ? value.toLocaleString() : `$${value.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
  }
  return String(value);
}

function truncate(text: string, max: number): string {
  return text.length > max ? text.slice(0, max) + "\u2026" : text;
}

// ─── Results Table (extracted to stay under 300 lines) ───────────────────────

const ResultsTable: React.FC<{
  result: QueryResultResponse;
  onClear: () => void;
}> = ({ result, onClear }) => (
  <div style={{ marginTop: "2rem", borderTop: "2px solid #2563eb", paddingTop: "1.5rem" }}>
    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "1rem" }}>
      <div>
        <h2 style={{ margin: 0, fontSize: "1.25rem" }}>{result.title}</h2>
        <span style={{ fontSize: "0.85rem", color: "#6b7280" }}>
          {result.row_count} result{result.row_count !== 1 ? "s" : ""}
        </span>
      </div>
      <button
        onClick={onClear}
        style={{
          padding: "0.4rem 1rem", backgroundColor: "#f3f4f6", border: "1px solid #d1d5db",
          borderRadius: "6px", cursor: "pointer", fontSize: "0.85rem", color: "#374151",
        }}
      >
        Clear Results
      </button>
    </div>

    {result.row_count === 0 ? (
      <div style={{ padding: "1rem", color: "#6b7280", backgroundColor: "#f9fafb", borderRadius: "6px" }}>
        No results found for this query.
      </div>
    ) : (
      <div style={{ overflowX: "auto" }}>
        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "0.85rem" }}>
          <thead>
            <tr>
              {result.columns.map((col) => (
                <th
                  key={col}
                  style={{
                    textAlign: "left", padding: "0.6rem 0.75rem", backgroundColor: "#f9fafb",
                    borderBottom: "2px solid #e5e7eb", fontWeight: 600, color: "#374151",
                    whiteSpace: "nowrap", position: "sticky", top: 0,
                  }}
                >
                  {col.replace(/_/g, " ")}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {result.rows.map((row, i) => (
              <tr key={i} style={{ backgroundColor: i % 2 === 0 ? "#ffffff" : "#f9fafb" }}>
                {result.columns.map((col) => {
                  const raw = formatCell(row[col]);
                  return (
                    <td
                      key={col}
                      title={raw.length > 100 ? raw : undefined}
                      style={{
                        padding: "0.5rem 0.75rem", borderBottom: "1px solid #e5e7eb",
                        color: "#374151", maxWidth: "350px", lineHeight: 1.4,
                      }}
                    >
                      {truncate(raw, 120)}
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    )}
  </div>
);

// ─── Main page ───────────────────────────────────────────────────────────────

const QueriesPage: React.FC = () => {
  const [categories, setCategories] = useState<QueryCategory[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Query execution state
  const [runningId, setRunningId] = useState<string | null>(null);
  const [result, setResult] = useState<QueryResultResponse | null>(null);
  const [runError, setRunError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    getQueries()
      .then((data) => { if (active) setCategories(data.categories); })
      .catch(() => { if (active) setError("Failed to load queries"); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, []);

  const handleRun = async (queryId: string) => {
    setRunningId(queryId);
    setResult(null);
    setRunError(null);
    try {
      const res = await runQuery(queryId);
      setResult(res);
    } catch (err) {
      setRunError(err instanceof Error ? err.message : "Query execution failed");
    } finally {
      setRunningId(null);
    }
  };

  if (loading) {
    return <div style={{ padding: "2rem", textAlign: "center", color: "#6b7280" }}>Loading queries...</div>;
  }
  if (error) {
    return (
      <div style={{ padding: "1rem", backgroundColor: "#fef2f2", border: "1px solid #fecaca", borderRadius: "6px", color: "#dc2626" }}>
        {error}
      </div>
    );
  }

  return (
    <div style={{ maxWidth: "1100px" }}>
      <Breadcrumb items={[{ label: "Dashboard", to: "/" }, { label: "Quick Queries" }]} />

      <h1 style={{ marginBottom: "0.25rem" }}>Quick Queries</h1>
      <p style={{ color: "#6b7280", fontSize: "0.9rem", marginTop: 0, marginBottom: "1.5rem" }}>
        Pre-built analytical queries — click to run
      </p>

      {/* Run error banner */}
      {runError && (
        <div style={{
          padding: "0.75rem 1rem", backgroundColor: "#fef2f2", border: "1px solid #fecaca",
          borderRadius: "6px", color: "#dc2626", marginBottom: "1rem",
        }}>
          {runError}
        </div>
      )}

      {/* Category sections */}
      {categories.map((cat) => (
        <div key={cat.name} style={{ marginBottom: "1.5rem" }}>
          <div style={{ marginBottom: "0.75rem" }}>
            <h3 style={{
              margin: 0, fontSize: "0.75rem", fontWeight: 700, textTransform: "uppercase",
              letterSpacing: "0.05em", color: "#6b7280",
            }}>
              {cat.name}
            </h3>
            <span style={{ fontSize: "0.85rem", color: "#9ca3af" }}>{cat.description}</span>
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(300px, 1fr))", gap: "0.75rem" }}>
            {cat.queries.map((q) => {
              const isRunning = runningId === q.id;
              return (
                <div
                  key={q.id}
                  style={{
                    padding: "1rem", backgroundColor: "#fff", border: "1px solid #e5e7eb",
                    borderRadius: "8px", display: "flex", flexDirection: "column",
                    justifyContent: "space-between", minHeight: "110px",
                  }}
                >
                  <div>
                    <div style={{ fontWeight: 600, fontSize: "0.95rem", color: "#1f2937", marginBottom: "0.35rem" }}>
                      {q.title}
                    </div>
                    <div style={{ fontSize: "0.8rem", color: "#6b7280", lineHeight: 1.4 }}>
                      {q.description}
                    </div>
                  </div>
                  <button
                    disabled={isRunning || runningId !== null}
                    onClick={() => handleRun(q.id)}
                    style={{
                      marginTop: "0.75rem", alignSelf: "flex-start",
                      padding: "0.35rem 0.9rem", backgroundColor: isRunning ? "#dbeafe" : "#2563eb",
                      color: isRunning ? "#1e40af" : "#fff", border: "none", borderRadius: "6px",
                      cursor: isRunning || runningId !== null ? "not-allowed" : "pointer",
                      fontSize: "0.8rem", fontWeight: 600,
                      opacity: runningId !== null && !isRunning ? 0.5 : 1,
                    }}
                  >
                    {isRunning ? "Running\u2026" : "Run \u2192"}
                  </button>
                </div>
              );
            })}
          </div>
        </div>
      ))}

      {/* Results section */}
      {result && <ResultsTable result={result} onClear={() => setResult(null)} />}
    </div>
  );
};

export default QueriesPage;
