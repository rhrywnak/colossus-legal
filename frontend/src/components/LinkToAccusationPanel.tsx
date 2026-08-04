// =============================================================================
// LinkToAccusationPanel — the control on a stuck card (task 2.10)
// =============================================================================
//
// Under a defer-only card's quote, in place of a dead end: tick the accusations
// this statement bears on, say which way it cuts, save. That is the whole of what
// makes 94 of S-2's 148 cards rulable (measured on DEV, 2026-08-04).
//
// ## Checkboxes, not a dropdown, and a SHORT list
//
// The design's research, applied literally: a checkbox is one click and a
// dropdown is two; one statement can bear on several accusations (the lens
// model); and limiting the options produces a review that is both faster and more
// accurate. So the list shows what this scenario already serves, and the full
// complaint sits behind "Show all" with a filter box.
//
// ## Every card has its own panel (ruling R1, binding)
//
// The same law as 1.7G's ruling buttons. `onSave` and `onUnlink` are bound by the
// list to the card they are printed on, so this component never learns which card
// it is about and cannot reach another one. No selection is required first, and
// "Save and next" is a convenience rather than the only path.
//
// ## The list scrolls; the buttons do not move (Roman's ruling)
//
// The accusations sit in a fixed-height scroll region. A 120-paragraph complaint
// must never push the cut control or the Save row off the screen.
//
// ## Not one word here is written in this file (R4)
//
// Every string comes from `LinkPanelWording`, which the backend reads out of
// `app_settings`. Change a label on the Settings page and this panel changes with
// no rebuild. The only vocabulary this file owns is its own glyphs, which is the
// same class the state chip's ✓ and ✕ belong to — and they never stand alone.

import React, { useState } from "react";

import {
  canSave,
  EMPTY_DRAFT,
  refusalFor,
  toggleAllegation,
  visibleOptions,
  type LinkDraft,
} from "./cardLinking";
import { fillDetail } from "../services/evidenceLinks";
import type { AllegationOptions, LinkCut } from "../services/evidenceLinks";

const panelStyle: React.CSSProperties = {
  background: "var(--v3-context-panel)",
  borderRadius: "10px",
  padding: "14px 18px",
  display: "flex",
  flexDirection: "column",
  gap: "10px",
};

const headingStyle: React.CSSProperties = {
  fontSize: "11px",
  fontWeight: 600,
  letterSpacing: "0.05em",
  textTransform: "uppercase",
  color: "var(--text-muted)",
};

const noticeStyle: React.CSSProperties = {
  fontSize: "12.5px",
  lineHeight: 1.5,
  color: "var(--text-secondary)",
};

/**
 * The accusation list's scroll region — Roman's ruling, as a style.
 *
 * A fixed maximum height with its own scrollbar. The cut control and the Save row
 * are siblings BELOW it, so they stay on screen while this scrolls beneath them.
 */
const listStyle: React.CSSProperties = {
  maxHeight: "190px",
  overflowY: "auto",
  display: "flex",
  flexDirection: "column",
  gap: "2px",
  paddingRight: "4px",
};

const optionStyle: React.CSSProperties = {
  display: "flex",
  gap: "8px",
  alignItems: "baseline",
  fontSize: "13px",
  lineHeight: 1.5,
  color: "var(--text-primary)",
  cursor: "pointer",
  padding: "3px 4px",
  borderRadius: "6px",
};

const countChipStyle: React.CSSProperties = {
  fontSize: "11.5px",
  color: "var(--text-muted)",
  whiteSpace: "nowrap",
};

const actionStyle: React.CSSProperties = {
  fontFamily: "inherit",
  fontSize: "12.5px",
  fontWeight: 600,
  border: "none",
  borderRadius: "8px",
  padding: "6px 14px",
  cursor: "pointer",
};

const quietActionStyle: React.CSSProperties = {
  ...actionStyle,
  fontWeight: 500,
  background: "transparent",
  color: "var(--text-secondary)",
};

const filterStyle: React.CSSProperties = {
  fontFamily: "inherit",
  fontSize: "12.5px",
  fontWeight: 400,
  border: "1px solid var(--border-default)",
  borderRadius: "6px",
  padding: "5px 8px",
};

/** A cut button: filled when chosen, quiet when not. Colour never stands alone. */
const cutButtonStyle = (chosen: boolean, danger: boolean): React.CSSProperties => ({
  ...actionStyle,
  background: chosen
    ? danger
      ? "var(--state-danger-strong)"
      : "var(--state-success-strong)"
    : "var(--v3-chrome)",
  color: chosen ? "var(--v3-on-fill)" : "var(--text-secondary)",
});

/**
 * The link control for ONE card.
 *
 * `onSave` and `onUnlink` are already bound to this card by the list — see the
 * header. The panel holds only the DRAFT: what has been ticked and not yet saved.
 * Nothing about the card's own state lives here, so two panels on screen cannot
 * interfere with each other.
 */
