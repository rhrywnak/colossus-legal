import React, { useEffect, useState } from "react";
import { getAllegations, AllegationDto } from "../services/allegations";
import {
  getEvidenceChain,
  EvidenceChainResponse,
  MotionClaimWithEvidence,
  EvidenceWithDocument,
} from "../services/evidenceChain";

// Professional color palette
const COLORS = {
  bgPage: "#f8fafc",
  bgCard: "#ffffff",
  border: "#e2e8f0",
  textPrimary: "#1e293b",
  textSecondary: "#64748b",
  textMuted: "#94a3b8",
  accentExpanded: "#059669",
  accentMotionClaim: "#3b82f6",
  accentEvidence: "#8b5cf6",
  badgeIdBg: "#f1f5f9",
  badgeIdText: "#475569",
  badgeProvenBg: "#ecfdf5",
  badgeProvenText: "#059669",
  badgePartialBg: "#fffbeb",
  badgePartialText: "#d97706",
  badgeUnprovenBg: "#fef2f2",
  badgeUnprovenText: "#dc2626",
  badgeLegalBg: "#e0e7ff",
  badgeLegalText: "#4338ca",
};

const STATUS_STYLES: Record<string, { bg: string; text: string }> = {
  PROVEN: { bg: COLORS.badgeProvenBg, text: COLORS.badgeProvenText },
  PARTIAL: { bg: COLORS.badgePartialBg, text: COLORS.badgePartialText },
  UNPROVEN: { bg: COLORS.badgeUnprovenBg, text: COLORS.badgeUnprovenText },
};

const DEFAULT_STATUS_STYLE = { bg: COLORS.badgeIdBg, text: COLORS.textSecondary };

