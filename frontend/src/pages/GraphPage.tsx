import React, { useEffect, useState, useRef } from "react";
import dagre from "dagre";
import { getAllegations, AllegationDto } from "../services/allegations";
import {
  getEvidenceChain,
  EvidenceChainResponse,
} from "../services/evidenceChain";
import { GraphNode, GraphEdge, GraphNodeType } from "../services/graph";

// Professional color palette (matching Explorer page)
const COLORS = {
  bgPage: "#f8fafc",
  bgCard: "#ffffff",
  border: "#e2e8f0",
  textPrimary: "#1e293b",
  textSecondary: "#64748b",
  textMuted: "#94a3b8",
};

// Left border accent colors by node type
const ACCENT_COLORS: Record<GraphNodeType, string> = {
  legal_count: "#3b82f6",
  allegation: "#059669",
  motion_claim: "#3b82f6",
  evidence: "#8b5cf6",
  document: "#6b7280",
};

// Status badge colors
const STATUS_COLORS: Record<string, { bg: string; text: string }> = {
  PROVEN: { bg: "#ecfdf5", text: "#059669" },
  PARTIAL: { bg: "#fffbeb", text: "#d97706" },
  UNPROVEN: { bg: "#fef2f2", text: "#dc2626" },
};

const NODE_WIDTH = 220;
const NODE_HEIGHT = 70;
const ACCENT_WIDTH = 4;

type LayoutNode = GraphNode & { x: number; y: number };
type LayoutEdge = GraphEdge & { points: { x: number; y: number }[] };

function transformChainToGraph(
  chain: EvidenceChainResponse
): { nodes: GraphNode[]; edges: GraphEdge[] } {
  const nodes: GraphNode[] = [];
  const edges: GraphEdge[] = [];
  const documentIds = new Set<string>();

  // Add allegation node (top level)
  nodes.push({
    id: chain.allegation.id,
    label: chain.allegation.title,
    node_type: "allegation",
    subtitle: chain.allegation.evidence_status,
  });

  // Add motion claims and connect FROM allegation (for correct top-down layout)
  chain.motion_claims.forEach((mc) => {
    nodes.push({
      id: mc.id,
      label: mc.title,
      node_type: "motion_claim",
    });
    // Edge flows DOWN: allegation → motion_claim
    edges.push({
      source: chain.allegation.id,
      target: mc.id,
      relationship: "PROVED_BY",
    });

    // Add evidence and connect from motion claim
    mc.evidence.forEach((ev) => {
      nodes.push({
        id: ev.id,
        label: ev.title,
        node_type: "evidence",
      });
      // Edge flows DOWN: motion_claim → evidence
      edges.push({
        source: mc.id,
        target: ev.id,
        relationship: "RELIES_ON",
      });

      // Add document and connect from evidence (deduplicated)
      if (ev.document) {
        if (!documentIds.has(ev.document.id)) {
          documentIds.add(ev.document.id);
          nodes.push({
            id: ev.document.id,
            label: ev.document.title,
            node_type: "document",
            subtitle: ev.document.page_number
              ? `p. ${ev.document.page_number}`
              : undefined,
          });
        }
        // Edge flows DOWN: evidence → document
        edges.push({
          source: ev.id,
          target: ev.document.id,
          relationship: "SOURCED_FROM",
        });
      }
    });
  });

  return { nodes, edges };
}

