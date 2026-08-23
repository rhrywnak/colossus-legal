// practiceWalkStyles.ts — practice mode (mockup v7, views 5–7).
//
// Centred, sparse, and deliberately unlike every other practice screen: this is
// the one surface where nothing is being recorded and nothing is being judged,
// and it should not look like the ones where both are true.

import type { CSSProperties } from "react";

export const centre: CSSProperties = { textAlign: "center", padding: "26px 22px" };

/** `PRACTICE · THE DEFENSE ASKS · 2 OF 5` */
export const counter: CSSProperties = {
  fontSize: 12,
  color: "var(--practice-muted)",
  margin: "0 0 20px",
  letterSpacing: ".06em",
};

/** The question, while she is answering it out loud. */
export const question: CSSProperties = {
  fontFamily: "Georgia, 'Times New Roman', serif",
  fontSize: 22,
  lineHeight: 1.4,
  margin: "0 auto 22px",
  maxWidth: 610,
  color: "var(--practice-ink)",
};

/** The same question after the reveal — smaller and grey, because her own words
 *  are now the thing on the page and the question is context for them. */
export const questionSmall: CSSProperties = { ...question, fontSize: 18, color: "var(--practice-muted)" };

/**
 * The area that holds her answer before she asks for it.
 *
 * Dashed and grey, and it CARRIES ITS INSTRUCTION — standing rule of
 * 2026-08-19: no control on a practice page is dim and silent. An empty grey
 * box would read as something that failed to load.
 */
export const grey: CSSProperties = {
  maxWidth: 640,
  margin: "0 auto",
  border: "1px dashed var(--practice-control-border)",
  borderRadius: 7,
  background: "var(--practice-pale)",
  color: "var(--practice-muted)",
  padding: "22px 20px",
  fontSize: 14.5,
};

/** Her answer, revealed. */
export const mine: CSSProperties = {
  background: "var(--practice-resume-bg)",
  border: "1px solid var(--practice-resume-border)",
  borderRadius: 9,
  padding: "17px 20px",
  textAlign: "left",
  maxWidth: 640,
  margin: "0 auto",
};

export const mineLabel: CSSProperties = {
  fontSize: 10.5,
  letterSpacing: ".1em",
  textTransform: "uppercase",
  color: "var(--practice-blue)",
  fontWeight: 700,
  margin: "0 0 7px",
};

export const mineText: CSSProperties = {
  fontFamily: "Georgia, 'Times New Roman', serif",
  fontSize: 17.5,
  lineHeight: 1.5,
  margin: 0,
  color: "var(--practice-ink)",
};

export const buttons: CSSProperties = {
  margin: "20px 0 0",
  display: "flex",
  gap: 9,
  justifyContent: "center",
  alignItems: "center",
  flexWrap: "wrap",
};

/** An anchor wearing a button's clothes — same box, no underline. */
export const bigLink: CSSProperties = { textDecoration: "none", display: "inline-block" };

export const back: CSSProperties = {
  color: "var(--practice-blue)",
  fontSize: 13.5,
  textDecoration: "none",
};

export const skipHint: CSSProperties = {
  fontSize: 11.5,
  color: "var(--practice-muted)",
  fontStyle: "italic",
  margin: "10px 0 0",
};

export const endTitle: CSSProperties = { fontSize: 22, margin: "0 0 8px", color: "var(--practice-ink)" };
export const endCount: CSSProperties = { color: "var(--practice-muted)", margin: "0 0 22px" };

/** The empty case: this side has nothing answered, and the line says why. */
export const none: CSSProperties = {
  color: "var(--practice-muted)",
  fontSize: 15,
  maxWidth: 520,
  margin: "0 auto",
};
