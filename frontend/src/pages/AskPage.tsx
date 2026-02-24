import React, { useEffect, useRef, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { askTheCase, AskResponse } from "../services/ask";

const AskPage: React.FC = () => {
  const [searchParams] = useSearchParams();
  const initialQuestion = searchParams.get("q") || "";

  const [question, setQuestion] = useState(initialQuestion);
  const [response, setResponse] = useState<AskResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [phase, setPhase] = useState("");

  // Auto-submit when arriving with a ?q= parameter
  const autoSubmitted = useRef(false);

  // Core submit logic — reused by form handler and auto-submit effect
  const submitQuestion = async (q: string) => {
    if (!q.trim() || loading) return;

    setLoading(true);
    setError(null);
    setResponse(null);

    // Simulate pipeline phase labels while waiting for the single API call
    setPhase("Embedding question...");
    const t1 = setTimeout(() => setPhase("Searching evidence..."), 800);
    const t2 = setTimeout(() => setPhase("Expanding graph context..."), 2000);
    const t3 = setTimeout(() => setPhase("Synthesizing answer..."), 3500);

    try {
      const result = await askTheCase(q.trim());
      setResponse(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unknown error");
    } finally {
      clearTimeout(t1);
      clearTimeout(t2);
      clearTimeout(t3);
      setLoading(false);
      setPhase("");
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    submitQuestion(question);
  };

  // If the page loaded with ?q=..., auto-submit once
  useEffect(() => {
    if (initialQuestion && !autoSubmitted.current) {
      autoSubmitted.current = true;
      submitQuestion(initialQuestion);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [initialQuestion]);

  const handleClear = () => {
    setQuestion("");
    setResponse(null);
    setError(null);
  };

  return (
    <div style={{ maxWidth: "900px", margin: "0 auto" }}>
      {/* Header */}
      <div style={{ marginBottom: "1.5rem" }}>
        <h1 style={{ margin: 0, fontSize: "1.75rem" }}>Ask the Case</h1>
        <p style={{ margin: "0.25rem 0 0", color: "#6b7280", fontSize: "0.9rem" }}>
          Minerva — AI-powered case analysis with cited evidence
        </p>
      </div>

      {/* Question form */}
      <form onSubmit={handleSubmit} style={{ marginBottom: "1.5rem" }}>
        <textarea
          value={question}
          onChange={(e) => setQuestion(e.target.value)}
          placeholder='Ask a question about the case... e.g. "What did Phillips say about Marie&#39;s bankruptcy?"'
          rows={3}
          style={{
            width: "100%", padding: "0.75rem 1rem", border: "1px solid #d1d5db",
            borderRadius: "8px", fontSize: "1rem", fontFamily: "inherit",
            resize: "vertical", outline: "none", boxSizing: "border-box",
          }}
        />
        <div style={{ display: "flex", gap: "0.5rem", marginTop: "0.5rem" }}>
          <button
            type="submit"
            disabled={loading || !question.trim()}
            style={{
              padding: "0.6rem 1.5rem", backgroundColor: loading ? "#93c5fd" : "#2563eb",
              color: "#fff", border: "none", borderRadius: "6px", fontSize: "0.95rem",
              cursor: loading ? "wait" : "pointer", fontWeight: 600,
            }}
          >
            {loading ? "Thinking..." : "Ask"}
          </button>
          {response && (
            <button
              type="button"
              onClick={handleClear}
              style={{
                padding: "0.6rem 1.25rem", backgroundColor: "#f3f4f6",
                color: "#374151", border: "1px solid #d1d5db", borderRadius: "6px",
                fontSize: "0.95rem", cursor: "pointer",
              }}
            >
              Ask another question
            </button>
          )}
        </div>
      </form>

      {/* Loading phase indicator */}
      {loading && phase && (
        <div style={{
          padding: "1.5rem", textAlign: "center", color: "#2563eb",
          backgroundColor: "#eff6ff", borderRadius: "8px", marginBottom: "1rem",
        }}>
          <div style={{ fontSize: "1.1rem", fontWeight: 500, marginBottom: "0.25rem" }}>
            {phase}
          </div>
          <div style={{ fontSize: "0.8rem", color: "#6b7280" }}>
            This usually takes 5-15 seconds
          </div>
        </div>
      )}

      {/* Error */}
      {error && (
        <div style={{
          padding: "1rem", backgroundColor: "#fef2f2", border: "1px solid #fecaca",
          borderRadius: "8px", color: "#dc2626", marginBottom: "1rem",
        }}>
          {error}
        </div>
      )}

      {/* Answer display */}
      {response && <AnswerDisplay response={response} />}
    </div>
  );
};

// ---------------------------------------------------------------------------
// Answer display component
// ---------------------------------------------------------------------------

const AnswerDisplay: React.FC<{ response: AskResponse }> = ({ response }) => {
  const stats = response.retrieval_stats;
  const totalSeconds = (stats.total_ms / 1000).toFixed(1);

  return (
    <div>
      {/* Answer text */}
      <div style={{
        padding: "1.5rem", backgroundColor: "#f9fafb", borderRadius: "8px",
        border: "1px solid #e5e7eb", lineHeight: 1.7, marginBottom: "1rem",
      }}>
        {response.answer.split("\n\n").map((paragraph, idx) => (
          <p key={idx} style={{ margin: idx === 0 ? 0 : "1rem 0 0" }}>
            {paragraph.split("\n").map((line, lineIdx) => (
              <React.Fragment key={lineIdx}>
                {lineIdx > 0 && <br />}
                {line}
              </React.Fragment>
            ))}
          </p>
        ))}
      </div>

      {/* Stats bar */}
      <div style={{
        display: "flex", flexWrap: "wrap", gap: "0.5rem", alignItems: "center",
        padding: "0.75rem 1rem", backgroundColor: "#f3f4f6", borderRadius: "6px",
        fontSize: "0.85rem", color: "#374151", marginBottom: "0.5rem",
      }}>
        <StatBadge label="Evidence hits" value={stats.qdrant_hits} color="#dbeafe" />
        <span style={{ color: "#9ca3af" }}>→</span>
        <StatBadge label="Nodes expanded" value={stats.graph_nodes_expanded} color="#d1fae5" />
        <span style={{ color: "#9ca3af" }}>→</span>
        <span>answered in <strong>{totalSeconds}s</strong> by {response.provider}</span>
      </div>

      {/* Timing breakdown + tokens */}
      <div style={{
        display: "flex", flexWrap: "wrap", gap: "1.5rem", fontSize: "0.8rem",
        color: "#6b7280", padding: "0 0.25rem",
      }}>
        <span>Search: {stats.search_ms}ms</span>
        <span>Expand: {stats.expand_ms}ms</span>
        <span>Synthesis: {stats.synthesis_ms}ms</span>
        <span>Context: ~{stats.context_tokens.toLocaleString()} tokens</span>
        <span>
          Tokens: {stats.input_tokens.toLocaleString()} in / {stats.output_tokens.toLocaleString()} out
        </span>
      </div>
    </div>
  );
};

// Small stat badge used in the stats bar
const StatBadge: React.FC<{ label: string; value: number; color: string }> = ({
  label, value, color,
}) => (
  <span style={{
    padding: "0.2rem 0.6rem", backgroundColor: color, borderRadius: "4px", fontWeight: 600,
  }}>
    {value} {label.toLowerCase()}
  </span>
);

export default AskPage;
