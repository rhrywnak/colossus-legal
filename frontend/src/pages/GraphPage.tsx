import React, { useEffect, useState, useRef } from "react";
import dagre from "dagre";
import {
  getLegalProofGraph,
  GraphNode,
  GraphEdge,
  GraphNodeType,
  GraphResponse,
} from "../services/graph";

// Node colors by type
const NODE_COLORS: Record<GraphNodeType, { bg: string; border: string; text: string }> = {
  legal_count: { bg: "#dbeafe", border: "#3b82f6", text: "#1e40af" },
  allegation: { bg: "#dcfce7", border: "#22c55e", text: "#166534" },
  motion_claim: { bg: "#fef3c7", border: "#eab308", text: "#854d0e" },
  evidence: { bg: "#f3e8ff", border: "#a855f7", text: "#6b21a8" },
  document: { bg: "#f3f4f6", border: "#6b7280", text: "#374151" },
};

const NODE_WIDTH = 180;
const NODE_HEIGHT = 60;

type LayoutNode = GraphNode & { x: number; y: number };
type LayoutEdge = GraphEdge & { points: { x: number; y: number }[] };

function computeLayout(
  nodes: GraphNode[],
  edges: GraphEdge[]
): { nodes: LayoutNode[]; edges: LayoutEdge[]; width: number; height: number } {
  const g = new dagre.graphlib.Graph();

  g.setGraph({
    rankdir: "TB",
    nodesep: 40,
    ranksep: 80,
    marginx: 20,
    marginy: 20,
  });

  g.setDefaultEdgeLabel(() => ({}));

  // Add nodes
  nodes.forEach((node) => {
    g.setNode(node.id, { width: NODE_WIDTH, height: NODE_HEIGHT });
  });

  // Add edges
  edges.forEach((edge) => {
    g.setEdge(edge.source, edge.target);
  });

  dagre.layout(g);

  // Extract positioned nodes
  const layoutNodes: LayoutNode[] = nodes.map((node) => {
    const layoutNode = g.node(node.id);
    return {
      ...node,
      x: layoutNode?.x ?? 0,
      y: layoutNode?.y ?? 0,
    };
  });

  // Extract edge points
  const layoutEdges: LayoutEdge[] = edges.map((edge) => {
    const dagreEdge = g.edge(edge.source, edge.target);
    return {
      ...edge,
      points: dagreEdge?.points ?? [],
    };
  });

  const graphMeta = g.graph();
  const width = (graphMeta.width ?? 800) + 40;
  const height = (graphMeta.height ?? 600) + 40;

  return { nodes: layoutNodes, edges: layoutEdges, width, height };
}

function truncateLabel(label: string, maxLen: number = 22): string {
  if (label.length <= maxLen) return label;
  return label.substring(0, maxLen - 1) + "...";
}

