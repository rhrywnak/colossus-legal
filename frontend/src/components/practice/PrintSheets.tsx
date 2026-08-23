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
import * as p from "./printStyles";

type Props = {
  plan: PrintPlan;
  code: string;
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
const After: React.FC<{ after: PrintRow["after"]; wording: PracticeWording }> = ({
  after,
  wording,
}) => {
  if (after === null) return null;
  if (after.kind === "missing") {
    return <p style={p.after}>{wordingOf(wording, "print_after_missing")}</p>;
  }
  const template = wordingOf(wording, "print_after_template");
  const key = after.antecedent.deck_key ?? "";
  const [before, quoted] = template.split("{question}");
  return (
    <p style={p.after}>
      {fill(before, { key })}
      <i style={p.afterQuote}>“{after.antecedent.text}”</i>
      {quoted ?? ""}
    </p>
  );
};

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
      {row.question.deck_key !== null && <span style={p.key}>{row.question.deck_key}</span>}
      <Tag question={row.question} />
      {row.question.draft_by !== null && (
        <span style={p.draft}>{wordingOf(wording, "badge_draft")}</span>
      )}
    </div>
    <After after={row.after} wording={wording} />
    <p style={p.qtx}>{row.question.text}</p>
    <Source question={row.question} />
    <Lines count={row.after === null ? 2 : 3} />
  </div>
);

const Sheet: React.FC<{
  sheet: PrintSheet;
  index: number;
  total: number;
  deckTotal: number;
  code: string;
  deckAsOf: string | null;
  printedAt: string;
  wording: PracticeWording;
  tail: React.ReactNode;
}> = ({ sheet, index, total, deckTotal, code, deckAsOf, printedAt, wording, tail }) => {
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

      <div style={p.ftr}>
        <span>
          {fill(w("print_footer_template"), {
            code,
            sheet: sheet.title,
            n: String(sheet.rows.length),
          })}
        </span>
        {/* SHEET, not page. The browser paginates physical pages itself; a sheet
            with eight questions runs onto two of them and both would otherwise
            have claimed to be the same numbered page. */}
        <span>
          {fill(w("print_sheet_number_template"), {
            n: String(index + 1),
            m: String(total),
          })}
        </span>
      </div>
    </div>
  );
};

/**
 * Up to three sheets, in order, and only the ones with questions on them.
 *
 * A sheet with no questions is not printed at all and the sheet count adjusts —
 * a defense-only deck prints "sheet 1 of 1", never three sheets two of which are
 * a heading over white space. Most decks are partial; that is the normal case.
 */
const PrintSheets: React.FC<Props> = ({ plan, code, deckAsOf, printedAt, wording }) => {
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
          index={index}
          total={plan.sheets.length}
          deckTotal={plan.deckTotal}
          code={code}
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
