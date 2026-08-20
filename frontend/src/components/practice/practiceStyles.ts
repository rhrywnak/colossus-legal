// =============================================================================
// practiceStyles.ts — the mockup's stylesheet, as typed style objects
// =============================================================================
//
// PRACTICE_MOCKUP_v2_2026-08-17.html's `<style>` block, transcribed. The values
// are the mockup's own — 26px serif questions, the gold watch box, the red/green
// feedback rails, the 860px column — because R4 says CC reproduces the mockup
// exactly and a "close enough" pixel is a deviation nobody wrote down.
//
// ## Why this file exists rather than a CSS module
//
// Every other page in this product styles inline from a `*Styles.ts` sibling
// (`rehearsalStyles.ts`, `trialPrepCardStyles.ts`, `caseHealthStyles.ts`). A CSS
// module here would be the only one, and the visual-language test that reads
// styles off disk would have nothing to read.
//
// ## Where the colours live, and why not the product's tokens
//
// The product's token palette (`--v3-*`) is the WORKING surfaces' palette. This
// screen is Marie's, it was designed on its own, and Roman approved these exact
// values in the mockup. Mapping them onto `--v3-*` would be a redesign performed
// silently during a transcription — the one thing R4 forbids.
//
// So the mockup's palette is declared in `styles/tokens.css` under
// `[data-surface="practice"]`, VERBATIM and by the mockup's own names, and every
// value below reads it through `var(--practice-…)`. One named place a designer
// edits, no hex literal in a component (Rule 2), and the scoping is the same
// move ruling R1 made for the v3 palette. The page root carries the attribute;
// without it these variables resolve to nothing, which is why
// `PracticePage` sets it and `practiceStyles.test.ts` pins that it does.

import type { CSSProperties } from "react";

/**
 * The mockup's `:root` custom properties, referenced by the mockup's own names.
 *
 * These are `var()` REFERENCES, not values — the values live in `tokens.css`.
 * Named here so a style object reads like the mockup's CSS did, and so a
 * component that needs one inline (the neutral read rail) does not have to
 * spell the variable out.
 */
export const INK = "var(--practice-ink)";
export const MUTED = "var(--practice-muted)";
export const LINE = "var(--practice-line)";
export const BLUE = "var(--practice-blue)";
export const PALE = "var(--practice-pale)";
export const GOLD = "var(--practice-gold)";
export const RED = "var(--practice-red)";
export const GREEN = "var(--practice-green)";
export const PAPER = "var(--practice-paper)";
/** The soft wash behind a read that never happened — neither green nor red. */
export const QUIET_BG = "var(--practice-quiet-bg)";

/** `main` — the 860px column the whole drill lives in. */
export const page: CSSProperties = {
  maxWidth: 860,
  margin: "28px auto",
  padding: "0 20px",
  color: INK,
  fontSize: 18,
  lineHeight: 1.45,
  fontFamily: '-apple-system, "Segoe UI", Helvetica, Arial, sans-serif',
};

/** `.card` */
export const card: CSSProperties = {
  background: PAPER,
  border: `1px solid ${LINE}`,
  borderRadius: 12,
  padding: "28px 32px",
  marginBottom: 18,
};

/** `.kicker` */
export const kicker: CSSProperties = {
  fontSize: 13,
  letterSpacing: ".06em",
  textTransform: "uppercase",
  color: MUTED,
  fontWeight: 600,
};

/** `h1` */
export const h1: CSSProperties = { fontSize: 28, margin: "6px 0 4px" };

/** `h2` */
export const h2: CSSProperties = { fontSize: 22, margin: "0 0 10px" };

/** `.sub` */
export const sub: CSSProperties = { color: MUTED, fontSize: 16 };

/** `.q` — the question, in Georgia, at 26px. */
export const question: CSSProperties = {
  fontSize: 26,
  lineHeight: 1.35,
  margin: "14px 0 6px",
  fontFamily: 'Georgia, "Times New Roman", serif',
};

/** `.q` on the reveal, where it shrinks and greys. */
export const questionEcho: CSSProperties = {
  ...question,
  fontSize: 20,
  color: MUTED,
};

/** `.from` */
export const from: CSSProperties = { fontSize: 15, color: MUTED };

/** `textarea` */
export const textarea: CSSProperties = {
  width: "100%",
  minHeight: 110,
  font: "inherit",
  fontSize: 18,
  padding: "12px 14px",
  border: "1px solid var(--practice-control-border)",
  borderRadius: 8,
  resize: "vertical",
};

