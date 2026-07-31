import React, { useState } from "react";
import { AllegationDto } from "../services/allegations";
import { AllegationStrength } from "../services/analysisApi";
import { EvidenceChainResponse } from "../services/evidenceChain";
import { COLORS, getStatusStyle, MotionClaimSection } from "./EvidenceChainParts";
import { displayStatus } from "../utils/legalTerms";

// ─── Types ───────────────────────────────────────────────────────────────────

export type CountGroup = {
  /** Display name for the count (from LegalCount.name / Neo4j title). */
  countName: string;
  /** Stable LegalCount id, or null for the synthetic "Other" bucket. */
  countId: string | null;
  /**
   * `LegalCount.count_number` — the schema-required integer (I=1, II=2, …).
   * `0` for the "Other" bucket so it sorts last and suppresses the numeral.
   */
  countNumber: number;
  allegations: AllegationDto[];
};

const ROMAN_NUMERALS: Record<number, string> = {
  1: "I", 2: "II", 3: "III", 4: "IV", 5: "V", 6: "VI", 7: "VII", 8: "VIII",
};

function toNumeral(countNumber: number): string {
  if (countNumber <= 0) return "";
  return ROMAN_NUMERALS[countNumber] ?? String(countNumber);
}

// ─── CountSection ────────────────────────────────────────────────────────────

type CountSectionProps = {
  group: CountGroup;
  collapsed: boolean;
  onToggleCollapse: () => void;
  expandedIds: Set<string>;
  loadingIds: Set<string>;
  chainCache: Map<string, EvidenceChainResponse>;
  /** Per-allegation chain-fetch error messages keyed by allegation id. */
  errorMap: Map<string, string>;
  onToggleAllegation: (id: string) => void;
  /** Re-run the chain fetch for an allegation whose previous attempt errored. */
  onRetryAllegation: (id: string) => void;
  strengthMap: Map<string, AllegationStrength>;
};

