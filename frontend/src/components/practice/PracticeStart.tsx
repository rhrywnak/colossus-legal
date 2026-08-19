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
  OpenSession,
  PracticeAttachOption,
  PracticeChanged as Changed,
  PracticeNote,
  PracticeQuestion,
  PracticeWording,
} from "../../services/practice";
import type { PracticeEditor } from "../../pages/usePracticeEditor";
import type { DeckView, PracticeDeckControls } from "../../pages/usePracticeDeckControls";
import { wordingOf } from "../../services/practice";
import * as s from "./practiceStyles";
import PracticeChangedBox from "./PracticeChanged";
import PracticeDeckList from "./PracticeDeckList";
import PracticeNotes from "./PracticeNotes";
import PracticeResume from "./PracticeResume";

/**
 * The shorter counts the mockup offers, when the deck is longer than they are.
 *
 * Literals because they are the mockup's own proposals — neither is derived from
 * this deck and neither is a parameter anything reads. Mockup v3 shows a pill
 * ONLY when it is smaller than what is available: a "5" beside a four-question
 * deck is a control that cannot do what it says.
 */
const SHORT_COUNTS = [5, 8];

/** Which deck she is choosing. The three values the backend accepts. */
export type PracticeWho = "george" | "chuck" | "mixed";

interface Props {
  code: string;
  title: string;
  wording: PracticeWording;
  /** The composed sentence — the last session's, or the "none yet" one. */
  lastSessionLine: string;
  who: PracticeWho;
  onWhoChange: (who: PracticeWho) => void;
  onStart: () => void;
  /** True while the session POST is in flight; the control says so. */
  starting: boolean;
  /**
   * The start screen's own state and its two row controls, as one object.
   *
   * Passed whole rather than as nine props: they are one thing (what the start
   * card can do to a question before a sitting), they move together, and nine
   * positional props is where one eventually gets wired to the wrong handler.
   */
  controls: PracticeDeckControls;
  /** This side's questions, and what is left of them after today's skips. */
  view: DeckView;
  /** The sitting she walked out of, or `null` — which withdraws the blue box. */
  openSession: OpenSession | null;
  onResume: () => void;
  onStartOver: () => void;
  /** True while a resume / start-over write is in flight. */
  resuming: boolean;
  /** That write's failure sentence, or null. */
  resumeError: string | null;
  /** This scenario's receipts, for the picker under the answer box. */
  onPracticeOne: (question: PracticeQuestion) => void;
  /** True while the one-question session POST is in flight. */
  startingOne: boolean;
  /** Open one question's review page (task B3). */
  onReview: (question: PracticeQuestion) => void;
  /** The deck editor's state and its four writes (task B1). */
  editor: PracticeEditor;
  /** What a new question may attach to. */
  attachOptions: PracticeAttachOption[];
  /** What changed since her last sitting, or `null` (task B2). */
  changed: Changed | null;
  /** The notes on this scenario (task B4). */
  notes: PracticeNote[];
  onSaveNote: (author: string, text: string) => void;
  onStrikeNote: (note: PracticeNote) => void;
  savingNote: boolean;
  noteError: string | null;
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
  wording,
  lastSessionLine,
  who,
  onWhoChange,
  onStart,
  starting,
  controls,
  view,
  openSession,
  onResume,
  onStartOver,
  resuming,
  resumeError,
  onPracticeOne,
  startingOne,
  onReview,
  editor,
  attachOptions,
  changed,
  notes,
  onSaveNote,
  onStrikeNote,
  savingNote,
  noteError,
}) => {
  const w = (key: string) => wordingOf(wording, key);
  const available = view.available.length;
  const count = view.count;

  // The three choices, in the mockup's order. A table rather than three copies
  // of the same JSX: the only things that differ are the value and its two
  // stored strings, and three near-identical blocks is where one of them
  // eventually stops matching the others.
  const choices: Array<{ value: PracticeWho; title: string; detail: string }> = [
    { value: "george", title: w("who_george_title"), detail: w("who_george_detail") },
    { value: "chuck", title: w("who_chuck_title"), detail: w("who_chuck_detail") },
    { value: "mixed", title: w("who_mixed_title"), detail: w("who_mixed_detail") },
  ];

  return (
    <section style={s.card}>
      <div style={s.kicker}>{w("kicker")}</div>
      <h1 style={s.h1}>
        {code} · {title}
      </h1>
      <p style={s.sub}>{w("intro")}</p>

      <p style={{ marginTop: 22 }}>
        <b>{w("who_heading")}</b>
      </p>
      <div style={s.choice}>
        {choices.map((c) => (
          <button
            key={c.value}
            type="button"
            style={who === c.value ? s.choiceButtonSelected : s.choiceButton}
            aria-pressed={who === c.value}
            onClick={() => onWhoChange(c.value)}
          >
            <span style={s.choiceTitle}>{c.title}</span>
            <span style={s.choiceDetail}>{c.detail}</span>
          </button>
        ))}
      </div>

      {/* The unfinished sitting, offered back. It sits ABOVE the count pills —
          where the mockup draws it — because a witness who left one open should
          be asked about it before she is asked to configure a new one. */}
      {openSession !== null && (
        <PracticeResume
          wording={wording}
          session={openSession}
          onResume={onResume}
          onStartOver={onStartOver}
          busy={resuming}
          error={resumeError}
        />
      )}

      {/* The count pills FOLLOW what is available (mockup v3): a shorter count
          appears only when it is smaller than the deck she can actually be
          asked, and the last pill always reads "all N" with N = available. A
          pill naming a number no deck has is the kind of small wrongness a
          witness stops trusting a screen over. */}
      <p style={{ marginTop: 22 }}>
        <b>{w("how_many_heading")}</b>{" "}
        {SHORT_COUNTS.filter((v) => v < available).map((v) => (
          <React.Fragment key={v}>
            <button
              type="button"
              style={{ ...s.pill, cursor: "pointer", opacity: count === v ? 1 : 0.5 }}
              aria-pressed={count === v}
              onClick={() => controls.setCount(v)}
            >
              {v}
            </button>{" "}
          </React.Fragment>
        ))}
        <button
          type="button"
          style={{
            ...s.pill,
            cursor: "pointer",
            opacity: count >= available ? 1 : 0.5,
          }}
          aria-pressed={count >= available}
          onClick={() => controls.setCount(available)}
        >
          {w("count_all_template").replace("{n}", String(available))}
        </button>
      </p>

      {/* What changed since she was last here, above the deck as v4 draws it —
          a witness who left a sitting open should be told the questions moved
          before she reads them, not after. */}
      {changed !== null && <PracticeChangedBox wording={wording} changed={changed} />}

      <PracticeDeckList
        // The EDITOR sees hidden questions so they can be put back; Marie's list
        // does not, because a hidden question is one she will not be asked.
        questions={editor.editing ? view.all : view.ordered}
        wording={wording}
        controls={controls}
        editor={editor}
        attachOptions={attachOptions}
        onPracticeOne={onPracticeOne}
        onReview={onReview}
        startingOne={startingOne}
      />

      <PracticeNotes
        wording={wording}
        notes={notes}
        titleKey="notes_scenario_title"
        onSave={onSaveNote}
        onStrike={onStrikeNote}
        saving={savingNote}
        error={noteError}
      />

      <div style={{ ...s.row, marginTop: 22 }}>
        <button
          type="button"
          style={{ ...s.buttonPrimary, ...s.buttonBig }}
          onClick={onStart}
          disabled={starting || available === 0}
        >
          {/* A disabled button still reading "Start" is a screen refusing without
              saying why. */}
          {available === 0 ? w("nothing_left_label") : w("start_label")}
        </button>
        <span style={s.progress}>{lastSessionLine}</span>
      </div>

      <AlwaysCard wording={wording} />
    </section>
  );
};

export default PracticeStart;
