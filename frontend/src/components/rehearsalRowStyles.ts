// =============================================================================
// rehearsalRowStyles.ts — the content inside the rehearsal page's cards
// =============================================================================
//
// The second half of the signed mockup's `<style>` block: the accusation, the
// instance rows, the drawn timeline, the talking points and the watch items.
// `rehearsalStyles` holds the FRAME those sit in — the page box, the header
// controls, the section cards, the Always strip — and its header carries the
// full mockup-variable → token mapping these rules follow.
//
// Split for Rule 17. The seam is the page's own: a change to the frame is a
// change to how the page is laid out, and a change here is a change to how one
// kind of row reads.

import type React from "react";

import { HAIRLINE, HAIRLINE_STRONG } from "./rehearsalStyles";

/** `.accusation` — the sentence the whole page is about. */
export const accusationStyle: React.CSSProperties = {
  borderLeft: "3px solid var(--accent-primary)",
  background: "var(--accent-bg-soft)",
  borderRadius: "0 8px 8px 0",
  padding: "14px 18px",
  margin: "8px 0 0",
  fontSize: "17px",
  fontWeight: 500,
  lineHeight: 1.5,
};

/** `.countline` */
export const countLineStyle: React.CSSProperties = {
  fontSize: "13px",
  color: "var(--text-muted)",
  margin: "12px 2px 2px",
};

/** `.instance` */
export const instanceStyle: React.CSSProperties = {
  borderTop: HAIRLINE,
  marginTop: "14px",
  paddingTop: "12px",
};

/** `.inst-meta` — the clickable summary line. */
export const instanceMetaStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  flexWrap: "wrap",
  gap: "8px 14px",
  fontSize: "13.5px",
  width: "100%",
  border: "none",
  background: "transparent",
  padding: 0,
  fontFamily: "inherit",
  textAlign: "left",
  color: "var(--text-primary)",
  cursor: "pointer",
};

/** `.inst-num` — the filled row number. */
export const instanceNumberStyle: React.CSSProperties = {
  width: "22px",
  height: "22px",
  borderRadius: "50%",
  background: "var(--text-primary)",
  color: "var(--v3-on-fill)",
  fontSize: "12px",
  fontWeight: 600,
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  flex: "none",
};

/** `.chip` — the kind of statement. */
export const chipStyle: React.CSSProperties = {
  fontSize: "11.5px",
  padding: "2px 9px",
  borderRadius: "999px",
  background: "var(--v3-chrome)",
  color: "var(--text-secondary)",
  whiteSpace: "nowrap",
};

/** `.minitag` — the red NO ANSWER tag. */
export const miniTagStyle: React.CSSProperties = {
  fontSize: "10.5px",
  fontWeight: 700,
  letterSpacing: "0.04em",
  color: "var(--v3-red-text)",
  background: "var(--state-danger-bg-soft)",
  borderRadius: "4px",
  padding: "2px 7px",
  whiteSpace: "nowrap",
};

/** `.minitag.ok` — the green ANSWERED tag. */
export const miniTagOkStyle: React.CSSProperties = {
  ...miniTagStyle,
  color: "var(--v3-green-text)",
  background: "var(--state-success-bg-soft)",
};

/** `.inst-firstline` — the quote's opening, when the row is compact. */
export const instanceFirstLineStyle: React.CSSProperties = {
  margin: "4px 0 0 36px",
  fontSize: "14px",
  color: "var(--text-muted)",
  fontStyle: "italic",
};

/** `.quote` */
export const quoteStyle: React.CSSProperties = {
  margin: "10px 0 0 36px",
  padding: "2px 0 2px 14px",
  borderLeft: "2px solid var(--border-card)",
  fontSize: "15px",
  maxWidth: "62ch",
};

/** `.answer` */
export const answerStyle: React.CSSProperties = {
  margin: "12px 0 4px 36px",
  padding: "10px 14px",
  borderLeft: "3px solid var(--v3-green-text)",
  background: "var(--state-success-bg-soft)",
  borderRadius: "0 8px 8px 0",
  maxWidth: "66ch",
};

/** `.answer .label` */
export const answerLabelStyle: React.CSSProperties = {
  fontSize: "11.5px",
  fontWeight: 700,
  letterSpacing: "0.05em",
  color: "var(--v3-green-text)",
  textTransform: "uppercase",
};

/** `.answer .a-meta` */
export const answerMetaStyle: React.CSSProperties = {
  fontSize: "13px",
  color: "var(--text-secondary)",
  marginTop: "3px",
};

/** `.answer .a-quote` */
export const answerQuoteStyle: React.CSSProperties = {
  marginTop: "5px",
  fontSize: "14.5px",
};

