// =============================================================================
// rehearsalStyles.ts — the signed mockup's visual language, in tokens
// =============================================================================
//
// REHEARSAL_PAGE_MOCKUP_v2_2026-08-06.html is the signed visual spec for this
// page. Its `<style>` block is reproduced here one rule at a time, with its ten
// colour variables mapped onto the tokens this app already has — because a hex
// literal in a component is the one thing that cannot follow a theme change.
//
//   mockup --accent       → --accent-primary
//   mockup --accent-soft  → --accent-bg-soft
//   mockup --hairline     → --border-default
//   mockup --hairline-dark→ --border-card
//   mockup --danger       → --v3-red-text
//   mockup --danger-soft  → --state-danger-bg-soft
//   mockup --answer-rule  → --v3-green-text
//   mockup --answer-soft  → --state-success-bg-soft
//   mockup --chip-bg      → --v3-chrome
//   mockup --text-*       → the same three names
//
// Every mapping had a token waiting for it; nothing here invents a colour.
//
// ## The type scale, and the doc comments this file contradicts
//
// Task 2.11 B2 argued in three places that this surface departs from §2c with a
// LARGER scale — "read aloud, from a distance, under stress". The signed mockup
// is §2c scale: 15px body, 17px accusation, 15px quotes. The mockup supersedes
// (ruling C6), and those three comments were corrected in the same commit rather
// than left arguing the opposite of what ships.
//
// ## Why the numbers live in two modules and not in eight components
//
// The rhythm IS the design — 18px between sections, 36px of indent under a row
// number, 62ch of measure on a quote. Eight components each holding their own
// numbers is eight places for the rhythm to drift by two pixels at a time.
//
// Two modules rather than one, and the seam is the page's own: this file is the
// FRAME — the page box, the breadcrumb, the header controls, the section cards,
// the Always strip, the editor box every block types into. `rehearsalRowStyles`
// is the CONTENT those cards hold: the accusation, the instance rows, the
// timeline, the two authored lists. Rule 17 forced the split; the seam was
// already there.

import type React from "react";

/** The mockup's `--hairline`: every internal rule on this page. */
export const HAIRLINE = "1px solid var(--border-default)";
/** The mockup's `--hairline-dark`: the page's own edges and control borders. */
export const HAIRLINE_STRONG = "1px solid var(--border-card)";

/** `.page` — 860px, and the bottom room a long scenario needs. */
export const pageStyle: React.CSSProperties = {
  maxWidth: "860px",
  margin: "0 auto",
  padding: "28px 32px 80px",
  fontSize: "15px",
  lineHeight: 1.55,
  color: "var(--text-primary)",
};

/** `.crumb` */
export const crumbStyle: React.CSSProperties = {
  fontSize: "12.5px",
  color: "var(--text-muted)",
  marginBottom: "18px",
};

/** `.pagehead` */
export const pageHeadStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "baseline",
  gap: "14px",
  borderBottom: HAIRLINE_STRONG,
  paddingBottom: "14px",
  flexWrap: "wrap",
};

/** `.pagehead h1` */
export const pageTitleStyle: React.CSSProperties = {
  fontSize: "21px",
  fontWeight: 600,
  letterSpacing: "-0.01em",
  margin: 0,
};

/** `.pagehead .code` — the scenario handle, in the accent. */
export const codeStyle: React.CSSProperties = {
  color: "var(--accent-primary)",
  fontWeight: 600,
};

/** `.navgroup` */
export const navGroupStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "6px",
  fontSize: "13px",
  color: "var(--text-muted)",
};

/** `.navbtn` */
export const navButtonStyle: React.CSSProperties = {
  border: HAIRLINE_STRONG,
  background: "var(--bg-surface)",
  borderRadius: "6px",
  padding: "4px 12px",
  fontSize: "13px",
  fontFamily: "inherit",
  color: "var(--text-secondary)",
  cursor: "pointer",
};

/** `.foldrow .navbtn` — the same control, one notch smaller. */
export const foldButtonStyle: React.CSSProperties = {
  ...navButtonStyle,
  fontSize: "12px",
  padding: "3px 10px",
};

/** `.editbtn` — every Edit / Save / Cancel / Add on the page. */
export const editButtonStyle: React.CSSProperties = {
  border: HAIRLINE_STRONG,
  background: "var(--bg-surface)",
  borderRadius: "6px",
  padding: "2px 10px",
  fontSize: "12px",
  fontFamily: "inherit",
  color: "var(--text-secondary)",
  cursor: "pointer",
};

