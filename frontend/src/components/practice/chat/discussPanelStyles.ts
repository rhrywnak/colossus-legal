// discussPanelStyles.ts — the discussion panel, value for value from mockup v2.
//
// Mockup of record: DISCUSS_CHAT_MOCKUP_v2_2026-09-21.html (ratified 2026-09-21).
// Every number and colour below is copied from its inline styles; where a value
// here differs, the build report's REPRODUCED/DEVIATED table says why.

import type { CSSProperties } from "react";

// Board 1 — the side panel.
export const shell: CSSProperties = { display: "flex", alignItems: "stretch", background: "#f4f3f0" };

/**
 * The shell with the panel OPEN fills the window below its own top edge, so the
 * question and the thread scroll independently and the composer is always on
 * screen — the mockup's board is one fixed-height frame. `top` is where the shell
 * starts (under the app's header), measured by the page.
 */
export function shellOpen(top: number): CSSProperties {
  return { ...shell, height: `calc(100vh - ${Math.max(0, Math.round(top))}px)`, overflow: "hidden" };
}
/** The same shell with the panel shut: no colour of its own, so the page looks as it always did. */
export const shellShut: CSSProperties = { display: "flex", alignItems: "stretch" };
// `flexShrink: 0`: the left pane keeps the width the divider gave it; a long
// header line in the panel must never squeeze the question.
export const leftPane: CSSProperties = { boxSizing: "border-box", minWidth: 0, overflowY: "auto", flexShrink: 0 };
export const divider: CSSProperties = {
  width: 10,
  boxSizing: "border-box",
  background: "#ece8e0",
  borderLeft: "1px solid #ddd8cf",
  borderRight: "1px solid #ddd8cf",
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  cursor: "col-resize",
  flexShrink: 0,
};
export const dividerGrip: CSSProperties = { width: 3, height: 44, borderRadius: 2, background: "#b9b2a4" };
export const panel: CSSProperties = {
  flexGrow: 1,
  minWidth: 0,
  boxSizing: "border-box",
  display: "flex",
  flexDirection: "column",
  background: "#ffffff",
  color: "#23221f",
  height: "100%",
  fontFamily: "system-ui, sans-serif",
};
export const header: CSSProperties = {
  boxSizing: "border-box",
  padding: "14px 24px",
  borderBottom: "1px solid #e2ddd3",
  display: "flex",
  alignItems: "center",
  gap: 12,
};
export const headerFull: CSSProperties = { ...header, padding: "12px 24px", alignItems: "flex-start" };
export const switcherButton: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 8,
  background: "#faf9f5",
  border: "1px solid #cfc9bd",
  borderRadius: 8,
  padding: "6px 12px",
  cursor: "pointer",
  fontSize: 14,
  fontWeight: 700,
  color: "#23221f",
  whiteSpace: "nowrap",
  flexShrink: 0,
};
export const switcherButtonOpen: CSSProperties = { ...switcherButton, border: "1px solid #3d3a33" };
export const visibility: CSSProperties = { flexGrow: 1, display: "flex", flexDirection: "column", gap: 2 };
export const visibilityText: CSSProperties = { fontSize: 12, color: "#8a8578" };
export const chip: CSSProperties = {
  padding: "4px 10px",
  fontSize: 12,
  fontWeight: 600,
  color: "#2c6e49",
  background: "#e9f3ec",
  border: "1px solid #bcd9c6",
  borderRadius: 999,
  whiteSpace: "nowrap",
};
export const iconButton: CSSProperties = {
  width: 32,
  height: 32,
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  background: "#ffffff",
  border: "1px solid #cfc9bd",
  borderRadius: 8,
  cursor: "pointer",
  flexShrink: 0,
};
export const messages: CSSProperties = {
  flexGrow: 1,
  boxSizing: "border-box",
  padding: "20px 24px",
  display: "flex",
  flexDirection: "column",
  gap: 16,
  overflowY: "auto",
};
export const messagesFull: CSSProperties = { ...messages, padding: "20px 180px" };
export const userBubble: CSSProperties = {
  alignSelf: "flex-end",
  maxWidth: 480,
  background: "#eef0f4",
  borderRadius: "12px 12px 4px 12px",
  padding: "12px 16px",
  fontSize: 15,
  lineHeight: 1.5,
  whiteSpace: "pre-wrap",
};
export const userBubbleFull: CSSProperties = { ...userBubble, maxWidth: 520 };
export const replyColumn: CSSProperties = {
  alignSelf: "flex-start",
  maxWidth: 560,
  display: "flex",
  flexDirection: "column",
  gap: 10,
};
export const replyColumnFull: CSSProperties = { ...replyColumn, maxWidth: 640 };
export const replyBubbleFirst: CSSProperties = {
  background: "#faf9f5",
  border: "1px solid #e6e1d6",
  borderRadius: "12px 12px 12px 4px",
  padding: "14px 18px",
  fontSize: 15,
  lineHeight: 1.55,
};
export const replyBubble: CSSProperties = { ...replyBubbleFirst, borderRadius: 12 };
export const card: CSSProperties = {
  background: "#ffffff",
  border: "1px solid #d8d2c6",
  borderRadius: 8,
  padding: "10px 14px",
  display: "flex",
  flexDirection: "column",
  gap: 4,
};
export const cardTitle: CSSProperties = {
  fontSize: 12,
  fontWeight: 700,
  color: "#8a5a2b",
  textTransform: "uppercase",
};
export const cardQuote: CSSProperties = { fontSize: 13.5, fontStyle: "italic", color: "#55503f", lineHeight: 1.5 };
/** The Earlier discussion's author · time line (ADDENDUM_1; not on the mockup). */
export const byline: CSSProperties = { fontSize: 11.5, color: "#8a8578", marginBottom: -10 };
export const quietLine: CSSProperties = { fontSize: 13, color: "#8a8578", lineHeight: 1.5 };
export const failureLine: CSSProperties = {
  alignSelf: "flex-start",
  fontSize: 13.5,
  color: "#a07030",
  background: "#fdf6ee",
  border: "1px solid #ecd9bd",
  borderRadius: 8,
  padding: "8px 12px",
};
export const composer: CSSProperties = {
  boxSizing: "border-box",
  padding: "14px 24px 18px",
  borderTop: "1px solid #e2ddd3",
  display: "flex",
  gap: 10,
  alignItems: "center",
};
export const composerFull: CSSProperties = { ...composer, padding: "14px 180px 18px" };
export const input: CSSProperties = {
  flexGrow: 1,
  boxSizing: "border-box",
  padding: "12px 16px",
  fontSize: 15,
  border: "1px solid #cfc9bd",
  borderRadius: 10,
  background: "#faf9f5",
};
export const send: CSSProperties = {
  padding: "12px 22px",
  fontSize: 15,
  fontWeight: 600,
  background: "#3d3a33",
  color: "#ffffff",
  border: "none",
  borderRadius: 10,
  cursor: "pointer",
};
export const sendDisabled: CSSProperties = { ...send, opacity: 0.5, cursor: "default" };

