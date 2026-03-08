import React from "react";
import { AskResponse } from "../services/ask";
import MarkdownAnswer from "./MarkdownAnswer";
import ExportButtons from "./ExportButtons";

interface Props {
  response: AskResponse;
}

const AnswerDisplay: React.FC<Props> = ({ response }) => {
  const stats = response.retrieval_stats;
  const totalSeconds = (stats.total_ms / 1000).toFixed(1);

  return (
    <div>
      {/* Answer text with markdown rendering */}
      <div style={{
        padding: "1.5rem", backgroundColor: "#ffffff", borderRadius: "8px",
        border: "1px solid #e5e7eb", marginBottom: "1rem",
      }}>
        <MarkdownAnswer content={response.answer} />
        <div style={{ marginTop: "1rem", paddingTop: "0.75rem", borderTop: "1px solid #f3f4f6" }}>
          <ExportButtons markdown={response.answer} question={response.question} />
        </div>
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

export default AnswerDisplay;