/** `.purpose` */
export const purposeStyle: React.CSSProperties = {
  fontSize: "13px",
  color: "var(--text-muted)",
  margin: "10px 0 6px",
};

/** `.foldrow` */
export const foldRowStyle: React.CSSProperties = {
  display: "flex",
  gap: "8px",
  margin: "14px 0 6px",
};

/** `.section` — the bordered card each block sits in. */
export const sectionStyle: React.CSSProperties = {
  border: HAIRLINE,
  borderRadius: "10px",
  marginTop: "18px",
  overflow: "hidden",
};

/** `.sec-head` — a real button, so the fold works from the keyboard. */
export const sectionHeadStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "10px",
  padding: "12px 18px",
  width: "100%",
  border: "none",
  background: "transparent",
  fontFamily: "inherit",
  textAlign: "left",
  cursor: "pointer",
};

/** `.caret` */
export const caretStyle: React.CSSProperties = {
  fontSize: "11px",
  color: "var(--text-muted)",
  width: "12px",
  display: "inline-block",
};

/** `.sec-title` */
export const sectionTitleStyle: React.CSSProperties = {
  fontSize: "13px",
  fontWeight: 600,
  letterSpacing: "0.06em",
  textTransform: "uppercase",
  color: "var(--text-secondary)",
};

/** `.sec-count` */
export const sectionCountStyle: React.CSSProperties = {
  fontSize: "12.5px",
  color: "var(--text-muted)",
};

/** `.sec-body` */
export const sectionBodyStyle: React.CSSProperties = {
  padding: "4px 18px 20px",
};

/** `.whatline` */
export const whatLineStyle: React.CSSProperties = {
  fontSize: "15.5px",
  color: "var(--text-secondary)",
  margin: 0,
};

/** `.attribution` — who wrote the sentence above, and when. */
export const attributionStyle: React.CSSProperties = {
  fontSize: "12px",
  color: "var(--text-muted)",
  marginTop: "6px",
};

/** `.blockactions` */
export const blockActionsStyle: React.CSSProperties = {
  display: "flex",
  gap: "6px",
  marginTop: "8px",
};

/** `.always` — the one block that never folds. */
export const alwaysStyle: React.CSSProperties = {
  marginTop: "24px",
  border: "1px solid var(--accent-primary)",
  borderRadius: "8px",
  background: "var(--accent-bg-soft)",
  padding: "8px 16px",
  display: "flex",
  alignItems: "center",
  gap: "10px",
  flexWrap: "wrap",
};

/** `.always .lbl` */
export const alwaysLabelStyle: React.CSSProperties = {
  fontSize: "11px",
  fontWeight: 700,
  letterSpacing: "0.08em",
  textTransform: "uppercase",
  color: "var(--accent-primary)",
};

/** `.always .rules` */
export const alwaysRulesStyle: React.CSSProperties = {
  fontSize: "13.5px",
  fontWeight: 500,
  color: "var(--text-primary)",
};

/** `.always .rules span` — the separator between the standing lines. */
export const alwaysSeparatorStyle: React.CSSProperties = {
  color: "var(--text-muted)",
  padding: "0 6px",
};

/** Every link on this page: the accent, no underline. */
export const linkStyle: React.CSSProperties = {
  color: "var(--accent-primary)",
  textDecoration: "none",
  fontWeight: 500,
  whiteSpace: "nowrap",
};

/**
 * The box every editor on this page types into.
 *
 * `spellCheck` is set at each call site rather than here, because it is a DOM
 * attribute and not a style — and because it is a promise about behaviour worth
 * reading at the input it applies to: the browser advises while you type, and
 * what you save is stored verbatim.
 */
export const editorFieldStyle: React.CSSProperties = {
  width: "100%",
  fontFamily: "inherit",
  fontSize: "15px",
  lineHeight: 1.55,
  padding: "10px 12px",
  border: HAIRLINE_STRONG,
  borderRadius: "8px",
  color: "var(--text-primary)",
  background: "var(--bg-surface)",
};

/** The failure line every editor shows in place, without losing the draft. */
export const errorStyle: React.CSSProperties = {
  fontSize: "12.5px",
  color: "var(--v3-red-text)",
  marginTop: "6px",
};
