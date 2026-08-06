// =============================================================================
// RehearsalSection — one foldable block, in the signed mockup's form
// =============================================================================
//
// A bordered card with its heading, its honest count, and a caret. Rebuilt in
// task 2.11 C to reproduce `.section` / `.sec-head` / `.sec-body` from
// REHEARSAL_PAGE_MOCKUP_v2_2026-08-06.html exactly; B2's version was a
// borderless top-rule.
//
// ## The count is rendered whether the section is open or shut
//
// That is not a style choice — it is the engineering answer to the known hazard
// of collapsible sections, which is that content behind a fold gets missed. A
// folded accusation block still reads "said 4 times · 2 gaps", so nothing this
// page knows about can hide behind a caret. The count arrives composed; this
// component never counts anything.
//
// ## Why the gap count is coloured separately
//
// The mockup's `.gapct` puts the gap number in the danger colour inside an
// otherwise muted line. The backend sends ONE composed sentence, so the split is
// made here by finding the count's own text — see `GapCount` below for why that
// is done by matching a served number rather than by parsing prose.

import React from "react";

import {
  caretStyle,
  sectionBodyStyle,
  sectionCountStyle,
  sectionHeadStyle,
  sectionStyle,
  sectionTitleStyle,
} from "./rehearsalStyles";

interface Props {
  /** The block's stored heading. */
  heading: string;
  /**
   * The block's honest count line, ALWAYS rendered — this is the hazard answer.
   * Composed server-side; `null` only for a block that genuinely has no count.
   */
  count: string | null;
  /**
   * How many gaps this block's count line reports, when it reports any.
   *
   * Sent as its own number by the payload rather than parsed out of the
   * sentence: the sentence is Roman's to reword, and a component reading "2" out
   * of "said 4 times · 2 gaps" would silently stop colouring the day he changed
   * the separator. `0` renders the whole line muted.
   */
  gapCount?: number;
  open: boolean;
  onToggle: () => void;
  children: React.ReactNode;
}

/**
 * The count line, with its gap number in the danger colour.
 *
 * The split is positional and deliberately conservative: the LAST occurrence of
 * the gap number is coloured, and only when that number is non-zero and actually
 * appears. Anything else renders the sentence whole and muted — a colour that
 * cannot be placed honestly is better dropped than guessed at.
 */
const CountLine: React.FC<{ count: string; gapCount: number }> = ({ count, gapCount }) => {
  const marker = String(gapCount);
  const at = gapCount > 0 ? count.lastIndexOf(marker) : -1;

  if (at < 0) {
    return <span style={sectionCountStyle}>{count}</span>;
  }

  return (
    <span style={sectionCountStyle}>
      {count.slice(0, at)}
      <span style={{ color: "var(--v3-red-text)", fontWeight: 600 }}>
        {count.slice(at)}
      </span>
    </span>
  );
};

const RehearsalSection: React.FC<Props> = ({
  heading,
  count,
  gapCount = 0,
  open,
  onToggle,
  children,
}) => (
  <section style={sectionStyle}>
    {/* A real <button>, so the section opens from the keyboard and announces its
        state — `aria-expanded` is what a screen reader reads to say "collapsed",
        and a <div> with an onClick would say nothing at all. */}
    <button type="button" style={sectionHeadStyle} onClick={onToggle} aria-expanded={open}>
      <span style={caretStyle}>{open ? "▼" : "▶"}</span>
      <span style={sectionTitleStyle}>{heading}</span>
      {/* The honest count, folded or open. Never conditional on `open` — that
          conditional IS the hazard this component was written to remove. */}
      {count && <CountLine count={count} gapCount={gapCount} />}
    </button>
    {open && <div style={sectionBodyStyle}>{children}</div>}
  </section>
);

export default RehearsalSection;
