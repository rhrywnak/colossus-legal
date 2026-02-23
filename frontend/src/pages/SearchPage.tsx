import React, { useCallback, useEffect, useState } from "react";
import { Link, useSearchParams } from "react-router-dom";
import { semanticSearch, SearchHit } from "../services/search";

// Node type colors for badge styling
const NODE_TYPE_COLORS: Record<string, { bg: string; text: string }> = {
  Evidence: { bg: "#dbeafe", text: "#1d4ed8" },
  ComplaintAllegation: { bg: "#fef3c7", text: "#92400e" },
  MotionClaim: { bg: "#e0e7ff", text: "#3730a3" },
  Document: { bg: "#d1fae5", text: "#065f46" },
  Person: { bg: "#fce7f3", text: "#9d174d" },
  Organization: { bg: "#f3e8ff", text: "#6b21a8" },
  Harm: { bg: "#fee2e2", text: "#991b1b" },
};
const DEFAULT_COLOR = { bg: "#f3f4f6", text: "#374151" };

// Display-friendly labels for filter chips
const NODE_TYPE_LABELS: Record<string, string> = {
  Evidence: "Evidence",
  ComplaintAllegation: "Allegations",
  MotionClaim: "Claims",
  Document: "Documents",
  Person: "People",
  Organization: "Organizations",
  Harm: "Harms",
};
const ALL_NODE_TYPES = Object.keys(NODE_TYPE_LABELS);

function getDetailLink(hit: SearchHit): string | null {
  switch (hit.node_type) {
    case "Evidence": return "/evidence";
    case "ComplaintAllegation": return `/allegations/${hit.node_id}/detail`;
    case "Document": return hit.document_id ? `/documents/${hit.document_id}` : "/documents";
    case "Person": return `/people/${hit.node_id}`;
    case "Harm": return "/damages";
    case "MotionClaim": return "/claims";
    case "Organization": return "/people";
    default: return null;
  }
}

