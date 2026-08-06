// =============================================================================
// AccusationTextBlock — the accusation in a human's plain words (task 2.11 B1)
// =============================================================================
//
// One sentence, and the three things a human can do to it: write it, change it,
// withdraw it.
//
// ## What this block exists to stop
//
// The shipped rehearsal page reads `definition->>'attack_text'` under the heading
// "What they say". On S-2 that column holds a verbatim first-person quote FROM
// THE RECORD — one thing somebody said once — so the page silently promotes one
// piece of evidence into the summary of all of it. That is beta.371 defect (a),
// and it is fixed at the root: this block writes a different column, and nothing
// ever backfills it from the quote.
//
// ## The gap is shown HERE, not only on the rehearsal page
//
// The honest-gap law says every absence is named. Naming it only where Marie
// reads it would leave the person who can FIX it — Roman, on this page — with a
// blank space and no idea one was expected. So the missing notice renders here
// too, in the store's own words.

import React, { useState } from "react";

import { absentStyle, DIVIDER, ghostButtonStyle } from "./scenarioSectionStyles";
import type { AccusationWordingDto } from "../services/scenarioAccusation";

interface Props {
  /** The stored sentence, or `null` when nobody has written one. */
  text: string | null;
  wording: AccusationWordingDto;
  /** `null` withdraws it. Rejects nothing itself — the backend rules. */
  onSave: (text: string | null) => void;
  /** True while a write is in flight, so a double-click cannot send twice. */
  busy: boolean;
}

const textareaStyle: React.CSSProperties = {
  width: "100%",
  minHeight: "4.5rem",
  fontFamily: "inherit",
  fontSize: "14px",
  lineHeight: 1.55,
  padding: "10px 12px",
  border: DIVIDER,
  borderRadius: "8px",
  color: "var(--text-primary)",
  background: "var(--bg-surface)",
};

const sentenceStyle: React.CSSProperties = {
  margin: 0,
  fontSize: "15px",
  lineHeight: 1.55,
  fontWeight: 500,
  color: "var(--text-primary)",
};

const labelStyle: React.CSSProperties = {
  fontSize: "12px",
  color: "var(--text-muted)",
};

const AccusationTextBlock: React.FC<Props> = ({ text, wording, onSave, busy }) => {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");

  const open = () => {
    // The draft is seeded from what is STORED every time the editor opens, never
    // kept from a previous edit. A dialog holding a ten-minute-old draft lets a
    // human overwrite a change made since without ever seeing it.
    setDraft(text ?? "");
    setEditing(true);
  };

  if (!editing) {
    return (
      <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
        <span style={labelStyle}>{wording.text_label}</span>
        {/* An absent sentence is NAMED, never filled in and never left blank. */}
        {text ? (
          <p style={sentenceStyle}>{text}</p>
        ) : (
          <p style={{ ...absentStyle, margin: 0 }}>{wording.text_missing_notice}</p>
        )}
        <div style={{ display: "flex", gap: "0.5rem" }}>
          <button type="button" onClick={open} style={ghostButtonStyle} disabled={busy}>
            {wording.text_edit_label}
          </button>
          {/* Withdraw is offered only when there is something to withdraw — a
              control that would do nothing is a control that should not be there. */}
          {text && (
            <button
              type="button"
              onClick={() => onSave(null)}
              style={ghostButtonStyle}
              disabled={busy}
            >
              {wording.text_clear_label}
            </button>
          )}
        </div>
      </div>
    );
  }

  const trimmed = draft.trim();

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
      <span style={labelStyle}>{wording.text_label}</span>
      <textarea
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        placeholder={wording.text_placeholder}
        style={textareaStyle}
      />
      <div style={{ display: "flex", gap: "0.5rem" }}>
        {/* Save is disabled on an empty box rather than sending "" — the backend
            refuses a blank on purpose, because withdrawing and mistyping are
            different intentions, and Withdraw above is how you mean the first. */}
        <button
          type="button"
          onClick={() => {
            onSave(trimmed);
            setEditing(false);
          }}
          style={ghostButtonStyle}
          disabled={busy || trimmed.length === 0}
        >
          {wording.text_save_label}
        </button>
        <button
          type="button"
          onClick={() => setEditing(false)}
          style={ghostButtonStyle}
          disabled={busy}
        >
          {wording.text_cancel_label}
        </button>
      </div>
    </div>
  );
};

export default AccusationTextBlock;