/** `.row` */
export const row: CSSProperties = {
  display: "flex",
  gap: 12,
  flexWrap: "wrap",
  alignItems: "center",
  marginTop: 14,
};

/** `button` */
export const button: CSSProperties = {
  font: "inherit",
  fontSize: 17,
  padding: "11px 18px",
  borderRadius: 8,
  border: "1px solid var(--practice-control-border)",
  background: "var(--practice-paper)",
  color: INK,
  cursor: "pointer",
};

/** `button.primary` */
export const buttonPrimary: CSSProperties = {
  ...button,
  background: BLUE,
  borderColor: BLUE,
  color: "var(--practice-paper)",
  fontWeight: 600,
};

/** `button.quiet` */
export const buttonQuiet: CSSProperties = {
  ...button,
  background: "transparent",
  borderColor: "transparent",
  color: BLUE,
};

/** `button.big` */
export const buttonBig: CSSProperties = { fontSize: 19, padding: "14px 22px" };

/** `.pill` */
export const pill: CSSProperties = {
  display: "inline-block",
  fontSize: 13,
  padding: "3px 9px",
  borderRadius: 999,
  background: PALE,
  color: "var(--practice-navy)",
  border: "1px solid var(--practice-pill-border)",
};

/** `.pill.george` */
export const pillGeorge: CSSProperties = {
  ...pill,
  background: "var(--practice-george-bg)",
  color: "var(--practice-george-text)",
  borderColor: "var(--practice-george-border)",
};

/** `.pill.chuck` */
export const pillChuck: CSSProperties = {
  ...pill,
  background: "var(--practice-chuck-bg)",
  color: "var(--practice-chuck-text)",
  borderColor: "var(--practice-chuck-border)",
};

/** `.pill.braid` */
export const pillBraid: CSSProperties = {
  ...pill,
  background: "var(--practice-braid-bg)",
  color: "var(--practice-braid-text)",
  borderColor: "var(--practice-braid-border)",
};

/** `.tactic` — the grey tag beside the pill. */
export const tacticTag: CSSProperties = {
  display: "inline-block",
  fontSize: 12,
  padding: "2px 8px",
  borderRadius: 4,
  background: "var(--practice-tactic-bg)",
  color: "var(--practice-tactic-text)",
  border: "1px solid var(--practice-tactic-border)",
  marginLeft: 8,
  verticalAlign: "middle",
};

/** `.receipt` */
export const receipt: CSSProperties = { fontSize: 14, color: MUTED };

/** `.pair` */
export const pair: CSSProperties = {
  display: "grid",
  gridTemplateColumns: "1fr 1fr",
  gap: 14,
  marginTop: 8,
};

/** `.pair > div` */
export const pairCell: CSSProperties = {
  background: PALE,
  borderRadius: 8,
  padding: "12px 14px",
  fontSize: 16,
};

/** `.pair .lab` */
export const pairLabel: CSSProperties = {
  fontSize: 12,
  textTransform: "uppercase",
  letterSpacing: ".06em",
  color: MUTED,
  fontWeight: 700,
  marginBottom: 4,
};

/** `.watch` */
export const watch: CSSProperties = {
  background: GOLD,
  border: "1px solid var(--practice-gold-border)",
  borderRadius: 8,
  padding: "12px 14px",
  marginTop: 12,
};

/** `.yours` */
export const yours: CSSProperties = {
  background: "var(--practice-quiet-bg)",
  borderLeft: "4px solid var(--practice-control-border)",
  padding: "10px 14px",
  margin: "8px 0 14px",
  fontStyle: "italic",
};

/** `.fb` — the read, red rail by default. */
export const feedback: CSSProperties = {
  borderLeft: `4px solid ${RED}`,
  background: "var(--practice-read-bad-bg)",
  padding: "12px 14px",
  borderRadius: "0 8px 8px 0",
  marginTop: 14,
  fontSize: 18,
};

/** `.fb.ok` — green rail. */
export const feedbackOk: CSSProperties = {
  ...feedback,
  borderLeftColor: GREEN,
  background: "var(--practice-read-ok-bg)",
};

/** `.fb small` */
export const feedbackNote: CSSProperties = {
  display: "block",
  color: MUTED,
  fontSize: 13,
  marginTop: 6,
};

/** `.tag` — the little grey "system read" chip. */
export const tag: CSSProperties = {
  fontSize: 12,
  color: "var(--practice-paper)",
  background: "var(--practice-tag-bg)",
  padding: "2px 7px",
  borderRadius: 4,
  verticalAlign: "middle",
};

