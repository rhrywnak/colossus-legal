// printStyles.ts — the sheets, on screen and on paper.
//
// Reproduces SCENARIO_PRINT_MOCKUP_v1_2026-08-22.html's three white sheets. The
// mockup's grey note boxes and its on-screen `.app` block are annotations for
// Roman and are not reproduced.
//
// ## Why this print CSS is SMALL, and the other one is not
//
// `practiceStyles.PRINT_CSS` hides a whole application — `body * { visibility:
// hidden }` — because it prints one block out of a page full of controls. This
// page IS the sheets, so the only things to hide are its own two buttons. That
// difference is the practical half of why the print view got its own address
// rather than a stylesheet on the practice page.

import type { CSSProperties } from "react";

// Every colour is a token from `styles/tokens.css`, scoped to `[data-print-desk]`
// — one named place a designer edits (Rule 2), and separate from the practice
// palette because these are a DOCUMENT's colours rather than a screen's. See that
// file's header for why the two must not be mapped onto one another.
const INK = "var(--print-ink)";
const DIM = "var(--print-dim)";
const FAINT = "var(--print-faint)";
const RULE = "var(--print-rule)";
const KEY = "var(--print-key)";
const SERIF = 'Georgia,"Times New Roman",serif';
const SANS = '-apple-system,BlinkMacSystemFont,"Segoe UI",Helvetica,Arial,sans-serif';

/** The grey desk the sheets sit on — screen only. */
export const page: CSSProperties = {
  background: "var(--print-desk)",
  minHeight: "100vh",
  padding: "24px 18px 70px",
  color: INK,
  font: `15px/1.5 ${SANS}`,
};

/** The two controls above the sheets. Hidden in print by `PRINT_CSS`. */
export const bar: CSSProperties = {
  maxWidth: 816,
  margin: "0 auto 18px",
  display: "flex",
  alignItems: "center",
  gap: 14,
};

export const printButton: CSSProperties = {
  background: "var(--print-key)",
  border: "1px solid var(--print-key)",
  color: "var(--print-paper)",
  borderRadius: 6,
  padding: "9px 18px",
  fontSize: 14,
  fontWeight: 600,
  cursor: "pointer",
};

export const backLink: CSSProperties = {
  color: "var(--print-key)",
  fontSize: 13.5,
  textDecoration: "none",
};

/** One sheet — 816px is US Letter at 96dpi, as the mockup has it. */
export const sheet: CSSProperties = {
  background: "var(--print-paper)",
  width: "100%",
  maxWidth: 816,
  margin: "0 auto 26px",
  padding: "52px 60px 46px",
  boxShadow: "0 2px 10px rgba(0,0,0,.16)",
  fontFamily: SERIF,
  color: INK,
};

export const hdr: CSSProperties = {
  borderBottom: `2px solid ${INK}`,
  paddingBottom: 10,
  marginBottom: 6,
  display: "flex",
  justifyContent: "space-between",
  alignItems: "flex-end",
  gap: 20,
};

export const hdrTitle: CSSProperties = { fontSize: 20, fontWeight: 700, lineHeight: 1.25 };

export const hdrSub: CSSProperties = {
  display: "block",
  fontWeight: 400,
  fontSize: 13,
  color: DIM,
  marginTop: 3,
};

export const hdrMeta: CSSProperties = {
  fontFamily: SANS,
  fontSize: 11,
  color: DIM,
  textAlign: "right",
  lineHeight: 1.5,
  whiteSpace: "nowrap",
};

export const howto: CSSProperties = {
  fontFamily: SANS,
  fontSize: 11.5,
  color: DIM,
  borderBottom: `1px solid ${RULE}`,
  padding: "9px 0 11px",
  margin: "0 0 6px",
};

/** One question block. `breakInside: avoid` is the rule that keeps it whole. */
export const qb: CSSProperties = {
  padding: "16px 0 4px",
  borderBottom: `1px solid ${RULE}`,
  breakInside: "avoid",
};

export const qtop: CSSProperties = {
  display: "flex",
  alignItems: "baseline",
  gap: 11,
  marginBottom: 7,
};

/** The blue box. The whole point of the paper: it never moves. */
export const key: CSSProperties = {
  fontFamily: "ui-monospace,SFMono-Regular,Menlo,monospace",
  fontSize: 12,
  fontWeight: 700,
  color: KEY,
  border: "1px solid var(--print-key-border)",
  borderRadius: 3,
  padding: "1px 7px",
  flex: "none",
};

