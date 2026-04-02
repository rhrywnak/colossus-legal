/**
 * ContentPanel — Displays extraction items for a document.
 *
 * Shows entity type filter, item cards with type badges, labels,
 * truncated quotes, grounding status, and [View in PDF] cross-tab navigation.
 */
import React, { useMemo, useState } from "react";
import type { ExtractionItem } from "../../services/pipelineApi";

interface ContentPanelProps {
  items: ExtractionItem[] | null;
  loading: boolean;
  error: string | null;
  onViewInPdf: (page: number) => void;
}

// ── Styles ──────────────────────────────────────────────────────

const TYPE_COLORS: Record<string, string> = {
  Person: "#2563eb", Evidence: "#059669", Allegation: "#dc2626",
  Claim: "#7c3aed", Document: "#d97706", Event: "#0891b2",
};
const itemCardStyle: React.CSSProperties = {
  backgroundColor: "#ffffff", border: "1px solid #e2e8f0", borderRadius: "8px",
  padding: "0.75rem 1rem", marginBottom: "0.5rem",
};
const typeBadge = (color: string): React.CSSProperties => ({
  display: "inline-block", padding: "0.1rem 0.45rem", borderRadius: "4px",
  fontSize: "0.68rem", fontWeight: 600, backgroundColor: color, color: "#fff",
});
const groundBadge = (ok: boolean): React.CSSProperties => ({
  display: "inline-block", padding: "0.1rem 0.4rem", borderRadius: "9999px",
  fontSize: "0.68rem", fontWeight: 600,
  backgroundColor: ok ? "#dcfce7" : "#fef9c3", color: ok ? "#166534" : "#854d0e",
});
const pdfBtnStyle: React.CSSProperties = {
  padding: "0.2rem 0.5rem", fontSize: "0.72rem", fontWeight: 500, border: "1px solid #e2e8f0",
  borderRadius: "4px", backgroundColor: "#f8fafc", color: "#2563eb", cursor: "pointer", fontFamily: "inherit",
};
const filterStyle: React.CSSProperties = {
  padding: "0.35rem 0.6rem", fontSize: "0.8rem", borderRadius: "6px", border: "1px solid #e2e8f0",
  fontFamily: "inherit", color: "#334155", backgroundColor: "#ffffff", marginBottom: "0.75rem",
};
const emptyStyle: React.CSSProperties = {
  padding: "3rem", textAlign: "center", color: "#94a3b8", fontSize: "0.9rem",
};

// ── Component ───────────────────────────────────────────────────

const ContentPanel: React.FC<ContentPanelProps> = ({ items, loading, error, onViewInPdf }) => {
  const [entityFilter, setEntityFilter] = useState("all");

  const filteredItems = useMemo(() => {
    if (!items) return [];
    if (entityFilter === "all") return items;
    return items.filter((it) => it.entity_type === entityFilter);
  }, [items, entityFilter]);

  const entityTypes = useMemo(() => {
    if (!items) return [];
    return Array.from(new Set(items.map((it) => it.entity_type))).sort();
  }, [items]);

  if (loading) return <div style={emptyStyle}>Loading extracted content...</div>;
  if (error) return <div style={{ ...emptyStyle, color: "#dc2626" }}>{error}</div>;
  if (items && items.length === 0) return <div style={emptyStyle}>No extracted content yet.</div>;
  if (!items) return null;

  return (
    <div>
      <div style={{ display: "flex", alignItems: "center", gap: "0.75rem", marginBottom: "0.5rem" }}>
        <select style={filterStyle} value={entityFilter} onChange={(e) => setEntityFilter(e.target.value)}>
          <option value="all">All types ({items.length})</option>
          {entityTypes.map((t) => (
            <option key={t} value={t}>{t} ({items.filter((i) => i.entity_type === t).length})</option>
          ))}
        </select>
        <span style={{ fontSize: "0.76rem", color: "#64748b" }}>
          {filteredItems.length} item{filteredItems.length !== 1 ? "s" : ""}
        </span>
      </div>
      <div style={{ maxHeight: "calc(100vh - 340px)", overflowY: "auto" }}>
        {filteredItems.map((item) => (
          <div key={item.id} style={itemCardStyle}>
            <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", marginBottom: "0.35rem" }}>
              <span style={typeBadge(TYPE_COLORS[item.entity_type] || "#6b7280")}>{item.entity_type}</span>
              <span style={{ fontSize: "0.88rem", fontWeight: 600, color: "#0f172a" }}>{item.label}</span>
              {item.grounding_status && (
                <span style={groundBadge(item.grounding_status === "grounded")}>
                  {item.grounding_status}
                </span>
              )}
              {item.grounded_page && (
                <button style={pdfBtnStyle} onClick={() => onViewInPdf(item.grounded_page!)}>
                  View in PDF (p.{item.grounded_page})
                </button>
              )}
            </div>
            {item.verbatim_quote && (
              <div style={{ fontSize: "0.78rem", color: "#64748b", fontStyle: "italic", lineHeight: 1.4 }}>
                "{item.verbatim_quote.length > 150
                  ? item.verbatim_quote.slice(0, 150) + "..."
                  : item.verbatim_quote}"
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
};

export default ContentPanel;