/** `.nogap` — the short banner inside an opened, unanswered row. */
export const noAnswerBannerStyle: React.CSSProperties = {
  margin: "12px 0 4px 36px",
  padding: "8px 14px",
  borderLeft: "3px solid var(--v3-red-text)",
  background: "var(--state-danger-bg-soft)",
  borderRadius: "0 8px 8px 0",
  fontSize: "13px",
  fontWeight: 700,
  letterSpacing: "0.03em",
  color: "var(--v3-red-text)",
};

/** `.preplist` */
export const prepListStyle: React.CSSProperties = {
  marginTop: "20px",
  borderTop: HAIRLINE,
  paddingTop: "12px",
};

/** `.preplist h4` */
export const prepHeadingStyle: React.CSSProperties = {
  fontSize: "12.5px",
  fontWeight: 600,
  letterSpacing: "0.05em",
  textTransform: "uppercase",
  color: "var(--text-secondary)",
  marginBottom: "8px",
};

/** `.prep-item` */
export const prepItemStyle: React.CSSProperties = {
  fontSize: "13.5px",
  color: "var(--v3-red-text)",
  fontWeight: 600,
  padding: "3px 0",
};

/** `.prep-item a` — the jump link. */
export const prepJumpStyle: React.CSSProperties = {
  color: "var(--accent-primary)",
  fontWeight: 500,
  fontSize: "12.5px",
  marginLeft: "6px",
  background: "none",
  border: "none",
  padding: 0,
  fontFamily: "inherit",
  cursor: "pointer",
};

/** `.tl-filter` */
export const timelineFilterStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "8px",
  fontSize: "13px",
  color: "var(--text-muted)",
  margin: "6px 0 14px",
};

/** `.tl-filter select` */
export const timelineSelectStyle: React.CSSProperties = {
  fontFamily: "inherit",
  fontSize: "13px",
  padding: "3px 8px",
  border: HAIRLINE_STRONG,
  borderRadius: "6px",
  color: "var(--text-secondary)",
  background: "var(--bg-surface)",
};

/** `.tl` — the spine. */
export const timelineSpineStyle: React.CSSProperties = {
  position: "relative",
  marginLeft: "8px",
  paddingLeft: "24px",
  borderLeft: "2px solid var(--border-card)",
};

/** `.tl-row` */
export const timelineRowStyle: React.CSSProperties = {
  position: "relative",
  padding: "8px 0 14px",
};

/** `.tl-dot` — sits ON the spine, ringed in the page's background. */
export const timelineDotStyle: React.CSSProperties = {
  position: "absolute",
  left: "-31px",
  top: "13px",
  width: "12px",
  height: "12px",
  borderRadius: "50%",
  border: "2px solid var(--bg-surface)",
};

/** `.tl-date` */
export const timelineDateStyle: React.CSSProperties = {
  fontSize: "12.5px",
  fontWeight: 600,
  color: "var(--text-secondary)",
};

/** `.tl-side` — the base of the two side markers. */
export const timelineSideStyle: React.CSSProperties = {
  fontSize: "10.5px",
  fontWeight: 700,
  letterSpacing: "0.05em",
  borderRadius: "4px",
  padding: "1px 7px",
  marginLeft: "8px",
};

/** `.tl-body` */
export const timelineBodyStyle: React.CSSProperties = {
  fontSize: "14px",
  marginTop: "3px",
  maxWidth: "62ch",
};

/** `.tl-src` */
export const timelineSourceStyle: React.CSSProperties = {
  fontSize: "12.5px",
  color: "var(--text-muted)",
  marginTop: "2px",
};

/** `.point` */
export const pointStyle: React.CSSProperties = {
  display: "flex",
  gap: "12px",
  marginTop: "12px",
  alignItems: "flex-start",
};

/** `.pt-num` — the accent pill. */
export const pointNumberStyle: React.CSSProperties = {
  width: "22px",
  height: "22px",
  borderRadius: "50%",
  background: "var(--accent-primary)",
  color: "var(--v3-on-fill)",
  flex: "none",
  fontSize: "12px",
  fontWeight: 600,
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  marginTop: "2px",
};

/** `.pt-text` */
export const pointTextStyle: React.CSSProperties = {
  fontSize: "15.5px",
  maxWidth: "58ch",
};

/** `.pt-exhibit` */
export const pointExhibitStyle: React.CSSProperties = {
  fontSize: "12.5px",
  color: "var(--text-muted)",
  marginTop: "3px",
};

/** `.addrow` */
export const addRowStyle: React.CSSProperties = {
  marginTop: "14px",
  display: "flex",
  alignItems: "center",
  gap: "10px",
  flexWrap: "wrap",
};

/** `.addnote` */
export const addNoteStyle: React.CSSProperties = {
  fontSize: "12px",
  color: "var(--text-muted)",
};

/** `.watchitem` */
export const watchItemStyle: React.CSSProperties = {
  display: "flex",
  gap: "10px",
  marginTop: "10px",
  fontSize: "15px",
  maxWidth: "60ch",
  alignItems: "flex-start",
};

