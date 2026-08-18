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

import type { PracticeQuestion, PracticeWording } from "../../services/practice";
import { wordingOf } from "../../services/practice";
import * as s from "./practiceStyles";
import PracticeDeckList from "./PracticeDeckList";

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
  /** This side's questions, in the order the sitting will deal them. */
  questions: PracticeQuestion[];
  /** How many are available after today's skips. `0` withdraws Start. */
  available: number;
  /** How many she has chosen to be asked. */
  count: number;
  onCountChange: (count: number) => void;
  /** Ids kept out of this sitting. Session-scoped; never stored. */
  skippedToday: ReadonlySet<string>;
  onToggleSkip: (id: string) => void;
  onSaveFlag: (id: string, note: string) => void;
  savingFlagFor: string | null;
  flagError: string | null;
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
  questions,
  available,
  count,
  onCountChange,
  skippedToday,
  onToggleSkip,
  onSaveFlag,
  savingFlagFor,
  flagError,
}) => {
  const w = (key: string) => wordingOf(wording, key);

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
              onClick={() => onCountChange(v)}
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
          onClick={() => onCountChange(available)}
        >
          {w("count_all_template").replace("{n}", String(available))}
        </button>
      </p>

      <PracticeDeckList
        questions={questions}
        wording={wording}
        skippedToday={skippedToday}
        onToggleSkip={onToggleSkip}
        onSaveFlag={onSaveFlag}
        savingFlagFor={savingFlagFor}
        flagError={flagError}
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
