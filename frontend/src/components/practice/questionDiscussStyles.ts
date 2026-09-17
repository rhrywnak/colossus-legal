// =============================================================================
// questionDiscussStyles.ts — the "Discuss with AI" drawer, as drawn
// =============================================================================
//
// QUESTION_CHAT_DOCK_RULED_2026-09-17. Tokens only — `--practice-*` and the house
// tokens; no literal colour (Rule 2).
//
// STRUCTURAL: geometry transcribed from one approved drawing (a ~430 px right
// drawer over a dimmed page) — not settings.

import type { CSSProperties } from "react";

import { BLUE, INK, LINE, MUTED, PALE, PAPER } from "./practiceStyles";

/** The dim over the page. The page stays mounted underneath — see the guard test. */
export const scrim: CSSProperties = {
  position: "fixed",
  inset: 0,
  background: "var(--practice-ink)",
  opacity: 0.28,
  zIndex: 900,
};

/** The drawer itself. */
export const drawer: CSSProperties = {
  position: "fixed",
  top: 0,
  right: 0,
  bottom: 0,
  width: "min(430px, 100vw)",
  background: PAPER,
  boxShadow: "var(--shadow-raised)",
  zIndex: 901,
  display: "flex",
  flexDirection: "column",
};

/** Title block, model chip and ✕. */
export const header: CSSProperties = {
  display: "flex",
  alignItems: "flex-start",
  gap: 10,
  padding: "14px 16px",
  borderBottom: `1px solid ${LINE}`,
};

export const title: CSSProperties = { fontSize: 19, fontWeight: 700, color: INK, margin: 0 };
export const subtitle: CSSProperties = { fontSize: 14, color: MUTED };

/** The model picker, drawn as the mockup's chip. */
export const picker: CSSProperties = {
  marginLeft: "auto",
  font: "inherit",
  fontSize: 14,
  padding: "4px 8px",
  borderRadius: 8,
  border: `1px solid ${LINE}`,
  background: PALE,
  color: INK,
  maxWidth: 170,
};

export const close: CSSProperties = {
  background: "none",
  border: "none",
  fontSize: 22,
  lineHeight: 1,
  color: MUTED,
  cursor: "pointer",
  padding: "2px 4px",
};

/** The scrolling thread. */
export const thread: CSSProperties = { flex: 1, overflowY: "auto", padding: "12px 16px" };

export const contextLine: CSSProperties = {
  textAlign: "center",
  fontSize: 13,
  color: MUTED,
  margin: "0 0 12px",
};

/** The author line over a bubble: small, upper case. */
export function authorLine(role: "user" | "model"): CSSProperties {
  return {
    fontSize: 12,
    fontWeight: 700,
    letterSpacing: "0.05em",
    textTransform: "uppercase",
    color: MUTED,
    margin: role === "user" ? "10px 0 4px 40px" : "10px 0 4px",
  };
}

/** A bubble: a person's on the pale ground, indented; a model's on paper. */
export function bubble(role: "user" | "model"): CSSProperties {
  return {
    border: `1px solid ${LINE}`,
    borderRadius: 10,
    padding: "10px 12px",
    fontSize: 15,
    lineHeight: 1.5,
    color: INK,
    whiteSpace: "pre-wrap",
    background: role === "user" ? "var(--practice-choice-selected-bg)" : PAPER,
    margin: role === "user" ? "0 0 0 40px" : "0 40px 0 0",
  };
}

/** The per-reply cost line — small and quiet. */
export const cost: CSSProperties = { fontSize: 12, color: MUTED, margin: "4px 40px 0 2px" };

export const status: CSSProperties = { fontSize: 14, color: MUTED, margin: "10px 0" };
export const failure: CSSProperties = { fontSize: 14, color: "var(--practice-red)", margin: "10px 0" };

/** The input row and footer. */
export const composer: CSSProperties = { borderTop: `1px solid ${LINE}`, padding: "12px 16px" };
export const inputRow: CSSProperties = { display: "flex", gap: 8 };
export const input: CSSProperties = {
  flex: 1,
  font: "inherit",
  fontSize: 15,
  padding: "8px 10px",
  borderRadius: 8,
  border: "1px solid var(--practice-control-border)",
};
export const send: CSSProperties = {
  font: "inherit",
  fontSize: 15,
  fontWeight: 600,
  padding: "8px 16px",
  borderRadius: 8,
  border: "none",
  background: BLUE,
  color: PAPER,
  cursor: "pointer",
};
export const footer: CSSProperties = { fontSize: 12, color: MUTED, marginTop: 8 };

/** The page's "Discuss with AI" button: dashed, like the mockup. */
export const openButton: CSSProperties = {
  font: "inherit",
  fontSize: 16,
  padding: "10px 16px",
  borderRadius: 8,
  border: `1px dashed ${BLUE}`,
  background: PAPER,
  color: BLUE,
  cursor: "pointer",
};
