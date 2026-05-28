// =============================================================================
// ElementDetailPanel.tsx — floating, draggable Element detail panel
// -----------------------------------------------------------------------------
// Opens when the user clicks an Element row on the Home page (CC Instruction
// E2). Read-only Allegation list + editable, auto-saved review notes.
//
// Rendering: the panel is a sibling of the page content, position:fixed, with
// react-draggable wrapping the panel div. Drag is gated to the header bar via
// the `handle` prop so dragging the body (textarea, allegation list) doesn't
// move the panel.
//
// Data flow:
//   - On mount / elementId change → fetchElementDetail() once.
//   - Notes: textarea content saved on blur AND on a 2s debounce of typing.
//     `null` and `""` are intentionally distinguishable (Rule 1).
//   - No edit/reassign of Allegation mappings — view + notes only (per spec).
// =============================================================================

import React, { useCallback, useEffect, useRef, useState } from "react";
import Draggable from "react-draggable";
import {
  AllegationSummary,
  ElementDetailResponse,
  fetchElementDetail,
  saveElementNotes,
} from "../services/elementDetailService";

// ─── Public props ───────────────────────────────────────────────────────────

/**
 * Props for the floating Element detail panel.
 *
 * The parent (Home page) owns the "is this panel open and for which Element"
 * state. Clicking a different Element row updates `elementId` in place and
 * the panel re-fetches; it does not need to close and reopen.
 */
export interface ElementDetailPanelProps {
  caseSlug: string;
  elementId: string;
  /** Displayed in the header while the fetch is in flight. */
  elementName: string;
  /** Header badge value, taken from the row the user clicked. */
  allegationCount: number;
  onClose: () => void;
}

// ─── Tunable constants (CONST — design-spec values, not env-config) ────────
//
// CONST: these are §4 layout/UX values from the spec. They are not
// per-environment runtime config — they are the design system's contract
// for this panel. If they need to flex (e.g., user-resizable panel), promote
// them to props or context; for now they live here so no magic numbers leak
// into the JSX.

/** Wait this many ms of no typing before auto-saving notes. */
const NOTES_DEBOUNCE_MS = 2000;

/** Panel offset from the top of the viewport (default starting position). */
const PANEL_TOP_PX = 80;

/** Panel offset from the right of the viewport (default starting position). */
const PANEL_RIGHT_PX = 40;

/** Panel width in pixels. */
const PANEL_WIDTH_PX = 520;

/** Max height as a viewport-height fraction (panel cannot exceed 80% of vh). */
const PANEL_MAX_HEIGHT_VH = 80;

/** Z-index for the panel; high enough to float above the page content. */
const PANEL_Z_INDEX = 1000;

/** Minimum textarea height when the notes section is expanded. */
const NOTES_EXPANDED_MIN_HEIGHT_PX = 120;

// ─── Pure helpers (exported for unit testing — no DOM, no React) ────────────

/**
 * Parse the leading numeric prefix of a paragraph_number string. Returns
 * `null` for inputs with no leading digit. Range strings like `"16-18"`
 * yield `16` (the start of the range) so they sort by their first
 * paragraph.
 *
 * ## React/TS Learning: pure helpers as named exports
 * Defining this at module scope (not inside the component) keeps it pure
 * and unit-testable without rendering React. The component imports and
 * uses it via [`sortAllegationsByParagraph`].
 */
export function parseLeadingParagraph(paragraphNumber: string): number | null {
  let i = 0;
  while (i < paragraphNumber.length && paragraphNumber[i] >= "0" && paragraphNumber[i] <= "9") {
    i++;
  }
  if (i === 0) return null;
  const parsed = Number.parseInt(paragraphNumber.slice(0, i), 10);
  return Number.isFinite(parsed) ? parsed : null;
}

