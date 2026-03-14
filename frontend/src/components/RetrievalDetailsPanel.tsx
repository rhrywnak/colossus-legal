import React, { useState } from "react";
import { AskResponse, RetrievalDetail } from "../services/ask";

const NODE_TYPE_COLORS: Record<string, { bg: string; text: string }> = {
  Evidence:            { bg: "#dbeafe", text: "#1e40af" },
  ComplaintAllegation: { bg: "#ede9fe", text: "#5b21b6" },
  MotionClaim:         { bg: "#fef3c7", text: "#92400e" },
  Harm:                { bg: "#fee2e2", text: "#991b1b" },
  LegalCount:          { bg: "#d1fae5", text: "#065f46" },
  Document:            { bg: "#e0e7ff", text: "#3730a3" },
};
const DEFAULT_COLOR = { bg: "#f3f4f6", text: "#374151" };

/** Truncate a string to maxLen chars, adding ellipsis if needed. */
function truncate(s: string, maxLen: number): string {
  return s.length > maxLen ? s.slice(0, maxLen - 1) + "\u2026" : s;
}

/** Small colored pill for the node type. */
const NodeBadge: React.FC<{ nodeType: string }> = ({ nodeType }) => {
  const color = NODE_TYPE_COLORS[nodeType] || DEFAULT_COLOR;
  return (
    <span style={{
      display: "inline-block", padding: "0.1rem 0.45rem", borderRadius: "4px",
      fontSize: "0.72rem", fontWeight: 600, whiteSpace: "nowrap",
      backgroundColor: color.bg, color: color.text,
    }}>
      {nodeType}
    </span>
  );
};

/** Inline score bar — width proportional to score, max 80px. */
const ScoreBar: React.FC<{ score: number }> = ({ score }) => (
  <div style={{
    display: "inline-flex", alignItems: "center", gap: "0.35rem",
  }}>
    <div style={{
      width: "80px", height: "6px", backgroundColor: "#e5e7eb",
      borderRadius: "3px", overflow: "hidden",
    }}>
      <div style={{
        width: `${Math.min(score, 1) * 100}%`, height: "100%",
        backgroundColor: "#3b82f6", borderRadius: "3px",
      }} />
    </div>
    <span style={{ fontSize: "0.75rem", color: "#6b7280", minWidth: "2rem" }}>
      {score.toFixed(2)}
    </span>
  </div>
);

/** A single retrieval detail row. */
const DetailRow: React.FC<{ d: RetrievalDetail }> = ({ d }) => {
  const source = [d.document_title, d.page_number != null ? `p.${d.page_number}` : null]
    .filter(Boolean).join(", ");

  return (
    <div style={{ padding: "0.3rem 0", borderBottom: "1px solid #f3f4f6" }}>
      <div style={{
        display: "flex", alignItems: "center", gap: "0.5rem", flexWrap: "wrap",
      }}>
        <NodeBadge nodeType={d.node_type} />
        <span style={{ fontSize: "0.82rem", color: "#111827", flex: 1, minWidth: 0 }}>
          {truncate(d.title, 60)}
        </span>
        {d.origin === "qdrant" && <ScoreBar score={d.score} />}
        {source && (
          <span style={{ fontSize: "0.75rem", color: "#6b7280", whiteSpace: "nowrap" }}>
            {source}
          </span>
        )}
      </div>
      {d.quote_preview && (
        <div style={{
          fontSize: "0.75rem", color: "#9ca3af", fontStyle: "italic",
          marginTop: "0.15rem", paddingLeft: "0.5rem",
        }}>
          "{d.quote_preview}"
        </div>
      )}
    </div>
  );
};

/** Section header for vector / graph groups. */
const SectionHeader: React.FC<{ label: string; count: number }> = ({ label, count }) => (
  <div style={{
    fontSize: "0.78rem", fontWeight: 600, color: "#374151",
    marginTop: "0.6rem", marginBottom: "0.25rem",
  }}>
    {label} ({count})
  </div>
);

/** Collapsible panel showing retrieval details from the RAG pipeline. */
const RetrievalDetailsPanel: React.FC<{ response: AskResponse }> = ({ response }) => {
  const [expanded, setExpanded] = useState(false);

  const details = response.retrieval_details;
  if (!details || details.length === 0) return null;

  const qdrantHits = details.filter(d => d.origin === "qdrant");
  const graphNodes = details.filter(d => d.origin === "graph");
  const strategy = response.strategy || "unknown";

  const arrow = expanded ? "\u25be" : "\u25b8";
  const summary = `${arrow} Retrieval details (${qdrantHits.length} vector hits + ${graphNodes.length} graph nodes) \u00b7 Strategy: ${strategy}`;

  return (
    <div style={{ marginBottom: "0.75rem" }}>
      {/* Toggle bar */}
      <div
        onClick={() => setExpanded(!expanded)}
        style={{
          padding: "0.5rem 1rem", backgroundColor: "#f9fafb",
          border: "1px solid #e5e7eb", borderRadius: expanded ? "6px 6px 0 0" : "6px",
          fontSize: "0.82rem", color: "#6b7280", cursor: "pointer",
          userSelect: "none",
        }}
      >
        {summary}
      </div>

      {/* Expanded content */}
      {expanded && (
        <div style={{
          padding: "0.5rem 1rem", border: "1px solid #e5e7eb", borderTop: "none",
          borderRadius: "0 0 6px 6px", backgroundColor: "#ffffff",
        }}>
          {qdrantHits.length > 0 && (
            <>
              <SectionHeader label="Vector Search Results" count={qdrantHits.length} />
              {qdrantHits.map(d => <DetailRow key={d.node_id} d={d} />)}
            </>
          )}
          {graphNodes.length > 0 && (
            <>
              <SectionHeader label="Graph Expansion" count={graphNodes.length} />
              {graphNodes.map(d => <DetailRow key={d.node_id} d={d} />)}
            </>
          )}
        </div>
      )}
    </div>
  );
};

export default RetrievalDetailsPanel;
