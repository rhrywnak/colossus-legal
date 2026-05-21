import React from "react";
import { Link } from "react-router-dom";
import { ElementInfo, LegalCountInfo } from "../services/caseSummary";
import { stripCountPrefix, toCountLabel } from "../utils/countFormat";
import InfoPopup from "./InfoPopup";

// ─── Constants ───────────────────────────────────────────────────────────────

// Placeholder shown in the controlling-authority popover when the Element's
// `controlling_authority` property is absent or empty. The popover icon is
// ALWAYS present — empty authority shows this text so the operator can
// distinguish "missing data" (icon present, placeholder text) from
// "intentionally no icon" (we never render that state).
const AUTHORITY_PLACEHOLDER =
  "Authority pending review of canonical Element library.";

// ─── Helpers ─────────────────────────────────────────────────────────────────

/**
 * Resolve the user-facing display name for an Element row.
 *
 * Prefers the human-readable `title` (e.g. "Breach of Duty") and falls back
 * to the snake_case `element_name` (e.g. "breach_of_duty"), and finally to
 * a positional fallback (`Element 1`). Exported so the unit-style tests in
 * the service test pattern can lock this behavior — never let a card row
 * render visually blank because all three fields were empty.
 */
export const resolveElementDisplayName = (
  element: ElementInfo,
  positionalIndex: number,
): string => {
  if (element.title.trim().length > 0) return element.title;
  if (element.element_name.trim().length > 0) return element.element_name;
  return `Element ${positionalIndex + 1}`;
};

/**
 * Resolve the popover body for an Element's controlling authority.
 *
 * `controlling_authority` is `undefined` until the canonical Element
 * library is approved and extraction templates are updated. An empty
 * string from the graph is treated identically to undefined — both show
 * the pending-review placeholder.
 */
export const resolveAuthorityText = (
  controllingAuthority: string | undefined,
): string => {
  const trimmed = (controllingAuthority ?? "").trim();
  return trimmed.length > 0 ? trimmed : AUTHORITY_PLACEHOLDER;
};

// ─── Component ───────────────────────────────────────────────────────────────

type CountCardProps = {
  count: LegalCountInfo;
};

/**
 * A single Count card on the Home page.
 *
 * Layout:
 *   ┌─────────────────────────────────────────────┐
 *   │ COUNT I                                     │  ← header sub-Link
 *   │ Breach of Fiduciary Duty                    │
 *   │                                             │
 *   │ • Breach of Duty           12 allegations ⓘ │  ← Element rows
 *   │ • Causation                 4 allegations ⓘ │
 *   │ • Damages                   8 allegations ⓘ │
 *   └─────────────────────────────────────────────┘
 *
 * The outer wrapper is a `<div>` rather than `<Link>` because the card
 * contains interactive children (ⓘ buttons) — nesting `<button>` inside
 * `<a>` is invalid HTML and React warns about it. Only the header text
 * region is wrapped in a `<Link>` to the allegations page.
 */
// ─── Subcomponents ───────────────────────────────────────────────────────────

/**
 * One Element row inside a Count card.
 *
 * Extracted so `CountCard` itself stays under the 50-line function cap
 * (CLAUDE.md §4-18). The popover is the existing `InfoPopup`; the row is
 * non-clickable on purpose — the only interactive child is the ⓘ button.
 */
const ElementRow: React.FC<{ element: ElementInfo; index: number }> = ({
  element,
  index,
}) => (
  <li
    style={{
      display: "flex",
      alignItems: "center",
      justifyContent: "space-between",
      gap: "0.5rem",
      fontSize: "0.82rem",
      color: "#334155",
      lineHeight: 1.4,
    }}
  >
    <span style={{ flex: 1, minWidth: 0 }}>
      {resolveElementDisplayName(element, index)}
    </span>
    <span
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: "0.25rem",
        flexShrink: 0,
      }}
    >
      <span
        style={{
          padding: "0.15rem 0.45rem",
          borderRadius: "5px",
          fontSize: "0.7rem",
          fontWeight: 600,
          letterSpacing: "0.02em",
          backgroundColor: "#eff6ff",
          color: "#1d4ed8",
          whiteSpace: "nowrap",
        }}
      >
        {element.allegation_count}{" "}
        {element.allegation_count === 1 ? "allegation" : "allegations"}
      </span>
      <InfoPopup>
        <div style={{ fontWeight: 600, marginBottom: "0.35rem" }}>
          Controlling authority
        </div>
        <div>{resolveAuthorityText(element.controlling_authority)}</div>
      </InfoPopup>
    </span>
  </li>
);

/**
 * Header strip of a Count card — clickable, links to the allegations page.
 *
 * Extracted from `CountCard` so the parent component stays under §4-18.
 * The outer `CountCard` wrapper is a `<div>` (not `<Link>`) because the
 * Element rows contain interactive children (ⓘ buttons), and nesting
 * `<button>` inside `<a>` is invalid HTML.
 */
const CountCardHeader: React.FC<{ count: LegalCountInfo }> = ({ count }) => (
  <Link
    to={`/allegations?count=${encodeURIComponent(count.id)}`}
    style={{ textDecoration: "none", color: "inherit", display: "block" }}
  >
    <div
      style={{
        fontSize: "0.68rem",
        fontWeight: 700,
        color: "#2563eb",
        textTransform: "uppercase",
        letterSpacing: "0.05em",
        marginBottom: "0.2rem",
      }}
    >
      {toCountLabel(count.count_number)}
    </div>
    <div
      style={{
        fontSize: "0.92rem",
        fontWeight: 600,
        color: "#0f172a",
        lineHeight: 1.3,
      }}
    >
      {stripCountPrefix(count.name)}
    </div>
  </Link>
);

const CountCard: React.FC<CountCardProps> = ({ count }) => (
  <div
    style={{
      backgroundColor: "#ffffff",
      border: "1px solid #e2e8f0",
      borderRadius: "10px",
      padding: "1.15rem 1.25rem",
      display: "flex",
      flexDirection: "column",
      gap: "0.75rem",
    }}
  >
    <CountCardHeader count={count} />
    {count.elements.length > 0 ? (
      <ul
        style={{
          listStyle: "none",
          margin: 0,
          padding: 0,
          display: "flex",
          flexDirection: "column",
          gap: "0.35rem",
        }}
      >
        {count.elements.map((element, index) => (
          <ElementRow key={element.id} element={element} index={index} />
        ))}
      </ul>
    ) : (
      // Standing Rule 1: surface the "no elements extracted" state in the
      // UI rather than letting the card silently collapse to header-only.
      <div
        style={{
          fontSize: "0.78rem",
          color: "#94a3b8",
          fontStyle: "italic",
        }}
      >
        No elements extracted yet.
      </div>
    )}
  </div>
);

export default CountCard;