/**
 * Sort Allegations by parsed paragraph_number ascending. Non-numeric entries
 * sort last (stably preserved among themselves by their original index).
 *
 * The backend already returns the list sorted, but a frontend re-sort makes
 * the panel resilient to any future server-side ordering change and makes
 * the helper independently testable.
 *
 * Pattern: copy first (`[...allegations]`) so callers' arrays are not
 * mutated — same idiom as CountCard.sortElements.
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
    // Same leading int → preserve original order as a stable tiebreaker.
    return x.idx - y.idx;
  });
  return indexed.map((e) => e.a);
}

/**
 * Build the panel header label: "Element {N}.{M} — {name}" when
 * count_number + order_in_count are present; falls back to just the name
 * otherwise. Pure for testability — the component composes it from the
 * fetched payload's metadata.
 */
export function formatElementHeader(
  elementName: string,
  countNumber: number | null,
  orderInCount: number | null,
): string {
  if (countNumber != null && orderInCount != null) {
    return `Element ${countNumber}.${orderInCount} — ${elementName}`;
  }
  return `Element — ${elementName}`;
}

// ─── Save-status indicator ──────────────────────────────────────────────────

/**
 * Discriminated-union state for the notes save status. Each variant is a
 * distinct observable per Rule 1 — the status indicator next to "Review
 * notes" surfaces them all explicitly.
 *
 * ## React/TS Learning: tagged union state
 * Using a `kind` discriminator instead of multiple booleans (`isSaving`,
 * `hasError`) keeps the states mutually exclusive at the type level. The
 * switch in the indicator renderer is exhaustive — adding a new state forces
 * a TypeScript error until every site handles it.
 */
type SaveStatus =
  | { kind: "idle" }
  | { kind: "saving" }
  | { kind: "saved" }
  | { kind: "error"; message: string };

// ─── Inline style objects ───────────────────────────────────────────────────
//
// Styling is inline + var(--token) per Phase-2 redesign conventions. No new
// tokens are introduced: amber (Common) reuses the burden-warning pair, blue
// (Dedicated) reuses the accent pair, as approved.

const HANDLE_CLASS = "element-panel-handle";

const PANEL_STYLE: React.CSSProperties = {
  position: "fixed",
  top: `${PANEL_TOP_PX}px`,
  right: `${PANEL_RIGHT_PX}px`,
  width: `${PANEL_WIDTH_PX}px`,
  maxHeight: `${PANEL_MAX_HEIGHT_VH}vh`,
  display: "flex",
  flexDirection: "column",
  backgroundColor: "var(--bg-surface)",
  border: "0.5px solid var(--border-default)",
  borderRadius: "12px",
  // boxShadow uses rgba inline — no shadow token exists (precedent:
  // AuthorityPopover, InfoPopup). Candidate for a future --shadow-* token.
  boxShadow: "0 4px 24px rgba(0,0,0,0.12)",
  zIndex: PANEL_Z_INDEX,
  overflow: "hidden",
};

const HEADER_STYLE: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  padding: "10px 14px",
  backgroundColor: "var(--bg-page)",
  borderBottom: "1px solid var(--border-default)",
  cursor: "grab",
  userSelect: "none",
  fontFamily: "var(--font-sans)",
  fontSize: "14px",
  fontWeight: 600,
  color: "var(--text-primary)",
};

const HEADER_LEFT_STYLE: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "8px",
  overflow: "hidden",
  textOverflow: "ellipsis",
  whiteSpace: "nowrap",
};

const HEADER_RIGHT_STYLE: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "10px",
};

const BADGE_STYLE: React.CSSProperties = {
  display: "inline-block",
  padding: "2px 10px",
  borderRadius: "12px",
  backgroundColor: "var(--accent-bg-soft)",
  color: "var(--accent-primary)",
  fontSize: "12px",
  fontWeight: 600,
};

const CLOSE_BTN_STYLE: React.CSSProperties = {
  background: "transparent",
  border: "none",
  color: "var(--text-secondary)",
  cursor: "pointer",
  fontSize: "16px",
  lineHeight: 1,
  padding: "2px 6px",
};

