// =============================================================================
// CountCard.tsx — one Cause-of-Action card for the Home page (§7)
// -----------------------------------------------------------------------------
// A full-width card per legal Count: a header (COUNT <roman> — <name> + a
// burden/authority line) over a four-column table of canonical Elements.
// Data comes from GET /api/cases/:slug/causes-of-action (services/causesOfAction.ts).
//
// PRESENTATIONAL: receives one Count's data via props; the fetch lives in
// Home.tsx. All colors are var(--token); typography uses the tokens.css classes.
//
// Scope (Phase 2D): the burden and authority are PLAIN TEXT here — the burden
// pill (BurdenBadge) and the authority popover (AuthorityPopover) arrive in
// Phase 2E. The "ⓘ" is a static glyph for now, not an interactive trigger.
// =============================================================================

import React, { useState } from "react";
import { useNavigate } from "react-router-dom";
import { CountDetail, ElementDetail } from "../services/causesOfAction";
import BurdenBadge from "./BurdenBadge";
import AuthorityPopover from "./AuthorityPopover";

// ─── Pure helpers (exported for unit testing — no DOM, no React) ─────────────

/**
 * Convert a positive integer to a Roman numeral (1 → "I", 4 → "IV", 9 → "IX").
 * Used for the "COUNT <roman>" header. Non-positive / non-integer input is
 * returned as a plain string (defensive — a Count should always be ≥ 1).
 */
export function toRomanNumeral(n: number): string {
  if (!Number.isInteger(n) || n <= 0) return String(n);
  const table: [number, string][] = [
    [1000, "M"], [900, "CM"], [500, "D"], [400, "CD"],
    [100, "C"], [90, "XC"], [50, "L"], [40, "XL"],
    [10, "X"], [9, "IX"], [5, "V"], [4, "IV"], [1, "I"],
  ];
  let remaining = n;
  let out = "";
  for (const [value, sym] of table) {
    while (remaining >= value) {
      out += sym;
      remaining -= value;
    }
  }
  return out;
}

/**
 * Build the Element ordinal shown in the "#" column: "{countNumber}.{order}",
 * e.g. Count 1 / order 1 → "1.1", Count 2 / order 11 → "2.11". The count number
 * stays Arabic here (the Roman form is only for the header).
 */
export function formatElementNumber(countNumber: number, order: number): string {
  return `${countNumber}.${order}`;
}

/**
 * Sort Elements for display: `order_in_count` ascending, with null ordering last,
 * then `element_name` alphabetically as the tie-breaker (§7 sort order).
 *
 * ## React/TS Learning: returns a NEW array
 * `Array.prototype.sort` mutates in place. Sorting `props.elements` directly
 * would mutate the parent's data and can cause subtle render bugs. We copy with
 * `[...elements]` first so this helper is pure — same input, same output, no
 * side effects — which also makes it trivially unit-testable.
 */
export function sortElements(elements: ElementDetail[]): ElementDetail[] {
  return [...elements].sort((a, b) => {
    const ao = a.order_in_count ?? Number.MAX_SAFE_INTEGER;
    const bo = b.order_in_count ?? Number.MAX_SAFE_INTEGER;
    if (ao !== bo) return ao - bo;
    return a.element_name.localeCompare(b.element_name);
  });
}

/**
 * Build the click-through URL for an Element row: the Evidence tab, filtered to
 * this Element's allegations (§8). The id is URL-encoded defensively.
 *
 * NOTE (Phase 2D): `/evidence` redirects to `/explorer` preserving the query
 * (see App.tsx `RedirectPreservingQuery`), so `element_id` reaches the Evidence
 * Explorer. The Evidence page does not yet read it, so the actual allegation
 * filtering is separate, later work.
 */
export function buildEvidenceUrl(elementId: string): string {
  return `/evidence?element_id=${encodeURIComponent(elementId)}`;
}

// ─── Shared inline styles ────────────────────────────────────────────────────
// CONST: table cell spacing is the §9 spacing spec — 12px vertical / 16px
// horizontal — a fixed design-system layout value, not env-configurable. The
// column widths (60/280/120px) in the <colgroup> below are likewise the §7
// table-layout spec, and the pill/hover colors are var(--token) references.

const TH_STYLE: React.CSSProperties = { textAlign: "left", padding: "12px 16px" };
const TD_STYLE: React.CSSProperties = {
  padding: "12px 16px",
  borderTop: "1px solid var(--border-default)",
  verticalAlign: "top",
};

// ─── ElementRow (internal) ───────────────────────────────────────────────────

