// =============================================================================
// ElementDetailContent.tsx — one Element's detail, in normal page flow
// -----------------------------------------------------------------------------
// The non-floating body extracted for the routed Count-detail page
// (Stage 1-1): "What plaintiff must prove" + an auto-saved review-notes editor
// + the mapped Allegations grouped Common / Dedicated / Other and sorted by
// paragraph number.
//
// This is the full-page replacement for the floating ElementDetailPanel's body.
// It deliberately ports the panel's data fetch, notes debounce + SaveStatus,
// and allegation grouping/sort VERBATIM, but drops every floating-panel concern
// (react-draggable, CSS resize, position:fixed, the grip, the close button,
// role="dialog"). It is self-contained — it does NOT import from
// ElementDetailPanel, which Home instruction (2 of 2) removes.
//
// All counts shown here derive from BEARS_ON via the element-detail endpoint;
// no legal_count_ids, no client-side count math.
// =============================================================================

import React, { useCallback, useEffect, useRef, useState } from "react";
import {
  AllegationSummary,
  ElementDetailResponse,
  fetchElementDetail,
  saveElementNotes,
} from "../services/elementDetailService";
import { parseLeadingParagraph } from "../utils/paragraphSort";

export interface ElementDetailContentProps {
  caseSlug: string;
  /** Stable Element id like `element-1-1`. Changing it re-fetches. */
  elementId: string;
  /** Header name shown while the /detail fetch is in flight. */
  elementName: string;
}

/** Wait this many ms of no typing before auto-saving notes (ported). */
const NOTES_DEBOUNCE_MS = 2000;

/** Minimum textarea height when the notes section is expanded. */
const NOTES_EXPANDED_MIN_HEIGHT_PX = 120;

// ─── Pure helper (ported from the panel) ─────────────────────────────────────

/**
 * Sort Allegations by parsed paragraph_number ascending; non-numeric entries
 * sort last, stable among themselves. Copy-first so the caller's array isn't
 * mutated. Self-contained (does not import the panel's copy).
 */
export function sortAllegationsByParagraph(
  allegations: AllegationSummary[],
): AllegationSummary[] {
  const indexed = allegations.map((a, idx) => ({ a, idx }));
  indexed.sort((x, y) => {
    const px = parseLeadingParagraph(x.a.paragraph_number);
    const py = parseLeadingParagraph(y.a.paragraph_number);
    if (px === null && py === null) return x.idx - y.idx;
    if (px === null) return 1;
    if (py === null) return -1;
    if (px !== py) return px - py;
    return x.idx - y.idx;
  });
  return indexed.map((e) => e.a);
}

// ─── Save-status (ported tagged union) ───────────────────────────────────────

type SaveStatus =
  | { kind: "idle" }
  | { kind: "saving" }
  | { kind: "saved" }
  | { kind: "error"; message: string };

// ─── Styles (page-flow versions of the panel's body styles) ──────────────────
// Common: amber palette (--burden-warning-*). Dedicated: blue (--accent-*).
// Unknown: muted secondary. No floating-panel styles here.

const SECTION_COLOR_COMMON = "var(--burden-warning-text)";
const SECTION_BG_COMMON = "var(--burden-warning-bg)";
const SECTION_COLOR_DEDICATED = "var(--accent-primary)";
const SECTION_BG_DEDICATED = "var(--accent-bg-soft)";

const PROOF_LABEL_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "11px",
  fontWeight: 600,
  letterSpacing: "0.05em",
  textTransform: "uppercase",
  color: "var(--text-secondary)",
  marginBottom: "4px",
};

const PROOF_TEXT_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "14px",
  color: "var(--text-primary)",
  lineHeight: 1.5,
};

const NOTES_HEADER_STYLE: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  padding: "10px 0",
  cursor: "pointer",
  fontFamily: "var(--font-sans)",
  fontSize: "13px",
  fontWeight: 600,
  color: "var(--text-primary)",
  userSelect: "none",
};

const NOTES_TEXTAREA_STYLE: React.CSSProperties = {
  display: "block",
  width: "100%",
  minHeight: `${NOTES_EXPANDED_MIN_HEIGHT_PX}px`,
  resize: "vertical",
  fontFamily: "var(--font-sans)",
  fontSize: "13px",
  color: "var(--text-primary)",
  backgroundColor: "var(--bg-surface)",
  border: "1px solid var(--border-default)",
  borderRadius: "6px",
  padding: "8px 10px",
  boxSizing: "border-box",
  lineHeight: 1.45,
  marginBottom: "12px",
};

const ALLEGATIONS_HEADER_STYLE: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  padding: "10px 0",
  fontFamily: "var(--font-sans)",
  fontSize: "12px",
  fontWeight: 600,
  color: "var(--text-secondary)",
  textTransform: "uppercase",
  letterSpacing: "0.05em",
};

const SECTION_DIVIDER_STYLE_BASE: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "8px",
  margin: "12px 0 8px",
  fontFamily: "var(--font-sans)",
  fontSize: "11px",
  fontWeight: 700,
  letterSpacing: "0.05em",
  textTransform: "uppercase",
};