const PROOF_SECTION_STYLE: React.CSSProperties = {
  padding: "12px 16px",
  borderBottom: "1px solid var(--border-default)",
  flex: "0 0 auto",
};

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
  fontSize: "13px",
  fontWeight: 400,
  color: "var(--text-primary)",
  lineHeight: 1.45,
};

const NOTES_SECTION_STYLE: React.CSSProperties = {
  borderBottom: "1px solid var(--border-default)",
  flex: "0 0 auto",
};

const NOTES_HEADER_STYLE: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  padding: "10px 16px",
  cursor: "pointer",
  fontFamily: "var(--font-sans)",
  fontSize: "13px",
  fontWeight: 600,
  color: "var(--text-primary)",
  userSelect: "none",
};

const NOTES_TEXTAREA_STYLE: React.CSSProperties = {
  display: "block",
  width: "calc(100% - 32px)",
  margin: "0 16px 12px",
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
};

const ALLEGATIONS_HEADER_STYLE: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  padding: "10px 16px",
  fontFamily: "var(--font-sans)",
  fontSize: "12px",
  fontWeight: 600,
  color: "var(--text-secondary)",
  textTransform: "uppercase",
  letterSpacing: "0.05em",
  flex: "0 0 auto",
};

const ALLEGATIONS_SCROLL_STYLE: React.CSSProperties = {
  overflowY: "auto",
  flex: "1 1 auto",
  padding: "0 16px 16px",
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
  fontSize: "12px",
  fontWeight: 400,
  color: "var(--text-muted)",
  marginLeft: "8px",
};

const QUOTE_TEXT_STYLE_BASE: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "12px",
  fontWeight: 400,
  color: "var(--text-secondary)",
  marginTop: "6px",
  paddingLeft: "8px",
  lineHeight: 1.45,
};

const STATUS_INDICATOR_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "11px",
  fontWeight: 400,
  color: "var(--text-muted)",
};

const STATUS_ERROR_STYLE: React.CSSProperties = {
  ...STATUS_INDICATOR_STYLE,
  color: "var(--status-dropped-text)",
};