/**
 * One table row for a canonical Element. The entire row is clickable and
 * navigates to the Evidence tab for this Element (§8). Hover paints a subtle
 * page-colored highlight so the affordance is obvious.
 */
const ElementRow: React.FC<{ element: ElementDetail; elementNumber: string }> = ({
  element,
  elementNumber,
}) => {
  const navigate = useNavigate();
  const [hovered, setHovered] = useState(false);

  const proof = element.what_plaintiff_must_prove?.trim()
    ? element.what_plaintiff_must_prove
    : "—"; // null/blank → em dash, never an empty cell (Rule 1)

  return (
    <tr
      onClick={() => navigate(buildEvidenceUrl(element.element_id))}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      style={{
        cursor: "pointer",
        backgroundColor: hovered ? "var(--bg-page)" : "transparent",
      }}
    >
      <td className="element-number" style={TD_STYLE}>{elementNumber}</td>
      <td className="element-name" style={TD_STYLE}>{element.element_name}</td>
      <td className="proof-text" style={TD_STYLE}>{proof}</td>
      <td style={TD_STYLE}>
        {element.allegation_count > 0 ? (
          <span
            style={{
              display: "inline-block",
              padding: "2px 10px",
              borderRadius: "12px",
              backgroundColor: "var(--accent-bg-soft)",
              color: "var(--accent-primary)",
              fontSize: "13px",
              fontWeight: 600,
            }}
          >
            {element.allegation_count}
          </span>
        ) : (
          // Zero mapped allegations → muted "0" (§7 empty state per row).
          <span style={{ color: "var(--text-muted)" }}>0</span>
        )}
      </td>
    </tr>
  );
};

// ─── CountCard ───────────────────────────────────────────────────────────────

/**
 * CountCard — one Cause-of-Action card.
 *
 * @param count one Count's data from the causes-of-action endpoint
 */
const CountCard: React.FC<{ count: CountDetail }> = ({ count }) => {
  const roman = toRomanNumeral(count.count_number);
  const title = count.count_name ? `COUNT ${roman} — ${count.count_name}` : `COUNT ${roman}`;

  // Burden → styled pill (BurdenBadge); primary authority → text + the ⓘ
  // popover of all controlling authorities (AuthorityPopover). Phase 2E.
  const burden = count.burden_of_proof?.trim() ? count.burden_of_proof : null;
  const primaryAuthority = count.controlling_authority_primary?.trim()
    ? count.controlling_authority_primary
    : null;

  const elements = sortElements(count.elements);

  return (
    <div
      style={{
        border: "1px solid var(--border-default)",
        backgroundColor: "var(--bg-surface)",
        borderRadius: "8px",
        padding: "24px",
      }}
    >
      {/* Count header */}
      <div className="count-header">{title}</div>
      <div
        className="burden-strip"
        style={{ marginTop: "4px", display: "flex", alignItems: "center", gap: "6px", flexWrap: "wrap" }}
      >
        <span>Burden:</span>
        {burden ? <BurdenBadge burden={burden} /> : <span>—</span>}
        {primaryAuthority && (
          <>
            <span>· {primaryAuthority}</span>
            {/* Renders the ⓘ trigger only when there are authorities to show. */}
            <AuthorityPopover authorities={count.controlling_authorities} />
          </>
        )}
      </div>

      {/* Element table — or the empty-Count message (§7 empty state per Count) */}
      {elements.length === 0 ? (
        <div style={{ marginTop: "16px", color: "var(--text-muted)", fontSize: "14px" }}>
          No Elements loaded for this Count. Run the canonical Element loader.
        </div>
      ) : (
        <table style={{ width: "100%", borderCollapse: "collapse", marginTop: "16px" }}>
          <colgroup>
            <col style={{ width: "60px" }} />
            <col style={{ width: "280px" }} />
            <col />
            <col style={{ width: "120px" }} />
          </colgroup>
          <thead>
            <tr>
              <th className="table-col-header" style={TH_STYLE}>#</th>
              <th className="table-col-header" style={TH_STYLE}>Element</th>
              <th className="table-col-header" style={TH_STYLE}>What Plaintiff Must Prove</th>
              <th className="table-col-header" style={TH_STYLE}>Allegations</th>
            </tr>
          </thead>
          <tbody>
            {elements.map((element, i) => (
              <ElementRow
                key={element.element_id}
                element={element}
                // Use the canonical order_in_count; fall back to 1-based row
                // position when it's null so the ordinal is never "1.null".
                elementNumber={formatElementNumber(
                  count.count_number,
                  element.order_in_count ?? i + 1,
                )}
              />
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
};

export default CountCard;
