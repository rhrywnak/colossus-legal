import React, { useEffect, useState } from "react";
import { getAllegations, AllegationDto } from "../services/allegations";
import {
  getEvidenceChain,
  EvidenceChainResponse,
  MotionClaimWithEvidence,
  EvidenceWithDocument,
} from "../services/evidenceChain";

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

const EvidenceExplorerPage: React.FC = () => {
  const [allegations, setAllegations] = useState<AllegationDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Track expanded allegations
  const [expandedIds, setExpandedIds] = useState<Set<string>>(new Set());
  // Cache fetched evidence chains
  const [chainCache, setChainCache] = useState<Map<string, EvidenceChainResponse>>(
    new Map()
  );
  // Track loading states for individual chains
  const [loadingIds, setLoadingIds] = useState<Set<string>>(new Set());

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

  const handleToggle = async (allegationId: string) => {
    // If already expanded, just collapse
    if (expandedIds.has(allegationId)) {
      setExpandedIds((prev) => {
        const next = new Set(prev);
        next.delete(allegationId);
        return next;
      });
      return;
    }

    // If already cached, just expand
    if (chainCache.has(allegationId)) {
      setExpandedIds((prev) => new Set(prev).add(allegationId));
      return;
    }

    // Fetch the chain
    setLoadingIds((prev) => new Set(prev).add(allegationId));

    try {
      const chain = await getEvidenceChain(allegationId);
      setChainCache((prev) => new Map(prev).set(allegationId, chain));
      setExpandedIds((prev) => new Set(prev).add(allegationId));
    } catch (err) {
      console.error("Failed to fetch evidence chain:", err);
    } finally {
      setLoadingIds((prev) => {
        const next = new Set(prev);
        next.delete(allegationId);
        return next;
      });
    }
  };

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
      <h1 style={{ marginBottom: "0.5rem" }}>Evidence Explorer</h1>
      <p style={{ color: "#6b7280", marginBottom: "1.5rem" }}>
        Click an allegation to see its evidence chain
      </p>

      {allegations.length === 0 ? (
        <div style={{ color: "#6b7280", padding: "1rem" }}>
          No allegations found.
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
          {allegations.map((allegation) => (
            <AllegationRow
              key={allegation.id}
              allegation={allegation}
              isExpanded={expandedIds.has(allegation.id)}
              isLoading={loadingIds.has(allegation.id)}
              chain={chainCache.get(allegation.id)}
              onToggle={() => handleToggle(allegation.id)}
            />
          ))}
        </div>
      )}
    </div>
  );
};

type AllegationRowProps = {
  allegation: AllegationDto;
  isExpanded: boolean;
  isLoading: boolean;
  chain?: EvidenceChainResponse;
  onToggle: () => void;
};