const SearchPage: React.FC = () => {
  const [searchParams, setSearchParams] = useSearchParams();
  const queryFromUrl = searchParams.get("q") || "";
  const typesFromUrl = searchParams.get("types")?.split(",").filter(Boolean) || [];

  const [inputValue, setInputValue] = useState(queryFromUrl);
  const [activeTypes, setActiveTypes] = useState<string[]>(typesFromUrl);
  const [results, setResults] = useState<SearchHit[]>([]);
  const [total, setTotal] = useState(0);
  const [durationMs, setDurationMs] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [searched, setSearched] = useState(false);

  const runSearch = useCallback(async (q: string, types: string[]) => {
    if (!q.trim()) return;
    setLoading(true);
    setError(null);
    try {
      const resp = await semanticSearch(q, 20, types.length > 0 ? types : undefined);
      setResults(resp.results);
      setTotal(resp.total);
      setDurationMs(resp.duration_ms);
      setSearched(true);
    } catch {
      setError("Search failed. Is the backend running?");
      setResults([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (queryFromUrl) {
      setInputValue(queryFromUrl);
      setActiveTypes(typesFromUrl);
      runSearch(queryFromUrl, typesFromUrl);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [queryFromUrl, searchParams.get("types")]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!inputValue.trim()) return;
    const params: Record<string, string> = { q: inputValue.trim() };
    if (activeTypes.length > 0) params.types = activeTypes.join(",");
    setSearchParams(params);
  };

  const toggleType = (type: string) => {
    const next = activeTypes.includes(type)
      ? activeTypes.filter((t) => t !== type)
      : [...activeTypes, type];
    setActiveTypes(next);
    if (queryFromUrl) {
      const params: Record<string, string> = { q: queryFromUrl };
      if (next.length > 0) params.types = next.join(",");
      setSearchParams(params);
    }
  };

  return (
    <div>
      <h1 style={{ marginBottom: "1rem" }}>Semantic Search</h1>

      {/* Search form */}
      <form onSubmit={handleSubmit} style={{ marginBottom: "1rem" }}>
        <div style={{ display: "flex", gap: "0.5rem" }}>
          <input
            type="text"
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            placeholder="Search by meaning... e.g. 'bankruptcy', 'discredit Marie'"
            style={{
              flex: 1, padding: "0.6rem 1rem", border: "1px solid #d1d5db",
              borderRadius: "6px", fontSize: "0.95rem", outline: "none",
            }}
          />
          <button
            type="submit"
            style={{
              padding: "0.6rem 1.25rem", backgroundColor: "#2563eb", color: "#fff",
              border: "none", borderRadius: "6px", fontSize: "0.95rem",
              cursor: "pointer", fontWeight: 500,
            }}
          >
            Search
          </button>
        </div>
      </form>

      {/* Type filter chips */}
      <div style={{ display: "flex", flexWrap: "wrap", gap: "0.5rem", marginBottom: "1.5rem" }}>
        {ALL_NODE_TYPES.map((type) => {
          const active = activeTypes.includes(type);
          const colors = NODE_TYPE_COLORS[type] || DEFAULT_COLOR;
          return (
            <button
              key={type}
              onClick={() => toggleType(type)}
              style={{
                padding: "0.3rem 0.75rem", borderRadius: "16px",
                border: active ? `2px solid ${colors.text}` : "2px solid #e5e7eb",
                backgroundColor: active ? colors.bg : "#fff",
                color: active ? colors.text : "#6b7280",
                fontSize: "0.8rem", fontWeight: active ? 600 : 400, cursor: "pointer",
              }}
            >
              {NODE_TYPE_LABELS[type]}
            </button>
          );
        })}
      </div>

      {/* Loading */}
      {loading && (
        <div style={{ padding: "2rem", textAlign: "center", color: "#6b7280" }}>
          Searching...
        </div>
      )}

      {/* Error */}
      {error && (
        <div style={{
          padding: "1rem", backgroundColor: "#fef2f2",
          border: "1px solid #fecaca", borderRadius: "6px", color: "#dc2626",
        }}>
          {error}
        </div>
      )}

      {/* Results summary */}
      {!loading && searched && !error && (
        <div style={{
          padding: "0.75rem 1rem", backgroundColor: "#f3f4f6",
          borderRadius: "6px", marginBottom: "1rem", color: "#374151",
        }}>
          Found <strong>{total}</strong> result{total !== 1 ? "s" : ""} in{" "}
          <strong>{durationMs}ms</strong>
        </div>
      )}

      {/* Empty state */}
      {!loading && searched && !error && results.length === 0 && (
        <div style={{ color: "#6b7280", padding: "1rem" }}>
          No results found for &ldquo;{queryFromUrl}&rdquo;
        </div>
      )}

      {/* Results list */}
      {!loading && results.length > 0 && (
        <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
          {results.map((hit, idx) => {
            const colors = NODE_TYPE_COLORS[hit.node_type] || DEFAULT_COLOR;
            const link = getDetailLink(hit);
            const scorePercent = Math.round(hit.score * 100);
            return (
              <div
                key={`${hit.node_id}-${idx}`}
                style={{
                  padding: "1rem", backgroundColor: "#fff",
                  border: "1px solid #e5e7eb", borderRadius: "8px",
                }}
              >
                <div style={{
                  display: "flex", alignItems: "center",
                  gap: "0.5rem", marginBottom: "0.5rem",
                }}>
                  <span style={{
                    padding: "0.2rem 0.5rem", backgroundColor: colors.bg,
                    color: colors.text, borderRadius: "4px",
                    fontSize: "0.75rem", fontWeight: 600,
                  }}>
                    {hit.node_type}
                  </span>
                  <span style={{ fontSize: "0.75rem", color: "#6b7280" }}>
                    {scorePercent}% match
                  </span>
                  {hit.page_number && (
                    <span style={{
                      padding: "0.1rem 0.4rem", backgroundColor: "#dbeafe",
                      color: "#1e40af", borderRadius: "3px",
                      fontSize: "0.75rem", fontWeight: 600,
                    }}>
                      p. {hit.page_number}
                    </span>
                  )}
                </div>
                <div style={{ fontWeight: 600, fontSize: "1rem" }}>
                  {link ? (
                    <Link to={link} style={{ color: "#2563eb", textDecoration: "none" }}>
                      {hit.title || hit.node_id}
                    </Link>
                  ) : (
                    hit.title || hit.node_id
                  )}
                </div>
                <div style={{ fontSize: "0.75rem", color: "#9ca3af", marginTop: "0.25rem" }}>
                  {hit.node_id}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};

export default SearchPage;