const GraphPage: React.FC = () => {
  const [graphData, setGraphData] = useState<GraphResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [countFilter, setCountFilter] = useState<string>("");
  const svgRef = useRef<SVGSVGElement>(null);

  useEffect(() => {
    let active = true;

    const fetchGraph = async () => {
      setLoading(true);
      try {
        const data = await getLegalProofGraph(countFilter || undefined);
        if (!active) return;
        setGraphData(data);
        setError(null);
      } catch {
        if (!active) return;
        setGraphData(null);
        setError("Failed to load graph data");
      } finally {
        if (active) setLoading(false);
      }
    };

    fetchGraph();

    return () => {
      active = false;
    };
  }, [countFilter]);

  // Extract unique legal counts for filter dropdown
  const legalCounts = graphData
    ? graphData.nodes.filter((n) => n.node_type === "legal_count")
    : [];

  if (loading) {
    return (
      <div style={{ padding: "2rem", textAlign: "center", color: "#6b7280" }}>
        Loading graph...
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

  if (!graphData || graphData.nodes.length === 0) {
    return (
      <div>
        <h1 style={{ marginBottom: "1rem" }}>Legal Proof Graph</h1>
        <div style={{ color: "#6b7280", padding: "1rem" }}>
          No graph data available.
        </div>
      </div>
    );
  }

  const { nodes, edges, width, height } = computeLayout(
    graphData.nodes,
    graphData.edges
  );

  return (
    <div>
      <h1 style={{ marginBottom: "0.5rem" }}>Legal Proof Graph</h1>

      {/* Filter controls */}
      <div
        style={{
          padding: "0.75rem 1rem",
          backgroundColor: "#f3f4f6",
          borderRadius: "6px",
          marginBottom: "1rem",
          display: "flex",
          alignItems: "center",
          gap: "1rem",
          flexWrap: "wrap",
        }}
      >
        <label style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
          <span style={{ color: "#374151", fontWeight: 500 }}>Filter by Count:</span>
          <select
            value={countFilter}
            onChange={(e) => setCountFilter(e.target.value)}
            style={{
              padding: "0.35rem 0.5rem",
              borderRadius: "4px",
              border: "1px solid #d1d5db",
              backgroundColor: "#fff",
            }}
          >
            <option value="">All Counts</option>
            {legalCounts.map((c) => (
              <option key={c.id} value={c.id}>
                {c.label}
              </option>
            ))}
          </select>
        </label>

        <div style={{ display: "flex", gap: "0.75rem", flexWrap: "wrap" }}>
          <span style={{ fontSize: "0.8rem", color: "#6b7280" }}>
            {graphData.nodes.length} nodes, {graphData.edges.length} edges
          </span>
        </div>
      </div>

      {/* Legend */}
      <div
        style={{
          display: "flex",
          gap: "1rem",
          marginBottom: "1rem",
          flexWrap: "wrap",
        }}
      >
        {(Object.keys(NODE_COLORS) as GraphNodeType[]).map((type) => (
          <div
            key={type}
            style={{
              display: "flex",
              alignItems: "center",
              gap: "0.35rem",
              fontSize: "0.8rem",
            }}
          >
            <div
              style={{
                width: "14px",
                height: "14px",
                backgroundColor: NODE_COLORS[type].bg,
                border: `2px solid ${NODE_COLORS[type].border}`,
                borderRadius: "3px",
              }}
            />
            <span style={{ color: "#4b5563" }}>
              {type.replace("_", " ").replace(/\b\w/g, (c) => c.toUpperCase())}
            </span>
          </div>
        ))}
      </div>

      {/* SVG Graph */}
      <div
        style={{
          border: "1px solid #e5e7eb",
          borderRadius: "8px",
          backgroundColor: "#fafafa",
          overflow: "auto",
          maxHeight: "70vh",
        }}
      >
        <svg ref={svgRef} width={width} height={height}>
          {/* Edges */}
          {edges.map((edge, i) => {
            if (edge.points.length < 2) return null;
            const pathData = edge.points
              .map((p, idx) => `${idx === 0 ? "M" : "L"} ${p.x} ${p.y}`)
              .join(" ");
            return (
              <g key={`edge-${i}`}>
                <path
                  d={pathData}
                  fill="none"
                  stroke="#9ca3af"
                  strokeWidth={1.5}
                  markerEnd="url(#arrowhead)"
                />
              </g>
            );
          })}

          {/* Arrow marker definition */}
          <defs>
            <marker
              id="arrowhead"
              markerWidth="10"
              markerHeight="7"
              refX="9"
              refY="3.5"
              orient="auto"
            >
              <polygon points="0 0, 10 3.5, 0 7" fill="#9ca3af" />
            </marker>
          </defs>

          {/* Nodes */}
          {nodes.map((node) => {
            const colors = NODE_COLORS[node.node_type];
            const x = node.x - NODE_WIDTH / 2;
            const y = node.y - NODE_HEIGHT / 2;

            return (
              <g key={node.id}>
                <rect
                  x={x}
                  y={y}
                  width={NODE_WIDTH}
                  height={NODE_HEIGHT}
                  rx={6}
                  ry={6}
                  fill={colors.bg}
                  stroke={colors.border}
                  strokeWidth={2}
                />
                <text
                  x={node.x}
                  y={node.y - 6}
                  textAnchor="middle"
                  fill={colors.text}
                  fontSize="11"
                  fontWeight="600"
                  fontFamily="Inter, system-ui, sans-serif"
                >
                  {truncateLabel(node.label)}
                </text>
                {node.subtitle && (
                  <text
                    x={node.x}
                    y={node.y + 10}
                    textAnchor="middle"
                    fill={colors.text}
                    fontSize="9"
                    fontFamily="Inter, system-ui, sans-serif"
                    opacity={0.8}
                  >
                    {node.subtitle}
                  </text>
                )}
              </g>
            );
          })}
        </svg>
      </div>
    </div>
  );
};

export default GraphPage;
