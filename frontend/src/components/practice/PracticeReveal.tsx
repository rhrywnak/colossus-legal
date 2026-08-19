// =============================================================================
// PracticeReveal.tsx — screen S2 of PRACTICE_MOCKUP_v2
// =============================================================================
//
// What Marie sees after every answer, in the mockup's order: the question again
// (small, grey) · WHAT YOU SAID · the one-sentence read · YOUR POINTS · the pair ·
// WATCH FOR · CHECK YOURSELF · the collapsed stronger answer · the two buttons.
//
// ## The order is the design, not a layout
//
// Her own words come FIRST, before any judgement. Then one sentence. Then her
// three points — the things she is entitled to say. The machine's contribution is
// one line in the middle of a screen that is otherwise entirely hers, and the
// boxes at the bottom are the only grading in the product.
//
// ## When the read failed
//
// The rail is neutral, the sentence is the stored "no system read this time", and
// every other block on this screen stands unchanged. A model being down costs the
// session one sentence, not the session.

import React from "react";

import type {
  PracticePoint,
  PracticeQuestion as Question,
  PracticeWording,
  SelfCheck,
} from "../../services/practice";
import { wordingOf } from "../../services/practice";
import * as f from "./practiceFlowStyles";
import * as s from "./practiceStyles";
import { QuestionPills } from "./PracticeQuestion";
import { RECEIPT_JOIN } from "./PracticePointsTo";
import PracticeTopBar from "./PracticeTopBar";

interface Props {
  question: Question;
  wording: PracticeWording;
  position: number;
  total: number;
  /** Her answer, verbatim as she typed it. */
  answer: string;
  /** The one sentence, or null when there was no read. */
  readText: string | null;
  /** true green, false red, null neutral. Three states, never two. */
  readOk: boolean | null;
  points: PracticePoint[];
  selfCheck: SelfCheck;
  onSelfCheckChange: (next: SelfCheck) => void;
  /** Called the first time the drawer opens; records `help_opened`. */
  onHelpOpened: () => void;
  /**
   * True when that write FAILED. The drawer still opened and her answer is
   * untouched — the only casualty is one cell on Chuck's sheet, and the notice
   * below says exactly that rather than implying anything was lost.
   */
  helpNotRecorded: boolean;
  /**
   * The stored sentence to show when the write settling this answer failed — or
   * `null` when it has not. She stays on this screen and the buttons still work,
   * because the honest response to a failed write is to let her retry it, not to
   * carry her forward as if it had landed.
   */
  markError: string | null;
  onNext: () => void;
  onAgainLater: () => void;
  /** The receipts she named when she answered. EMPTY withdraws the line. */
  pointsTo: string[];
  /** The two exits at the top of the screen (mockup v3, item B6). */
  onBack: () => void;
  onEnd: () => void;
  /** True while the write settling this answer is in flight. */
  busy: boolean;
}

/** The four self-check boxes, in the mockup's order. */
const CHECKS: Array<{ field: keyof SelfCheck; key: string }> = [
  { field: "only_asked", key: "check_only_asked" },
  { field: "accepted_premise", key: "check_accepted_premise" },
  { field: "explained_unasked", key: "check_explained_unasked" },
  { field: "guessed", key: "check_guessed" },
];

