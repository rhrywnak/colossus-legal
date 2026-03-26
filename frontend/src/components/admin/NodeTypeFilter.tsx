/**
 * NodeTypeFilter — Dropdown to filter evidence cards by node type.
 *
 * Extracted from DocumentWorkspace to keep that page under 300 lines.
 * Computes type counts from the items array and renders a styled <select>.
 * Only shows node types that actually exist in the current data.
 */
import React, { useMemo } from "react";
import { DocumentEvidence } from "../../services/documentEvidence";
import { getNodeTypeDisplay } from "../../utils/nodeTypeDisplay";

interface NodeTypeFilterProps {
  items: DocumentEvidence[];
  filterType: string;
  onFilterChange: (type: string) => void;
}

const selectStyle: React.CSSProperties = {
  padding: "0.35rem 0.5rem",
  fontSize: "0.82rem",
  fontFamily: "inherit",
  border: "1px solid #e2e8f0",
  borderRadius: "6px",
  backgroundColor: "#fff",
  color: "#334155",
  cursor: "pointer",
  width: "100%",
  marginBottom: "0.5rem",
};

const NodeTypeFilter: React.FC<NodeTypeFilterProps> = ({
  items, filterType, onFilterChange,
}) => {
  const typeCounts = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const item of items) {
      const t = item.node_type || "Evidence";
      counts[t] = (counts[t] || 0) + 1;
    }
    return counts;
  }, [items]);

  const typeEntries = Object.entries(typeCounts).sort((a, b) => a[0].localeCompare(b[0]));

  if (typeEntries.length <= 1) return null;

  return (
    <select
      style={selectStyle}
      value={filterType}
      onChange={(e) => onFilterChange(e.target.value)}
    >
      <option value="all">All Types ({items.length})</option>
      {typeEntries.map(([nodeType, count]) => (
        <option key={nodeType} value={nodeType}>
          {getNodeTypeDisplay(nodeType).label} ({count})
        </option>
      ))}
    </select>
  );
};

export default NodeTypeFilter;
