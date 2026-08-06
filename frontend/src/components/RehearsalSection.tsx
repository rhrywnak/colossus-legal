// =============================================================================
// RehearsalSection — one collapsible block (task 2.11 B2, addendum §2)
// =============================================================================
//
// Witness-prep doctrine is short sessions on one topic at a time, so the page
// folds. That makes the collapse layer doctrinally right rather than cosmetic —
// but accordions have a known hazard, and it is the one thing this component
// exists to engineer against: **content behind a fold gets missed.**
//
// ## The answer to that hazard, and it is a law not a nicety
//
// The header carries its own honest count, and it is visible OPEN OR FOLDED.
// "said 2 times · 3 gaps" stays on screen with the section shut. That is the
// honest-gap law's form of the accordion rule: a collapsed section never hides a
// gap count. The count is composed by the backend and arrives finished; this
// component renders it and computes nothing.
//
// ## What the Always card does not get
//
// This component. §10 makes the standing card the one thing never scrolled away
// from, and the addendum makes it the one section that never collapses — so it
// is rendered directly by the page, with no fold to close.
//
// ## Collapse state is per-visit, never stored
//
// Folding a section says nothing about the case. It is where this reader is in
// this session, and storing it would make one person's session decide another's.

import React from "react";

import { DIVIDER } from "./scenarioSectionStyles";

interface Props {
  /** The block's stored heading. */
  heading: string;
  /**
   * The block's honest count line, ALWAYS rendered — this is the hazard answer.
   * Composed server-side; `null` only for a block that genuinely has no count.
   */
  count: string | null;
  open: boolean;
  onToggle: () => void;
  children: React.ReactNode;
}

const headerStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "baseline",
  gap: "0.75rem",
  width: "100%",
  padding: "0.9rem 0",
  border: "none",
  borderTop: DIVIDER,
  background: "transparent",
  cursor: "pointer",
  fontFamily: "inherit",
  textAlign: "left",
};

const headingStyle: React.CSSProperties = {
  fontSize: "0.8rem",
  letterSpacing: "0.06em",
  textTransform: "uppercase",
  color: "var(--text-muted)",
};

/** The caret. A signifier, per the accordion rules — never colour alone. */
const caretStyle: React.CSSProperties = {
  fontSize: "0.7rem",
  color: "var(--text-muted)",
  width: "0.8rem",
  display: "inline-block",
};

const RehearsalSection: React.FC<Props> = ({
  heading,
  count,
  open,
  onToggle,
  children,
}) => (
  <section>
    {/* A real <button>, so the section opens from the keyboard and announces its
        state — `aria-expanded` is what a screen reader reads to say "collapsed",
        and a <div> with an onClick would say nothing at all. */}
    <button type="button" style={headerStyle} onClick={onToggle} aria-expanded={open}>
      <span style={caretStyle}>{open ? "▾" : "▸"}</span>
      <span style={headingStyle}>{heading}</span>
      {/* The honest count, folded or open. Never conditional on `open` — that
          conditional IS the hazard this component was written to remove. */}
      {count && (
        <span style={{ fontSize: "0.85rem", color: "var(--text-secondary)" }}>{count}</span>
      )}
    </button>
    {open && <div style={{ paddingBottom: "0.5rem" }}>{children}</div>}
  </section>
);

export default RehearsalSection;
