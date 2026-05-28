import React, { useEffect, useMemo, useState } from "react";
import { Link, useSearchParams } from "react-router-dom";
import { displayStatus } from "../utils/legalTerms";
import { toCountNumeral } from "../utils/countFormat";
import { useCase } from "../context/CaseContext";
import {
  getAllegations,
  AllegationDto,
  AllegationSummary,
} from "../services/allegations";

const STATUS_COLORS: Record<string, { bg: string; text: string }> = {
  PROVEN: { bg: "var(--state-success-bg-soft)", text: "var(--status-active-text)" },
  PARTIAL: { bg: "var(--burden-warning-bg)", text: "var(--burden-warning-text)" },
  UNPROVEN: { bg: "var(--state-danger-bg-soft)", text: "var(--status-dropped-text)" },
};

const DEFAULT_STATUS_COLOR = { bg: "var(--bg-page)", text: "var(--text-secondary)" };

function getStatusStyle(status: string | undefined) {
  if (!status) return DEFAULT_STATUS_COLOR;
  return STATUS_COLORS[status.toUpperCase()] || DEFAULT_STATUS_COLOR;
}

const AllegationsPage: React.FC = () => {
  const [searchParams] = useSearchParams();
  const countFilter = searchParams.get("count");
  const { caseData } = useCase();

  // Resolve the human-readable count for the current ?count= filter.
  // Undefined if no filter is set, or if the filter id doesn't match any
  // LegalCount (stale URL, different case loaded).
  const activeCount = useMemo(() => {
    if (!countFilter || !caseData) return undefined;
    return caseData.legal_count_details.find((lc) => lc.id === countFilter);
  }, [countFilter, caseData]);

  const [allegations, setAllegations] = useState<AllegationDto[]>([]);
  const [summary, setSummary] = useState<AllegationSummary>({
    proven: 0,
    partial: 0,
    unproven: 0,
  });
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Filter allegations when a ?count= param is present (match against IDs)
  const displayedAllegations = useMemo(() => {
    if (!countFilter) return allegations;
    return allegations.filter(
      (a) => a.legal_count_ids?.includes(countFilter),
    );
  }, [allegations, countFilter]);

  useEffect(() => {
    let active = true;

    const fetchAllegations = async () => {
      try {
        const result = await getAllegations();
        if (!active) return;
        setAllegations(result.allegations);
        setSummary(result.summary);
        setTotal(result.total);
        setError(null);
      } catch {
        if (!active) return;
        setAllegations([]);
        setError("Failed to load allegations");
      } finally {
        if (active) {
          setLoading(false);
        }
      }
    };

    fetchAllegations();

    return () => {
      active = false;
    };
  }, []);

  if (loading) {
    return (
      <div style={{ padding: "2rem", textAlign: "center", color: "var(--text-muted)" }}>
        Loading allegations...
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
      <h1 style={{ marginBottom: "0.5rem" }}>Allegations</h1>

      {/* Filter banner when ?count= is active */}
      {countFilter && (
        <div
          style={{
            padding: "0.75rem 1rem",
            backgroundColor: "var(--accent-bg-soft)",
            border: "1px solid var(--accent-bg-soft)",
            borderRadius: "6px",
            marginBottom: "1rem",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
          }}
        >
          <span style={{ color: "var(--accent-primary-hover)", fontWeight: 500 }}>
            {activeCount ? (
              <>
                <strong>
                  Count {toCountNumeral(activeCount.count_number)} —{" "}
                  {activeCount.name}
                </strong>
                {" "}({displayedAllegations.length} {displayedAllegations.length === 1 ? "allegation" : "allegations"})
              </>
            ) : caseData ? (
              <>Filtered view ({displayedAllegations.length} of {total})</>
            ) : (
              <>Loading count…</>
            )}
          </span>
          <Link
            to="/allegations"
            style={{ color: "var(--accent-primary)", textDecoration: "none", fontWeight: 500 }}
          >
            Show All
          </Link>
        </div>
      )}

      <div
        style={{
          padding: "0.75rem 1rem",
          backgroundColor: "var(--bg-page)",
          borderRadius: "6px",
          marginBottom: "1.5rem",
          color: "var(--text-secondary)",
          display: "flex",
          flexWrap: "wrap",
          gap: "0.5rem",
          alignItems: "center",
        }}
      >
        <strong>{total} Allegations:</strong>
        <span
          style={{
            padding: "0.25rem 0.5rem",
            backgroundColor: "var(--state-success-bg-soft)",
            color: "var(--status-active-text)",
            borderRadius: "4px",
            fontSize: "0.875rem",
          }}
        >
          {summary.proven} Proven
        </span>
        <span style={{ color: "var(--text-disabled)" }}>&bull;</span>
        <span
          style={{
            padding: "0.25rem 0.5rem",
            backgroundColor: "var(--burden-warning-bg)",
            color: "var(--burden-warning-text)",
            borderRadius: "4px",
            fontSize: "0.875rem",
          }}
        >
          {summary.partial} Partial
        </span>
        <span style={{ color: "var(--text-disabled)" }}>&bull;</span>
        <span
          style={{
            padding: "0.25rem 0.5rem",
            backgroundColor: "var(--state-danger-bg-soft)",
            color: "var(--status-dropped-text)",
            borderRadius: "4px",
            fontSize: "0.875rem",
          }}
        >
          {summary.unproven} Unproven
        </span>
      </div>

      {displayedAllegations.length === 0 ? (
        <div style={{ color: "var(--text-muted)", padding: "1rem" }}>
          {countFilter ? "No allegations match this filter." : "No allegations found."}
        </div>
      ) : (
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: "1rem",
          }}
        >
          {displayedAllegations.map((allegation) => {
            const statusStyle = getStatusStyle(allegation.evidence_status);
            return (
              <div
                key={allegation.id}
                style={{
                  padding: "1rem",
                  backgroundColor: "var(--bg-surface)",
                  border: "1px solid var(--border-default)",
                  borderRadius: "8px",
                }}
              >
                <div
                  style={{
                    display: "flex",
                    flexWrap: "wrap",
                    alignItems: "center",
                    gap: "0.5rem",
                    marginBottom: "0.5rem",
                  }}
                >
                  <span
                    style={{
                      padding: "0.2rem 0.5rem",
                      backgroundColor: "var(--border-default)",
                      color: "var(--text-secondary)",
                      borderRadius: "4px",
                      fontSize: "0.75rem",
                      fontFamily: "monospace",
                    }}
                  >
                    {allegation.id}
                  </span>
                  {allegation.paragraph && (
                    <span
                      style={{
                        fontSize: "0.8rem",
                        color: "var(--text-muted)",
                      }}
                    >
                      ¶{allegation.paragraph}
                    </span>
                  )}
                  {allegation.evidence_status && (
                    <span
                      style={{
                        padding: "0.2rem 0.5rem",
                        backgroundColor: statusStyle.bg,
                        color: statusStyle.text,
                        borderRadius: "4px",
                        fontSize: "0.75rem",
                        fontWeight: "600",
                      }}
                    >
                      {displayStatus(allegation.evidence_status)}
                    </span>
                  )}
                  {allegation.category && (
                    <span
                      style={{
                        padding: "0.2rem 0.5rem",
                        backgroundColor: "var(--bg-page)",
                        color: "var(--text-muted)",
                        borderRadius: "4px",
                        fontSize: "0.75rem",
                      }}
                    >
                      {allegation.category}
                    </span>
                  )}
                  {allegation.severity !== undefined && (
                    <span
                      style={{
                        fontSize: "0.75rem",
                        color: "var(--text-muted)",
                      }}
                    >
                      Severity: {allegation.severity}/10
                    </span>
                  )}
                </div>

                <div
                  style={{
                    fontWeight: "600",
                    fontSize: "1rem",
                    marginBottom: "0.5rem",
                  }}
                >
                  {allegation.title}
                </div>

                {allegation.allegation && (
                  <div
                    style={{
                      color: "var(--text-secondary)",
                      fontSize: "0.9rem",
                      lineHeight: "1.5",
                      marginBottom: "0.5rem",
                    }}
                  >
                    {allegation.allegation}
                  </div>
                )}

                {allegation.legal_counts && allegation.legal_counts.length > 0 && (
                  <div
                    style={{
                      fontSize: "0.8rem",
                      color: "var(--text-muted)",
                      fontStyle: "italic",
                    }}
                  >
                    Supports: {allegation.legal_counts.join(", ")}
                  </div>
                )}

                <div style={{ marginTop: "0.5rem" }}>
                  <Link
                    to={`/allegations/${allegation.id}/detail`}
                    style={{
                      color: "var(--accent-primary)",
                      textDecoration: "none",
                      fontSize: "0.85rem",
                      fontWeight: 500,
                    }}
                  >
                    View Detail &rarr;
                  </Link>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};

export default AllegationsPage;
