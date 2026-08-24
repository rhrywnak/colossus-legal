// =============================================================================
// PracticeStart.tsx — screen S0 of PRACTICE_MOCKUP_v2
// =============================================================================
//
// The card Marie opens on: the accusation by name, the terms of the session, who
// is asking, how many, Start, the last-session line, and the ALWAYS card.
//
// ## Every string here comes from the payload
//
// There is not one literal sentence in this file. `w()` reads the store and
// THROWS on a missing key rather than falling back — see `wordingOf`'s note for
// why a blank is the one outcome worse than a stated failure.

import React from "react";

import type {
  PracticeAttachOption,
  PracticeQuestion,
  PracticeWording,
} from "../../services/practice";
import type { PracticeEditor } from "../../pages/usePracticeEditor";
import type { DeckView } from "../../pages/usePracticeDeckControls";
import { wordingOf } from "../../services/practice";
import * as s from "./practiceStyles";
import PracticeDeckList from "./PracticeDeckList";
import PracticeTitleRow from "./PracticeTitleRow";

/** Which deck she is choosing. The three values the backend accepts. */
export type PracticeWho = "george" | "chuck" | "mixed";

interface Props {
  code: string;
  title: string;
  /** Where the printed QUESTIONS live. Composed by the page, so this component
      holds no route knowledge of its own. */
  printHref: string;
  /** Where the printed ANSWERS live. */
  answersHref: string;
  wording: PracticeWording;
  /** This scenario's questions. `view.all` includes what the editor may see. */
  view: DeckView;
  /** The deck editor's state and its writes (task B1). */
  editor: PracticeEditor;
  /** What a new question may attach to. */
  attachOptions: PracticeAttachOption[];
  /** Remove a question from the deck. The mechanism is the existing hide. */
  onDelete: (question: PracticeQuestion) => void;
  /** Put a deleted question back. */
  onUndoDelete: (question: PracticeQuestion) => void;
  /** The question whose delete is in flight, or null. */
  deletingId: string | null;
  /** A delete or undo that failed, already composed. */
  deleteError: string | null;
  /** Where one question's own page lives. */
  questionHref: (question: PracticeQuestion) => string;
  /** Where the practice walk for one side lives. */
  walkHref: (side: "george" | "chuck") => string;
}

/**
 * The ALWAYS card — the five rules that never move.
 *
 * Exported because the reveal has no room for it but the START does, and because
 * it is also an INPUT to the read (the model is given this same stored line). One
 * component, one source, so what Marie is told and what the model is told cannot
 * drift.
 */
export const AlwaysCard: React.FC<{ wording: PracticeWording }> = ({ wording }) => (
  <div style={s.always}>
    <b style={{ color: s.INK }}>{wordingOf(wording, "always_label")}</b>{" "}
    · {wordingOf(wording, "always_line")}
  </div>
);

const PracticeStart: React.FC<Props> = ({
  code,
  title,
  printHref,
  answersHref,
  wording,
  view,
  editor,
  attachOptions,
  onDelete,
  onUndoDelete,
  deletingId,
  deleteError,
  questionHref,
  walkHref,
}) => {
  const w = (key: string) => wordingOf(wording, key);
  // Which row has its editor field stack open. Owned HERE because the control
  // that guards it — Edit the deck — is in the title row above, and a guard
  // reading a copy of the state it protects is not a guard.
  const [fieldsFor, setFieldsFor] = React.useState<string | null>(null);
  // Which side the practice bar offers. Local and unpersisted: it is a choice
  // about the next thirty seconds, not a setting.
  const [side, setSide] = React.useState<"george" | "chuck">("george");

  /**
   * Turn the editor off, asking first if a row's fields are still open.
   *
   * `window.confirm` and not a custom dialog: this is a one-line yes/no about
   * discarding one row's unsaved fields, it must block the state change, and a
   * bespoke modal would be a second confirm implementation for a question the
   * browser already answers. The sentence is the STORE'S, with the row's number
   * in it — "the unsaved edit" without saying which row is a question a person
   * cannot answer with two rows on screen.
   */
  const leaveEditing = () => {
    if (fieldsFor !== null) {
      const n = view.ordered.findIndex((q) => q.id === fieldsFor) + 1;
      const asked = w("editor_discard_confirm_template").replace("{n}", String(n));
      if (!window.confirm(asked)) return;
      setFieldsFor(null);
    }
    editor.toggleEditing();
  };

  /**
   * Warn on RELOAD or tab-close while a row's fields are open.
   *
   * Standing Rule 1's shape for a browser event: the only thing a page may do
   * here is set `returnValue`, and the browser prints its own sentence — ours is
   * not allowed through. It covers reload and close, which is where an open edit
   * is actually lost; edit mode itself is deliberately NOT restored on reload
   * (it is a mode, not a place), so there is nothing to come back to.
   */
  React.useEffect(() => {
    if (!editor.editing || fieldsFor === null) return undefined;
    const warn = (event: BeforeUnloadEvent) => {
      event.preventDefault();
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", warn);
    return () => window.removeEventListener("beforeunload", warn);
  }, [editor.editing, fieldsFor]);

  return (
    <section style={s.card}>
      <div style={s.kicker}>{w("kicker")}</div>
      <PracticeTitleRow
        code={code}
        title={title}
        printHref={printHref}
        answersHref={answersHref}
        onToggleEditing={editor.editing ? leaveEditing : editor.toggleEditing}
        questions={view.all}
        editing={editor.editing}
        wording={wording}
      />

      {/* A WARNING, not an invitation. Every deck on this system is seeded from
          the record and unreviewed, and the line this replaced told a witness to
          rehearse answers to questions no attorney had read. Permanent: it does
          not vanish once a deck is reviewed, because "reviewed" is not a state
          this system tracks. */}
      <p style={s.warning}>{w("intro")}</p>

      {/* ⚑ THE PRACTICE BAR. Everything it starts WRITES NOTHING — no model
          call, no database write, no session. It walks questions she has
          already answered, on the chosen side, in deck order, with her answer
          hidden until she asks for it.

          The side is a form control and the button is a LINK wearing a button's
          clothes, so the walk has an address a reload can land on. */}
      <div style={s.practiceBar}>
        <span style={s.practiceBarLabel}>{w("practice_mode_label")}</span>
        <select
          style={s.practiceBarSelect}
          value={side}
          aria-label={w("practice_mode_label")}
          onChange={(event) => setSide(event.target.value === "chuck" ? "chuck" : "george")}
        >
          <option value="george">{w("who_george_title")}</option>
          <option value="chuck">{w("who_chuck_title")}</option>
        </select>
        <a style={{ ...s.buttonPrimary, ...s.practiceBarGo }} href={walkHref(side)}>
          {w("start_practising_label")}
        </a>
        {/* Standing rule of 2026-08-19: no control on a practice page is dim and
            silent. This one replaces a Start button that opened a sitting and
            wrote rows, in the same position on the same page. */}
        <span style={s.practiceBarHint}>{w("practice_hint")}</span>
      </div>

      <PracticeDeckList
        questions={view.ordered}
        wording={wording}
        editor={editor}
        attachOptions={attachOptions}
        onDelete={onDelete}
        onUndoDelete={onUndoDelete}
        deletingId={deletingId}
        deleteError={deleteError}
        questionHref={questionHref}
        fieldsFor={fieldsFor}
        setFieldsFor={setFieldsFor}
      />

      {/* The one line on this page about how to TESTIFY rather than about the
          software. It costs a line and it is the only thing here Marie will
          still need when the screen is closed. */}
      <AlwaysCard wording={wording} />
    </section>
  );
};

export default PracticeStart;
