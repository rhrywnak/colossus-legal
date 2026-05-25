// =============================================================================
// BurdenBadge.tsx — the burden-of-proof pill in each CountCard header (§9)
// -----------------------------------------------------------------------------
// A small inline pill showing a Count's standard of proof. Two variants,
// keyed off the raw backend value (snake_case: "preponderance" /
// "clear_and_convincing"):
//   - preponderance       → neutral grey   (--burden-neutral-bg / -text)
//   - clear_and_convincing → amber/warning  (--burden-warning-bg / -text)
//   - anything else        → neutral (defensive default), text still shown
// Domain note: clear-and-convincing is the heightened civil burden, so it gets
// visual weight; preponderance ("more likely than not") is the neutral baseline.
// =============================================================================

import React from "react";

/** Which token set a burden value maps to. */
export type BurdenVariant = "neutral" | "warning";

/**
 * Select the pill variant for a burden value. Normalizes case and the
 * underscore/space difference so "clear_and_convincing", "clear and convincing",
 * and "Clear And Convincing" all resolve to the warning variant; everything
 * else (including "preponderance" and any unknown value) is neutral.
 */
export function burdenVariant(burden: string): BurdenVariant {
  const normalized = burden.trim().toLowerCase().replace(/\s+/g, "_");
  return normalized === "clear_and_convincing" ? "warning" : "neutral";
}

/**
 * Format a raw burden value for display: underscores → spaces, only the first
 * letter capitalized. "clear_and_convincing" → "Clear and convincing";
 * "preponderance" → "Preponderance". Already-formatted input is preserved.
 */
export function formatBurden(burden: string): string {
  const words = burden.trim().toLowerCase().replace(/_/g, " ");
  return words.charAt(0).toUpperCase() + words.slice(1);
}

/**
 * BurdenBadge — the styled burden pill.
 *
 * @param burden the raw `burden_of_proof` value from the causes-of-action DTO
 */
const BurdenBadge: React.FC<{ burden: string }> = ({ burden }) => {
  const warning = burdenVariant(burden) === "warning";
  return (
    <span
      style={{
        display: "inline-block",
        borderRadius: "9999px",
        padding: "2px 10px",
        fontSize: "13px",
        fontWeight: 500,
        backgroundColor: warning ? "var(--burden-warning-bg)" : "var(--burden-neutral-bg)",
        color: warning ? "var(--burden-warning-text)" : "var(--burden-neutral-text)",
      }}
    >
      {formatBurden(burden)}
    </span>
  );
};

export default BurdenBadge;