const SECTION_DIVIDER_RULE_STYLE: React.CSSProperties = {
  flex: 1,
  height: "1px",
  backgroundColor: "var(--border-default)",
};

const ALLEGATION_CARD_STYLE: React.CSSProperties = {
  padding: "10px 12px",
  marginBottom: "8px",
  border: "1px solid var(--border-default)",
  borderRadius: "8px",
  backgroundColor: "var(--bg-surface)",
};

const PARAGRAPH_LABEL_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: "15px",
  fontWeight: 700,
  color: "var(--text-primary)",
};

const SUMMARY_TEXT_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "13px",
  color: "var(--text-secondary)",
  marginLeft: "8px",
};

// Layout-only style for the verbatim quote. Typography (size/weight/color/font)
// comes from the `.proof-text` utility class in tokens.css — the design system's
// canonical body/proof treatment — so this object keeps only spacing and the
// dynamic left border.
const QUOTE_TEXT_STYLE_BASE: React.CSSProperties = {
  marginTop: "6px",
  paddingLeft: "8px",
  lineHeight: 1.45,
};

const STATUS_INDICATOR_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "11px",
  color: "var(--text-muted)",
};

const STATUS_ERROR_STYLE: React.CSSProperties = {
  ...STATUS_INDICATOR_STYLE,
  color: "var(--status-dropped-text)",
};

const BODY_MESSAGE_STYLE: React.CSSProperties = {
  padding: "24px 0",
  textAlign: "center",
  fontFamily: "var(--font-sans)",
  fontSize: "13px",
  color: "var(--text-secondary)",
};

const RETRY_BTN_STYLE: React.CSSProperties = {
  marginTop: "12px",
  padding: "6px 14px",
  border: "1px solid var(--accent-primary)",
  backgroundColor: "transparent",
  color: "var(--accent-primary)",
  borderRadius: "6px",
  fontSize: "12px",
  fontWeight: 600,
  cursor: "pointer",
};

// ─── Component ────────────────────────────────────────────────────────────────

/**
 * Renders one Element's detail in normal document flow. Fetches on
 * mount / `elementId` change with a cancel flag (ported pattern), debounces
 * note saves, and surfaces save status as a distinct observable (Rule 1).
 */
