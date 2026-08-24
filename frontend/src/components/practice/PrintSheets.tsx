// PrintSheets.tsx — the three white sheets of SCENARIO_PRINT_MOCKUP_v1.
//
// PURE: payload in, sheets out. No fetch, no navigation, no state. Everything it
// DECIDES was decided in `printSheets.ts`; this file only draws.
//
// ## What is on the paper, and what is deliberately not
//
// On it: the deck key in its blue box, the tactic tag where one exists, the
// question in serif, the `Built from:` line, and ruled space. Redirects also
// carry the draft badge and the defense question they repair, quoted above.
//
// NOT on it: watch-fors, stronger answers, notes, Marie's answers, any read.
// **This is question development, not practice material** — which is also why
// nothing here needs an FRE/MRE 612 ruling: nothing of Marie's is on the page.

import React from "react";

import { wordingOf, type PracticeQuestion, type PracticeWording } from "../../services/practice";
import { fill, type PrintPlan, type PrintRow, type PrintSheet } from "./printSheetPlan";
import PrintAntecedent from "./PrintAntecedent";
import * as p from "./printStyles";

type Props = {
  plan: PrintPlan;
  /** The deck's own last change, already formatted. `null` when unknown. */
  deckAsOf: string | null;
  /** When this copy came off the printer, already formatted. */
  printedAt: string;
  wording: PracticeWording;
};

/**
 * The `Built from:` line, rendered VERBATIM.
 *
 * ## Why no prefix is added here
 *
 * The mockup shows a uniform "Built from:" on every row. The stored column is not
 * uniform — **[measured]** it holds "Built from: …", "Built from what Phillips
 * told the court: …", "A braid — Barrage rows 1 + 2 + 5. …" and "Chuck's direct —
 * establishes point 1 …". A code-side prefix would print "Built from: Chuck's
 * direct — establishes point 1". The data carries its own opening; this renders it.
 */
const Source: React.FC<{ question: PracticeQuestion }> = ({ question }) =>
  question.receipt === null ? null : <p style={p.src}>{question.receipt}</p>;

/** The tactic tag, with the braid suffix already resolved by the server. */
const Tag: React.FC<{ question: PracticeQuestion }> = ({ question }) =>
  question.tactic === null ? null : <span style={p.tac}>{question.tactic}</span>;

/**
 * The defense question a redirect repairs, quoted above it.
 *
 * A redirect read alone means nothing, and the judgement Chuck is making is
 * whether this repairs that. When `follows_key` names nothing in the deck the
 * absence is SAID — a blank quote box would read as a redirect that repairs
 * nothing, which is a different claim.
 */

/** Two ruled lines for a question, three for a redirect — as the mockup has it. */
const Lines: React.FC<{ count: number }> = ({ count }) => (
  <div style={p.lines}>
    {Array.from({ length: count }, (_, i) => (
      <div key={i} style={p.line} />
    ))}
  </div>
);

const Row: React.FC<{ row: PrintRow; wording: PracticeWording }> = ({ row, wording }) => (
  // `data-print-row` is what carries `break-inside: avoid` into print: a question
  // split from its own ruled space is a note written under the wrong question.
  <div style={p.qb} data-print-row>
    <div style={p.qtop}>
      {/* No question code and no draft badge. Codes left the screen with the
          sequential number they contradicted, and paper that carried one Chuck
          could not find on screen was the whole defect. The draft badge went
          with them: `draft_by` is never populated — Roman's manual process
          covers telling Chuck what is a draft — so it rendered nothing here
          while the screen showed nothing either. Screen and paper agree by
          both being silent. */}
      <Tag question={row.question} />
    </div>
    <PrintAntecedent after={row.after} wording={wording} />
    <p style={p.qtx}>{row.question.text}</p>
    <Source question={row.question} />
    <Lines count={row.after === null ? 2 : 3} />
  </div>
);

const Sheet: React.FC<{
  sheet: PrintSheet;
  deckTotal: number;
  deckAsOf: string | null;
  printedAt: string;
  wording: PracticeWording;
  tail: React.ReactNode;
}> = ({ sheet, deckTotal, deckAsOf, printedAt, wording, tail }) => {
  const w = (k: string) => wordingOf(wording, k);
  return (
    <div style={p.sheet} data-print-sheet>
      <div style={p.hdr}>
        <div style={p.hdrTitle}>
          {sheet.title}
          <small style={p.hdrSub}>{sheet.subtitle}</small>
        </div>
        <div style={p.hdrMeta}>
          {fill(w("print_printed_template"), { when: printedAt })}
          {deckAsOf !== null && (
            <>
              <br />
              {/* {n} is THIS SHEET, {m} is the WHOLE DECK — Roman's ruling. A
                  sheet showing only its own count cannot tell Chuck how much of
                  the deck he is not holding. */}
              {fill(w("print_deck_as_of_template"), {
                date: deckAsOf,
                n: String(sheet.rows.length),
                m: String(deckTotal),
              })}
            </>
          )}
        </div>
      </div>
      <p style={p.howto}>{sheet.howto}</p>

      {sheet.rows.map((row) => (
        <Row key={row.question.id} row={row} wording={wording} />
      ))}

      {tail}

      {/* NO FOOTER. Nothing trails a sheet's content, which is what makes it
          impossible for a sheet to end on a page carrying only a footer — the
          .405 defect. `break-before: avoid` was supposed to prevent that and
          measurably did not: a forced footer-only page rendered byte-identical
          with and without the rule. Chrome's own header carries the scenario
          and the page number onto every page, per Roman's ruling, so nothing
          is lost by removing ours. */}
    </div>
  );
};

/**
 * Up to three sheets, in order, and only the ones with questions on them.
 *
 * A sheet with no questions is not printed at all — a defense-only deck prints
 * one sheet, never three of which two are a heading over white space. Most decks
 * are partial; that is the normal case.
 *
 * The scenario code is not a prop: it reaches the paper inside each sheet's own
 * composed subtitle, and Chrome's page header carries it onto every physical
 * page. Taking it twice would be two places for one fact to be wrong in.
 */
const PrintSheets: React.FC<Props> = ({ plan, deckAsOf, printedAt, wording }) => {
  // What the deck does NOT contain rides the LAST sheet — it is information Chuck
  // acts on, and a sheet he is not holding cannot carry it.
  const tailFor = (index: number): React.ReactNode => {
    if (index !== plan.sheets.length - 1) return null;
    if (plan.missingLine === null && plan.hiddenLine === null) return null;
    return (
      <div style={p.absent}>
        {plan.missingLine !== null && <div>{plan.missingLine}</div>}
        {plan.hiddenLine !== null && <div>{plan.hiddenLine}</div>}
      </div>
    );
  };

  return (
    <>
      {plan.sheets.map((sheet, index) => (
        <Sheet
          key={sheet.kind}
          sheet={sheet}
          deckTotal={plan.deckTotal}
          deckAsOf={deckAsOf}
          printedAt={printedAt}
          wording={wording}
          tail={tailFor(index)}
        />
      ))}
    </>
  );
};

export default PrintSheets;
