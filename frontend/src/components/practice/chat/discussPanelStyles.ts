// discussPanelStyles.ts — the discussion panel, value for value from mockup v2.
//
// Mockup of record: DISCUSS_CHAT_MOCKUP_v2_2026-09-21.html (ratified 2026-09-21).
// Every number below is copied from its inline styles; every colour is a
// `--chat-*` token in `styles/tokens.css`, whose VALUES are the mockup's. Where a
// value differs, the build report's REPRODUCED/DEVIATED table says why.

import type { CSSProperties } from "react";

import { button as practiceButton } from "../practiceStyles";

// Board 1 — the side panel.
export const shell: CSSProperties = { display: "flex", alignItems: "stretch", background: "var(--chat-anchor-bg)" };

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
  background: "var(--chat-divider-bg)",
  borderLeft: "1px solid var(--chat-border-strong)",
  borderRight: "1px solid var(--chat-border-strong)",
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  cursor: "col-resize",
  flexShrink: 0,
};
export const dividerGrip: CSSProperties = { width: 3, height: 44, borderRadius: 2, background: "var(--chat-divider-grip)" };
export const panel: CSSProperties = {
  flexGrow: 1,
  minWidth: 0,
  boxSizing: "border-box",
  display: "flex",
  flexDirection: "column",
  background: "var(--chat-surface)",
  color: "var(--chat-ink)",
  height: "100%",
  fontFamily: "system-ui, sans-serif",
};
export const header: CSSProperties = {
  boxSizing: "border-box",
  padding: "14px 24px",
  borderBottom: "1px solid var(--chat-rule)",
  display: "flex",
  alignItems: "center",
  gap: 12,
};
export const headerFull: CSSProperties = { ...header, padding: "12px 24px", alignItems: "flex-start" };
export const switcherButton: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 8,
  background: "var(--chat-field-bg)",
  border: "1px solid var(--chat-control-border)",
  borderRadius: 8,
  padding: "6px 12px",
  cursor: "pointer",
  fontSize: 14,
  fontWeight: 700,
  color: "var(--chat-ink)",
  whiteSpace: "nowrap",
  // v2.2.2 GO: the switcher gives way on a narrow panel — its label ellipsizes
  // (switcherLabelText) so the expand and close buttons never leave the panel.
  // `maxWidth` too: a button sizes to its content unless capped, and uncapped it
  // spilled over the expand button while its anchor shrank beneath it.
  minWidth: 0, maxWidth: "100%",
  flexShrink: 1,
};
export const switcherLabelText: CSSProperties = { overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", minWidth: 0 };
export const switcherButtonOpen: CSSProperties = { ...switcherButton, border: "1px solid var(--chat-accent-dark)" };
// v2.2.2: the side header is a COLUMN — a top row that never wraps text, then the
// visibility line on its own full-width line. It used to share the row with the
// switcher, chip and two buttons, and a long line was squeezed to ~70px (PROD, S-11).
export const headerSide: CSSProperties = { ...header, flexDirection: "column", alignItems: "stretch", gap: 6 };
export const headerRow: CSSProperties = { display: "flex", alignItems: "center", gap: 12 };
export const visibilityText: CSSProperties = { fontSize: 12, color: "var(--chat-muted)" };
export const chip: CSSProperties = {
  padding: "4px 10px",
  fontSize: 12,
  fontWeight: 600,
  color: "var(--chat-grounded-ink)",
  background: "var(--chat-grounded-bg)",
  border: "1px solid var(--chat-grounded-border)",
  borderRadius: 999,
  whiteSpace: "nowrap",
  // v2.2.2: the one control that may give way. At the narrowest panel the row's
  // fixed widths exceed the room, and without this the expand and close buttons
  // were pushed past the panel's edge. The chip ellipsizes; its title keeps the
  // whole text. `flexShrink: 3` so it gives way before the thread's name does —
  // the name is what the reader needs; the model is one hover away.
  minWidth: 0, flexShrink: 3,
  overflow: "hidden",
  textOverflow: "ellipsis",
};
export const iconButton: CSSProperties = {
  width: 32,
  height: 32,
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  background: "var(--chat-surface)",
  border: "1px solid var(--chat-control-border)",
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
  background: "var(--chat-user-bubble)",
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
  background: "var(--chat-field-bg)",
  border: "1px solid var(--chat-reply-border)",
  borderRadius: "12px 12px 12px 4px",
  padding: "14px 18px",
  fontSize: 15,
  lineHeight: 1.55,
};
export const replyBubble: CSSProperties = { ...replyBubbleFirst, borderRadius: 12 };
export const card: CSSProperties = {
  background: "var(--chat-surface)",
  border: "1px solid var(--chat-card-border)",
  borderRadius: 8,
  padding: "10px 14px",
  display: "flex",
  flexDirection: "column",
  gap: 4,
};
export const cardTitle: CSSProperties = {
  fontSize: 12,
  fontWeight: 700,
  color: "var(--chat-card-title)",
  textTransform: "uppercase",
};
export const cardQuote: CSSProperties = { fontSize: 13.5, fontStyle: "italic", color: "var(--chat-quote-ink)", lineHeight: 1.5 };
/** The Earlier discussion's author · time line (ADDENDUM_1; not on the mockup). */
export const byline: CSSProperties = { fontSize: 11.5, color: "var(--chat-muted)", marginBottom: -10 };
export const quietLine: CSSProperties = { fontSize: 13, color: "var(--chat-muted)", lineHeight: 1.5 };
export const failureLine: CSSProperties = {
  alignSelf: "flex-start",
  fontSize: 13.5,
  color: "var(--chat-warn-ink)",
  background: "var(--chat-warn-bg)",
  border: "1px solid var(--chat-warn-border)",
  borderRadius: 8,
  padding: "8px 12px",
};
export const composer: CSSProperties = {
  boxSizing: "border-box",
  padding: "14px 24px 18px",
  borderTop: "1px solid var(--chat-rule)",
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
  border: "1px solid var(--chat-control-border)",
  borderRadius: 10,
  background: "var(--chat-field-bg)",
};
export const send: CSSProperties = {
  padding: "12px 22px",
  fontSize: 15,
  fontWeight: 600,
  background: "var(--chat-accent-dark)",
  color: "var(--chat-surface)",
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
  background: "var(--chat-surface)",
  color: "var(--chat-ink)",
  fontFamily: "system-ui, sans-serif",
};
export const strip: CSSProperties = {
  boxSizing: "border-box",
  padding: "10px 24px",
  background: "var(--chat-anchor-bg)",
  borderBottom: "1px solid var(--chat-border-strong)",
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
  color: "var(--chat-quote-ink)",
  padding: "4px 0",
};
export const stripText: CSSProperties = { flexGrow: 1, display: "flex", flexDirection: "column", gap: 1, minWidth: 0 };
export const stripCode: CSSProperties = { fontSize: 11, letterSpacing: "0.08em", color: "var(--chat-muted)", fontWeight: 600 };
export const stripLine: CSSProperties = {
  fontSize: 14,
  fontWeight: 600,
  whiteSpace: "nowrap",
  overflow: "hidden",
  textOverflow: "ellipsis",
};
export const menuAnchor: CSSProperties = { position: "relative", minWidth: 0, flexShrink: 1 };
export const menu: CSSProperties = {
  position: "absolute",
  top: 42,
  left: 0,
  width: 380,
  background: "var(--chat-surface)",
  border: "1px solid var(--chat-control-border)",
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
  color: "var(--chat-ink)",
  fontFamily: "inherit",
};
export const menuRowSelected: CSSProperties = { ...menuRow, background: "var(--chat-row-selected)" };
export const avatar: CSSProperties = {
  width: 30,
  height: 30,
  borderRadius: "50%",
  color: "var(--chat-surface)",
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  fontWeight: 700,
  fontSize: 13,
  flexShrink: 0,
};
/** The three avatar colours of the mockup, in switcher order (M, C, R). */
export const AVATAR_COLOURS = ["var(--chat-accent-dark)", "var(--chat-avatar-2)", "var(--chat-avatar-3)"];
export const rowText: CSSProperties = { display: "flex", flexDirection: "column", gap: 2, minWidth: 0 };
export const rowTitle: CSSProperties = { fontSize: 13.5, fontWeight: 700 };
export const rowTitleEmpty: CSSProperties = { fontSize: 13, color: "var(--chat-muted)" };
export const rowPreview: CSSProperties = {
  fontSize: 12.5,
  color: "var(--chat-preview-ink)",
  whiteSpace: "nowrap",
  overflow: "hidden",
  textOverflow: "ellipsis",
};
export const rowUnread: CSSProperties = { color: "var(--chat-warn-ink)" };
export const rowReadOnly: CSSProperties = { fontSize: 11.5, color: "var(--chat-muted)" };
export const menuFooter: CSSProperties = {
  borderTop: "1px solid var(--chat-divider-bg)",
  marginTop: 4,
  padding: "9px 12px",
  fontSize: 12,
  color: "var(--chat-muted)",
  lineHeight: 1.45,
};

// The left pane's two additions (board 1): the dark Discuss button and its hint.
//
// ⚑ It SPREADS the practice `button` base (v2.2.1, Fix 4 — Roman, 2026-09-21).
// It sits beside Answer, and with its own 15px / 12px padding / no border it came
// out a different height and lettering from the button next to it. Now font,
// size, padding, border width and radius come from the one base Answer also
// spreads, and only the dark fill, its border colour and the text colour differ
// — the same four things `buttonPrimary` itself overrides.
export const openButton: CSSProperties = {
  ...practiceButton,
  fontWeight: 600,
  background: "var(--chat-accent-dark)",
  borderColor: "var(--chat-accent-dark)",
  color: "var(--chat-surface)",
};
export const openHint: CSSProperties = { fontSize: 13, color: "var(--chat-muted)", lineHeight: 1.5, marginTop: 10 };