export const tac: CSSProperties = {
  fontFamily: SANS,
  fontSize: 10.5,
  letterSpacing: ".06em",
  textTransform: "uppercase",
  color: DIM,
  border: `1px solid ${RULE}`,
  borderRadius: 3,
  padding: "1px 7px",
  flex: "none",
};

export const draft: CSSProperties = {
  fontFamily: SANS,
  fontSize: 10.5,
  letterSpacing: ".06em",
  textTransform: "uppercase",
  color: "var(--print-draft-text)",
  background: "var(--print-draft-bg)",
  border: "1px solid var(--print-draft-border)",
  borderRadius: 3,
  padding: "1px 7px",
  flex: "none",
};

export const qtx: CSSProperties = { fontSize: 17, lineHeight: 1.42, margin: "0 0 7px" };

export const src: CSSProperties = {
  fontFamily: SANS,
  fontSize: 11.5,
  color: DIM,
  lineHeight: 1.5,
  margin: "0 0 12px",
  paddingLeft: 11,
  borderLeft: "2px solid var(--print-source-bar)",
};

/** The quoted antecedent above a redirect. */
export const after: CSSProperties = {
  fontFamily: SANS,
  fontSize: 11.5,
  color: DIM,
  background: "var(--print-after-bg)",
  borderLeft: "3px solid var(--print-after-bar)",
  padding: "7px 11px",
  margin: "0 0 9px",
};

export const afterQuote: CSSProperties = {
  color: "var(--print-quote)",
  fontFamily: SERIF,
  fontSize: 13.5,
  fontStyle: "normal",
};

/** The ruled space. No label above it — Roman removed those deliberately. */
export const lines: CSSProperties = { margin: "4px 0 12px" };
export const line: CSSProperties = { borderBottom: "1px solid var(--print-hairline)", height: 22 };

export const ftr: CSSProperties = {
  marginTop: 26,
  paddingTop: 9,
  borderTop: `1px solid ${RULE}`,
  fontFamily: SANS,
  fontSize: 10.5,
  color: FAINT,
  display: "flex",
  justifyContent: "space-between",
};

/** The line naming what the deck does not contain, and what is hidden. */
export const absent: CSSProperties = {
  fontFamily: SANS,
  fontSize: 11.5,
  color: DIM,
  marginTop: 18,
  paddingTop: 10,
  borderTop: `1px solid ${RULE}`,
};

/**
 * The print rules a style object cannot express.
 *
 * ## The two that matter
 *
 * `page-break-after: always` on each sheet, so the three sheets are three
 * documents rather than one long scroll — and `break-inside: avoid` on every
 * question block, so a question and its ruled space are never split across a
 * page boundary. A half question with the lines on the next sheet is worse than
 * useless: Chuck writes his note under the wrong one.
 *
 * ## The third: the application itself
 *
 * MEASURED on .405, on paper, by Roman — the product's navigation bar printed
 * across the top of page 1. This view is a ROUTE INSIDE THE APP SHELL, so the
 * shell renders around it, and hiding this page's own two buttons said nothing
 * about the shell's header. `!important` is required and not decoration: the
 * header sets its `display` from an inline style object, which outranks a
 * stylesheet rule of any specificity. `printChrome.test.ts` pins this rule to the
 * `data-app-chrome` mark in `Header.tsx` so a rename in either file is caught —
 * the instruction was explicit that this must not be a `display: none` a later
 * change can quietly undo.
 *
 * ## Domain note: this does NOT number physical pages
 *
 * The sheet number in the footer counts SHEETS. A sheet with enough questions
 * runs onto a second and third piece of paper — S-7 has eight directs — and the
 * browser paginates that itself. Trying to number physical pages from here would
 * put "page 2 of 3" on both halves of one sheet.
 */
export const PRINT_CSS = `
@media print {
  [data-app-chrome] { display: none !important; }
  [data-print-chrome] { display: none !important; }
  [data-print-desk] { background: var(--print-paper) !important; padding: 0 !important; }
  [data-print-sheet] {
    box-shadow: none !important;
    margin: 0 !important;
    padding: 0 !important;
    max-width: none !important;
    page-break-after: always;
  }
  [data-print-sheet]:last-child { page-break-after: auto; }
  [data-print-row] { break-inside: avoid; page-break-inside: avoid; }
  @page { margin: 14mm; }
}
`;
