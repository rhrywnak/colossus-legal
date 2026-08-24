// PrintAntecedent.tsx — the defense question a redirect repairs, quoted above it.
//
// ## Why this is shared and not copied
//
// It is drawn on BOTH printed sheets — the questions sheet Chuck marks up and
// the answers sheet he reads — and Roman's ruling of 2026-08-23 was explicit:
// do not invent a new visual language for the answers sheet, reuse what the
// questions sheet does. Two copies would be two things to keep in step, and the
// one that drifted would be the one nobody was looking at.
//
// ## ⚑ Why a redirect NEEDS this, on both sheets
//
// A redirect read on its own means nothing — it repairs a specific defense
// question, and without that question in front of him Chuck cannot tell whether
// it repairs it. On the questions sheet the antecedent is the pointer. On the
// answers sheet it is the ONLY pointer: the code that used to carry that role
// ("after G3") is being removed from the receipts, because it named something
// that appears nowhere else on paper or on screen.
//
// ## Domain note: the question TEXT, never its ANSWER
//
// Even on the answers sheet. An answer printed in two places is two things that
// can disagree, and Chuck reads the defense sheet first.

import React from "react";

import type { PracticeWording } from "../../services/practice";
import { wordingOf } from "../../services/practice";
import type { PrintRow } from "./printSheetPlan";
import * as p from "./printStyles";

const PrintAntecedent: React.FC<{
  after: PrintRow["after"];
  wording: PracticeWording;
}> = ({ after, wording }) => {
  if (after === null) return null;

  // The question it repairs is gone from the deck. Said plainly rather than
  // withheld: a redirect with no antecedent is not a redirect Chuck can judge,
  // and silence would leave him judging it as though it stood alone.
  if (after.kind === "missing") {
    return <p style={p.after}>{wordingOf(wording, "print_after_missing")}</p>;
  }

  // The template carries `{question}` and nothing else. It carried a `{key}`
  // until 2026-08-23, when question codes left the paper — the quoted question
  // IS the identification now, and it is the one Chuck can act on without a
  // lookup.
  const [before, quoted] = wordingOf(wording, "print_after_template").split("{question}");
  return (
    <p style={p.after}>
      {before}
      <i style={p.afterQuote}>“{after.antecedent.text}”</i>
      {quoted ?? ""}
    </p>
  );
};

export default PrintAntecedent;