// Board 2 — full screen, the anchor collapsed to a strip, the switcher open.
export const full: CSSProperties = {
  position: "fixed",
  inset: 0,
  zIndex: 950,
  display: "flex",
  flexDirection: "column",
  background: "#ffffff",
  color: "#23221f",
  fontFamily: "system-ui, sans-serif",
};
export const strip: CSSProperties = {
  boxSizing: "border-box",
  padding: "10px 24px",
  background: "#f4f3f0",
  borderBottom: "1px solid #ddd8cf",
  display: "flex",
  alignItems: "center",
  gap: 14,
};
export const back: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 6,
  background: "none",
  border: "none",
  cursor: "pointer",
  fontSize: 13.5,
  color: "#55503f",
  padding: "4px 0",
};
export const stripText: CSSProperties = { flexGrow: 1, display: "flex", flexDirection: "column", gap: 1, minWidth: 0 };
export const stripCode: CSSProperties = { fontSize: 11, letterSpacing: "0.08em", color: "#8a8578", fontWeight: 600 };
export const stripLine: CSSProperties = {
  fontSize: 14,
  fontWeight: 600,
  whiteSpace: "nowrap",
  overflow: "hidden",
  textOverflow: "ellipsis",
};
export const menuAnchor: CSSProperties = { position: "relative" };
export const menu: CSSProperties = {
  position: "absolute",
  top: 42,
  left: 0,
  width: 380,
  background: "#ffffff",
  border: "1px solid #cfc9bd",
  borderRadius: 10,
  boxShadow: "0 8px 24px rgba(0,0,0,0.14)",
  padding: 6,
  display: "flex",
  flexDirection: "column",
  zIndex: 5,
};
export const menuRow: CSSProperties = {
  display: "flex",
  gap: 10,
  alignItems: "flex-start",
  padding: "10px 12px",
  borderRadius: 8,
  background: "none",
  border: "none",
  textAlign: "left",
  cursor: "pointer",
  color: "#23221f",
  fontFamily: "inherit",
};
export const menuRowSelected: CSSProperties = { ...menuRow, background: "#f1efe9" };
export const avatar: CSSProperties = {
  width: 30,
  height: 30,
  borderRadius: "50%",
  color: "#fff",
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  fontWeight: 700,
  fontSize: 13,
  flexShrink: 0,
};
/** The three avatar colours of the mockup, in switcher order (M, C, R). */
export const AVATAR_COLOURS = ["#3d3a33", "#6b7d8f", "#a08c5a"];
export const rowText: CSSProperties = { display: "flex", flexDirection: "column", gap: 2, minWidth: 0 };
export const rowTitle: CSSProperties = { fontSize: 13.5, fontWeight: 700 };
export const rowTitleEmpty: CSSProperties = { fontSize: 13, color: "#8a8578" };
export const rowPreview: CSSProperties = {
  fontSize: 12.5,
  color: "#6e6a5e",
  whiteSpace: "nowrap",
  overflow: "hidden",
  textOverflow: "ellipsis",
};
export const rowUnread: CSSProperties = { color: "#a07030" };
export const rowReadOnly: CSSProperties = { fontSize: 11.5, color: "#8a8578" };
export const menuFooter: CSSProperties = {
  borderTop: "1px solid #ece8e0",
  marginTop: 4,
  padding: "9px 12px",
  fontSize: 12,
  color: "#8a8578",
  lineHeight: 1.45,
};

// The left pane's two additions (board 1): the dark Discuss button and its hint.
export const openButton: CSSProperties = {
  padding: "12px 18px",
  fontSize: 15,
  fontWeight: 600,
  background: "#3d3a33",
  color: "#ffffff",
  border: "none",
  borderRadius: 8,
  cursor: "pointer",
};
export const openHint: CSSProperties = { fontSize: 13, color: "#8a8578", lineHeight: 1.5, marginTop: 10 };
