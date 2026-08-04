// =============================================================================
// RulingButtons — the coloured triage controls (task 1.7D, item 2)
// =============================================================================
//
// Roman's 2026-08-03 ruling put the four rulings AT THE TOP of the candidate card,
// beside its number. 1.7C had them in a row beneath the quote, which read as a
// footer: the eye travelled code -> quote -> speaker -> pinpoint -> buttons, and
// the decision arrived last. Beside the candidate number they are part of the
// card's identity — *this is C-95, and these are the four things you can do to it*.
//
// ## Why this is its own file
//
// `CandidateCard` was 162 non-comment lines before 1.7D and 362 after, against a
// 300-line limit (Rule 17). The seam is real rather than arithmetic: this file is
// the CONTROLS (what a human can do), and what remains in `CandidateCard` is the
// EVIDENCE (what they are looking at while they decide).
//
// ## One state machine, two input devices
//
// A click dispatches the SAME `{type: "key"}` event the keyboard produces, so
// auto-advance, the defer prompt, the `defer_required` short-circuit that refuses
// I/E on an unrulable card, and single-step undo are all inherited rather than
// reimplemented. `cardTriage.ts` is untouched by this task and its 31 §7 tests are
// byte-identical. A parallel click path would have been a second machine to keep
// in step, and the one thing worse than no buttons is buttons that disagree with
// the keys.

import React from "react";

import type { StateChip } from "./candidateFilters";
import { chipStyle, stateChipTone } from "./scenarioSectionStyles";

/** Mockup `.rbtn kbd` on a filled button: a translucent white chip. */
const kbdOnFillStyle: React.CSSProperties = {
  fontFamily: "inherit",
  fontSize: "11.5px",
  borderRadius: "5px",
  padding: "0 6px",
  background: "rgba(255,255,255,.25)",
  color: "inherit",
};

/** Mockup `.rbtn.def kbd, .rbtn.undo kbd`: a translucent dark chip. */
const kbdOnSoftStyle: React.CSSProperties = {
  ...kbdOnFillStyle,
  background: "rgba(0,0,0,.06)",
};

/** Mockup `.rbtn`: 13px/600, radius 9, padding 7px 14px, icon + label + kbd. */
const rulingButtonBase: React.CSSProperties = {
  fontFamily: "inherit",
  fontSize: "13px",
  fontWeight: 600,
  padding: "7px 14px",
  borderRadius: "9px",
  border: "none",
  cursor: "pointer",
  display: "inline-flex",
  alignItems: "center",
  gap: "6px",
};

/** Mockup `.ccode`: 14px/700, chrome fill, radius 8, padding 4px 12px. */
export const codeBadgeStyle: React.CSSProperties = {
  fontSize: "14px",
  fontWeight: 700,
  color: "var(--text-primary)",
  background: "var(--v3-chrome)",
  borderRadius: "8px",
  padding: "4px 12px",
};

// ─── The ruling buttons (task 1.7D, item 2) ─────────────────────────────────

/**
 * Which ruling a button performs, as the KEY the reducer already understands.
 *
 * ## Why a key and not a new event type
 *
 * `cardTriage.queueReducer` accepts `{ type: "key"; key; typing }` and nothing
 * else for a ruling. So a click dispatches the SAME event the keyboard produces,
 * through the same reducer, and every downstream behaviour — auto-advance, the
 * defer prompt, the `defer_required` short-circuit that refuses I/E on an
 * unrulable card, single-step undo — is inherited rather than reimplemented.
 *
 * That is the point of item 2's "buttons dispatch the SAME reducer actions as the
 * keys": `cardTriage.ts` is untouched by this task and its 31 §7 tests stay
 * byte-identical. A parallel click path would have been a second state machine to
 * keep in step, and the one thing worse than no buttons is buttons that disagree
 * with the keys.
 */
export type RulingKey = "i" | "e" | "d" | "u";

/** One ruling button: icon, label, key hint. Mockup `.card-top` order. */
const RULING_BUTTONS: {
  key: RulingKey;
  icon: string;
  label: string;
  fill: React.CSSProperties;
  kbd: React.CSSProperties;
}[] = [
  {
    key: "i",
    icon: "✓",
    label: "Include",
    // Mockup `.rbtn.inc`: filled green, white text.
    fill: { background: "var(--state-success-strong)", color: "var(--v3-on-fill)" },
    kbd: kbdOnFillStyle,
  },
  {
    key: "e",
    icon: "✕",
    label: "Exclude",
    // Mockup `.rbtn.exc`: filled red, white text.
    fill: { background: "var(--state-danger-strong)", color: "var(--v3-on-fill)" },
    kbd: kbdOnFillStyle,
  },
  {
    key: "d",
    icon: "⏸",
    label: "Defer",
    // Mockup `.rbtn.def`: SOFT amber with amber text — deliberately quieter than
    // include/exclude. Deferring is not a verdict, and a third saturated fill
    // would give it the same weight as one.
    fill: { background: "var(--state-warning-bg-soft)", color: "var(--v3-amber-text)" },
    kbd: kbdOnSoftStyle,
  },
  {
    key: "u",
    icon: "↩",
    // Mockup renders undo as the glyph and the key only — no word. It is the way
    // back, not a fifth verdict, and labelling it would put it in the same visual
    // class as the three that decide something.
    label: "",
    fill: { background: "var(--v3-chrome)", color: "var(--text-secondary)" },
    kbd: kbdOnSoftStyle,
  },
];

