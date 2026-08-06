// =============================================================================
// AuthoredLineEditor — one human-authored line, edited in place
// =============================================================================
//
// A talking point or a watch item: read it, press Edit, fix the words, Save. New
// in task 2.11 C (ruling C4b), and shared by all four places those two lists are
// rendered — the scenario working page and the rehearsal page, points and watch
// items alike.
//
// ## Why ONE component for four sites
//
// The behaviour below is the whole reason: the draft is seeded from what is
// stored, a failed save KEEPS the draft on screen and names the failure, and a
// save in flight disables the controls. Written four times, that is four places
// for the failure branch to be got wrong — and the failure branch is the one a
// human meets while holding a sentence they just wrote.
//
// The visual language arrives as props, exactly as `SentenceEditor`'s does. Two
// surfaces, two looks, one behaviour.
//
// ## Why the edit is an UPDATE and not a remove-and-re-add
//
// Enforced on the server (`update_response_item_text`, `update_human_fact_text`),
// but worth knowing here too: the row keeps who wrote it and when. Before this
// existed, fixing a typo in a watch item meant deleting it and typing it again,
// which stamped the editor as its author and today as the day it was written.

import React, { useState } from "react";

interface Props {
  /** The stored line, as authored. */
  text: string;
  wording: {
    editLabel: string;
    saveLabel: string;
    cancelLabel: string;
    /** Shown while a write is in flight. Optional — omit to keep the label. */
    savingLabel?: string;
  };
  /**
   * Store the edited line. Rejecting is the backend's job; this only reports.
   *
   * Returns a promise so the control can disable itself for the round trip —
   * a second click during a slow save would send the same edit twice.
   */
  onSave: (text: string) => Promise<void>;
  /** Used when a failure carries no message of its own. Stored, never a literal. */
  saveFailedNotice: string;
  /** Names this box for a screen reader. Composed from a stored template. */
  fieldLabel: string;
  textStyle: React.CSSProperties;
  fieldStyle: React.CSSProperties;
  buttonStyle: React.CSSProperties;
  errorStyle: React.CSSProperties;
  /** Rendered under the line when not editing — the exhibit note, the tag. */
  children?: React.ReactNode;
}

const AuthoredLineEditor: React.FC<Props> = ({
  text,
  wording,
  onSave,
  saveFailedNotice,
  fieldLabel,
  textStyle,
  fieldStyle,
  buttonStyle,
  errorStyle,
  children,
}) => {
  const [draft, setDraft] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const editing = draft !== null;

  const save = () => {
    // Guarded by the disabled control below; checked again because a keyboard
    // Enter could reach here past a disabled attribute in some browsers.
    const trimmed = (draft ?? "").trim();
    if (trimmed.length === 0) return;

    setSaving(true);
    onSave(trimmed)
      .then(() => {
        setDraft(null);
        setError(null);
      })
      // Standing Rule 1: the draft STAYS on screen and the failure is named. The
      // human just wrote these words; discarding them on a failed save would be
      // the worst possible response.
      .catch((e: unknown) => {
        setError(e instanceof Error ? e.message : saveFailedNotice);
      })
      .finally(() => setSaving(false));
  };

  if (!editing) {
    return (
      <div>
        <div style={textStyle}>{text}</div>
        {children}
        {error && <div style={errorStyle}>{error}</div>}
        <div style={{ marginTop: "4px" }}>
          <button
            type="button"
            style={buttonStyle}
            onClick={() => {
              // Seeded from what is STORED every time, never kept from a previous
              // edit: a stale draft lets a human overwrite a change made since
              // without ever seeing it.
              setDraft(text);
              setError(null);
            }}
          >
            {wording.editLabel}
          </button>
        </div>
      </div>
    );
  }

  const trimmed = (draft ?? "").trim();

  return (
    <div>
      <textarea
        value={draft ?? ""}
        onChange={(e) => setDraft(e.target.value)}
        aria-label={fieldLabel}
        rows={2}
        style={fieldStyle}
        // The browser advises while you type; what is saved is stored verbatim.
        spellCheck
      />
      {error && (
        <div role="alert" style={errorStyle}>
          {error}
        </div>
      )}
      <div style={{ display: "flex", gap: "6px", marginTop: "4px" }}>
        <button
          type="button"
          style={buttonStyle}
          disabled={saving || trimmed.length === 0}
          onClick={save}
        >
          {saving ? (wording.savingLabel ?? wording.saveLabel) : wording.saveLabel}
        </button>
        <button
          type="button"
          style={buttonStyle}
          disabled={saving}
          onClick={() => {
            setDraft(null);
            setError(null);
          }}
        >
          {wording.cancelLabel}
        </button>
      </div>
    </div>
  );
};

export default AuthoredLineEditor;
