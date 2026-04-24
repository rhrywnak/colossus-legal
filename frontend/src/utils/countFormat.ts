// Shared helpers for rendering legal count labels.
//
// Extracted from Home.tsx so the Allegations page subtitle and any
// future count-related UI can format the same way without duplicating
// the roman-numeral table.

const ROMAN_NUMERALS: Record<number, string> = {
  1: "I",
  2: "II",
  3: "III",
  4: "IV",
  5: "V",
  6: "VI",
  7: "VII",
  8: "VIII",
  9: "IX",
  10: "X",
};

// Render a count number as "COUNT I" / "COUNT II" / ...
// Falls back to the raw number if we don't have a numeral mapped.
export const toCountLabel = (countNumber: number): string => {
  const numeral = ROMAN_NUMERALS[countNumber] ?? String(countNumber);
  return `COUNT ${numeral}`;
};

// Render just the roman numeral (no "COUNT " prefix) for inline use
// like "Count I — Breach of Fiduciary Duty".
export const toCountNumeral = (countNumber: number): string =>
  ROMAN_NUMERALS[countNumber] ?? String(countNumber);
