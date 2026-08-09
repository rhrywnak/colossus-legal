// =============================================================================
// AllegationTypeahead — linking by typing, not by scrolling (Pieces 4a, 4b)
// =============================================================================
//
// ONE_CARD_GRAMMAR §3.4. The checkbox wall dies: 120 accusations in a scroll
// region, on a card that could not be ruled until one of them was ticked, with a
// Save the human had to press before the ruling buttons woke up. Roman worked
// that surface on a real 148-candidate pool and it is the reason two thirds of
// the queue sat untouched.
//
// The replacement is the Casefleet @mention pattern (§2c): type "A-45", or a
// word from any allegation, pick from what matches. Which is what the `A-`
// prefix is FOR — see `domain::scenario_code::allegation_code`. Nobody can type
// the pilcrow it replaced, so the old handle could not have driven this control
// at all.
//
// ## The cut is still required, and still asked for
//
// A link with no cut cannot tell ammunition from hazard, the column is NOT NULL,
// and the write path refuses one. So picking an allegation reveals the two cut
// buttons rather than saving immediately — a second click, not a second screen.
// Skipping it would mean either inventing a cut (a claim the human never made)
// or a 400 they could not have predicted.
//
// ## Every word here is stored
//
// There is no user-facing literal in this file. The refusals are the SAME stored
// sentences the backend returns, so this control's pre-check and the server's
// rejection cannot tell a person two different things about one mistake (R4).

import React, { useMemo, useState } from "react";

import type {
  AllegationOption,
  AllegationOptions,
  LinkCut,
} from "../services/evidenceLinks";

/** How many matches the list offers at once. */
//
// Presentational, and therefore NOT a stored tunable (the standing R5 line): it
// chooses how many of the SAME matches are visible before scrolling, never which
// accusations exist or which ones match. The list is ranked, so the cut-off
// falls on the least relevant.
const VISIBLE_MATCHES = 6;

const inputStyle: React.CSSProperties = {
  font: "inherit",
  fontSize: "13px",
  width: "100%",
  padding: "7px 10px",
  border: "1px solid var(--border-default)",
  borderRadius: "8px",
  fontFamily: "inherit",
};

const listStyle: React.CSSProperties = {
  position: "absolute",
  top: "38px",
  left: 0,
  right: 0,
  background: "var(--bg-surface)",
  border: "1px solid var(--border-default)",
  borderRadius: "8px",
  boxShadow: "var(--shadow-raised)",
  zIndex: 40,
  maxHeight: "200px",
  overflowY: "auto",
};

const itemStyle: React.CSSProperties = {
  display: "block",
  width: "100%",
  textAlign: "left",
  padding: "7px 10px",
  fontSize: "12.5px",
  border: "none",
  borderBottom: "1px solid var(--border-card)",
  background: "none",
  cursor: "pointer",
  fontFamily: "inherit",
  color: "var(--text-primary)",
};

const cutButtonStyle: React.CSSProperties = {
  fontFamily: "inherit",
  fontSize: "12.5px",
  padding: "5px 12px",
  borderRadius: "8px",
  border: "1px solid var(--border-default)",
  background: "var(--bg-surface)",
  cursor: "pointer",
};

/**
 * Rank the matches: the accusations this scenario already serves come first.
 *
 * The payload keeps `serving` and `others` apart and says why (the short list is
 * "what this scenario is about"). Concatenating rather than merging preserves
 * that ranking through the filter — a human typing "divide" should meet their own
 * scenario's anchor before the 90th paragraph of the complaint.
 *
 * Matched against `filter_text`, which the BACKEND lowercased and composed from
 * the label and the count. Searching anything else here would let a hit come from
 * a field the human cannot see.
 */
export function matchAllegations(
  options: AllegationOptions,
  term: string,
  limit: number,
): AllegationOption[] {
  const needle = term.trim().toLowerCase();
  const all = [...options.serving, ...options.others];
  if (!needle) return all.slice(0, limit);
  return all.filter((o) => o.filter_text.includes(needle)).slice(0, limit);
}

/**
 * The link control on one card.
 *
 * @param options the accusations and every word this control speaks
 * @param onSave  commits the link. Already bound to THIS card by the list, the
 *                same discipline `onRule` follows — a control cannot reach
 *                another card because it never learns another card's id.
 */
