/**
 * PeopleLinksPanel — Shows people and organizations extracted from a document.
 *
 * Groups extraction items by Person and Organization entity types,
 * counts references per entity, and displays them as a simple list.
 * Shares the workspace-level items cache to avoid duplicate fetches.
 */
import React, { useEffect, useMemo } from "react";
import type { ExtractionItem } from "../../services/pipelineApi";

interface PeopleLinksPanelProps {
  documentId: string;
  items: ExtractionItem[] | null;
  onLoadItems: () => void;
}

const PEOPLE_TYPES = new Set(["Person", "Organization"]);

const cardStyle: React.CSSProperties = {
  backgroundColor: "#ffffff", border: "1px solid #e2e8f0", borderRadius: "8px",
  padding: "0.75rem 1rem", marginBottom: "0.5rem",
  display: "flex", alignItems: "center", justifyContent: "space-between",
};
const groupTitle: React.CSSProperties = {
  fontSize: "0.88rem", fontWeight: 600, color: "#334155", marginBottom: "0.5rem",
  marginTop: "1rem",
};
const emptyStyle: React.CSSProperties = {
  padding: "3rem", textAlign: "center", color: "#94a3b8", fontSize: "0.9rem",
  backgroundColor: "#ffffff", borderRadius: "8px", border: "1px solid #e2e8f0",
};

interface EntityGroup {
  label: string;
  entityType: string;
  count: number;
}

const PeopleLinksPanel: React.FC<PeopleLinksPanelProps> = ({ documentId, items, onLoadItems }) => {
  // Trigger item loading if not yet loaded
  useEffect(() => {
    if (items === null) onLoadItems();
  }, [items, onLoadItems]);

  const grouped = useMemo(() => {
    if (!items) return { people: [] as EntityGroup[], orgs: [] as EntityGroup[] };

    const map = new Map<string, EntityGroup>();
    for (const item of items) {
      if (!PEOPLE_TYPES.has(item.entity_type)) continue;
      const key = `${item.entity_type}::${item.label}`;
      const existing = map.get(key);
      if (existing) {
        existing.count++;
      } else {
        map.set(key, { label: item.label, entityType: item.entity_type, count: 1 });
      }
    }

    const all = Array.from(map.values());
    return {
      people: all.filter((e) => e.entityType === "Person").sort((a, b) => b.count - a.count),
      orgs: all.filter((e) => e.entityType === "Organization").sort((a, b) => b.count - a.count),
    };
  }, [items]);

  if (items === null) {
    return <div style={emptyStyle}>Loading...</div>;
  }

  if (grouped.people.length === 0 && grouped.orgs.length === 0) {
    return <div style={emptyStyle}>No people or organizations found in this document.</div>;
  }

  return (
    <div>
      {grouped.people.length > 0 && (
        <>
          <div style={groupTitle}>People ({grouped.people.length})</div>
          {grouped.people.map((p) => (
            <div key={p.label} style={cardStyle}>
              <div>
                <span style={{ fontSize: "0.88rem", fontWeight: 600, color: "#0f172a" }}>{p.label}</span>
              </div>
              <span style={{ fontSize: "0.76rem", color: "#64748b" }}>
                {p.count} reference{p.count !== 1 ? "s" : ""}
              </span>
            </div>
          ))}
        </>
      )}

      {grouped.orgs.length > 0 && (
        <>
          <div style={groupTitle}>Organizations ({grouped.orgs.length})</div>
          {grouped.orgs.map((o) => (
            <div key={o.label} style={cardStyle}>
              <div>
                <span style={{ fontSize: "0.88rem", fontWeight: 600, color: "#0f172a" }}>{o.label}</span>
              </div>
              <span style={{ fontSize: "0.76rem", color: "#64748b" }}>
                {o.count} reference{o.count !== 1 ? "s" : ""}
              </span>
            </div>
          ))}
        </>
      )}
    </div>
  );
};

export default PeopleLinksPanel;
