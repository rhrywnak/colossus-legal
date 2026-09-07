// =============================================================================
// FactCardRows.tsx — the two rows a v2.1 fact card actually renders
// =============================================================================
//
// Split out of `FactCardBody` when change E landed and pushed that file past the
// 300-line limit (CLAUDE.md rule 17). The split is not arbitrary: what is here is
// the two rows this task ADDED or REPLACED — the proof as a Q&A pair, and the
// Backs picker — while `FactCardBody` keeps the card's frame (the title bar, the
// source line, the save path, the error line) and the generic row renderer that
// draws whatever `cardRows` returns.
//
// ## Every visible word is served
//
// The two exceptions are `Q:` and `A:`, which are STRUCTURAL marks rather than
// vocabulary and are literals in `EvidenceCardParts.ExchangeRow` for the same
// reason. They live in `FactCardBody` beside the props that pair with them.

import React, { useState } from "react";

import { foldQuestion } from "./evidenceCardModel";
import type { ScenarioCard } from "../services/scenarioCards";
import type { CardGrammarWording, FactCardWording } from "../services/evidenceLinks";
import type { TalkingPointDto } from "../services/scenarioAugmentation";
import {
  INLINE_EDIT_STYLE,
  QUOTE_STYLE,
  ROW_LABEL_STYLE,
  ROW_VALUE_STYLE,
  SELECT_STYLE,
} from "./factCardStyles";

// STRUCTURAL: the two marks of a question-and-answer pair. Not vocabulary and
// therefore not stored rows — the same two characters a transcript prints, in
// every language this app has ever been asked for. `EvidenceCardBody` passes
// exactly these two literals to `ExchangeRow`; declared once here so the two
// lines of this card cannot drift apart from each other or from that sibling.
const QUESTION_PREFIX = "Q:";
const ANSWER_PREFIX = "A:";

/**
 * The proof: the record's own words, as a `Q:` / `A:` pair when it has both.
 *
 * ## Domain note: the question is HALF the evidence on a discovery answer
 *
 * "Admitted." is not evidence of anything on its own — what was admitted lives in
 * the request for admission it answers. The card rendered the answer alone until
 * v2.1, which is why a reader of C-40 saw one word under PROOF and had to open the
 * PDF to learn what it meant.
 *
 * ## Rust Learning parallel: `Option<String>` is not `String`
 *
 * `card.quote.question` is `Option<String>` on the wire (`dto::scenario_card`),
 * and the two states are genuinely different: `null` means "this statement is not
 * a Q&A pair at all" — transcript speech, a letter — and renders ONE line, the
 * statement itself. An empty string would mean "a Q&A pair whose question we do
 * not have", which is a gap somebody owes work on. Collapsing the two with a
 * `?? ""` is exactly the silent failure Standing Rule 1 forbids, so the branch is
 * on presence and nothing else.
 *
 * The fold length is SERVED (`card_question_truncate_chars`) and the show/hide
 * words are stored rows — the same three values `EvidenceCardBody` folds by, so
 * the same question folds identically wherever it is read.
 */
export const ProofRows: React.FC<{
  card: ScenarioCard;
  wording: FactCardWording;
  /** The question's fold words. They live on the card GRAMMAR block, not the
   *  fact-card block, because the fold is shared with `EvidenceCardBody`. */
  grammar: CardGrammarWording;
  questionChars: number;
}> = ({ card, wording, grammar, questionChars }) => {
  const [questionOpen, setQuestionOpen] = useState(false);
  const question = card.quote.question?.trim();
  const folded = question ? foldQuestion(question, questionChars) : null;

  return (
    <>
      <div style={ROW_LABEL_STYLE}>{wording.proof_label}</div>
      <div style={ROW_VALUE_STYLE}>
        {question && folded && (
          // The Q line leads and is MUTED, exactly as it is under the other
          // wrapper: the answer is the evidence, the question is what makes it
          // legible. A prominent question is what pushed the answer off the
          // screen before ONE_CARD_GRAMMAR folded it.
          <div style={{ color: "var(--text-secondary)", marginBottom: "4px" }}>
            {/* `Q:` and `A:` are literals here for the same reason they are
                literals in `EvidenceCardParts.ExchangeRow`, which is the
                precedent this card is copying: they are STRUCTURAL marks, not
                vocabulary — the same two characters a transcript prints, in
                every language this app has ever been asked for. Every actual
                word on this card is still a stored row. */}
            <b style={{ fontWeight: 600 }}>{QUESTION_PREFIX}</b>{" "}
            {questionOpen ? question : folded.short}
            {folded.truncated && (
              <button
                type="button"
                style={INLINE_EDIT_STYLE}
                // The card's title bar is a toggle; without this the click that
                // opens the question also collapses the card under it.
                onClick={(event) => {
                  event.stopPropagation();
                  setQuestionOpen(!questionOpen);
                }}
              >
                {questionOpen
                  ? grammar.question_collapse_label
                  : grammar.question_expand_label}
              </button>
            )}
          </div>
        )}
        <div>
          {/* The `A:` prefix is drawn ONLY when there is a `Q:` above it to pair
              with. On a statement that is not a Q&A pair the quote IS the proof,
              and an `A:` with nothing answering would be the card asserting a
              structure the record does not have. */}
          {question && (
            <b style={{ fontWeight: 600, color: "var(--text-secondary)" }}>
              {ANSWER_PREFIX}{" "}
            </b>
          )}
          <span style={QUOTE_STYLE}>{card.quote.text}</span>
        </div>
      </div>
    </>
  );
};

/**
 * Which talking point this fact backs — the one judgment set from this card.
 *
 * ## Why a picker and not the text box it replaces
 *
 * `backs_position` is a POINTER at a talking point, and the old row edited it as
 * free text: a human could type `7` on a scenario with three points, the write
 * would be accepted, and the composed `backs` sentence would come back `null`
 * because no point occupies position 7 — rendering as an em dash with nothing
 * saying why. The options here are the LIVE points, so an unoccupied position
 * cannot be chosen at all.
 *
 * ## Every word is served, including "none"
 *
 * There is no stored row for the word "none", and inventing one in code is what
 * the language law forbids. `wording.empty_value` — the same em dash every empty
 * field on this card already shows — is the vocabulary this card already has for
 * "nothing here", so it is what the clearing option wears. Each point's option
 * carries its own sentence as a `title`, because a bare position number says
 * nothing about which argument it is.
 */
export const BacksPicker: React.FC<{
  wording: FactCardWording;
  points: TalkingPointDto[];
  current: number | null;
  busy: boolean;
  onPick: (position: number | null) => void;
}> = ({ wording, points, current, busy, onPick }) => (
  <>
    <div style={ROW_LABEL_STYLE}>{wording.backs_label}</div>
    <div style={ROW_VALUE_STYLE}>
      <select
        aria-label={wording.backs_label}
        disabled={busy}
        value={current === null ? "" : String(current)}
        // The title bar above is a toggle and clicks bubble; without this,
        // choosing a point also folds the card you were reading.
        onClick={(event) => event.stopPropagation()}
        onChange={(event) => {
          const raw = event.target.value;
          onPick(raw === "" ? null : Number.parseInt(raw, 10));
        }}
        style={SELECT_STYLE}
      >
        <option value="">{wording.empty_value}</option>
        {points.map((point) => (
          <option key={point.position} value={String(point.position)} title={point.text}>
            {point.position}
          </option>
        ))}
      </select>
    </div>
  </>
);
