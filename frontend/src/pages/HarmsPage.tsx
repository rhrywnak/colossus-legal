import React, { useEffect, useState } from "react";
import { getHarms, HarmDto } from "../services/harms";

const CATEGORY_COLORS: Record<string, { bg: string; text: string }> = {
  financial_direct: { bg: "var(--state-danger-bg-soft)", text: "var(--status-dropped-text)" },
  financial_estate: { bg: "var(--burden-warning-bg)", text: "var(--burden-warning-text)" },
  reputational: { bg: "var(--bias-purple-bg-soft)", text: "var(--bias-purple-text)" },
};

const DEFAULT_CATEGORY_COLOR = { bg: "var(--bg-page)", text: "var(--text-secondary)" };

function getCategoryStyle(category: string | undefined) {
  if (!category) return DEFAULT_CATEGORY_COLOR;
  return CATEGORY_COLORS[category.toLowerCase()] || DEFAULT_CATEGORY_COLOR;
}

function formatCurrency(amount: number): string {
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
  }).format(amount);
}

const HarmsPage: React.FC = () => {
  const [harms, setHarms] = useState<HarmDto[]>([]);
  const [totalDamages, setTotalDamages] = useState(0);
  const [byCategory, setByCategory] = useState<Record<string, number>>({});
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;

    const fetchHarms = async () => {
      try {
        const result = await getHarms();
        if (!active) return;
        setHarms(result.harms);
        setTotalDamages(result.total_damages);
        setByCategory(result.by_category);
        setTotal(result.total);
        setError(null);
      } catch {
        if (!active) return;
        setHarms([]);
        setError("Failed to load harms data");
      } finally {
        if (active) {
          setLoading(false);
        }
      }
    };

    fetchHarms();

    return () => {
      active = false;
    };
  }, []);

  if (loading) {
    return (
      <div style={{ padding: "2rem", textAlign: "center", color: "var(--text-muted)" }}>
        Loading damages...
      </div>
    );
  }

  if (error) {
    return (
      <div
        style={{
          padding: "1rem",
          backgroundColor: "var(--state-danger-bg-soft)",
          border: "1px solid var(--state-danger-border)",
          borderRadius: "6px",
          color: "var(--state-danger-strong)",
        }}
      >
        {error}
      </div>
    );
  }

  return (
    <div>
      <h1 style={{ marginBottom: "0.5rem" }}>Harms & Damages</h1>

      {/* Total damages banner */}
      <div
        style={{
          padding: "1rem 1.5rem",
          backgroundColor: "var(--state-danger-bg-soft)",
          border: "1px solid var(--state-danger-border)",
          borderRadius: "8px",
          marginBottom: "1rem",
        }}
      >
        <div style={{ fontSize: "0.875rem", color: "var(--status-dropped-text)" }}>
          Total Quantifiable Damages
        </div>
        <div
          style={{
            fontSize: "2rem",
            fontWeight: "bold",
            color: "var(--state-danger-strong)",
          }}
        >
          {formatCurrency(totalDamages)}
        </div>
      </div>

      {/* Category breakdown */}
      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          gap: "1rem",
          marginBottom: "1.5rem",
        }}
      >
        {Object.entries(byCategory).map(([category, amount]) => {
          const style = getCategoryStyle(category);
          return (
            <div
              key={category}
              style={{
                padding: "0.75rem 1rem",
                backgroundColor: style.bg,
                borderRadius: "6px",
                minWidth: "150px",
              }}
            >
              <div
                style={{
                  fontSize: "0.75rem",
                  color: style.text,
                  textTransform: "capitalize",
                }}
              >
                {category.replace(/_/g, " ")}
              </div>
              <div
                style={{
                  fontSize: "1.25rem",
                  fontWeight: "600",
                  color: style.text,
                }}
              >
                {formatCurrency(amount)}
              </div>
            </div>
          );
        })}
      </div>

      {/* Summary */}
      <div
        style={{
          padding: "0.5rem 1rem",
          backgroundColor: "var(--bg-page)",
          borderRadius: "6px",
          marginBottom: "1.5rem",
          color: "var(--text-secondary)",
        }}
      >
        <strong>{total}</strong> documented harms
      </div>

      {/* Harms list */}
      {harms.length === 0 ? (
        <div style={{ color: "var(--text-muted)", padding: "1rem" }}>
          No harms documented.
        </div>
      ) : (
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: "1rem",
          }}
        >
          {harms.map((harm) => {
            const categoryStyle = getCategoryStyle(harm.category);

            return (
              <div
                key={harm.id}
                style={{
                  padding: "1rem",
                  backgroundColor: "var(--bg-surface)",
                  border: "1px solid var(--border-default)",
                  borderRadius: "8px",
                  borderLeft: `4px solid ${categoryStyle.text}`,
                }}
              >
                {/* Header with badges */}
                <div
                  style={{
                    display: "flex",
                    flexWrap: "wrap",
                    alignItems: "center",
                    gap: "0.5rem",
                    marginBottom: "0.5rem",
                  }}
                >
                  {harm.category && (
                    <span
                      style={{
                        padding: "0.2rem 0.5rem",
                        backgroundColor: categoryStyle.bg,
                        color: categoryStyle.text,
                        borderRadius: "4px",
                        fontSize: "0.75rem",
                        fontWeight: "500",
                        textTransform: "capitalize",
                      }}
                    >
                      {harm.category.replace(/_/g, " ")}
                    </span>
                  )}
                  {harm.subcategory && (
                    <span
                      style={{
                        padding: "0.2rem 0.5rem",
                        backgroundColor: "var(--bg-page)",
                        color: "var(--text-muted)",
                        borderRadius: "4px",
                        fontSize: "0.75rem",
                      }}
                    >
                      {harm.subcategory}
                    </span>
                  )}
                  {harm.date && (
                    <span
                      style={{
                        fontSize: "0.75rem",
                        color: "var(--text-disabled)",
                      }}
                    >
                      {harm.date}
                    </span>
                  )}
                </div>

                {/* Title and amount */}
                <div
                  style={{
                    display: "flex",
                    justifyContent: "space-between",
                    alignItems: "flex-start",
                    gap: "1rem",
                    marginBottom: "0.5rem",
                  }}
                >
                  <div style={{ fontWeight: "600", fontSize: "1rem" }}>
                    {harm.title}
                  </div>
                  {harm.amount !== undefined && (
                    <div
                      style={{
                        fontSize: "1.25rem",
                        fontWeight: "bold",
                        color: "var(--state-danger-strong)",
                        whiteSpace: "nowrap",
                      }}
                    >
                      {formatCurrency(harm.amount)}
                    </div>
                  )}
                </div>

                {/* Description */}
                {harm.description && (
                  <div
                    style={{
                      color: "var(--text-secondary)",
                      fontSize: "0.9rem",
                      lineHeight: "1.5",
                      marginBottom: "0.5rem",
                    }}
                  >
                    {harm.description}
                  </div>
                )}

                {/* Source reference */}
                {harm.source_reference && (
                  <div
                    style={{
                      fontSize: "0.8rem",
                      color: "var(--text-muted)",
                      fontStyle: "italic",
                      marginBottom: "0.5rem",
                    }}
                  >
                    Source: {harm.source_reference}
                  </div>
                )}

                {/* Related allegations and counts */}
                <div
                  style={{
                    display: "flex",
                    flexWrap: "wrap",
                    gap: "1rem",
                    fontSize: "0.8rem",
                    color: "var(--text-muted)",
                    marginTop: "0.5rem",
                  }}
                >
                  {harm.caused_by_allegations.length > 0 && (
                    <div>
                      <span style={{ fontWeight: "500" }}>Caused by: </span>
                      {harm.caused_by_allegations.join(", ")}
                    </div>
                  )}
                  {harm.damages_for_counts.length > 0 && (
                    <div>
                      <span style={{ fontWeight: "500" }}>Damages for: </span>
                      {harm.damages_for_counts.join(", ")}
                    </div>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};

export default HarmsPage;