function computeLayout(
  nodes: GraphNode[],
  edges: GraphEdge[]
): { nodes: LayoutNode[]; edges: LayoutEdge[]; width: number; height: number } {
  const g = new dagre.graphlib.Graph();

  g.setGraph({
    rankdir: "TB",
    nodesep: 50,
    ranksep: 90,
    marginx: 40,
    marginy: 40,
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
  const width = (graphMeta.width ?? 400) + 80;
  const height = (graphMeta.height ?? 300) + 80;

  return { nodes: layoutNodes, edges: layoutEdges, width, height };
}

function truncateLabel(label: string, maxLen: number = 28): string {
  if (label.length <= maxLen) return label;
  return label.substring(0, maxLen - 1) + "…";
}

const GraphPage: React.FC = () => {
  const [allegations, setAllegations] = useState<AllegationDto[]>([]);
  const [selectedAllegationId, setSelectedAllegationId] = useState<string | null>(null);
  const [evidenceChain, setEvidenceChain] = useState<EvidenceChainResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [chainLoading, setChainLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const svgRef = useRef<SVGSVGElement>(null);

  // Fetch allegations on mount
  useEffect(() => {
    let active = true;

    const fetchAllegations = async () => {
      try {
        const result = await getAllegations();
        if (!active) return;
        setAllegations(result.allegations);
        setError(null);
      } catch {
        if (!active) return;
        setAllegations([]);
        setError("Failed to load allegations");
      } finally {
        if (active) setLoading(false);
      }
    };

    fetchAllegations();

    return () => {
      active = false;
    };
  }, []);

  // Fetch evidence chain when selection changes
  useEffect(() => {
    if (!selectedAllegationId) {
      setEvidenceChain(null);
      return;
    }

    let active = true;

    const fetchChain = async () => {
      setChainLoading(true);
      try {
        const chain = await getEvidenceChain(selectedAllegationId);
        if (!active) return;
        setEvidenceChain(chain);
        setError(null);
      } catch {
        if (!active) return;
        setEvidenceChain(null);
        setError("Failed to load evidence chain");
      } finally {
        if (active) setChainLoading(false);
      }
    };

    fetchChain();

    return () => {
      active = false;
    };
  }, [selectedAllegationId]);

  // Transform evidence chain to graph data
  const graphData = evidenceChain ? transformChainToGraph(evidenceChain) : null;
  const layout = graphData ? computeLayout(graphData.nodes, graphData.edges) : null;

  if (loading) {
    return (
      <div
        style={{
          padding: "3rem",
          textAlign: "center",
          color: COLORS.textSecondary,
          backgroundColor: COLORS.bgPage,
          minHeight: "100vh",
        }}
      >
        Loading allegations...
      </div>
    );
  }

  return (
    <div
      style={{
        backgroundColor: COLORS.bgPage,
        minHeight: "100vh",
        padding: "2rem",
      }}
    >
      <div style={{ maxWidth: "1200px", margin: "0 auto" }}>
        {/* Header */}
        <h1
          style={{
            fontSize: "1.75rem",
            fontWeight: 600,
            color: COLORS.textPrimary,
            margin: 0,
          }}
        >
          Evidence Chain Graph
        </h1>
        <p
          style={{
            fontSize: "0.95rem",
            color: COLORS.textSecondary,
            marginTop: "0.25rem",
            marginBottom: 0,
          }}
        >
          Select an allegation to visualize its supporting evidence hierarchy
        </p>

        {/* Divider */}
        <div
          style={{
            borderBottom: `1px solid ${COLORS.border}`,
            margin: "1.5rem 0",
          }}
        />

        {/* Controls */}
        <div
          style={{
            backgroundColor: COLORS.bgCard,
            border: `1px solid ${COLORS.border}`,
            borderRadius: "8px",
            padding: "1rem 1.25rem",
            marginBottom: "1.5rem",
            display: "flex",
            alignItems: "center",
            gap: "1rem",
            flexWrap: "wrap",
          }}
        >
          <label
            style={{
              display: "flex",
              alignItems: "center",
              gap: "0.75rem",
              flex: 1,
              minWidth: "300px",
            }}
          >
            <span
              style={{
                color: COLORS.textPrimary,
                fontWeight: 500,
                fontSize: "0.95rem",
                whiteSpace: "nowrap",
              }}
            >
              Allegation:
            </span>
            <select
              value={selectedAllegationId || ""}
              onChange={(e) => setSelectedAllegationId(e.target.value || null)}
              style={{
                flex: 1,
                padding: "0.5rem 0.75rem",
                borderRadius: "6px",
                border: `1px solid ${COLORS.border}`,
                backgroundColor: COLORS.bgCard,
                fontSize: "0.9rem",
                color: COLORS.textPrimary,
                cursor: "pointer",
              }}
            >
              <option value="">Select an allegation...</option>
              {allegations.map((a) => (
                <option key={a.id} value={a.id}>
                  {a.id}: {a.title}
                </option>
              ))}
            </select>
          </label>

          {/* Node/edge count */}
          {graphData && (
            <span
              style={{
                fontSize: "0.85rem",
                color: COLORS.textSecondary,
                whiteSpace: "nowrap",
              }}
            >
              {graphData.nodes.length} nodes · {graphData.edges.length} edges
            </span>
          )}
        </div>

        {/* Error state */}
        {error && (
          <div
            style={{
              padding: "1rem 1.25rem",
              backgroundColor: "#fef2f2",
              border: `1px solid ${COLORS.border}`,
              borderRadius: "8px",
              color: "#dc2626",
              marginBottom: "1.5rem",
            }}
          >
            {error}
          </div>
        )}

        {/* Empty state - no selection */}
        {!selectedAllegationId && (
          <div
            style={{
              backgroundColor: COLORS.bgCard,
              border: `1px solid ${COLORS.border}`,
              borderRadius: "8px",
              padding: "4rem 2rem",
              textAlign: "center",
            }}
          >
            <div
              style={{
                fontSize: "1.1rem",
                color: COLORS.textMuted,
                marginBottom: "0.5rem",
              }}
            >
              Select an allegation above to see its evidence chain
            </div>
            <div style={{ fontSize: "0.9rem", color: COLORS.textMuted }}>
              The graph will show the hierarchy: Allegation → Motion Claims → Evidence → Documents
            </div>
          </div>
        )}

        {/* Loading chain */}
        {chainLoading && (
          <div
            style={{
              backgroundColor: COLORS.bgCard,
              border: `1px solid ${COLORS.border}`,
              borderRadius: "8px",
              padding: "4rem 2rem",
              textAlign: "center",
              color: COLORS.textSecondary,
            }}
          >
            Loading evidence chain...
          </div>
        )}

        {/* Graph visualization */}
        {!chainLoading && layout && (
          <>
            {/* Legend */}
            <div
              style={{
                display: "flex",
                gap: "1.5rem",
                marginBottom: "1rem",
                flexWrap: "wrap",
              }}
            >
              {(["allegation", "motion_claim", "evidence", "document"] as GraphNodeType[]).map(
                (type) => (
                  <div
                    key={type}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: "0.5rem",
                      fontSize: "0.85rem",
                    }}
                  >
                    <div
                      style={{
                        width: "20px",
                        height: "16px",
                        backgroundColor: COLORS.bgCard,
                        border: `1px solid ${COLORS.border}`,
                        borderLeft: `3px solid ${ACCENT_COLORS[type]}`,
                        borderRadius: "3px",
                      }}
                    />
                    <span style={{ color: COLORS.textSecondary }}>
                      {type
                        .replace("_", " ")
                        .replace(/\b\w/g, (c) => c.toUpperCase())}
                    </span>
                  </div>
                )
              )}
            </div>

            {/* SVG container */}
            <div
              style={{
                backgroundColor: COLORS.bgCard,
                border: `1px solid ${COLORS.border}`,
                borderRadius: "8px",
                overflow: "auto",
                maxHeight: "70vh",
              }}
            >
              {layout.nodes.length === 0 ? (
                <div
                  style={{
                    padding: "3rem",
                    textAlign: "center",
                    color: COLORS.textMuted,
                    fontStyle: "italic",
                  }}
                >
                  No evidence chain found for this allegation
                </div>
              ) : (
                <svg ref={svgRef} width={layout.width} height={layout.height}>
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
                      <polygon points="0 0, 10 3.5, 0 7" fill="#94a3b8" />
                    </marker>
                  </defs>

                  {/* Edges */}
                  {layout.edges.map((edge, i) => {
                    if (edge.points.length < 2) return null;
                    const pathData = edge.points
                      .map((p, idx) => `${idx === 0 ? "M" : "L"} ${p.x} ${p.y}`)
                      .join(" ");
                    return (
                      <g key={`edge-${i}`}>
                        <path
                          d={pathData}
                          fill="none"
                          stroke="#cbd5e1"
                          strokeWidth={1.5}
                          markerEnd="url(#arrowhead)"
                        />
                      </g>
                    );
                  })}

                  {/* Nodes */}
                  {layout.nodes.map((node) => {
                    const accentColor = ACCENT_COLORS[node.node_type];
                    const x = node.x - NODE_WIDTH / 2;
                    const y = node.y - NODE_HEIGHT / 2;
                    const isDocument = node.node_type === "document";
                    const isAllegation = node.node_type === "allegation";
                    const statusColors = node.subtitle ? STATUS_COLORS[node.subtitle.toUpperCase()] : null;

                    return (
                      <g key={node.id}>
                        {/* Main node rectangle */}
                        <rect
                          x={x}
                          y={y}
                          width={NODE_WIDTH}
                          height={NODE_HEIGHT}
                          rx={6}
                          ry={6}
                          fill={isDocument ? COLORS.bgPage : COLORS.bgCard}
                          stroke={COLORS.border}
                          strokeWidth={1}
                        />
                        {/* Left accent border */}
                        <rect
                          x={x}
                          y={y}
                          width={ACCENT_WIDTH}
                          height={NODE_HEIGHT}
                          rx={0}
                          ry={0}
                          fill={accentColor}
                          clipPath={`inset(0 0 0 0 round 6px 0 0 6px)`}
                        />
                        {/* Rounded left edge for accent */}
                        <path
                          d={`M ${x + 6} ${y}
                              L ${x + ACCENT_WIDTH} ${y}
                              L ${x + ACCENT_WIDTH} ${y + NODE_HEIGHT}
                              L ${x + 6} ${y + NODE_HEIGHT}
                              Q ${x} ${y + NODE_HEIGHT} ${x} ${y + NODE_HEIGHT - 6}
                              L ${x} ${y + 6}
                              Q ${x} ${y} ${x + 6} ${y}
                              Z`}
                          fill={accentColor}
                        />

                        {/* Node label */}
                        <text
                          x={x + ACCENT_WIDTH + 10}
                          y={y + (isAllegation && statusColors ? 26 : NODE_HEIGHT / 2)}
                          textAnchor="start"
                          dominantBaseline={isAllegation && statusColors ? "auto" : "middle"}
                          fill={COLORS.textPrimary}
                          fontSize="12"
                          fontWeight="500"
                          fontFamily="Inter, system-ui, sans-serif"
                        >
                          {truncateLabel(node.label)}
                        </text>

                        {/* Status badge for allegation */}
                        {isAllegation && statusColors && (
                          <>
                            <rect
                              x={x + ACCENT_WIDTH + 10}
                              y={y + 36}
                              width={60}
                              height={20}
                              rx={10}
                              ry={10}
                              fill={statusColors.bg}
                            />
                            <text
                              x={x + ACCENT_WIDTH + 40}
                              y={y + 50}
                              textAnchor="middle"
                              fill={statusColors.text}
                              fontSize="10"
                              fontWeight="600"
                              fontFamily="Inter, system-ui, sans-serif"
                            >
                              {node.subtitle}
                            </text>
                          </>
                        )}

                        {/* Subtitle for documents (page number) */}
                        {isDocument && node.subtitle && (
                          <text
                            x={x + ACCENT_WIDTH + 10}
                            y={y + NODE_HEIGHT / 2 + 14}
                            textAnchor="start"
                            fill={COLORS.textSecondary}
                            fontSize="10"
                            fontFamily="Inter, system-ui, sans-serif"
                          >
                            {node.subtitle}
                          </text>
                        )}
                      </g>
                    );
                  })}
                </svg>
              )}
            </div>
          </>
        )}
      </div>
    </div>
  );
};

export default GraphPage;