export const CountSection: React.FC<CountSectionProps> = ({
  group, collapsed, onToggleCollapse, expandedIds, loadingIds, chainCache, errorMap, onToggleAllegation, onRetryAllegation, strengthMap,
}) => {
  const numeral = toNumeral(group.countNumber);
  // LLM-authored LegalCount.name often already begins with "COUNT I –".
  // Only prepend the numeral when the label doesn't already carry it,
  // otherwise the card title reads "COUNT I: COUNT I – BREACH ...".
  const labelStartsWithCount = /^\s*count\b/i.test(group.countName);
  const title =
    numeral && !labelStartsWithCount
      ? `COUNT ${numeral}: ${group.countName}`
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
        {numeral && (
          <div style={{ fontSize: "0.82rem", color: COLORS.textSecondary, marginTop: "0.25rem", marginLeft: "1.3rem" }}>
            {group.allegations.length} allegation{group.allegations.length !== 1 ? "s" : ""}
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
              error={errorMap.get(a.id)}
              onToggle={() => onToggleAllegation(a.id)}
              onRetry={() => onRetryAllegation(a.id)}
              strength={strengthMap.get(a.id)}
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
  /** Present when the most recent chain fetch for this row failed. */
  error?: string;
  onToggle: () => void;
  /** Re-run the chain fetch from the inline error block. */
  onRetry: () => void;
  strength?: AllegationStrength;
};

const AllegationRow: React.FC<AllegationRowProps> = ({ allegation, isExpanded, isLoading, chain, error, onToggle, onRetry, strength }) => {
  const [showEvidence, setShowEvidence] = useState(false);
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

        {/* Paragraph badge — primary identifier, bold and prominent.
            When paragraph_number is missing the title carries the row alone. */}
        {allegation.paragraph && (
          <span style={{
            padding: "0.2rem 0.5rem", backgroundColor: COLORS.badgeParaBg, color: COLORS.badgeParaText,
            borderRadius: "4px", fontSize: "0.75rem", fontWeight: 600,
          }}>
            &para;{allegation.paragraph}
          </span>
        )}

        {/* Title — sits immediately next to the paragraph badge so the
            card reads left-to-right as "¶N {summary}". The raw
            stable_entity_id badge that previously sat between them was
            removed (CC Instruction F E2 — internal system IDs must not be
            shown to users). The 0.5rem marginLeft that used to space the
            title from the removed ID badge is dropped: the parent flex's
            gap (0.75rem) is now the sole spacing between paragraph badge
            and title, which is the intended adjacent-look. */}
        <span style={{ fontSize: "1rem", fontWeight: 500, color: COLORS.textPrimary, flex: 1 }}>
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

      {/* Evidence tally — the measured count, with the item titles behind a
          disclosure. The fabricated percentage badge and proportional bar that
          used to sit here were retired on 2026-07-27: both rendered a five-value
          lookup on this same count, so they added no information while reading
          as a measurement. */}
      {strength && (
        <div style={{ padding: "0 1.25rem 0.75rem 3.75rem" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
            <span style={{ fontSize: "0.7rem", color: COLORS.textMuted }}>
              {strength.supporting_evidence_count} evidence item
              {strength.supporting_evidence_count !== 1 ? "s" : ""}
            </span>
            {strength.supporting_evidence && strength.supporting_evidence.length > 0 && (
              <span
                onClick={(e) => { e.stopPropagation(); setShowEvidence((prev) => !prev); }}
                style={{
                  marginLeft: "auto", cursor: "pointer", fontSize: "0.8rem",
                  color: "var(--text-disabled)", userSelect: "none",
                }}
              >
                {showEvidence ? "▾ Less" : "▸ More"}
              </span>
            )}
          </div>
          {showEvidence && strength.supporting_evidence && (
            <div style={{
              marginTop: "0.5rem", marginLeft: "24px",
              paddingLeft: "0.75rem", borderLeft: "3px solid var(--border-default)",
            }}>
              {strength.supporting_evidence.map((title, i) => (
                <div key={i} style={{ fontSize: "0.85rem", color: "var(--text-muted)", padding: "0.15rem 0" }}>
                  {title}
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Expanded content — error block takes precedence over the chain so
          a stale-cache + error-on-retry doesn't render both. */}
      {isExpanded && error && (
        <div style={{ padding: "0 1.25rem 1.25rem 1.25rem" }}>
          <div
            style={{
              padding: "0.75rem 1rem",
              backgroundColor: "var(--state-danger-bg-soft)",
              border: "1px solid var(--state-danger-border)",
              borderRadius: "6px",
              color: "var(--status-dropped-text)",
              fontSize: "0.85rem",
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              gap: "1rem",
            }}
          >
            <span>Failed to load evidence chain: {error}</span>
            <button
              type="button"
              onClick={(e) => { e.stopPropagation(); onRetry(); }}
              style={{
                padding: "0.25rem 0.75rem",
                fontSize: "0.8rem",
                fontWeight: 600,
                border: "1px solid var(--status-dropped-text)",
                backgroundColor: "transparent",
                color: "var(--status-dropped-text)",
                borderRadius: "4px",
                cursor: "pointer",
                fontFamily: "inherit",
              }}
            >
              Retry
            </button>
          </div>
        </div>
      )}

      {isExpanded && !error && chain && (
        <div style={{ padding: "0 1.25rem 1.25rem 1.25rem" }}>
          {/* Verbatim complaint text */}
          {allegation.allegation && (
            <div style={{ marginBottom: "1rem" }}>
              <div style={{ fontSize: "0.75rem", fontWeight: 600, color: COLORS.textSecondary, textTransform: "uppercase", letterSpacing: "0.04em", marginBottom: "0.35rem" }}>
                From the Complaint:
              </div>
              <blockquote style={{
                margin: 0, padding: "0.75rem 1rem", borderLeft: `3px solid ${COLORS.badgeLegalBg}`,
                backgroundColor: "var(--bg-surface)", fontFamily: "Georgia, serif", fontSize: "0.9rem",
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
