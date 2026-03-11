import React from "react";
import { AllegationDto } from "../services/allegations";
import { EvidenceChainResponse } from "../services/evidenceChain";
import { COLORS, getStatusStyle, MotionClaimSection } from "./EvidenceChainParts";
import { displayStatus } from "../utils/legalTerms";

// ─── Types ───────────────────────────────────────────────────────────────────

export type CountGroup = {
  countName: string;
  numeral: string;
  legalBasis: string;
  paragraphs: string;
  allegations: AllegationDto[];
  provenCount: number;
};

// ─── CountSection ────────────────────────────────────────────────────────────

type CountSectionProps = {
  group: CountGroup;
  collapsed: boolean;
  onToggleCollapse: () => void;
  expandedIds: Set<string>;
  loadingIds: Set<string>;
  chainCache: Map<string, EvidenceChainResponse>;
  onToggleAllegation: (id: string) => void;
};

export const CountSection: React.FC<CountSectionProps> = ({
  group, collapsed, onToggleCollapse, expandedIds, loadingIds, chainCache, onToggleAllegation,
}) => {
  const title = group.numeral
    ? `COUNT ${group.numeral}: ${group.countName}`
    : group.countName;

  return (
    <div style={{
      backgroundColor: COLORS.bgCard, border: `1px solid ${COLORS.border}`,
      borderLeft: `4px solid ${COLORS.accentMotionClaim}`, borderRadius: "8px",
      boxShadow: "0 1px 3px rgba(0,0,0,0.06)", overflow: "hidden",
    }}>
      {/* Count header */}
      <div
        onClick={onToggleCollapse}
        style={{ padding: "1rem 1.25rem", cursor: "pointer", userSelect: "none" }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
          <span style={{ fontSize: "0.8rem", color: COLORS.textSecondary }}>{collapsed ? "▶" : "▼"}</span>
          <span style={{ fontSize: "1.15rem", fontWeight: 700, color: COLORS.textPrimary }}>{title}</span>
        </div>
        {group.numeral && (
          <div style={{ fontSize: "0.82rem", color: COLORS.textSecondary, marginTop: "0.25rem", marginLeft: "1.3rem" }}>
            {group.legalBasis}
            {group.paragraphs && <> &middot; Complaint &para;{group.paragraphs}</>}
            {" "}&middot; {group.allegations.length} allegation{group.allegations.length !== 1 ? "s" : ""}
            {" "}&middot; {group.provenCount} proven
          </div>
        )}
      </div>

      {/* Allegation rows */}
      {!collapsed && (
        <div style={{ padding: "0 1.25rem 1.25rem 1.25rem", display: "flex", flexDirection: "column", gap: "0.75rem" }}>
          {group.allegations.map((a) => (
            <AllegationRow
              key={a.id}
              allegation={a}
              isExpanded={expandedIds.has(a.id)}
              isLoading={loadingIds.has(a.id)}
              chain={chainCache.get(a.id)}
              onToggle={() => onToggleAllegation(a.id)}
            />
          ))}
        </div>
      )}
    </div>
  );
};

// ─── AllegationRow ───────────────────────────────────────────────────────────

type AllegationRowProps = {
  allegation: AllegationDto;
  isExpanded: boolean;
  isLoading: boolean;
  chain?: EvidenceChainResponse;
  onToggle: () => void;
};

const AllegationRow: React.FC<AllegationRowProps> = ({ allegation, isExpanded, isLoading, chain, onToggle }) => {
  const statusStyle = getStatusStyle(allegation.evidence_status);

  return (
    <div style={{
      backgroundColor: COLORS.bgCard, border: `1px solid ${COLORS.border}`,
      borderLeft: isExpanded ? `3px solid ${COLORS.accentExpanded}` : `1px solid ${COLORS.border}`,
      borderRadius: "8px", boxShadow: "0 1px 2px rgba(0,0,0,0.04)", overflow: "hidden",
    }}>
      {/* Header row */}
      <div onClick={onToggle} style={{ padding: "1rem 1.25rem", cursor: "pointer", display: "flex", alignItems: "center", gap: "0.75rem" }}>
        <span style={{ fontSize: "0.75rem", color: COLORS.textSecondary, width: "1.5rem", textAlign: "center" }}>
          {isLoading ? <span style={{ color: COLORS.textMuted }}>...</span> : isExpanded ? "▼" : "▶"}
        </span>

        {/* Paragraph badge */}
        {allegation.paragraph && (
          <span style={{
            padding: "0.2rem 0.5rem", backgroundColor: COLORS.badgeParaBg, color: COLORS.badgeParaText,
            borderRadius: "4px", fontSize: "0.75rem", fontWeight: 600,
          }}>
            &para;{allegation.paragraph}
          </span>
        )}

        {/* ID badge */}
        <span style={{
          padding: "0.2rem 0.5rem", backgroundColor: COLORS.badgeIdBg, color: COLORS.badgeIdText,
          borderRadius: "4px", fontSize: "0.75rem", fontFamily: "monospace",
        }}>
          {allegation.id}
        </span>

        {/* Title */}
        <span style={{ fontSize: "1rem", fontWeight: 500, color: COLORS.textPrimary, flex: 1, marginLeft: "0.5rem" }}>
          {allegation.title}
        </span>

        {/* Status badge */}
        {allegation.evidence_status && (
          <span style={{
            padding: "0.2rem 0.6rem", backgroundColor: statusStyle.bg, color: statusStyle.text,
            borderRadius: "9999px", fontSize: "0.7rem", fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.025em",
          }}>
            {displayStatus(allegation.evidence_status)}
          </span>
        )}
      </div>

      {/* Expanded content */}
      {isExpanded && chain && (
        <div style={{ padding: "0 1.25rem 1.25rem 1.25rem" }}>
          {/* Verbatim complaint text */}
          {allegation.allegation && (
            <div style={{ marginBottom: "1rem" }}>
              <div style={{ fontSize: "0.75rem", fontWeight: 600, color: COLORS.textSecondary, textTransform: "uppercase", letterSpacing: "0.04em", marginBottom: "0.35rem" }}>
                From the Complaint:
              </div>
              <blockquote style={{
                margin: 0, padding: "0.75rem 1rem", borderLeft: `3px solid ${COLORS.badgeLegalBg}`,
                backgroundColor: "#fafbff", fontFamily: "Georgia, serif", fontSize: "0.9rem",
                color: COLORS.textPrimary, lineHeight: 1.7, borderRadius: "0 6px 6px 0",
              }}>
                {allegation.allegation}
              </blockquote>
            </div>
          )}

          {/* Summary line */}
          <div style={{ fontSize: "0.85rem", color: COLORS.textSecondary, marginBottom: "0.5rem" }}>
            {chain.summary.motion_claim_count} claims &middot; {chain.summary.evidence_count} evidence &middot; {chain.summary.document_count} documents
          </div>

          {/* Evidence chain */}
          <div style={{ borderTop: `1px solid ${COLORS.border}`, paddingTop: "1rem", marginTop: "0.5rem" }}>
            {chain.motion_claims.length === 0 ? (
              <div style={{ color: COLORS.textMuted, fontStyle: "italic", padding: "1rem" }}>
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
