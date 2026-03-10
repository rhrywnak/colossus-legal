import React, { useEffect, useState } from "react";
import { useLocation } from "react-router-dom";
import { API_BASE_URL } from "../services/api";

// ─── Types ───────────────────────────────────────────────────────────────────

type Phase = {
  id: string;
  label: string;
  date_range: string;
  color: string;
  description: string;
};

type TimelineEvent = {
  id: string;
  phase: string;
  date: string;
  approximate: boolean;
  title: string;
  description: string;
  category: string;
  document_id: string | null;
  document_label: string | null;
};

type CategoryInfo = {
  color: string;
  label: string;
  icon: string;
};

type TimelineData = {
  phases: Phase[];
  events: TimelineEvent[];
  categories: Record<string, CategoryInfo>;
};

// ─── Helpers ─────────────────────────────────────────────────────────────────

function formatDate(dateStr: string, approximate: boolean): string {
  const d = new Date(dateStr + "T00:00:00");
  const formatted = d.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
  return approximate ? `~${formatted}` : formatted;
}

// ─── Component ───────────────────────────────────────────────────────────────

const TimelinePage: React.FC = () => {
  const location = useLocation();
  const [data, setData] = useState<TimelineData | null>(null);
  const [loading, setLoading] = useState(true);
  const [filter, setFilter] = useState<string | null>(null);

  useEffect(() => {
    fetch("/data/timeline.json")
      .then((r) => r.json())
      .then((d) => setData(d))
      .catch(() => {})
      .finally(() => setLoading(false));
  }, []);

  // Scroll to phase anchor after data loads
  useEffect(() => {
    if (!data || !location.hash) return;
    const el = document.getElementById(location.hash.slice(1));
    if (el) {
      setTimeout(() => el.scrollIntoView({ behavior: "smooth", block: "start" }), 100);
    }
  }, [data, location.hash]);

  if (loading) {
    return <div style={{ padding: "2rem", textAlign: "center", color: "#64748b" }}>Loading timeline...</div>;
  }

  if (!data) {
    return <div style={{ padding: "2rem", textAlign: "center", color: "#64748b" }}>Failed to load timeline data.</div>;
  }

  const filteredEvents = filter
    ? data.events.filter((e) => e.category === filter)
    : data.events;

  const categoryKeys = Object.keys(data.categories);

  return (
    <div style={{ paddingTop: "2rem", paddingBottom: "4rem" }}>
      {/* Header */}
      <div style={{ marginBottom: "1.5rem" }}>
        <h1 style={{ fontSize: "1.5rem", fontWeight: 700, color: "#0f172a", margin: 0, marginBottom: "0.3rem" }}>
          Case Timeline
        </h1>
        <p style={{ fontSize: "0.84rem", color: "#64748b", margin: 0 }}>
          {filteredEvents.length} event{filteredEvents.length !== 1 ? "s" : ""}
          {filter ? ` in ${data.categories[filter]?.label}` : ""} across {data.phases.length} phases
        </p>
      </div>

      {/* Filter chips */}
      <div style={{ display: "flex", gap: "0.4rem", flexWrap: "wrap", marginBottom: "1.75rem" }}>
        <button
          onClick={() => setFilter(null)}
          style={{
            padding: "0.3rem 0.75rem", borderRadius: "9999px", fontSize: "0.76rem",
            fontWeight: 600, cursor: "pointer", border: "1px solid",
            fontFamily: "inherit", transition: "all 0.15s ease",
            backgroundColor: filter === null ? "#0f172a" : "#ffffff",
            color: filter === null ? "#ffffff" : "#475569",
            borderColor: filter === null ? "#0f172a" : "#e2e8f0",
          }}
        >
          All
        </button>
        {categoryKeys.map((key) => {
          const cat = data.categories[key];
          const active = filter === key;
          return (
            <button
              key={key}
              onClick={() => setFilter(active ? null : key)}
              style={{
                padding: "0.3rem 0.75rem", borderRadius: "9999px", fontSize: "0.76rem",
                fontWeight: 600, cursor: "pointer", border: "1px solid",
                fontFamily: "inherit", transition: "all 0.15s ease",
                backgroundColor: active ? cat.color : "#ffffff",
                color: active ? "#ffffff" : cat.color,
                borderColor: active ? cat.color : "#e2e8f0",
              }}
            >
              {cat.label}
            </button>
          );
        })}
      </div>

      {/* Phases + events */}
      {data.phases.map((phase) => {
        const phaseEvents = filteredEvents.filter((e) => e.phase === phase.id);
        if (phaseEvents.length === 0 && filter) return null;

        return (
          <section
            key={phase.id}
            id={`phase-${phase.id}`}
            style={{ marginBottom: "2rem", scrollMarginTop: "80px" }}
          >
            {/* Phase header */}
            <div style={{
              borderLeft: `4px solid ${phase.color}`, paddingLeft: "1rem",
              marginBottom: "1rem",
            }}>
              <div style={{ fontSize: "1.05rem", fontWeight: 700, color: "#0f172a" }}>
                {phase.label}
              </div>
              <div style={{ fontSize: "0.78rem", color: "#64748b" }}>
                {phase.date_range} {"\u00b7"} {phaseEvents.length} event{phaseEvents.length !== 1 ? "s" : ""}
              </div>
            </div>

            {/* Events list */}
            <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem", paddingLeft: "1rem" }}>
              {phaseEvents.map((evt) => {
                const cat = data.categories[evt.category];
                return (
                  <div
                    key={evt.id}
                    style={{
                      backgroundColor: "#ffffff", border: "1px solid #e2e8f0",
                      borderRadius: "8px", padding: "1rem 1.25rem",
                      display: "flex", gap: "1rem", alignItems: "flex-start",
                    }}
                  >
                    {/* Dot + date column */}
                    <div style={{ flexShrink: 0, width: "90px", textAlign: "right", paddingTop: "0.15rem" }}>
                      <div style={{ fontSize: "0.72rem", color: "#64748b", fontWeight: 500, whiteSpace: "nowrap" }}>
                        {formatDate(evt.date, evt.approximate)}
                      </div>
                    </div>

                    {/* Dot */}
                    <div style={{
                      width: "10px", height: "10px", borderRadius: "50%",
                      backgroundColor: cat?.color ?? "#94a3b8",
                      flexShrink: 0, marginTop: "0.3rem",
                    }} />

                    {/* Content */}
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flexWrap: "wrap", marginBottom: "0.25rem" }}>
                        <span style={{ fontSize: "0.88rem", fontWeight: 600, color: "#0f172a" }}>
                          {evt.title}
                        </span>
                        {cat && (
                          <span style={{
                            display: "inline-block", padding: "0.1rem 0.5rem",
                            borderRadius: "9999px", fontSize: "0.65rem", fontWeight: 600,
                            backgroundColor: cat.color + "18", color: cat.color,
                          }}>
                            {cat.label}
                          </span>
                        )}
                      </div>
                      <div style={{ fontSize: "0.82rem", color: "#475569", lineHeight: 1.55, fontFamily: "'Georgia', serif" }}>
                        {evt.description}
                      </div>
                      {evt.document_id && evt.document_label && (
                        <a
                          href={`${API_BASE_URL}/documents/${evt.document_id}/file`}
                          target="_blank"
                          rel="noopener noreferrer"
                          style={{
                            display: "inline-block", marginTop: "0.4rem",
                            fontSize: "0.78rem", color: "#2563eb", textDecoration: "none", fontWeight: 500,
                          }}
                        >
                          {evt.document_label} {"\u2192"}
                        </a>
                      )}
                    </div>
                  </div>
                );
              })}
              {phaseEvents.length === 0 && (
                <div style={{ fontSize: "0.82rem", color: "#94a3b8", fontStyle: "italic", padding: "0.5rem 0" }}>
                  No events in this phase match the current filter.
                </div>
              )}
            </div>
          </section>
        );
      })}
    </div>
  );
};

export default TimelinePage;