const BODY_MESSAGE_STYLE: React.CSSProperties = {
  padding: "32px 16px",
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

// Common: amber palette (--burden-warning-*). Dedicated: blue palette
// (--accent-*). Unknown: muted secondary.
const SECTION_COLOR_COMMON = "var(--burden-warning-text)";
const SECTION_BG_COMMON = "var(--burden-warning-bg)";
const SECTION_COLOR_DEDICATED = "var(--accent-primary)";
const SECTION_BG_DEDICATED = "var(--accent-bg-soft)";

// ─── Component ──────────────────────────────────────────────────────────────

/**
 * Floating, draggable Element detail panel.
 *
 * ## React Learning: useEffect with a cancel flag
 * Matches Home.tsx — every fetch effect carries a local `cancelled` flag,
 * checked before each setState, and the cleanup sets it true. If the user
 * clicks a different Element while the first fetch is still in flight, the
 * first effect's cleanup fires (cancelled = true), the new effect kicks off,
 * and the first response — should it arrive after — sets no state.
 *
 * ## React Learning: useRef for a mutable timer handle
 * `setTimeout` returns a handle we need on cleanup. Storing it in a ref
 * (not state) keeps changes to it from triggering re-renders.
 */
const ElementDetailPanel: React.FC<ElementDetailPanelProps> = ({
  caseSlug,
  elementId,
  elementName,
  allegationCount,
  onClose,
}) => {
  const [detail, setDetail] = useState<ElementDetailResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  // Bumping this state value triggers a re-fetch (retry button).
  const [reloadToken, setReloadToken] = useState(0);

  // Notes editor state. Kept separate from `detail.review_notes` because the
  // user can edit the textarea while a save is in flight; the source of
  // truth is whatever the user last typed, not the server's last echo.
  const [notesValue, setNotesValue] = useState<string>("");
  const [notesExpanded, setNotesExpanded] = useState(false);
  const [saveStatus, setSaveStatus] = useState<SaveStatus>({ kind: "idle" });

  // Debounce timer handle. Stored in a ref so re-renders don't churn it.
  const debounceRef = useRef<number | null>(null);
  // Tracks the last value we successfully sent to the backend, so a blur
  // after a successful debounce-save doesn't fire a redundant request.
  const lastSavedRef = useRef<string | null>(null);

  // ── Fetch effect ──────────────────────────────────────────────────────
  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);

    fetchElementDetail(caseSlug, elementId)
      .then((data) => {
        if (cancelled) return;
        setDetail(data);
        const initialNotes = data.review_notes ?? "";
        setNotesValue(initialNotes);
        lastSavedRef.current = data.review_notes; // preserves null distinct from ""
        setSaveStatus({ kind: "idle" });
        setLoading(false);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        const message =
          err instanceof Error
            ? err.message
            : "Failed to load Element detail.";
        setError(message);
        setLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [caseSlug, elementId, reloadToken]);

  // ── Notes save ─────────────────────────────────────────────────────────

  /**
   * Send the current notes value to the backend. A blank textarea
   * unconditionally normalises to `null` ("clear the column"); any non-empty
   * string is sent as-is. The wire still preserves the null-vs-string
   * distinction the backend honors — what we collapse here is "empty
   * string" → "clear", because a user who deletes all their notes means to
   * remove the row's contents, not to persist a literal "".
   *
   * Wrapped in useCallback so the debounce effect doesn't re-register on
   * every keystroke.
   */
  const persistNotes = useCallback(
    async (raw: string) => {
      // Skip if the value hasn't moved from what we last saved — avoids a
      // pointless PATCH when the user blurs without editing.
      const candidate: string | null = raw === "" ? null : raw;
      if (candidate === lastSavedRef.current) return;

      setSaveStatus({ kind: "saving" });
      try {
        await saveElementNotes(caseSlug, elementId, candidate);
        lastSavedRef.current = candidate;
        setSaveStatus({ kind: "saved" });
      } catch (err: unknown) {
        // Distinct observable: surface the message in the indicator. The
        // notes value stays in the textarea so the user can retry on blur.
        const message =
          err instanceof Error ? err.message : "Failed to save notes.";
        setSaveStatus({ kind: "error", message });
      }
    },
    [caseSlug, elementId],
  );

  // Debounced save: every keystroke (re)starts a NOTES_DEBOUNCE_MS timer.
  // If the user keeps typing, the previous timer is cleared. After the
  // silence threshold, persistNotes fires.
  useEffect(() => {
    // No debounce while loading or before any fetched payload exists.
    if (detail === null) return;

    // Clear any pending timer from the previous keystroke.
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

  // ── Allegation grouping for render ─────────────────────────────────────
  const sortedAllegations = detail
    ? sortAllegationsByParagraph(detail.allegations)
    : [];
  const commonAllegations = sortedAllegations.filter(
    (a) => a.source_section === "Common",
  );
  const dedicatedAllegations = sortedAllegations.filter(
    (a) => a.source_section === "Dedicated",
  );
  const unknownAllegations = sortedAllegations.filter(
    (a) => a.source_section !== "Common" && a.source_section !== "Dedicated",
  );

  // ── Status indicator text ──────────────────────────────────────────────
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

  // ── Header line ────────────────────────────────────────────────────────
  const headerLabel = detail
    ? formatElementHeader(
        detail.element_name,
        detail.count_number,
        detail.order_in_count,
      )
    : `Element — ${elementName}`;

  // Header badge: show fetched count when available; fall back to the
  // parent's hint while loading so the badge is never blank.
  const headerBadge = detail?.allegation_count ?? allegationCount;

  // ── Render ─────────────────────────────────────────────────────────────

  // The react-draggable `nodeRef` API removes a React 18 warning about
  // findDOMNode; we attach a ref to the panel div and pass it to Draggable.
  const dragRef = useRef<HTMLDivElement>(null);

  return (
    <Draggable handle={`.${HANDLE_CLASS}`} nodeRef={dragRef}>
      <div ref={dragRef} style={PANEL_STYLE} role="dialog" aria-label={headerLabel}>
        {/* ── Drag handle / header bar ───────────────────────────────── */}
        <div className={HANDLE_CLASS} style={HEADER_STYLE}>
          <div style={HEADER_LEFT_STYLE} title={headerLabel}>
            <span aria-hidden="true">⋮⋮</span>
            <span>{headerLabel}</span>
          </div>
          <div style={HEADER_RIGHT_STYLE}>
            <span style={BADGE_STYLE}>{headerBadge}</span>
            <button
              type="button"
              style={CLOSE_BTN_STYLE}
              onClick={onClose}
              aria-label="Close panel"
            >
              ×
            </button>
          </div>
        </div>

        {/* ── Body — loading / error / loaded ────────────────────────── */}
        {loading && (
          <div style={BODY_MESSAGE_STYLE}>Loading Element detail...</div>
        )}

        {!loading && error && (
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
        )}

        {!loading && !error && detail && (
          <>
            {/* Proof requirements (pinned, not scrollable) */}
            <div style={PROOF_SECTION_STYLE}>
              <div style={PROOF_LABEL_STYLE}>What plaintiff must prove</div>
              <div style={PROOF_TEXT_STYLE}>
                {detail.what_plaintiff_must_prove}
              </div>
            </div>

            {/* Review notes (collapsible) */}
            <div style={NOTES_SECTION_STYLE}>
              <div
                style={NOTES_HEADER_STYLE}
                onClick={() => setNotesExpanded((v) => !v)}
                role="button"
                aria-expanded={notesExpanded}
                aria-controls="element-detail-notes-textarea"
              >
                <span>
                  {notesExpanded ? "▾" : "▸"} Review notes
                </span>
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

            {/* Allegations list header */}
            <div style={ALLEGATIONS_HEADER_STYLE}>
              <span>
                {detail.allegation_count} allegations mapped (
                {detail.common_count} common · {detail.dedicated_count} dedicated)
              </span>
              <span>by ¶ number</span>
            </div>

            {/* Allegations list — scrollable */}
            <div style={ALLEGATIONS_SCROLL_STYLE}>
              {detail.allegation_count === 0 ? (
                <div style={BODY_MESSAGE_STYLE}>
                  No allegations mapped to this Element
                </div>
              ) : (
                <>
                  {commonAllegations.length > 0 && (
                    <AllegationSection
                      label="Common Allegations"
                      labelColor={SECTION_COLOR_COMMON}
                      labelBg={SECTION_BG_COMMON}
                      accentColor={SECTION_COLOR_COMMON}
                      allegations={commonAllegations}
                    />
                  )}
                  {dedicatedAllegations.length > 0 && (
                    <AllegationSection
                      label={
                        detail.count_number != null
                          ? `Count ${detail.count_number} Specific`
                          : "Count Specific"
                      }
                      labelColor={SECTION_COLOR_DEDICATED}
                      labelBg={SECTION_BG_DEDICATED}
                      accentColor={SECTION_COLOR_DEDICATED}
                      allegations={dedicatedAllegations}
                    />
                  )}
                  {unknownAllegations.length > 0 && (
                    <AllegationSection
                      label="Other"
                      labelColor="var(--text-secondary)"
                      labelBg="var(--bg-page)"
                      accentColor="var(--text-muted)"
                      allegations={unknownAllegations}
                    />
                  )}
                </>
              )}
            </div>
          </>
        )}
      </div>
    </Draggable>
  );
};

// ─── AllegationSection (internal) ───────────────────────────────────────────

/**
 * One labeled section (Common / Dedicated / Other) inside the scrollable
 * Allegations list. Pulled out as a sub-component so the three call sites in
 * the panel body stay readable.
 */
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

export default ElementDetailPanel;
