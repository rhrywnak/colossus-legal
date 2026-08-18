// =============================================================================
// PracticeDeckList.tsx — the deck, listed on the start card (mockup v3)
// =============================================================================
//
// Roman's ruling of 2026-08-18: Marie reads the questions BEFORE she starts, and
// the list is open by default. Each row carries two controls — Skip today, which
// keeps it out of this sitting, and Flag, which tells Roman and Chuck what is
// wrong with it.
//
// ## Why skip and flag are two controls and not one
//
// They say different things. "Skip today" is about this evening; "Flag" is about
// the question. Collapsing them would lose the distinction that makes the flag
// worth reading — Roman needs to know which questions are WRONG, not which ones
// she was not in the mood for.
//
// ## Why there is no edit control
//
// Roman's ruling, and the migration header argues it: the deck text is what was
// proved verbatim against the mockup and what the read is prompted on. A text
// change mid-week with no record of who made it is the pairing-editor problem
// again. The flag carries the complaint; the edit is a human act on the seed.
//
// ## Every string here comes from the payload
//
// Not one literal sentence. `w()` reads the store and THROWS by name on a
// missing key rather than rendering a blank control.

import React from "react";

import type { PracticeQuestion, PracticeWording } from "../../services/practice";
import { wordingOf } from "../../services/practice";
import * as s from "./practiceStyles";
import * as d from "./practiceDeckStyles";

interface Props {
  /** This side's questions, in the order the sitting will deal them. */
  questions: PracticeQuestion[];
  wording: PracticeWording;
  /** Ids she has kept out of this sitting. Session-scoped; never stored. */
  skippedToday: ReadonlySet<string>;
  onToggleSkip: (id: string) => void;
  /** Writes the note to the question. Blank clears it. */
  onSaveFlag: (id: string, note: string) => void;
  /** The id whose flag write is in flight, so the control can say so. */
  savingFlagFor: string | null;
  /** Set when the last flag write failed — shown, never swallowed. */
  flagError: string | null;
}

/**
 * The pill on a row: George, Chuck, or the braid's third colour.
 *
 * A braid is answered differently from either side, which is why the mockup
 * gives it a colour of its own rather than a George pill with a note.
 */
const sidePill = (question: PracticeQuestion, wording: PracticeWording) => {
  const w = (key: string) => wordingOf(wording, key);
  if (question.braid) return { style: s.pillBraid, label: w("pill_braid") };
  if (question.side === "george") return { style: s.pillGeorge, label: w("pill_george") };
  return { style: s.pillChuck, label: w("pill_chuck") };
};

/**
 * The instruction sentence, with the two control labels rendered bold.
 *
 * The stored row carries `{skip}` and `{flag}` rather than the words themselves,
 * so renaming a button cannot leave the sentence naming one that no longer
 * exists. Split on the placeholders rather than injecting HTML: this is React,
 * and `dangerouslySetInnerHTML` over a stored string is how a wording row
 * becomes a script tag.
 */
const Instruction: React.FC<{ wording: PracticeWording }> = ({ wording }) => {
  const w = (key: string) => wordingOf(wording, key);
  const parts = w("deck_instruction_template").split(/(\{skip\}|\{flag\})/);
  return (
    <p style={d.deckInstruction}>
      {parts.map((part, i) => {
        if (part === "{skip}") return <b key={i}>{w("skip_today_label")}</b>;
        if (part === "{flag}") return <b key={i}>{w("flag_label")}</b>;
        return <React.Fragment key={i}>{part}</React.Fragment>;
      })}
    </p>
  );
};

