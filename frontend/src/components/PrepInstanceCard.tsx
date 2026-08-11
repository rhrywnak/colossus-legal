// =============================================================================
// PrepInstanceCard — the prep page's adapter over the shared pair card (task R4)
// =============================================================================
//
// This was the whole card until task R4. P3 made the card SHARED — the working
// page's accusation section renders the identical component now — so what is
// left here is the adapter: turn one `RehearsalInstance` into the model
// `PairCard` reads, and hand it this page's own words.
//
// ## What the prep page passes, and what it deliberately does not
//
// No `controls`. Marie reads this page in front of opposing counsel and there is
// nothing on it for her to press.
//
// No `gapNotice` either, and that is ruling C5 rather than an omission: the
// sentence naming an unanswered instance lives ONCE, in the prep list, which is
// what the header counts and what stays visible when the section is folded.
// Rendering it here as well was the beta.381 duplicate-gap defect.
//
// ## Why the chronology comment moved out
//
// The ordering that makes this list the page's timeline is the backend's
// (`rehearsal_instances::walk_instances`), and it was documented here only
// because this file used to be where a reader arrived. It is documented there.

import React from "react";

import PairCard from "./PairCard";
import { pairCardFromRehearsalAnswer, pairCardFromRehearsalInstance } from "./pairCardModel";
import type { RehearsalInstance, RehearsalWording } from "../services/rehearsal";

interface Props {
  instance: RehearsalInstance;
  wording: RehearsalWording;
}

const PrepInstanceCard: React.FC<Props> = ({ instance, wording }) => (
  <li id={`instance-${instance.position}`}>
    <PairCard
      card={{
        ...pairCardFromRehearsalInstance(instance),
        answer: instance.answer ? pairCardFromRehearsalAnswer(instance.answer) : null,
      }}
      answerLabel={wording.answer_label}
      // NO WORDS, so the fold renders as a chevron (see `PairCard.FoldedQuote`).
      // This page's nearest stored pair is `expand_all_label` /
      // `collapse_all_label`, and those mean "expand every instance" — under one
      // quote they would say something untrue. A row of its own is filed.
      showLabel={null}
      hideLabel={null}
      // See the header: the gap sentence lives once, in the prep list.
      gapNotice={null}
    />
  </li>
);

export default PrepInstanceCard;
