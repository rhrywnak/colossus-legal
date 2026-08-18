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

import type { PracticeWording } from "../../services/practice";
import { wordingOf } from "../../services/practice";
import { V0_QUESTION_COUNT } from "../../pages/practiceQueue";
import * as s from "./practiceStyles";

/**
 * The middle count pill, which v0 does not serve.
 *
 * A literal because it is the mockup's own proposal for a LATER build — eight is
 * not derived from this deck and is not a parameter anything reads. It renders
 * dimmed beside the live one, which is the mockup's way of showing what is
 * coming without offering it. When it becomes real it becomes a parameter.
 */
const DEFERRED_COUNT_PILL = 8;

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
  /** How many questions the chosen side has. `0` withdraws Start entirely. */
  available: number;
  /** The whole deck's size — what the "all {n}" pill reports. */
  deckSize: number;
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
  available,
  deckSize,
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

      {/* The count pills. Only 5 is live in v0 and the other two render dimmed,
          exactly as the mockup does — which is honest about what this build
          offers rather than showing controls that do nothing.

          The third one says "all {n}" with the DECK's own size, not the mockup's
          literal "all 12": S-5 carries ten questions, and a pill naming a number
          no deck has is the kind of small wrongness a witness stops trusting a
          screen over. */}
      <p style={{ marginTop: 22 }}>
        <b>{w("how_many_heading")}</b> <span style={s.pill}>{V0_QUESTION_COUNT}</span>{" "}
        <span style={{ ...s.pill, opacity: 0.5 }}>{DEFERRED_COUNT_PILL}</span>{" "}
        <span style={{ ...s.pill, opacity: 0.5 }}>
          {w("count_all_template").replace("{n}", String(deckSize))}
        </span>
      </p>

      <div style={{ ...s.row, marginTop: 22 }}>
        <button
          type="button"
          style={{ ...s.buttonPrimary, ...s.buttonBig }}
          onClick={onStart}
          disabled={starting || available === 0}
        >
          {w("start_label")}
        </button>
        <span style={s.progress}>{lastSessionLine}</span>
      </div>

      <AlwaysCard wording={wording} />
    </section>
  );
};

export default PracticeStart;