const PracticeReveal: React.FC<Props> = ({
  question,
  wording,
  position,
  total,
  answer,
  readText,
  readOk,
  points,
  selfCheck,
  onSelfCheckChange,
  onHelpOpened,
  helpNotRecorded,
  markError,
  onNext,
  onAgainLater,
  pointsTo,
  onBack,
  onEnd,
  busy,
}) => {
  const w = (key: string) => wordingOf(wording, key);

  // `readOk === null` is a THIRD state and gets the neutral rail: green would
  // congratulate her for a call that never happened, red would accuse her of
  // something nobody judged.
  const feedbackStyle =
    readOk === null
      ? { ...s.feedback, borderLeftColor: s.LINE, background: s.QUIET_BG }
      : readOk
        ? s.feedbackOk
        : s.feedback;

  return (
    <section style={s.card}>
      <PracticeTopBar
        wording={wording}
        screen="reveal"
        onBack={onBack}
        // The reveal offers no mid-sitting skip: the row already exists and is
        // already on Chuck's sheet. "Skip" there would mean relabelling an
        // answer she gave as one she set aside.
        onSkip={onBack}
        onEnd={onEnd}
        busy={busy}
      />
      <div style={{ ...s.row, justifyContent: "space-between", marginTop: 0 }}>
        <span style={s.progress}>
          {w("progress_template")
            .replace("{n}", String(position))
            .replace("{total}", String(total))}
        </span>
        <QuestionPills question={question} wording={wording} />
      </div>
      <div style={s.questionEcho}>{question.text}</div>

      <div style={{ ...s.kicker, marginTop: 16 }}>{w("what_you_said_kicker")}</div>
      <div style={s.yours}>{answer}</div>

      {/* What she said she would reach for. Withdrawn entirely when she named
          nothing: a prefix with an empty list after it reads as a list that
          failed to load, and naming no exhibit is not a fault to point at. */}
      {pointsTo.length > 0 && (
        <div style={f.pointsToChosen}>
          {w("points_to_reveal_prefix")} {pointsTo.join(RECEIPT_JOIN)}
        </div>
      )}

      <div style={feedbackStyle}>
        {readText ?? w("read_unavailable")}
        <small style={s.feedbackNote}>
          <span style={s.tag}>{w("read_tag")}</span> {w("read_footnote")}
        </small>
      </div>

      <div style={{ ...s.kicker, marginTop: 22 }}>{w("points_kicker")}</div>
      <ol>
        {points.map((point) => (
          <li key={point.position} style={s.pointItem}>
            {point.text}
            <div style={s.receipt}>
              {/* A point nobody paired an exhibit with says so in words. The
                  alternative — a blank line under the point — reads as a
                  receipt that failed to load. */}
              {point.exhibit === null
                ? w("point_no_receipt")
                : `${w("receipt_prefix")} ${point.exhibit}`}
            </div>
          </li>
        ))}
      </ol>

      {/* The pair, or nothing at all. Both halves are present together by
          construction (the column pair is validated at seed time), so one check
          decides the block. */}
      {question.pair_said !== null && question.pair_admitted !== null && (
        <>
          <div style={{ ...s.kicker, marginTop: 18 }}>{w("pair_kicker")}</div>
          <div style={s.pair}>
            <div style={s.pairCell}>
              <div style={s.pairLabel}>{w("pair_said_label")}</div>
              {question.pair_said}
            </div>
            <div style={s.pairCell}>
              <div style={s.pairLabel}>{w("pair_admitted_label")}</div>
              {question.pair_admitted}
            </div>
          </div>
        </>
      )}

      {question.watch_for !== null && <div style={s.watch}>{question.watch_for}</div>}

      <div style={{ ...s.kicker, marginTop: 22 }}>{w("check_kicker")}</div>
      <div>
        {CHECKS.map(({ field, key }) => (
          <label key={field} style={s.checkLabel}>
            <input
              type="checkbox"
              style={s.checkBox}
              checked={selfCheck[field]}
              onChange={(e) =>
                onSelfCheckChange({ ...selfCheck, [field]: e.target.checked })
              }
            />
            {w(key)}
          </label>
        ))}
      </div>

      {/* Collapsed by default, and opening it is RECORDED. Not to grade her —
          so Chuck knows where the help was needed (ruling R3). */}
      <details
        style={s.stronger}
        onToggle={(e) => {
          if ((e.currentTarget as HTMLDetailsElement).open) onHelpOpened();
        }}
      >
        <summary style={s.strongerSummary}>{w("stronger_summary")}</summary>
        {/* Three states, not two. A stored example is shown as written. A
            REDIRECT with none shows the line that belongs to a redirect —
            "Tell it — this is Chuck's time." — because the drill's ordinary
            "no receipt for this one, that's a Chuck question" sentence is
            exactly wrong here: a redirect is the one question where telling it
            at length IS the right answer, and there is nothing missing. Any
            other question with none shows the honest-gap line. */}
        <div style={s.strongerExample}>
          {question.stronger ??
            (question.kind === "redirect"
              ? w("redirect_stronger_line")
              : w("stronger_no_receipt"))}
        </div>
        {question.stronger !== null && question.stronger_lean !== null && (
          <div style={s.strongerLean}>{question.stronger_lean}</div>
        )}
        <div style={s.strongerNote}>
          {w("stronger_note_prefix")} <i>{w("stronger_note_emphasis")}</i>
          {w("stronger_note_suffix")}
        </div>
        {/* Standing Rule 1: a failed write says so on screen. It sits INSIDE the
            drawer, beside the thing whose recording failed, rather than as a
            banner over the whole reveal — the failure is about Chuck's sheet,
            not about her answer, and a page-level alarm would say otherwise. */}
        {helpNotRecorded && (
          <div style={{ ...s.strongerNote, color: s.RED }} role="alert">
            {w("help_not_recorded")}
          </div>
        )}
      </details>

      {/* Standing Rule 1. It sits ABOVE the two buttons because the action it
          asks for is pressing one of them again. */}
      {markError !== null && (
        <div style={{ ...s.feedback, marginTop: 20 }} role="alert">
          {markError}
        </div>
      )}

      <div style={{ ...s.row, marginTop: 20 }}>
        <button type="button" style={s.buttonPrimary} onClick={onNext}>
          {w("next_button")}
        </button>
        <button type="button" style={s.button} onClick={onAgainLater}>
          {w("again_button")}
        </button>
      </div>
    </section>
  );
};

export default PracticeReveal;
