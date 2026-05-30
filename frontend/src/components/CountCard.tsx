// =============================================================================
// CountCard.tsx — one Cause-of-Action SUMMARY card for the Home page
// -----------------------------------------------------------------------------
// Home is a dashboard: each Count renders as a single clickable summary surface
// (header + burden/authority strip + a one-line element/allegation summary) that
// navigates to the routed Count-detail page. The per-Element table that used to
// live here moved to that detail page (CountDetailPage); Home no longer shows
// Element rows.
//
// Data comes from GET /api/cases/:slug/causes-of-action (services/causesOfAction.ts).
// PRESENTATIONAL: one Count's data via props; the fetch lives in Home.tsx.
// Colors are var(--token); typography uses the tokens.css utility classes.
// =============================================================================

import React, { useState } from "react";
import { CountDetail, ElementDetail } from "../services/causesOfAction";
import BurdenBadge from "./BurdenBadge";
import AuthorityPopover from "./AuthorityPopover";

// ─── Pure helpers (exported for unit testing + reused by CountDetailPage) ─────
//
// NOTE: `formatElementNumber` and `sortElements` are imported by
// CountDetailPage.tsx (and locked by countCardHelpers.test.ts). They are kept
// exported here even though the Home summary card no longer renders an Element
// table — removing them would break the detail page. `toRomanNumeral` is still
// used below for the card header.

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
 * Build the Element ordinal "{countNumber}.{order}" (e.g. Count 2 / order 11 →
 * "2.11"). Used by CountDetailPage's Element list. The count number stays Arabic
 * here (the Roman form is only for the header).
 */
export function formatElementNumber(countNumber: number, order: number): string {
  return `${countNumber}.${order}`;
}

/**
 * Sort Elements for display: `order_in_count` ascending, null last, then
 * `element_name` alphabetically as the tie-breaker (§7 sort order). Used by
 * CountDetailPage.
 *
 * ## React/TS Learning: returns a NEW array
 * `Array.prototype.sort` mutates in place. Copying with `[...elements]` first
 * keeps this helper pure — same input, same output, no side effects — which
 * also makes it trivially unit-testable.
 */
export function sortElements(elements: ElementDetail[]): ElementDetail[] {
  return [...elements].sort((a, b) => {
    const ao = a.order_in_count ?? Number.MAX_SAFE_INTEGER;
    const bo = b.order_in_count ?? Number.MAX_SAFE_INTEGER;
    if (ao !== bo) return ao - bo;
    return a.element_name.localeCompare(b.element_name);
  });
}

// ─── Styles (inline + tokens; no new hex) ────────────────────────────────────
//
// Card chrome (decision A/B): a 1px resting border in --border-default; on hover
// the border becomes --accent-primary plus a soft shadow. The shadow is an
// inline rgba of the accent color at low alpha — the established panel/popover
// precedent for shadows (no shadow token exists) — not a new named color.

const SUMMARY_LINE_STYLE: React.CSSProperties = {
  marginTop: "12px",
  fontFamily: "var(--font-sans)",
  fontSize: "14px",
  color: "var(--text-secondary)",
};

// ─── CountCard ───────────────────────────────────────────────────────────────

/**
 * CountCard — one Cause-of-Action summary card.
 *
 * @param count one Count's data from the causes-of-action endpoint
 * @param onOpenCount fires when the card is activated (click or Enter/Space);
 *   Home navigates to the routed Count-detail page.
 */
const CountCard: React.FC<{
  count: CountDetail;
  onOpenCount: () => void;
}> = ({ count, onOpenCount }) => {
  const [hovered, setHovered] = useState(false);
  const roman = toRomanNumeral(count.count_number);
  const title = count.count_name ? `COUNT ${roman} — ${count.count_name}` : `COUNT ${roman}`;

  const burden = count.burden_of_proof?.trim() ? count.burden_of_proof : null;
  const primaryAuthority = count.controlling_authority_primary?.trim()
    ? count.controlling_authority_primary
    : null;

  return (
    // The whole card is the click target ("single clickable summary surface").
    // role="link" + tabIndex + Enter/Space keep it keyboard-accessible.
    <div
      role="link"
      tabIndex={0}
      onClick={onOpenCount}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onOpenCount();
        }
      }}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      title="Open Count detail"
      style={{
        border: `1px solid ${hovered ? "var(--accent-primary)" : "var(--border-default)"}`,
        backgroundColor: "var(--bg-surface)",
        borderRadius: "8px",
        padding: "24px",
        cursor: "pointer",
        boxShadow: hovered ? "0 2px 8px rgba(21, 112, 239, 0.12)" : "none",
        transition: "border-color 0.15s ease, box-shadow 0.15s ease",
      }}
    >
      {/* Header reads as the card title; it echoes the hover via color so the
          whole surface clearly signals it is clickable. */}
      <div
        className="count-header"
        style={{ color: hovered ? "var(--accent-primary)" : undefined }}
      >
        {title}
      </div>

      <div
        className="burden-strip"
        style={{ marginTop: "4px", display: "flex", alignItems: "center", gap: "6px", flexWrap: "wrap" }}
      >
        <span>Burden:</span>
        {burden ? <BurdenBadge burden={burden} /> : <span>—</span>}
        {primaryAuthority && (
          <>
            <span>· {primaryAuthority}</span>
            {/* The ⓘ popover is interactive; stop the click from bubbling up to
                the card's navigate handler so opening the popover stays put. */}
            <span
              onClick={(e) => e.stopPropagation()}
              onKeyDown={(e) => e.stopPropagation()}
            >
              <AuthorityPopover authorities={count.controlling_authorities} />
            </span>
          </>
        )}
      </div>

      {/* Summary line (§2.3): "{N} Elements · {allegation slot}". The element
          count is real (count.elements.length). The allegation slot is a PENDING
          placeholder — we do NOT sum per-Element allegation_count, which would
          double-count allegations mapped to two Elements in the same Count.
          PROOF-MATRIX SWAP POINT: in Stage 2, replace the muted placeholder span
          below with the real deduped total — `{count.allegation_total} allegations`
          in --text-secondary — a value/color swap inside this same span, no
          layout change. */}
      <div style={SUMMARY_LINE_STYLE}>
        {count.elements.length} Elements ·{" "}
        <span style={{ color: "var(--text-muted)" }}>— allegations</span>
      </div>
    </div>
  );
};

export default CountCard;
