// =============================================================================
// ScanConfirmBar — the question asked before 313 metered calls
// =============================================================================
//
// Mockup: a bar directly under the header card, carrying an accent stripe down
// its left edge, the sentence, and two buttons.
//
// ## Why the accent is a STRIPE and not a border
//
// Roman's standing rule (2026-08-31, fenced by `accentHairline.test.ts`):
// `--accent-primary` is INK or FILL, never a 1-px hairline. The stripe is a
// 3-px filled bar — the same construction the rule's own note exempts — and the
// bar's four sides take `--border-default` like every other hairline on the
// page.
//
// ## Why this is its own component
//
// `ScenarioFactsHeader` renders a row; this renders a block beneath it, with
// its own words, its own two buttons and the one call site in the application
// wired to starting a scan. Keeping them apart is what lets the header stay
// legible at a glance — and keeps both files under the 300-line limit.

import React from "react";

import { flattenConfirmSentence } from "./scanConfirmModel";
import type { ConfirmSentence } from "./scanConfirmModel";

/** The bar. Hairline box, accent stripe, card radius. */
const barStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  flexWrap: "wrap",
  gap: "12px",
  padding: "12px 16px",
  // No margin of its own: the header row above carries `sectionHeaderStyle`'s
  // bottom gap, and a second margin here would stack two spacings into one.
  border: "1px solid var(--border-default)",
  borderLeft: "3px solid var(--accent-primary)",
  borderRadius: "var(--radius-card)",
  background: "var(--bg-surface)",
};

const sentenceStyle: React.CSSProperties = {
  fontSize: "13.5px",
  color: "var(--text-primary)",
  flex: 1,
  minWidth: "240px",
};

/** The affirmative. Accent FILL with the on-fill foreground — a token pair. */
const runStyle: React.CSSProperties = {
  fontFamily: "inherit",
  fontSize: "13px",
  fontWeight: 600,
  padding: "7px 16px",
  borderRadius: "999px",
  border: "1px solid var(--accent-primary)",
  background: "var(--accent-primary)",
  color: "var(--v3-on-fill)",
  cursor: "pointer",
  flexShrink: 0,
};

/** The retreat. Same shape, no fill — it must be as easy to hit, not as loud. */
const cancelStyle: React.CSSProperties = {
  fontFamily: "inherit",
  fontSize: "13px",
  padding: "7px 16px",
  borderRadius: "999px",
  border: "1px solid var(--border-default)",
  background: "var(--bg-surface)",
  color: "var(--text-primary)",
  cursor: "pointer",
  flexShrink: 0,
};

interface Props {
  /** The composed question, split at its own `{model}` slot. Built by
   *  `confirmSentence`, never here. */
  sentence: ConfirmSentence;
  runLabel: string;
  cancelLabel: string;
  /** Starts the scan. The only control in the application that does. */
  onRun: () => void;
  onCancel: () => void;
}

/**
 * The confirmation bar.
 *
 * `role="alertdialog"` with `aria-label` on the sentence: this interrupts to ask
 * a question with consequences, and a screen reader reaching the header should
 * hear the question rather than discover two unexplained buttons. It is not a
 * modal — nothing is trapped and Cancel is not the only way out — because the
 * page behind it is where a human checks the history before answering.
 *
 * `stopPropagation` on the wrapper: the header row above is a collapse target,
 * and a click that answered the question and folded the section would leave the
 * human looking at a scan they could no longer see.
 */
const ScanConfirmBar: React.FC<Props> = ({
  sentence,
  runLabel,
  cancelLabel,
  onRun,
  onCancel,
}) => (
  <div
    style={barStyle}
    role="alertdialog"
    // The flat form, because an accessible name cannot carry markup. Composed
    // from the same parts as the visible copy, so the two cannot come apart.
    aria-label={flattenConfirmSentence(sentence)}
    onClick={(e) => e.stopPropagation()}
  >
    <span style={sentenceStyle}>
      {sentence.before}
      {/* The mockup sets the model's name in bold — it is the one word a human
          has to check before answering, and the only part of the question that
          decides what the click costs. */}
      <strong>{sentence.model}</strong>
      {sentence.after}
    </span>
    <button type="button" style={runStyle} onClick={onRun}>
      {runLabel}
    </button>
    <button type="button" style={cancelStyle} onClick={onCancel}>
      {cancelLabel}
    </button>
  </div>
);

export default ScanConfirmBar;
