// =============================================================================
// factCardStyles — every inline style the witness card draws with
// =============================================================================
//
// Split from `FactCardBody` when FACT_CARD_v2 pushed it past the 300-line limit
// (Rule 17), following `factRowStyles` and `rehearsalStyles`. Keeping the card's
// styles in ONE module also keeps the two text sizes checkable by reading a file
// rather than auditing every span in the component.
//
// Values are TOKENS (`var(--…)`), never literal colours — the card must follow
// the theme like everything else on the page.

import React from "react";

export const CARD_STYLE: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: "8px",
  minWidth: 0,
};

export const TITLE_BAR_STYLE: React.CSSProperties = {
  display: "flex",
  alignItems: "flex-start",
  gap: "12px",
  cursor: "pointer",
};

// Tabular numerals so a column of positions lines up down the deck.
// RETIRED IN v2.1 (ruling R37): `POSITION_STYLE`. It dressed the raw
// `display_ordinal` in the title bar — a sparse sort key running to five digits
// on live data — which §2 put there for a witness working a deck in sequence.
// Nobody reads this list in sequence. Deleted rather than left unused: an unused
// style is an invitation to put the number back without re-arguing it.

/**
 * The Backs picker's `<select>` (v2.1).
 *
 * Borderless on the chrome fill, like every other v3 control on this page, and
 * sized to the card's own row scale rather than the browser's default — an
 * unstyled select is a 13px system widget in the middle of a 14.5px card, which
 * reads as a form that leaked in from somewhere else.
 */
export const SELECT_STYLE: React.CSSProperties = {
  fontFamily: "inherit",
  fontSize: "14px",
  padding: "2px 6px",
  border: "none",
  borderRadius: "6px",
  background: "var(--v3-chrome)",
  color: "var(--text-primary)",
  cursor: "pointer",
};

export const TITLE_STYLE: React.CSSProperties = {
  flex: 1,
  fontFamily: "var(--font-sans)",
  fontSize: "17px",
  fontWeight: 650,
  lineHeight: 1.3,
  color: "var(--text-primary)",
  minWidth: 0,
};

// A collapsed card is a headline, not a heading — smaller, so an open card
// beside it still reads as the one being worked on.
export const COLLAPSED_TITLE_STYLE: React.CSSProperties = {
  ...TITLE_STYLE,
  fontSize: "15px",
  fontWeight: 600,
};

export const COUNT_TAG_STYLE: React.CSSProperties = {
  flexShrink: 0,
  padding: "3px 9px",
  borderRadius: "4px",
  backgroundColor: "var(--state-success-bg-soft)",
  color: "var(--state-success-strong)",
  fontFamily: "var(--font-sans)",
  fontSize: "12px",
  fontWeight: 600,
  whiteSpace: "nowrap",
};

export const SOURCE_STYLE: React.CSSProperties = {
  display: "flex",
  alignItems: "baseline",
  flexWrap: "wrap",
  gap: "8px",
  paddingLeft: "calc(1.6em + 12px)",
  fontFamily: "var(--font-sans)",
  fontSize: "12.5px",
  color: "var(--text-muted)",
};

export const KIND_CHIP_STYLE: React.CSSProperties = {
  padding: "1px 8px",
  borderRadius: "4px",
  backgroundColor: "var(--bg-page)",
  color: "var(--text-secondary)",
  fontSize: "12px",
};

export const LINK_STYLE: React.CSSProperties = {
  color: "var(--accent-primary)",
  textDecoration: "none",
};

export const CODE_STYLE: React.CSSProperties = {
  marginLeft: "auto",
  fontFamily: "var(--font-mono)",
  fontSize: "11.5px",
  color: "var(--text-muted)",
};

// A two-column grid: the labels form a left rail the eye can run down, which is
// what makes five rows scannable rather than five paragraphs.
export const ROWS_STYLE: React.CSSProperties = {
  display: "grid",
  gridTemplateColumns: "92px minmax(0, 1fr)",
  rowGap: "10px",
  columnGap: "14px",
  paddingLeft: "calc(1.6em + 12px)",
};

export const ROW_LABEL_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "11.5px",
  letterSpacing: "0.06em",
  textTransform: "uppercase",
  color: "var(--text-muted)",
  fontWeight: 600,
  paddingTop: "3px",
};

export const ROW_VALUE_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "14.5px",
  color: "var(--text-primary)",
  lineHeight: 1.45,
  minWidth: 0,
};

// The quote is highlighted rather than quoted: the marks would compete with the
// quotation marks inside a transcript line.
export const QUOTE_STYLE: React.CSSProperties = {
  backgroundColor: "var(--state-warning-bg-soft)",
  padding: "2px 4px",
  borderRadius: "2px",
  boxDecorationBreak: "clone",
  WebkitBoxDecorationBreak: "clone",
};

// Uppercase and tiny, so it reads as a stamp on the field rather than as part of
// the sentence.
export const DRAFT_STYLE: React.CSSProperties = {
  display: "inline-block",
  marginLeft: "8px",
  padding: "1px 6px",
  borderRadius: "3px",
  backgroundColor: "var(--bg-page)",
  color: "var(--text-muted)",
  fontFamily: "var(--font-sans)",
  fontSize: "10.5px",
  letterSpacing: "0.05em",
  textTransform: "uppercase",
  verticalAlign: "middle",
};

export const INLINE_EDIT_STYLE: React.CSSProperties = {
  marginLeft: "8px",
  padding: 0,
  border: "none",
  background: "none",
  color: "var(--accent-primary)",
  fontFamily: "var(--font-sans)",
  fontSize: "12px",
  cursor: "pointer",
};

export const BUTTON_STYLE: React.CSSProperties = {
  padding: "3px 12px",
  border: "1px solid var(--border-default)",
  borderRadius: "6px",
  backgroundColor: "var(--bg-surface)",
  color: "var(--text-secondary)",
  fontFamily: "var(--font-sans)",
  fontSize: "12px",
  fontWeight: 600,
  cursor: "pointer",
};

export const FIELD_STYLE: React.CSSProperties = {
  width: "100%",
  minHeight: "72px",
  resize: "vertical",
  padding: "8px 10px",
  border: "1px solid var(--border-default)",
  borderRadius: "6px",
  backgroundColor: "var(--bg-surface)",
  color: "var(--text-primary)",
  fontFamily: "var(--font-sans)",
  fontSize: "14px",
  boxSizing: "border-box",
};

// The editor closes on save, so this line is the only thing that says the
// sentence on screen is not the sentence stored (Standing Rule 1).
export const ERROR_STYLE: React.CSSProperties = {
  padding: "6px 10px",
  border: "1px solid var(--state-danger-border)",
  backgroundColor: "var(--state-danger-bg-soft)",
  borderRadius: "6px",
  color: "var(--state-danger-strong)",
  fontFamily: "var(--font-sans)",
  fontSize: "12.5px",
};
