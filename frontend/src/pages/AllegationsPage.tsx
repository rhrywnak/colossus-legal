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
  PROVEN: { bg: "#dcfce7", text: "#166534" },
  PARTIAL: { bg: "#fef3c7", text: "#92400e" },
  UNPROVEN: { bg: "#fee2e2", text: "#991b1b" },
};

const DEFAULT_STATUS_COLOR = { bg: "#f3f4f6", text: "#374151" };

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
      <div style={{ padding: "2rem", textAlign: "center", color: "#6b7280" }}>
        Loading allegations...
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

  return (
    <div>
      <h1 style={{ marginBottom: "0.5rem" }}>Allegations</h1>

      {/* Filter banner when ?count= is active */}
      {countFilter && (
        <div
          style={{
            padding: "0.75rem 1rem",
            backgroundColor: "#eff6ff",
            border: "1px solid #bfdbfe",
            borderRadius: "6px",
            marginBottom: "1rem",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
          }}
        >
          <span style={{ color: "#1e40af", fontWeight: 500 }}>
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
            style={{ color: "#2563eb", textDecoration: "none", fontWeight: 500 }}
          >
            Show All
          </Link>
        </div>
      )}

      <div
        style={{
          padding: "0.75rem 1rem",
          backgroundColor: "#f3f4f6",
          borderRadius: "6px",
          marginBottom: "1.5rem",
          color: "#374151",
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
            backgroundColor: "#dcfce7",
            color: "#166534",
            borderRadius: "4px",
            fontSize: "0.875rem",
          }}
        >
          {summary.proven} Proven
        </span>
        <span style={{ color: "#9ca3af" }}>&bull;</span>
        <span
          style={{
            padding: "0.25rem 0.5rem",
            backgroundColor: "#fef3c7",
            color: "#92400e",
            borderRadius: "4px",
            fontSize: "0.875rem",
          }}
        >
          {summary.partial} Partial
        </span>
        <span style={{ color: "#9ca3af" }}>&bull;</span>
        <span
          style={{
            padding: "0.25rem 0.5rem",
            backgroundColor: "#fee2e2",
            color: "#991b1b",
            borderRadius: "4px",
            fontSize: "0.875rem",
          }}
        >
          {summary.unproven} Unproven
        </span>
      </div>

      {displayedAllegations.length === 0 ? (
        <div style={{ color: "#6b7280", padding: "1rem" }}>
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
                  backgroundColor: "#fff",
                  border: "1px solid #e5e7eb",
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
                      backgroundColor: "#e5e7eb",
                      color: "#374151",
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
                        color: "#6b7280",
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
                        backgroundColor: "#f3f4f6",
                        color: "#6b7280",
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
                        color: "#6b7280",
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
                      color: "#4b5563",
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
                      color: "#6b7280",
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
                      color: "#2563eb",
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
