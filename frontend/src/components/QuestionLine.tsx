// =============================================================================
// QuestionLine — the §7.1 question, and correcting it (task 1.7F Part B)
// =============================================================================
//
// The interrogatory a discovery answer answers, with a badge saying who wrote it
// and — when a human wants to — a pencil to fix it.
//
// ## Why this is its own file
//
// `CandidateCard` was 235 non-comment lines before this task and the card is
// already the largest component in the feature (Rule 17/18). This is a seam
// rather than an arithmetic split: everything here is about ONE line of the card
// and the small editing surface that hangs off it, while the card is about
// assembling every §7 element.
//
// ## The words are the backend's; the icons are ours
//
// `question_authorship.label` arrives composed ("System", "roman · 4 Aug 2026")
// and renders verbatim, because how authorship reads is a display decision the
// language law puts on the server. The ⚙ and 👤 glyphs are this list's own
// control vocabulary, exactly like the state chip's ✓ and ✕ — and, like those,
// they never stand alone: the word is always beside the icon.
//
// ## Domain note: correcting this changes it everywhere (ruling R1)
//
// One summary per piece of evidence, case-wide. The confirmation line says so
// before the human commits, because "your edit changes another scenario's card"
// is exactly the kind of consequence a person should be told about while they can
// still decide otherwise.

import React, { useState } from "react";

import type { CardQuestionAuthorship } from "../services/scenarioCards";

const labelStyle: React.CSSProperties = {
  fontSize: "11px",
  fontWeight: 600,
  letterSpacing: "0.03em",
  color: "var(--text-muted)",
  whiteSpace: "nowrap",
};

/** The pencil / Revert affordances: quiet until hovered, never competing with
 *  the ruling buttons above for the eye. */
const actionStyle: React.CSSProperties = {
  fontFamily: "inherit",
  fontSize: "11.5px",
  fontWeight: 500,
  border: "none",
  background: "transparent",
  color: "var(--accent-primary)",
  cursor: "pointer",
  padding: "2px 4px",
};

const editorStyle: React.CSSProperties = {
  fontFamily: "inherit",
  fontSize: "13.5px",
  lineHeight: 1.6,
  width: "100%",
  minHeight: "4.5rem",
  padding: "8px 10px",
  borderRadius: "8px",
  border: "1px solid var(--border-default)",
  color: "var(--text-primary)",
  fontWeight: 400,
  resize: "vertical",
};

const noticeStyle: React.CSSProperties = {
  fontSize: "12px",
  color: "var(--text-muted)",
  lineHeight: 1.5,
};

/**
 * The question line, with its badge and its editor.
 *
 * `onSave` and `onRevert` return promises so this component can keep the editor
 * open, and the human's typing intact, until the SERVER has answered. Closing on
 * the click would be the optimistic pattern ruling R3 rejects for rows, applied
 * to text — the human would watch their correction "succeed" and then find the
 * machine's sentence back on the next read.
 */
export const QuestionLine: React.FC<{
  question: string;
  authorship?: CardQuestionAuthorship;
  onSave: (text: string) => Promise<void>;
  onRevert: () => Promise<void>;
  /**
   * The stored label for the Revert control (task 2.10, the 1.7F fold-in).
   *
   * `undefined` while the panel wording has not loaded — and then the control is
   * NOT rendered rather than falling back to a literal written here. A hardcoded
   * label would be the compiled-in string R4 deletes, and it would be the one on
   * screen on the day the settings store failed to load, which is the day you most
   * want to know.
   */
  revertLabel?: string;
}> = ({ question, authorship, onSave, onRevert, revertLabel }) => {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(question);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const human = authorship?.source === "human";

  /** Run one write, keeping the editor open and the draft intact if it fails. */
  const run = (work: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    work()
      .then(() => {
        setBusy(false);
        setEditing(false);
      })
      .catch((e: unknown) => {
        setBusy(false);
        // The refusal stays ON the editor, beside the text it refused, and the
        // draft is not discarded — a human who just typed three sentences must
        // not lose them to a failed save (Standing Rule 1).
        setError(e instanceof Error ? e.message : "That did not save. Try again.");
      });
  };

  if (editing) {
    return (
      <div
        style={{ display: "flex", flexDirection: "column", gap: "6px" }}
        // The editor lives inside a card whose click selects it; typing in here
        // is not a request to aim the keyboard at this card, and a click on the
        // textarea would otherwise do both.
        onClick={(event) => event.stopPropagation()}
      >
        <textarea
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          style={editorStyle}
          aria-label="The question this answers"
          autoFocus
          disabled={busy}
        />
        <div style={noticeStyle}>
          This question belongs to the evidence, not to this scenario — correcting it
          here corrects it wherever this item appears.
        </div>
        {error && (
          <div role="alert" style={{ ...noticeStyle, color: "var(--state-danger-strong)" }}>
            {error}
          </div>
        )}
        <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
          <button
            type="button"
            style={{ ...actionStyle, fontWeight: 600 }}
            disabled={busy}
            onClick={() => run(() => onSave(draft))}
          >
            {busy ? "Saving…" : "Save"}
          </button>
          <button
            type="button"
            style={{ ...actionStyle, color: "var(--text-secondary)" }}
            disabled={busy}
            onClick={() => {
              setDraft(question);
              setError(null);
              setEditing(false);
            }}
          >
            Cancel
          </button>
          {human && revertLabel && (
            <button
              type="button"
              style={{ ...actionStyle, color: "var(--text-secondary)", marginLeft: "auto" }}
              disabled={busy}
              onClick={() => run(onRevert)}
            >
              <span aria-hidden="true">↩</span> {revertLabel}
            </button>
          )}
        </div>
      </div>
    );
  }

  return (
    <div style={{ display: "flex", gap: "8px", alignItems: "baseline", flexWrap: "wrap" }}>
      <span style={{ fontSize: "13.5px", lineHeight: 1.7, color: "var(--text-secondary)" }}>
        {question}
      </span>
      {authorship && (
        <span style={labelStyle} title={human ? "A human corrected this question" : undefined}>
          <span aria-hidden="true">{human ? "👤" : "⚙"}</span> {authorship.label}
        </span>
      )}
      {/* Revert BESIDE THE BADGE (Roman's ruling, 2026-08-04, folded into 2.10).
          The in-editor control stays where it was; this one means undoing a
          correction no longer requires opening an editor to reach the button that
          throws that editor's contents away. Only for a human-authored question,
          because there is nothing to revert to otherwise. */}
      {human && revertLabel && (
        <button
          type="button"
          style={{ ...actionStyle, color: "var(--text-secondary)" }}
          aria-label={revertLabel}
          title={revertLabel}
          disabled={busy}
          onClick={(event) => {
            // Not also a selection: reverting is not aiming the keyboard.
            event.stopPropagation();
            run(onRevert);
          }}
        >
          <span aria-hidden="true">↩</span>
        </button>
      )}
      <button
        type="button"
        style={actionStyle}
        aria-label="Correct this question"
        title="Correct this question"
        onClick={(event) => {
          // Not also a selection: opening an editor is not aiming the keyboard.
          event.stopPropagation();
          setDraft(question);
          setEditing(true);
        }}
      >
        ✎
      </button>
    </div>
  );
};

export default QuestionLine;
