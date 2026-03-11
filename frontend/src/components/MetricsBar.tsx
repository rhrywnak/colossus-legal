import React from "react";
import { AskResponse } from "../services/ask";

/** Compact metrics bar showing retrieval stats and timing breakdown. */
const MetricsBar: React.FC<{ response: AskResponse }> = ({ response }) => {
  const s = response.retrieval_stats;
  const totalSec = (s.total_ms / 1000).toFixed(1);
  return (
    <div style={{ marginBottom: "1rem" }}>
      <div style={{
        display: "flex", flexWrap: "wrap", gap: "0.5rem", alignItems: "center",
        padding: "0.6rem 1rem", backgroundColor: "#f3f4f6", borderRadius: "6px 6px 0 0",
        fontSize: "0.85rem", color: "#374151",
      }}>
        <span style={{ padding: "0.15rem 0.5rem", backgroundColor: "#dbeafe", borderRadius: "4px", fontWeight: 600 }}>
          {s.qdrant_hits} evidence hits
        </span>
        <span style={{ color: "#9ca3af" }}>&rarr;</span>
        <span style={{ padding: "0.15rem 0.5rem", backgroundColor: "#d1fae5", borderRadius: "4px", fontWeight: 600 }}>
          {s.graph_nodes_expanded} nodes expanded
        </span>
        <span style={{ color: "#9ca3af" }}>&rarr;</span>
        <span>answered in <strong>{totalSec}s</strong> by {response.provider}</span>
      </div>
      <div style={{
        display: "flex", flexWrap: "wrap", gap: "1.25rem", fontSize: "0.78rem",
        color: "#6b7280", padding: "0.35rem 1rem", backgroundColor: "#f9fafb",
        borderRadius: "0 0 6px 6px", borderTop: "1px solid #e5e7eb",
      }}>
        <span>Search: {s.search_ms}ms</span>
        <span>Expand: {s.expand_ms}ms</span>
        <span>Synthesis: {s.synthesis_ms}ms</span>
        <span>Context: ~{s.context_tokens.toLocaleString()} tokens</span>
      </div>
    </div>
  );
};

export default MetricsBar;