const AllegationRow: React.FC<AllegationRowProps> = ({
  allegation,
  isExpanded,
  isLoading,
  chain,
  onToggle,
}) => {
  const statusStyle = getStatusStyle(allegation.evidence_status);

  return (
    <div
      style={{
        border: "1px solid #e5e7eb",
        borderRadius: "8px",
        backgroundColor: "#fff",
        overflow: "hidden",
      }}
    >
      {/* Allegation header row */}
      <div
        onClick={onToggle}
        style={{
          padding: "0.75rem 1rem",
          backgroundColor: isExpanded ? "#f0fdf4" : "#fff",
          cursor: "pointer",
          display: "flex",
          alignItems: "center",
          gap: "0.75rem",
        }}
      >
        {/* Toggle icon */}
        <span style={{ fontFamily: "monospace", color: "#6b7280", width: "1rem" }}>
          {isLoading ? "..." : isExpanded ? "▼" : "▶"}
        </span>

        {/* ID badge */}
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

        {/* Title */}
        <span style={{ fontWeight: 500, flex: 1 }}>{allegation.title}</span>

        {/* Status badge */}
        {allegation.evidence_status && (
          <span
            style={{
              padding: "0.2rem 0.5rem",
              backgroundColor: statusStyle.bg,
              color: statusStyle.text,
              borderRadius: "4px",
              fontSize: "0.75rem",
              fontWeight: 600,
            }}
          >
            {allegation.evidence_status}
          </span>
        )}
      </div>

      {/* Expanded content */}
      {isExpanded && chain && (
        <div
          style={{
            borderTop: "1px solid #e5e7eb",
            padding: "1rem",
            backgroundColor: "#fafafa",
          }}
        >
          {/* Summary line */}
          <div
            style={{
              fontSize: "0.85rem",
              color: "#6b7280",
              marginBottom: "0.75rem",
            }}
          >
            ({chain.summary.motion_claim_count} claims,{" "}
            {chain.summary.evidence_count} evidence,{" "}
            {chain.summary.document_count} docs)
            {chain.allegation.legal_counts &&
              chain.allegation.legal_counts.length > 0 && (
                <span style={{ marginLeft: "0.75rem" }}>
                  Supports:{" "}
                  {chain.allegation.legal_counts.map((count, i) => (
                    <span
                      key={i}
                      style={{
                        marginLeft: i > 0 ? "0.25rem" : 0,
                        padding: "0.15rem 0.4rem",
                        backgroundColor: "#dbeafe",
                        color: "#1e40af",
                        borderRadius: "4px",
                        fontSize: "0.75rem",
                      }}
                    >
                      {count}
                    </span>
                  ))}
                </span>
              )}
          </div>

          {/* Motion claims */}
          {chain.motion_claims.length === 0 ? (
            <div style={{ color: "#9ca3af", fontStyle: "italic" }}>
              No motion claims linked to this allegation.
            </div>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
              {chain.motion_claims.map((mc) => (
                <MotionClaimSection key={mc.id} motionClaim={mc} />
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
};

type MotionClaimSectionProps = {
  motionClaim: MotionClaimWithEvidence;
};

const MotionClaimSection: React.FC<MotionClaimSectionProps> = ({ motionClaim }) => {
  return (
    <div
      style={{
        marginLeft: "24px",
        padding: "0.75rem",
        backgroundColor: "#fef3c7",
        border: "1px solid #fcd34d",
        borderRadius: "6px",
      }}
    >
      <div style={{ fontWeight: 600, color: "#854d0e", marginBottom: "0.5rem" }}>
        {motionClaim.title}
      </div>

      {motionClaim.evidence.length === 0 ? (
        <div style={{ color: "#92400e", fontSize: "0.85rem", fontStyle: "italic" }}>
          No evidence linked.
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
          {motionClaim.evidence.map((ev) => (
            <EvidenceItem key={ev.id} evidence={ev} />
          ))}
        </div>
      )}
    </div>
  );
};

type EvidenceItemProps = {
  evidence: EvidenceWithDocument;
};

const EvidenceItem: React.FC<EvidenceItemProps> = ({ evidence }) => {
  return (
    <div
      style={{
        marginLeft: "24px",
        padding: "0.6rem 0.75rem",
        backgroundColor: "#f3e8ff",
        border: "1px solid #d8b4fe",
        borderRadius: "6px",
      }}
    >
      <div style={{ fontWeight: 500, color: "#6b21a8", marginBottom: "0.35rem" }}>
        {evidence.title}
      </div>

      {evidence.question && (
        <div style={{ fontSize: "0.85rem", color: "#7c3aed", marginBottom: "0.25rem" }}>
          <strong>Q:</strong> {evidence.question}
        </div>
      )}

      {evidence.answer && (
        <div style={{ fontSize: "0.85rem", color: "#7c3aed", marginBottom: "0.35rem" }}>
          <strong>A:</strong> {evidence.answer}
        </div>
      )}

      {evidence.document && (
        <DocumentLink document={evidence.document} />
      )}
    </div>
  );
};

type DocumentLinkProps = {
  document: { id: string; title: string; page_number?: number };
};

const DocumentLink: React.FC<DocumentLinkProps> = ({ document }) => {
  return (
    <div
      style={{
        marginLeft: "24px",
        marginTop: "0.35rem",
        padding: "0.4rem 0.6rem",
        backgroundColor: "#f3f4f6",
        border: "1px solid #d1d5db",
        borderRadius: "4px",
        fontSize: "0.8rem",
        color: "#374151",
        display: "flex",
        alignItems: "center",
        gap: "0.35rem",
      }}
    >
      <span>📄</span>
      <span>{document.title}</span>
      {document.page_number !== undefined && (
        <span style={{ color: "#6b7280" }}>(p.{document.page_number})</span>
      )}
    </div>
  );
};

export default EvidenceExplorerPage;