const PracticeDeckList: React.FC<Props> = ({
  questions,
  wording,
  skippedToday,
  onToggleSkip,
  onSaveFlag,
  savingFlagFor,
  flagError,
}) => {
  const w = (key: string) => wordingOf(wording, key);

  // Open by default — Roman's ruling. Deliberately NOT persisted: the fold is
  // for one page-load, until Chuck rules on whether Marie should see the deck
  // before a drill at all.
  const [open, setOpen] = React.useState(true);
  // Which row has its note editor showing, and what is typed in it. One row at a
  // time: two open editors is two half-written complaints and no way to tell
  // which one she meant to save.
  const [editing, setEditing] = React.useState<string | null>(null);
  const [draft, setDraft] = React.useState("");

  const george = questions.filter((q) => q.side === "george").length;
  const skippedHere = questions.filter((q) => skippedToday.has(q.id)).length;

  const count =
    w("deck_count_template")
      .replace("{n}", String(questions.length))
      .replace("{george}", String(george))
      .replace("{chuck}", String(questions.length - george)) +
    (skippedHere > 0
      ? ` ${w("deck_skipped_suffix_template").replace("{k}", String(skippedHere))}`
      : "");

  const openEditor = (question: PracticeQuestion) => {
    setEditing(question.id);
    setDraft(question.flag_note ?? "");
  };

  return (
    <div style={d.deck}>
      <div style={d.deckHeader}>
        <b>
          {w("deck_heading")} <span style={d.deckCount}>{count}</span>
        </b>
        <button
          type="button"
          style={d.deckToggle}
          aria-expanded={open}
          onClick={() => setOpen((was) => !was)}
        >
          {open ? w("deck_hide_link") : w("deck_show_link")}
        </button>
      </div>

      {open && (
        <>
          <Instruction wording={wording} />
          {flagError !== null && <p style={d.flagged}>{flagError}</p>}
          {questions.map((question, i) => {
            const skipped = skippedToday.has(question.id);
            const flagged = question.flag_note !== null && question.flag_note !== "";
            const pill = sidePill(question, wording);
            const muted = skipped ? d.questionSkipped : undefined;
            return (
              <div
                key={question.id}
                style={{
                  ...d.questionRow,
                  ...(flagged ? d.questionRowFlagged : {}),
                  ...(i === questions.length - 1 ? d.questionRowLast : {}),
                }}
              >
                <div style={d.questionNumber}>{i + 1}</div>
                <div>
                  <span style={{ ...pill.style, fontSize: 12 }}>{pill.label}</span>
                  {question.tactic !== null && (
                    <span style={s.tacticTag}>{question.tactic}</span>
                  )}
                  <div style={{ ...d.questionText, ...muted }}>{question.text}</div>
                  {question.receipt !== null && (
                    <div style={{ ...d.questionSource, ...muted }}>{question.receipt}</div>
                  )}
                  {flagged && (
                    <div style={d.flagged}>
                      ⚑ {w("flag_shown_template").replace("{note}", question.flag_note ?? "")}
                    </div>
                  )}
                  {editing === question.id && (
                    <div style={d.flagLine}>
                      <input
                        style={d.flagInput}
                        placeholder={w("flag_placeholder")}
                        value={draft}
                        onChange={(e) => setDraft(e.target.value)}
                        aria-label={w("flag_placeholder")}
                      />
                      <button
                        type="button"
                        style={d.rowButton}
                        disabled={savingFlagFor === question.id}
                        onClick={() => {
                          onSaveFlag(question.id, draft);
                          setEditing(null);
                        }}
                      >
                        {w("flag_save_label")}
                      </button>
                      <button
                        type="button"
                        style={d.rowButton}
                        onClick={() => setEditing(null)}
                      >
                        {w("flag_cancel_label")}
                      </button>
                    </div>
                  )}
                </div>
                <div style={d.rowControls}>
                  <button
                    type="button"
                    style={skipped ? d.rowButtonSkipped : d.rowButton}
                    aria-pressed={skipped}
                    onClick={() => onToggleSkip(question.id)}
                  >
                    {skipped ? w("skipped_today_label") : w("skip_today_label")}
                  </button>
                  <button
                    type="button"
                    style={d.rowButton}
                    onClick={() =>
                      editing === question.id ? setEditing(null) : openEditor(question)
                    }
                  >
                    {flagged ? w("flag_edit_label") : w("flag_label")}
                  </button>
                </div>
              </div>
            );
          })}
        </>
      )}
    </div>
  );
};

export default PracticeDeckList;