/** `.checks label` */
export const checkLabel: CSSProperties = {
  display: "block",
  margin: "6px 0",
  fontSize: 17,
};

/** `.checks input` */
export const checkBox: CSSProperties = { transform: "scale(1.25)", marginRight: 10 };

/** `.progress` */
export const progress: CSSProperties = { fontSize: 14, color: MUTED };

/** `.always` */
export const always: CSSProperties = {
  fontSize: 15,
  color: MUTED,
  borderTop: `1px dashed ${LINE}`,
  marginTop: 18,
  paddingTop: 10,
};

/** `details.stronger` */
export const stronger: CSSProperties = {
  marginTop: 18,
  border: `1px solid ${LINE}`,
  borderRadius: 8,
  padding: "10px 14px",
  background: "var(--practice-drawer-bg)",
};

/** `details.stronger summary` */
export const strongerSummary: CSSProperties = {
  cursor: "pointer",
  color: BLUE,
  fontWeight: 600,
  fontSize: 17,
};

/** `details.stronger .ex` */
export const strongerExample: CSSProperties = {
  fontFamily: "Georgia, serif",
  fontSize: 19,
  margin: "10px 0 6px",
};

/** `details.stronger .lean` */
export const strongerLean: CSSProperties = { fontSize: 14, color: MUTED };

/** `details.stronger .noscript` */
export const strongerNote: CSSProperties = {
  fontSize: 13,
  color: MUTED,
  borderTop: `1px dashed ${LINE}`,
  marginTop: 8,
  paddingTop: 6,
};

/** `.choice` */
export const choice: CSSProperties = { display: "flex", gap: 12, flexWrap: "wrap" };

/** `.choice button` */
export const choiceButton: CSSProperties = {
  ...button,
  flex: "1 1 220px",
  textAlign: "left",
  padding: 16,
};

/** `.choice button.sel` */
export const choiceButtonSelected: CSSProperties = {
  ...choiceButton,
  borderColor: BLUE,
  background: "var(--practice-choice-selected-bg)",
};

/** `.choice .t` */
export const choiceTitle: CSSProperties = { fontWeight: 600, display: "block" };

/** `.choice .d` */
export const choiceDetail: CSSProperties = { fontSize: 14, color: MUTED };

/**
 * The lawyers' word under a side card's title — `cross`, `direct`.
 *
 * Smaller and lighter than the detail line beneath it, deliberately: the reading
 * order is TITLE (what Marie needs), then term (what Chuck needs), then the
 * description. Making the term the same weight as the description would give a
 * one-word technical label the same emphasis as the sentence explaining the card.
 */
export const choiceTerm: CSSProperties = {
  display: "block",
  fontSize: 12,
  color: "var(--practice-separator)",
  letterSpacing: ".04em",
  marginTop: 1,
  marginBottom: 3,
};

/** `table` */
export const table: CSSProperties = {
  width: "100%",
  borderCollapse: "collapse",
  fontSize: 15,
};

/** `th, td` */
export const cell: CSSProperties = {
  textAlign: "left",
  padding: "8px 8px",
  borderBottom: `1px solid ${LINE}`,
  verticalAlign: "top",
};

/** `th` */
export const headerCell: CSSProperties = {
  ...cell,
  fontSize: 12,
  textTransform: "uppercase",
  letterSpacing: ".05em",
  color: MUTED,
};

/** `.stumble` */
export const markRepeat: CSSProperties = { color: RED, fontWeight: 600 };

/** `.fine` */
export const markFine: CSSProperties = { color: GREEN, fontWeight: 600 };

/** `.points li` */
export const pointItem: CSSProperties = { margin: "8px 0" };

/**
 * The one thing the mockup could not carry: `@media print`.
 *
 * Injected as a real stylesheet because inline styles cannot express a media
 * query. It hides everything but the sheet, so "Print Chuck's sheet" renders one
 * page of table rather than the app's chrome around it. Scoped by data attribute
 * so it can only ever affect this page.
 */
export const PRINT_CSS = `
@media print {
  body * { visibility: hidden; }
  [data-practice-print] , [data-practice-print] * { visibility: visible; }
  [data-practice-print] {
    position: absolute; left: 0; top: 0; width: 100%;
    padding: 0; margin: 0; border: none; box-shadow: none;
  }
  [data-practice-no-print] { display: none !important; }
  @page { margin: 14mm; }
}
`;

/** `.skipmark` — the third mark on Chuck's sheet (mockup v3). */
export const markSkipped: CSSProperties = { color: MUTED, fontWeight: 600 };