export const LinkToAccusationPanel: React.FC<{
  options: AllegationOptions;
  /** Already bound to THIS card by the list. */
  onSave: (allegationIds: string[], cut: LinkCut) => Promise<void>;
  /**
   * Told when the draft starts or stops holding unsaved choices (item B).
   *
   * ## Why the card is told rather than asked
   *
   * The reason Include is greyed has to appear on the BUTTON ROW, three inches
   * above this panel, and the draft lives here. Lifting the whole draft up to
   * `CandidateCard` would put the panel's internals in a component whose job is
   * to walk the §7 descriptor; reporting one boolean up keeps the draft where it
   * is used and tells the card exactly what it needs to know.
   */
  onDraftDirty: (dirty: boolean) => void;
}> = ({ options, onSave, onDraftDirty }) => {
  const [draft, setDraft] = useState<LinkDraft>(EMPTY_DRAFT);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const { wording } = options;
  const shown = visibleOptions(draft, options.serving, options.others);

  /**
   * Change the draft and tell the card whether anything is now unsaved.
   *
   * "Unsaved choices" is a tick OR a cut — either alone is enough to make a
   * human expect Include to work, which is exactly the moment they need telling
   * why it does not. Opening "Show all" or typing in the filter is not a choice
   * and does not count.
   */
  const change = (next: LinkDraft) => {
    setDraft(next);
    onDraftDirty(next.allegationIds.length > 0 || next.cut !== null);
  };

  /** Save, keeping the draft and the panel open until the SERVER has answered. */
  const save = () => {
    const refusal = refusalFor(draft, wording);
    if (refusal !== null) {
      // Refused here rather than by a round trip that returns 400 — and in the
      // backend's own stored words, so the two refusals cannot drift.
      setError(refusal);
      return;
    }
    if (draft.cut === null) return;

    setBusy(true);
    setError(null);
    onSave(draft.allegationIds, draft.cut)
      .then(() => {
        setBusy(false);
        // The choices are saved, so they are no longer unsaved — and Include is
        // about to unlock for real, on the card the human is still looking at.
        change(EMPTY_DRAFT);
      })
      .catch((e: unknown) => {
        setBusy(false);
        // The refusal stays ON the panel, beside the ticks it refused, and the
        // draft is not discarded — a human who has just worked through 120
        // accusations must not lose that to a failed save (Standing Rule 1).
        //
        // The words are the stored ones (R4); only the failure's own text is
        // dropped into the slot the sentence leaves for it.
        setError(fillDetail(wording.save_failed_template, e instanceof Error ? e.message : String(e)));
      });
  };

  return (
    <div
      style={panelStyle}
      // The panel lives inside a card whose click selects it. Ticking a box is not
      // a request to aim the keyboard at this card.
      onClick={(event) => event.stopPropagation()}
    >
      <div style={noticeStyle}>{wording.intro}</div>

      <div style={{ display: "flex", alignItems: "baseline", gap: "10px", flexWrap: "wrap" }}>
        <span style={headingStyle}>{wording.allegations_heading}</span>
        {!draft.showAll && options.others.length > 0 && (
          <button
            type="button"
            style={{ ...quietActionStyle, marginLeft: "auto", color: "var(--accent-primary)" }}
            onClick={() => setDraft({ ...draft, showAll: true })}
          >
            {wording.show_all_label}
          </button>
        )}
      </div>

      {draft.showAll && (
        <input
          value={draft.filter}
          onChange={(event) => setDraft({ ...draft, filter: event.target.value })}
          placeholder={wording.filter_placeholder}
          aria-label={wording.filter_placeholder}
          style={filterStyle}
          disabled={busy}
        />
      )}

      {options.total === 0 ? (
        // A case with no accusations at all is a different state from a filter
        // that matched nothing, and gets its own sentence (Standing Rule 1).
        <div style={noticeStyle}>{wording.empty_options_notice}</div>
      ) : shown.length === 0 ? (
        <div style={noticeStyle}>{wording.no_match_notice}</div>
      ) : (
        <div style={listStyle}>
          {shown.map((option) => (
            <label key={option.allegation_id} style={optionStyle}>
              <input
                type="checkbox"
                checked={draft.allegationIds.includes(option.allegation_id)}
                onChange={() => change(toggleAllegation(draft, option.allegation_id))}
                disabled={busy}
              />
              <span style={{ flex: 1 }}>{option.label}</span>
              {option.count_label && <span style={countChipStyle}>{option.count_label}</span>}
            </label>
          ))}
        </div>
      )}

      <span style={headingStyle}>{wording.cut_heading}</span>
      <div style={{ display: "flex", gap: "8px", flexWrap: "wrap" }}>
        <button
          type="button"
          style={cutButtonStyle(draft.cut === "supports", false)}
          aria-pressed={draft.cut === "supports"}
          disabled={busy}
          onClick={() => change({ ...draft, cut: "supports" })}
        >
          {wording.cut_supports_label}
        </button>
        <button
          type="button"
          style={cutButtonStyle(draft.cut === "against", true)}
          aria-pressed={draft.cut === "against"}
          disabled={busy}
          onClick={() => change({ ...draft, cut: "against" })}
        >
          {wording.cut_against_label}
        </button>
      </div>

      {/* The case-wide consequence, said BEFORE anyone commits to it (Q2). */}
      <div style={{ ...noticeStyle, color: "var(--text-muted)", fontSize: "12px" }}>
        {wording.scope_notice}
      </div>

      {error && (
        <div role="alert" style={{ ...noticeStyle, color: "var(--state-danger-strong)" }}>
          {error}
        </div>
      )}

      <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
        <button
          type="button"
          style={{
            ...actionStyle,
            background: "var(--accent-primary)",
            color: "var(--v3-on-fill)",
            // Disabled by the draft's own rule, so the refusal sentence is a
            // backstop for the keyboard path rather than the only guard.
            opacity: canSave(draft) && !busy ? 1 : 0.5,
            cursor: canSave(draft) && !busy ? "pointer" : "not-allowed",
          }}
          disabled={!canSave(draft) || busy}
          onClick={() => save()}
        >
          {busy ? "…" : wording.save_label}
        </button>
        <button
          type="button"
          style={quietActionStyle}
          disabled={busy}
          onClick={() => {
            change(EMPTY_DRAFT);
            setError(null);
          }}
        >
          {wording.cancel_label}
        </button>
      </div>
    </div>
  );
};

export default LinkToAccusationPanel;
