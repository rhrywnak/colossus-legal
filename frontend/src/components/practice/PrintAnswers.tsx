// PrintAnswers.tsx — the same sheets, carrying what Marie wrote.
//
// ## What is different from the questions sheet, and why each thing is
//
// NO RULED LINES. Chuck is reading, not marking. The lines exist on the other
// document because he develops questions on it; here they would invite him to
// annotate an answer, which is a conversation he has with Marie in person.
//
// NO FOOTER, for the same reason its sibling has none: nothing trails a sheet's
// content, so no sheet can end on a page carrying only a caption. Chrome's own
// header carries the scenario and the page number onto every physical page.
//
// HER ANSWER, and the day she gave it. Both: paper outlives the deck it came
// from, and a sheet carrying an answer without its date cannot tell a reader
// whether he is holding last week's.
//
// ## Domain note: the CURRENT answer only
//
// Not the earlier versions. Chuck is reading what Marie would say today; three
// versions of one answer would ask him to work out which is live, which is the
// one job the screen already does for him.

import React from "react";

import type { PracticeWording } from "../../services/practice";
import { wordingOf } from "../../services/practice";
import { answerLine, type AnswerPlan } from "./printAnswerPlan";
import type { PrintRow } from "./printSheetPlan";
import * as p from "./printStyles";
import * as a from "./printAnswerStyles";

const Row: React.FC<{
  row: PrintRow;
  plan: AnswerPlan;
  wording: PracticeWording;
}> = ({ row, plan, wording }) => {
  const answer = plan.answers.get(row.question.id) ?? null;
  const line = answerLine({ question: row.question, answer }, wording);
  return (
    // `data-print-row` carries `break-inside: avoid`: an answer split from the
    // question it answers is two half-facts on two pieces of paper.
    <div style={p.qb} data-print-row>
      <p style={p.qtx}>{row.question.text}</p>
      <div style={answer === null ? a.absent : a.answer}>
        {line.when !== null && <div style={a.when}>{line.when}</div>}
        <p style={a.text}>{line.text}</p>
      </div>
    </div>
  );
};

const PrintAnswers: React.FC<{
  plan: AnswerPlan;
  deckAsOf: string | null;
  printedAt: string;
  wording: PracticeWording;
}> = ({ plan, deckAsOf, printedAt, wording }) => {
  const w = (key: string) => wordingOf(wording, key);
  return (
    <>
      {plan.plan.sheets.map((sheet) => (
        <div key={sheet.kind} style={p.sheet} data-print-sheet>
          <div style={p.hdr}>
            <div style={p.hdrTitle}>
              {sheet.title}
              <small style={p.hdrSub}>{sheet.subtitle}</small>
            </div>
            <div style={p.hdrMeta}>
              <div>{printedAt}</div>
              {deckAsOf !== null && <div>{deckAsOf}</div>}
            </div>
          </div>
          {/* The answers sheet's own how-to, not the questions sheet's: that one
              tells Chuck how to mark up and where to type his changes, and
              neither is what he is doing here. */}
          <p style={p.howto}>{w("print_answers_howto")}</p>

          {sheet.rows.map((row) => (
            <Row key={row.question.id} row={row} plan={plan} wording={wording} />
          ))}
        </div>
      ))}
    </>
  );
};

export default PrintAnswers;
