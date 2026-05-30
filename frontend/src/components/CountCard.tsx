// =============================================================================
// CountCard.tsx — one Cause-of-Action SUMMARY card for the Home page
// -----------------------------------------------------------------------------
// Home is a dashboard. Each Count renders as a single clickable summary surface
// matching the frozen PROD layout (v2.0.0-beta.1): a blue "COUNT {roman}"
// eyebrow, a bold count name, a serif plain-language description, and a slim
// muted metrics line. Cards sit in a 2-column grid (laid out by Home). The whole
// card navigates to the routed Count-detail page.
//
// Three deliberate differences from the PROD card: no burden strip, no
// "Supported" status chip, and our new metrics line. Data comes from
// GET /api/cases/:slug/causes-of-action (services/causesOfAction.ts).
// PRESENTATIONAL: one Count's data + its resolved description via props; the
// fetches (counts AND descriptions) live in Home.tsx. Colors are var(--token);
// the description uses the --font-serif token.
// =============================================================================

import React, { useState } from "react";
import { CountDetail, ElementDetail } from "../services/causesOfAction";

// ─── Pure helpers (exported for unit testing + reused by CountDetailPage) ─────
//
// NOTE: `formatElementNumber` and `sortElements` are imported by
// CountDetailPage.tsx (and locked by countCardHelpers.test.ts). They are kept
// exported here even though the Home summary card renders neither an Element
// table nor element ordinals — removing them would break the detail page.
// `toRomanNumeral` is still used below for the card eyebrow.

/**
 * Convert a positive integer to a Roman numeral (1 → "I", 4 → "IV", 9 → "IX").
 * Used for the "COUNT <roman>" eyebrow. Non-positive / non-integer input is
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
 * here (the Roman form is only for the eyebrow).
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

// ─── Styles (inline + tokens; no new color hex) ──────────────────────────────
//
// Card chrome (PROD layout): 1px resting border in --border-default; on hover
// the border becomes --accent-primary plus a soft shadow. The shadow is an
// inline rgba of the accent color at low alpha — the established panel/popover
// precedent for shadows (no shadow token exists) — not a new named color.

/**
 * "COUNT {roman}" eyebrow. Blue, bold, uppercase, tracked — the PROD treatment.
 * Sized in rem to match the frozen card (~11px) without inventing a px token.
 */
const EYEBROW_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "0.68rem",
  fontWeight: 700,
  color: "var(--accent-primary)",
  textTransform: "uppercase",
  letterSpacing: "0.05em",
  marginBottom: "0.2rem",
};

/** Count name — near-black, semibold, the card's primary line. */
const NAME_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "0.92rem",
  fontWeight: 600,
  color: "var(--text-primary)",
  lineHeight: 1.3,
  marginBottom: "0.3rem",
};

/**
 * Serif plain-language description — the editorial prose voice (PROD used
 * Georgia; we use the --font-serif token, which now covers this usage).
 */
const DESCRIPTION_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-serif)",
  fontSize: "0.8rem",
  color: "var(--text-secondary)",
  lineHeight: 1.45,
};

/** Slim, quiet metrics line below the description. */
const METRICS_STYLE: React.CSSProperties = {
  marginTop: "0.5rem",
  fontFamily: "var(--font-sans)",
  fontSize: "0.78rem",
  color: "var(--text-secondary)",
};

// ─── CountCardContent (internal) ─────────────────────────────────────────────

/**
 * The card's stacked content — eyebrow, name, serif description, metrics line.
 * Pulled out of `CountCard` so the clickable-container component stays under the
 * 50-line limit (Rule 18). It is hover-independent (only the container's border
 * and shadow react to hover), so it takes no hover prop.
 *
 * @param count the Count whose summary this renders
 * @param eyebrow the precomputed "COUNT {roman}" label (shared with aria-label)
 * @param description resolved plain-language sentence; omitted when absent/blank
 */
const CountCardContent: React.FC<{
  count: CountDetail;
  eyebrow: string;
  description?: string;
}> = ({ count, eyebrow, description }) => {
  // Graceful degradation: a missing OR blank description renders no line.
  const hasDescription = description != null && description.trim() !== "";

  return (
    <>
      {/* 1. Eyebrow */}
      <div style={EYEBROW_STYLE}>{eyebrow}</div>

      {/* 2. Count name (separate from the eyebrow — not recombined) */}
      {count.count_name && <div style={NAME_STYLE}>{count.count_name}</div>}

      {/* 3. Serif plain-language description (omitted entirely when absent) */}
      {hasDescription && <div style={DESCRIPTION_STYLE}>{description}</div>}

      {/* 4. Metrics line: "{N} Elements · {allegation slot}". The element count
          is real (count.elements.length). The allegation slot is a PENDING
          placeholder — we do NOT sum per-Element allegation_count, which would
          double-count allegations mapped to two Elements in the same Count.
          PROOF-MATRIX SWAP POINT: in Stage 2, replace the muted placeholder span
          below with the real deduped total — `{count.allegation_total} allegations`
          in --text-secondary — a value/color swap inside this same span, no
          layout change. */}
      <div style={METRICS_STYLE}>
        {count.elements.length} Elements ·{" "}
        <span style={{ color: "var(--text-muted)" }}>— allegations</span>
      </div>
    </>
  );
};

// ─── CountCard ───────────────────────────────────────────────────────────────

/**
 * CountCard — one Cause-of-Action summary card (PROD layout).
 *
 * @param count one Count's data from the causes-of-action endpoint
 * @param description resolved plain-language sentence for this Count (looked up
 *   in Home by `count_number`); when absent/blank no description line renders
 * @param onOpenCount fires when the card is activated (click or Enter/Space);
 *   Home navigates to the routed Count-detail page.
 */
const CountCard: React.FC<{
  count: CountDetail;
  description?: string;
  onOpenCount: () => void;
}> = ({ count, description, onOpenCount }) => {
  const [hovered, setHovered] = useState(false);
  const eyebrow = `COUNT ${toRomanNumeral(count.count_number)}`;

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
      aria-label={
        count.count_name ? `${eyebrow} — ${count.count_name}` : eyebrow
      }
      style={{
        border: `1px solid ${hovered ? "var(--accent-primary)" : "var(--border-default)"}`,
        backgroundColor: "var(--bg-surface)",
        borderRadius: "10px",
        padding: "1.15rem 1.25rem",
        cursor: "pointer",
        boxShadow: hovered ? "0 2px 8px rgba(21, 112, 239, 0.12)" : "none",
        transition: "border-color 0.15s ease, box-shadow 0.15s ease",
      }}
    >
      <CountCardContent
        count={count}
        eyebrow={eyebrow}
        description={description}
      />
    </div>
  );
};

export default CountCard;