export const AllegationTypeahead: React.FC<{
  options: AllegationOptions;
  onSave: (allegationIds: string[], cut: LinkCut) => Promise<void>;
  /** Raised while a pick is uncommitted, so the wrapper's greyed buttons say why. */
  onDraftDirty?: (dirty: boolean) => void;
  /**
   * Raised when a link has LANDED (the .386 law: every action acknowledges
   * itself).
   *
   * The sentence renders on the card rather than here, because the control is
   * about to unmount: linking a locked card clears `defer_required`, the queue
   * re-reads, and this panel stops being offered. An acknowledgment that
   * disappeared with the control that produced it would be the silent-defer
   * defect in a new costume.
   */
  onLinked?: () => void;
}> = ({ options, onSave, onDraftDirty, onLinked }) => {
  const [term, setTerm] = useState("");
  const [open, setOpen] = useState(false);
  const [picked, setPicked] = useState<AllegationOption | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const matches = useMemo(
    () => matchAllegations(options, term, VISIBLE_MATCHES),
    [options, term],
  );

  const pick = (option: AllegationOption) => {
    setPicked(option);
    setTerm(option.label);
    setOpen(false);
    setError(null);
    onDraftDirty?.(true);
  };

  const commit = (cut: LinkCut) => {
    // The browser's pre-check speaks the SERVER's sentence, so one mistake has
    // one wording wherever it is caught.
    if (!picked) {
      setError(options.wording.missing_allegation_refusal);
      return;
    }
    setSaving(true);
    onSave([picked.allegation_id], cut)
      .then(() => {
        // The card is re-read from the server on success, so this control
        // unmounts. Clearing the draft flag first means a wrapper that survives
        // the re-read does not keep its buttons greyed over a committed link.
        onDraftDirty?.(false);
        setError(null);
        onLinked?.();
      })
      .catch((e: unknown) => {
        const detail = e instanceof Error ? e.message : String(e);
        setError(options.wording.save_failed_template.replace("{detail}", detail));
      })
      .finally(() => setSaving(false));
  };

  return (
    <div
      // A click inside the control is not a request to aim the keyboard at the
      // card behind it — and, more sharply, typing "i" into this box must never
      // reach the queue's one-key Include.
      onClick={(event) => event.stopPropagation()}
    >
      <div style={{ position: "relative", maxWidth: "460px" }}>
        <input
          value={term}
          placeholder={options.card_grammar.link_typeahead_placeholder}
          aria-label={options.card_grammar.link_typeahead_placeholder}
          onChange={(event) => {
            setTerm(event.target.value);
            setOpen(true);
            // Typing after a pick abandons it: the box now shows something the
            // human is still choosing, and a stale pick would let the cut buttons
            // commit an accusation that is no longer on screen.
            if (picked) {
              setPicked(null);
              onDraftDirty?.(false);
            }
          }}
          onFocus={() => setOpen(true)}
          style={inputStyle}
        />
        {open && (
          <div style={listStyle}>
            {matches.length === 0 ? (
              <div style={{ ...itemStyle, cursor: "default", color: "var(--text-muted)" }}>
                {options.card_grammar.link_typeahead_no_match}
              </div>
            ) : (
              matches.map((option) => (
                <button
                  key={option.allegation_id}
                  type="button"
                  style={itemStyle}
                  onClick={() => pick(option)}
                >
                  {option.label}
                  {option.count_label && (
                    <span style={{ color: "var(--text-muted)" }}> · {option.count_label}</span>
                  )}
                </button>
              ))
            )}
          </div>
        )}
      </div>

      {/* The cut, asked for only once there is something to cut. Two named
          buttons rather than a radio pair and a Save: the choice IS the commit,
          so there is no state left for a human to forget to press. */}
      {picked && (
        <div
          style={{
            display: "flex",
            gap: "0.5rem",
            alignItems: "center",
            flexWrap: "wrap",
            marginTop: "0.5rem",
          }}
        >
          <span style={{ fontSize: "12.5px", color: "var(--text-secondary)" }}>
            {options.wording.cut_heading}
          </span>
          <button
            type="button"
            disabled={saving}
            style={cutButtonStyle}
            onClick={() => commit("supports")}
          >
            {options.wording.cut_supports_label}
          </button>
          <button
            type="button"
            disabled={saving}
            style={cutButtonStyle}
            onClick={() => commit("against")}
          >
            {options.wording.cut_against_label}
          </button>
        </div>
      )}

      {error && (
        <div
          role="alert"
          style={{
            marginTop: "0.4rem",
            fontSize: "12.5px",
            color: "var(--state-danger-strong)",
          }}
        >
          {error}
        </div>
      )}
    </div>
  );
};

export default AllegationTypeahead;
