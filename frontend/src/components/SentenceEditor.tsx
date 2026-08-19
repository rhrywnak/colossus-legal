// =============================================================================
// SentenceEditor — one authored sentence, and the acts a human performs on it
// =============================================================================
//
// Write it, change it, withdraw it. Was `AccusationTextBlock` (task 2.11 B1);
// generalised in 2.11 C when ruling C4c said the rehearsal page's "What this is"
// editor is "the same leaf pattern as AccusationTextBlock — build it once, use
// it for both sentences".
//
// It now serves two callers:
//   · the working view's accusation sentence   (AccusationSection)
//   · the rehearsal page's "What this is"      (RehearsalScenarioBlocks)
//
// It served a third, `RehearsalAccusationBlock`, until the 08-10 prep-page change
// (1bbf354) replaced that block with `PrepTopBlock` and left it unimported. The
// nav cleanup removed it; git holds it. The naming here is kept accurate rather
// than left pointing at a file that no longer exists — a comment naming a
// deleted component is how the next reader loses ten minutes.
//
// ## What this block exists to stop
//
// The shipped rehearsal page read `definition->>'attack_text'` under the heading
// "What they say". On S-2 that column holds a verbatim first-person quote FROM
// THE RECORD — one thing somebody said once — so the page silently promoted one
// piece of evidence into the summary of all of it. That was beta.371 defect (a),
// fixed at the root: this block writes a different column, and nothing ever
// backfills it from the quote.
//
// ## Why the STYLES arrive as props
//
// Two surfaces with two visual languages and one behaviour. Forking the
// component would fork the behaviour with it — the draft-seeding rule below is
// exactly the kind of thing that gets fixed on one copy and not the other. The
// same reason `WatchListBlock` has taken its styles as props since task 1.5.
//
// ## Why the gap is shown HERE and not only where Marie reads it
//
// The honest-gap law says every absence is named. Naming it only on the
// rehearsal page would leave the person who can FIX it looking at a blank space
// with no idea one was expected.

import React, { useState } from "react";

import { absentStyle } from "./scenarioSectionStyles";

/** The four words this editor speaks. Both surfaces serve them from the store. */
export interface SentenceEditorWording {
  editLabel: string;
  saveLabel: string;
  cancelLabel: string;
  /**
   * Clears the sentence. Omit to offer no Withdraw at all — "What this is" has
   * no such control on the signed mockup, and a control that is offered has to
   * do something.
   */
  withdrawLabel?: string;
  /** The empty-box hint. */
  placeholder: string;
}

interface Props {
  /** The stored sentence, or `null` when nobody has written one. */
  text: string | null;
  /** The stored sentence naming that absence. Never a blank, never a quote. */
  missingNotice: string;
  /** An optional label above the sentence — the working view shows one. */
  label?: string;
  wording: SentenceEditorWording;
  /** `null` withdraws it. Rejects nothing itself — the backend rules. */
  onSave: (text: string | null) => void;
  /** True while a write is in flight, so a double-click cannot send twice. */
  busy: boolean;
  /** How the stored sentence renders. The two surfaces size it differently. */
  sentenceStyle: React.CSSProperties;
  buttonStyle: React.CSSProperties;
  fieldStyle: React.CSSProperties;
  /** Rendered under the sentence, above the controls — the attribution line. */
  children?: React.ReactNode;
}

const labelStyle: React.CSSProperties = {
  fontSize: "12px",
  color: "var(--text-muted)",
};

const SentenceEditor: React.FC<Props> = ({
  text,
  missingNotice,
  label,
  wording,
  onSave,
  busy,
  sentenceStyle,
  buttonStyle,
  fieldStyle,
  children,
}) => {
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
        {label && <span style={labelStyle}>{label}</span>}
        {/* An absent sentence is NAMED, never filled in and never left blank. */}
        {text ? (
          <p style={sentenceStyle}>{text}</p>
        ) : (
          <p style={{ ...absentStyle, margin: 0 }}>{missingNotice}</p>
        )}
        {children}
        <div style={{ display: "flex", gap: "6px" }}>
          <button type="button" onClick={open} style={buttonStyle} disabled={busy}>
            {wording.editLabel}
          </button>
          {/* Withdraw is offered only when there is something to withdraw AND the
              caller has a word for it — a control that would do nothing is a
              control that should not be there. */}
          {text && wording.withdrawLabel && (
            <button
              type="button"
              onClick={() => onSave(null)}
              style={buttonStyle}
              disabled={busy}
            >
              {wording.withdrawLabel}
            </button>
          )}
        </div>
      </div>
    );
  }

  const trimmed = draft.trim();

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
      {label && <span style={labelStyle}>{label}</span>}
      <textarea
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        placeholder={wording.placeholder}
        style={fieldStyle}
        rows={3}
        // The browser advises while you type; storage stays verbatim. Nothing
        // here autocorrects, and nothing downstream rewrites what is saved.
        spellCheck
      />
      <div style={{ display: "flex", gap: "6px" }}>
        {/* Save is disabled on an empty box rather than sending "" — the backend
            refuses a blank on purpose, because withdrawing and mistyping are
            different intentions, and Withdraw above is how you mean the first. */}
        <button
          type="button"
          onClick={() => {
            onSave(trimmed);
            setEditing(false);
          }}
          style={buttonStyle}
          disabled={busy || trimmed.length === 0}
        >
          {wording.saveLabel}
        </button>
        <button
          type="button"
          onClick={() => setEditing(false)}
          style={buttonStyle}
          disabled={busy}
        >
          {wording.cancelLabel}
        </button>
      </div>
    </div>
  );
};

export default SentenceEditor;