function getStatusStyle(status: string | undefined) {
  if (!status) return DEFAULT_STATUS_STYLE;
  return STATUS_STYLES[status.toUpperCase()] || DEFAULT_STATUS_STYLE;
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

  if (error) {
    return (
      <div
        style={{
          padding: "2rem",
          backgroundColor: COLORS.bgPage,
          minHeight: "100vh",
        }}
      >
        <div
          style={{
            padding: "1rem 1.25rem",
            backgroundColor: COLORS.badgeUnprovenBg,
            border: `1px solid ${COLORS.border}`,
            borderRadius: "8px",
            color: COLORS.badgeUnprovenText,
            maxWidth: "1200px",
            margin: "0 auto",
          }}
        >
          {error}
        </div>
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
          Evidence Explorer
        </h1>
        <p
          style={{
            fontSize: "0.95rem",
            color: COLORS.textSecondary,
            marginTop: "0.25rem",
            marginBottom: 0,
          }}
        >
          Click an allegation to view its supporting evidence chain
        </p>

        {/* Divider */}
        <div
          style={{
            borderBottom: `1px solid ${COLORS.border}`,
            margin: "1.5rem 0",
          }}
        />

        {/* Allegations list */}
        {allegations.length === 0 ? (
          <div
            style={{
              color: COLORS.textMuted,
              padding: "2rem",
              textAlign: "center",
              fontStyle: "italic",
            }}
          >
            No allegations found.
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
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
        backgroundColor: COLORS.bgCard,
        border: `1px solid ${COLORS.border}`,
        borderLeft: isExpanded ? `3px solid ${COLORS.accentExpanded}` : `1px solid ${COLORS.border}`,
        borderRadius: "8px",
        boxShadow: "0 1px 2px rgba(0,0,0,0.04)",
        overflow: "hidden",
        transition: "box-shadow 0.15s ease",
      }}
    >
      {/* Allegation header row */}
      <div
        onClick={onToggle}
        style={{
          padding: "1rem 1.25rem",
          cursor: "pointer",
          display: "flex",
          alignItems: "center",
          gap: "0.75rem",
        }}
        onMouseEnter={(e) => {
          e.currentTarget.style.boxShadow = "0 2px 4px rgba(0,0,0,0.08)";
        }}
        onMouseLeave={(e) => {
          e.currentTarget.style.boxShadow = "none";
        }}
      >
        {/* Toggle icon */}
        <span
          style={{
            fontSize: "0.75rem",
            color: COLORS.textSecondary,
            width: "1.5rem",
            textAlign: "center",
            transition: "color 0.15s ease",
          }}
        >
          {isLoading ? (
            <span style={{ color: COLORS.textMuted }}>...</span>
          ) : isExpanded ? (
            "▼"
          ) : (
            "▶"
          )}
        </span>

        {/* ID badge */}
        <span
          style={{
            padding: "0.2rem 0.5rem",
            backgroundColor: COLORS.badgeIdBg,
            color: COLORS.badgeIdText,
            borderRadius: "4px",
            fontSize: "0.75rem",
            fontFamily: "monospace",
          }}
        >
          {allegation.id}
        </span>

        {/* Title */}
        <span
          style={{
            fontSize: "1rem",
            fontWeight: 500,
            color: COLORS.textPrimary,
            flex: 1,
            marginLeft: "0.5rem",
          }}
        >
          {allegation.title}
        </span>

        {/* Status badge */}
        {allegation.evidence_status && (
          <span
            style={{
              padding: "0.2rem 0.6rem",
              backgroundColor: statusStyle.bg,
              color: statusStyle.text,
              borderRadius: "9999px",
              fontSize: "0.7rem",
              fontWeight: 600,
              textTransform: "uppercase",
              letterSpacing: "0.025em",
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
            padding: "0 1.25rem 1.25rem 1.25rem",
          }}
        >
          {/* Summary line */}
          <div
            style={{
              fontSize: "0.85rem",
              color: COLORS.textSecondary,
              marginBottom: "0.5rem",
            }}
          >
            {chain.summary.motion_claim_count} claims · {chain.summary.evidence_count} evidence · {chain.summary.document_count} documents
          </div>

          {/* Legal counts */}
          {chain.allegation.legal_counts && chain.allegation.legal_counts.length > 0 && (
            <div style={{ marginBottom: "1rem", display: "flex", alignItems: "center", gap: "0.5rem", flexWrap: "wrap" }}>
              <span style={{ fontSize: "0.85rem", color: COLORS.textSecondary }}>Supports:</span>
              {chain.allegation.legal_counts.map((count, i) => (
                <span
                  key={i}
                  style={{
                    padding: "0.15rem 0.5rem",
                    backgroundColor: COLORS.badgeLegalBg,
                    color: COLORS.badgeLegalText,
                    borderRadius: "4px",
                    fontSize: "0.7rem",
                    fontWeight: 500,
                  }}
                >
                  {count}
                </span>
              ))}
            </div>
          )}

          {/* Evidence chain container */}
          <div
            style={{
              borderTop: `1px solid ${COLORS.border}`,
              paddingTop: "1rem",
              marginTop: "0.5rem",
            }}
          >
            {/* Motion claims */}
            {chain.motion_claims.length === 0 ? (
              <div
                style={{
                  color: COLORS.textMuted,
                  fontStyle: "italic",
                  padding: "1rem",
                }}
              >
                No supporting evidence found
              </div>
            ) : (
              <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
                {chain.motion_claims.map((mc) => (
                  <MotionClaimSection key={mc.id} motionClaim={mc} />
                ))}
              </div>
            )}
          </div>
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
        backgroundColor: COLORS.bgCard,
        border: `1px solid ${COLORS.border}`,
        borderLeft: `3px solid ${COLORS.accentMotionClaim}`,
        borderRadius: "6px",
        padding: "1rem",
      }}
    >
      <div
        style={{
          fontWeight: 500,
          fontSize: "0.95rem",
          color: COLORS.textPrimary,
          marginBottom: "0.75rem",
        }}
      >
        {motionClaim.title}
      </div>

      {motionClaim.evidence.length === 0 ? (
        <div
          style={{
            color: COLORS.textMuted,
            fontSize: "0.85rem",
            fontStyle: "italic",
          }}
        >
          No evidence linked
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
        marginLeft: "1rem",
        backgroundColor: COLORS.bgCard,
        border: `1px solid ${COLORS.border}`,
        borderLeft: `3px solid ${COLORS.accentEvidence}`,
        borderRadius: "6px",
        padding: "0.875rem",
      }}
    >
      <div
        style={{
          fontWeight: 500,
          fontSize: "0.9rem",
          color: COLORS.textPrimary,
          marginBottom: "0.5rem",
        }}
      >
        {evidence.title}
      </div>

      {evidence.question && (
        <div
          style={{
            fontSize: "0.85rem",
            color: COLORS.textSecondary,
            marginBottom: "0.35rem",
          }}
        >
          <span style={{ fontWeight: 600 }}>Q:</span>{" "}
          <span style={{ color: COLORS.badgeIdText }}>{evidence.question}</span>
        </div>
      )}

      {evidence.answer && (
        <div
          style={{
            fontSize: "0.85rem",
            color: COLORS.textSecondary,
            marginBottom: "0.5rem",
          }}
        >
          <span style={{ fontWeight: 600 }}>A:</span>{" "}
          <span style={{ color: COLORS.textPrimary, fontStyle: "italic" }}>
            "{evidence.answer}"
          </span>
        </div>
      )}

      {evidence.document && <DocumentLink document={evidence.document} />}
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
        marginLeft: "1.5rem",
        marginTop: "0.5rem",
        padding: "0.5rem 0.75rem",
        backgroundColor: COLORS.bgPage,
        border: `1px solid ${COLORS.border}`,
        borderRadius: "4px",
        fontSize: "0.85rem",
        color: COLORS.badgeIdText,
        display: "flex",
        alignItems: "center",
        gap: "0.5rem",
      }}
    >
      <span style={{ color: COLORS.textSecondary }}>Source:</span>
      <span style={{ fontWeight: 500 }}>{document.title}</span>
      {document.page_number !== undefined && (
        <span style={{ color: COLORS.textSecondary }}>(p. {document.page_number})</span>
      )}
    </div>
  );
};

export default EvidenceExplorerPage;