const ElementDetailContent: React.FC<ElementDetailContentProps> = ({
  caseSlug,
  elementId,
  elementName,
}) => {
  const [detail, setDetail] = useState<ElementDetailResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [reloadToken, setReloadToken] = useState(0);

  const [notesValue, setNotesValue] = useState<string>("");
  const [notesExpanded, setNotesExpanded] = useState(false);
  const [saveStatus, setSaveStatus] = useState<SaveStatus>({ kind: "idle" });

  const debounceRef = useRef<number | null>(null);
  const lastSavedRef = useRef<string | null>(null);

  // ── Fetch effect (ported) ──────────────────────────────────────────────
  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);

    fetchElementDetail(caseSlug, elementId)
      .then((data) => {
        if (cancelled) return;
        setDetail(data);
        setNotesValue(data.review_notes ?? "");
        lastSavedRef.current = data.review_notes; // preserves null ≠ ""
        setSaveStatus({ kind: "idle" });
        setNotesExpanded(false);
        setLoading(false);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        setError(
          err instanceof Error ? err.message : "Failed to load Element detail.",
        );
        setLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [caseSlug, elementId, reloadToken]);

  // ── Notes save (ported) ─────────────────────────────────────────────────
  const persistNotes = useCallback(
    async (raw: string) => {
      const candidate: string | null = raw === "" ? null : raw;
      if (candidate === lastSavedRef.current) return;

      setSaveStatus({ kind: "saving" });
      try {
        await saveElementNotes(caseSlug, elementId, candidate);
        lastSavedRef.current = candidate;
        setSaveStatus({ kind: "saved" });
      } catch (err: unknown) {
        setSaveStatus({
          kind: "error",
          message: err instanceof Error ? err.message : "Failed to save notes.",
        });
      }
    },
    [caseSlug, elementId],
  );

  // Debounced save (ported): each keystroke (re)starts the timer.
  useEffect(() => {
    if (detail === null) return;
    if (debounceRef.current !== null) {
      window.clearTimeout(debounceRef.current);
    }
    debounceRef.current = window.setTimeout(() => {
      persistNotes(notesValue);
    }, NOTES_DEBOUNCE_MS);
    return () => {
      if (debounceRef.current !== null) {
        window.clearTimeout(debounceRef.current);
        debounceRef.current = null;
      }
    };
  }, [notesValue, detail, persistNotes]);

  // ── Allegation grouping (ported) ───────────────────────────────────────
  const sorted = detail ? sortAllegationsByParagraph(detail.allegations) : [];
  const common = sorted.filter((a) => a.source_section === "Common");
  const dedicated = sorted.filter((a) => a.source_section === "Dedicated");
  const unknown = sorted.filter(
    (a) => a.source_section !== "Common" && a.source_section !== "Dedicated",
  );

  const statusText = (() => {
    switch (saveStatus.kind) {
      case "idle":
        return "";
      case "saving":
        return "saving...";
      case "saved":
        return "✓ saved";
      case "error":
        return `save failed (${saveStatus.message})`;
    }
  })();

  if (loading) {
    return <div style={BODY_MESSAGE_STYLE}>Loading {elementName} detail...</div>;
  }

  if (error) {
    return (
      <div style={BODY_MESSAGE_STYLE}>
        <div style={{ color: "var(--status-dropped-text)" }}>{error}</div>
        <button
          type="button"
          style={RETRY_BTN_STYLE}
          onClick={() => setReloadToken((n) => n + 1)}
        >
          Retry
        </button>
      </div>
    );
  }

  if (!detail) {
    return <div style={BODY_MESSAGE_STYLE}>No Element detail available.</div>;
  }

  return (
    <div>
      {/* What plaintiff must prove */}
      <div style={{ marginBottom: "16px" }}>
        <div style={PROOF_LABEL_STYLE}>What plaintiff must prove</div>
        <div style={PROOF_TEXT_STYLE}>{detail.what_plaintiff_must_prove}</div>
      </div>

      {/* Review notes (collapsible) */}
      <div style={{ borderTop: "1px solid var(--border-default)" }}>
        <div
          style={NOTES_HEADER_STYLE}
          onClick={() => setNotesExpanded((v) => !v)}
          role="button"
          tabIndex={0}
          aria-expanded={notesExpanded}
          aria-controls="element-detail-notes-textarea"
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
              setNotesExpanded((v) => !v);
            }
          }}
        >
          <span>{notesExpanded ? "▾" : "▸"} Review notes</span>
          <span
            style={
              saveStatus.kind === "error"
                ? STATUS_ERROR_STYLE
                : STATUS_INDICATOR_STYLE
            }
          >
            {statusText}
          </span>
        </div>
        {notesExpanded && (
          <textarea
            id="element-detail-notes-textarea"
            style={NOTES_TEXTAREA_STYLE}
            value={notesValue}
            onChange={(e) => setNotesValue(e.target.value)}
            onBlur={() => persistNotes(notesValue)}
            placeholder="Write mapping review notes here..."
          />
        )}
      </div>

      {/* Allegations header */}
      <div style={{ borderTop: "1px solid var(--border-default)" }}>
        <div style={ALLEGATIONS_HEADER_STYLE}>
          <span>
            {detail.allegation_count} allegations mapped ({detail.common_count}{" "}
            common · {detail.dedicated_count} dedicated)
          </span>
          <span>by ¶ number</span>
        </div>
      </div>

      {/* Allegations list */}
      {detail.allegation_count === 0 ? (
        <div style={BODY_MESSAGE_STYLE}>
          No allegations mapped to this Element
        </div>
      ) : (
        <>
          {common.length > 0 && (
            <AllegationSection
              label="Common Allegations"
              labelColor={SECTION_COLOR_COMMON}
              labelBg={SECTION_BG_COMMON}
              accentColor={SECTION_COLOR_COMMON}
              allegations={common}
            />
          )}
          {dedicated.length > 0 && (
            <AllegationSection
              label={
                detail.count_number != null
                  ? `Count ${detail.count_number} Specific`
                  : "Count Specific"
              }
              labelColor={SECTION_COLOR_DEDICATED}
              labelBg={SECTION_BG_DEDICATED}
              accentColor={SECTION_COLOR_DEDICATED}
              allegations={dedicated}
            />
          )}
          {unknown.length > 0 && (
            <AllegationSection
              label="Other"
              labelColor="var(--text-secondary)"
              labelBg="var(--bg-page)"
              accentColor="var(--text-muted)"
              allegations={unknown}
            />
          )}
        </>
      )}
    </div>
  );
};

// ─── AllegationSection (ported) ──────────────────────────────────────────────

const AllegationSection: React.FC<{
  label: string;
  labelColor: string;
  labelBg: string;
  accentColor: string;
  allegations: AllegationSummary[];
}> = ({ label, labelColor, labelBg, accentColor, allegations }) => (
  <div>
    <div style={SECTION_DIVIDER_STYLE_BASE}>
      <span
        style={{
          color: labelColor,
          backgroundColor: labelBg,
          padding: "2px 8px",
          borderRadius: "12px",
        }}
      >
        {label}
      </span>
      <span style={SECTION_DIVIDER_RULE_STYLE} />
    </div>
    {allegations.map((a) => (
      <div key={a.allegation_id} style={ALLEGATION_CARD_STYLE}>
        <div>
          <span style={PARAGRAPH_LABEL_STYLE}>¶{a.paragraph_number}</span>
          {a.summary && <span style={SUMMARY_TEXT_STYLE}>{a.summary}</span>}
        </div>
        {a.verbatim_quote && (
          <div
            className="proof-text"
            style={{
              ...QUOTE_TEXT_STYLE_BASE,
              borderLeft: `3px solid ${accentColor}`,
            }}
          >
            {a.verbatim_quote}
          </div>
        )}
      </div>
    ))}
  </div>
);

export default ElementDetailContent;
