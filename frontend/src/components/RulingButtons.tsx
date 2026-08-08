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
// ## One state machine, two input devices — and, since 1.7G, two TARGETS
//
// A click still reaches the same §7 semantics the keyboard does, so auto-advance,
// the defer prompt, the `defer_required` short-circuit and single-step undo are
// inherited rather than reimplemented. What changed in 1.7G is what a click aims
// at: it dispatches `{type: "rule"}` carrying the id of the card the button is
// printed on, instead of `{type: "key"}` aimed at whatever was selected.
//
// That distinction was the whole beta.369 defect. Until 1.7G every button in a
// 148-card list resolved its target through one shared index, so which card got
// included depended on click order — and only the selected card had buttons at
// all. Roman's ruling R1: "Every card's Include / Exclude / Defer buttons act on
// the card they are printed on. Period."

import React from "react";

import type { RulingKey } from "./cardTriage";
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
 * ## Why the type moved to `cardTriage` in 1.7G
 *
 * The reducer's own `{type: "rule"}` event now carries one of these, and a pure
 * state machine that imports its vocabulary from a React component is one that
 * cannot be tested without a DOM. So the reducer owns the type and this file
 * re-exports it — every existing import site keeps working, and there is still
 * exactly one definition of what a ruling is.
 */
export type { RulingKey };

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
 * ## Every card in the list has one of these (1.7G, ruling R1)
 *
 * Until 1.7G the head rendered its buttons only on the SELECTED card, so 147 of
 * 148 rows were inert and a human had to click a card before they could rule it.
 * `onRule` is now supplied by every card's own render scope, already bound to that
 * card's id — so the button cannot aim anywhere else, and the acceptance test
 * (click the LAST card's Include first, with nothing selected) works by
 * construction rather than by the selection happening to be in the right place.
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
  /**
   * Suppress the sentence BELOW the buttons, without disarming them.
   *
   * ## Roman's ruling, 2026-08-04 (task 2.10, Q4): one sentence on screen
   *
   * A stuck card now carries the link panel three inches lower, and that panel
   * opens with its own explanation of the same problem. Printing both would say
   * the same thing twice on one card — the clutter ruling, again.
   *
   * This is a deliberate change to 1.7E-a's behaviour, which made the button-row
   * sentence "the ONLY place that sentence appears". It still is, for every card
   * with no panel: a card with no verbatim QUOTE is defer-only and linking cannot
   * help it, so nothing is offered there and the sentence stays.
   *
   * The reason itself is NOT dropped — it remains the disabled buttons' `title`
   * and the keyboard refusal's spoken text, so I and E still explain themselves.
   */
  reasonShownElsewhere?: boolean;
  /**
   * The stored sentence for "you have unsaved link choices" (task 2.12, item B).
   *
   * ## Why this OVERRIDES the backend's defer reason rather than joining it
   *
   * Both explain the same greyed buttons, and printing two sentences about one
   * refusal is the clutter Roman has ruled against repeatedly. This one is also
   * the more useful of the two at that moment: the human has already read the
   * panel's explanation and acted on it, so what they need now is "you are one
   * click short", not "this card is not linked" — which they can see is no
   * longer the whole truth.
   *
   * `null` whenever the panel holds nothing unsaved, which puts the ordinary
   * Q4 behaviour back.
   */
  unsavedLinkReason?: string | null;
  /** True when the human just pressed I or E and was refused. */
  keyboardRefused: boolean;
  /**
   * The stored label that introduces the standing condition (D3a, 2026-08-08).
   *
   * `null` until the wording loads, which suppresses the whole line rather than
   * inventing one — the absent-not-fake law.
   */
  lockedConditionLabel?: string | null;
  onRule: (key: RulingKey) => void;
}> = ({
  code,
  chip,
  chipTitle,
  deferOnly,
  deferOnlyReason,
  keyboardRefused,
  reasonShownElsewhere = false,
  unsavedLinkReason = null,
  lockedConditionLabel = null,
  onRule,
}) => (
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
            // The reason travels with the control it disables. While a draft is
            // unsaved that reason is the draft's, not the payload's.
            title={shut ? (unsavedLinkReason ?? deferOnlyReason ?? undefined) : undefined}
            // `stopPropagation` is load-bearing, not hygiene. The button sits
            // inside a card whose own `onClick` selects that card, so without it
            // ONE physical click produced TWO dispatches: the ruling, and then a
            // select on the card just ruled — which overwrote the auto-advance and
            // left the human wherever the filter rescue happened to put them (the
            // top of the list). Ruling on a card is not also a request to aim the
            // keyboard at it; the ruling itself decides where the highlight goes.
            onClick={(event) => {
              event.stopPropagation();
              onRule(button.key);
            }}
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

    {/* D3a (Roman's session finding, 2026-08-08): a locked card states its
        condition ON ITS FACE.
        
        This overrides Q4's suppression, and deliberately. Q4 removed the sentence
        whenever the link panel below was explaining the same problem — one
        sentence on screen, which was right about clutter and wrong about
        discovery: the condition was then reachable only by hovering the disabled
        buttons, and a condition most humans never find is also the PROMISE that
        Defer will work on this card. The stored label carries it now, so the
        sentence has an owner on the face and the panel's own copy still explains
        what to DO about it. */}
    {deferOnly && lockedConditionLabel && !unsavedLinkReason && !keyboardRefused && (
      <div
        style={{
          marginTop: "8px",
          fontSize: "12.5px",
          lineHeight: 1.5,
          color: "var(--text-secondary)",
        }}
      >
        <b style={{ fontWeight: 600 }}>{lockedConditionLabel}</b> {deferOnlyReason}
      </div>
    )}

    {/* The louder variants: an unsaved draft, or a key the human just pressed and
        was refused. Both are events rather than standing information, so they
        keep the tinted treatment and the alert role. */}
    {deferOnly &&
      (unsavedLinkReason ?? (keyboardRefused ? deferOnlyReason : null)) && (
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
          // Tinted for the two cases a human needs to NOTICE: a key they just
          // pressed being refused, and choices they have made but not saved.
          // The standing defer sentence stays quiet.
          background:
            keyboardRefused || unsavedLinkReason
              ? "var(--state-warning-bg-soft)"
              : "transparent",
          color:
            keyboardRefused || unsavedLinkReason
              ? "var(--v3-amber-text)"
              : "var(--text-muted)",
        }}
      >
        {unsavedLinkReason ?? deferOnlyReason}
      </div>
    )}
  </div>
);