/**
 * The DISABLED look for Include and Exclude on a defer-only card (1.7E, item 5).
 *
 * Greyed and un-clickable, but still legibly the same control — a button that
 * vanished when unavailable would leave the human wondering whether this card is
 * a different kind of card. It is the same card; two of its four doors are shut,
 * and the sentence beside them says why.
 */
const disabledRulingStyle: React.CSSProperties = {
  ...rulingButtonBase,
  background: "var(--v3-chrome)",
  color: "var(--text-muted)",
  cursor: "not-allowed",
};

/** The state chip: icon + word + tone. Colour is never the only signal. */
export const StateChipView: React.FC<{ chip: StateChip; title?: string }> = ({ chip, title }) => (
  <span style={{ ...chipStyle, ...stateChipTone(chip.tone) }} title={title}>
    <span aria-hidden="true">{chip.icon}</span> {chip.label}
  </span>
);

/**
 * The card head: the C-code, the four ruling buttons, then the state chip.
 *
 * ## Why the buttons are HERE and not at the bottom
 *
 * Roman's ruling, 2026-08-03. 1.7C put them in a row beneath the card, which read
 * as a footer to the quote — so the eye travelled code → quote → speaker →
 * pinpoint → buttons, and the decision arrived last. Beside the candidate number
 * the buttons are part of the card's identity: *this is C-95, and these are the
 * four things you can do to it*.
 *
 * Colour never stands alone: every button carries an icon, and the three that
 * decide something carry a word too.
 *
 * ## The refusal is ON the button row (task 1.7E, items 5 and 8)
 *
 * A defer-only card renders Include and Exclude visibly disabled with the
 * backend's own reason beside them, and that is the ONLY place that sentence
 * appears — 1.7D printed it as a paragraph in the card body, where it competed
 * with the quote for the reader's attention and still left the buttons looking
 * live. Pressing I or E surfaces the SAME sentence with an alert role (the
 * `keyboardRefused` flag), so the key explains itself instead of doing nothing.
 */
export const CardHead: React.FC<{
  code: string;
  chip: StateChip;
  /** The human's own parking reason, shown as the chip's tooltip when present. */
  chipTitle?: string;
  /** True when this card cannot be included or excluded as it stands. */
  deferOnly: boolean;
  /** The backend's sentence explaining why. Rendered verbatim; never composed. */
  deferOnlyReason: string | null;
  /** True when the human just pressed I or E and was refused. */
  keyboardRefused: boolean;
  onRule: (key: RulingKey) => void;
}> = ({ code, chip, chipTitle, deferOnly, deferOnlyReason, keyboardRefused, onRule }) => (
  <div style={{ marginBottom: "14px" }}>
    <div style={{ display: "flex", alignItems: "center", gap: "10px", flexWrap: "wrap" }}>
      <span style={codeBadgeStyle}>{code}</span>

      {RULING_BUTTONS.map((button) => {
        const shut = deferOnly && (button.key === "i" || button.key === "e");
        return (
          <button
            key={button.key}
            type="button"
            disabled={shut}
            style={shut ? disabledRulingStyle : { ...rulingButtonBase, ...button.fill }}
            // The accessible name has to carry what the glyph means — "↩" alone tells a
            // screen reader nothing, and neither does a bare "✓".
            aria-label={
              button.label
                ? `${button.label} (keyboard ${button.key.toUpperCase()})`
                : `Undo the last ruling (keyboard U)`
            }
            // The reason travels with the control it disables, so a hover explains
            // it without the reader hunting for the sentence below.
            title={shut ? (deferOnlyReason ?? undefined) : undefined}
            onClick={() => onRule(button.key)}
          >
            <span aria-hidden="true">{button.icon}</span>
            {button.label && <span>{button.label}</span>}
            <kbd style={shut ? kbdOnSoftStyle : button.kbd} aria-hidden="true">
              {button.key.toUpperCase()}
            </kbd>
          </button>
        );
      })}

      {/* Mockup `.card-top .state`, now a chip: pushed hard right. */}
      <span style={{ marginLeft: "auto" }}>
        <StateChipView chip={chip} title={chipTitle} />
      </span>
    </div>

    {deferOnly && deferOnlyReason && (
      // `role="alert"` only on the keypress, deliberately: the sentence is
      // standing information on this card and a live region that announced itself
      // on every selection would talk over the human browsing the list.
      <div
        role={keyboardRefused ? "alert" : undefined}
        style={{
          marginTop: "8px",
          fontSize: "12.5px",
          lineHeight: 1.5,
          padding: "6px 10px",
          borderRadius: "8px",
          background: keyboardRefused ? "var(--state-warning-bg-soft)" : "transparent",
          color: keyboardRefused ? "var(--v3-amber-text)" : "var(--text-muted)",
        }}
      >
        {deferOnlyReason}
      </div>
    )}
  </div>
);
