// =============================================================================
// PracticeSheet.tsx — screen S3 of PRACTICE_MOCKUP_v2, and the only print
// =============================================================================
//
// The sheet Chuck reads: her questions, her answers verbatim, fine/repeat, and
// where she opened the help.
//
// ## FRE/MRE 612
//
// This is the ONLY print control in the drill, and there is deliberately no
// "print for Marie" anywhere. What a witness reviews to refresh memory may be
// discoverable; until Chuck rules, the session is on screen only and the printed
// sheet goes to him. The absence of the other button is a decision, not a gap.
//
// ## Every cell arrived as a word
//
// "opened", "repeat", "George · braid" — all composed server-side from stored
// rows. This component decides emphasis and nothing else; it holds no vocabulary
// to translate a boolean into.

import React from "react";

import type { PracticeSheet as Sheet, PracticeWording } from "../../services/practice";
import { wordingOf } from "../../services/practice";
import * as s from "./practiceStyles";

interface Props {
  sheet: Sheet;
  wording: PracticeWording;
  onPracticeAgain: () => void;
}

/**
 * The colour of one mark cell.
 *
 * ## Why this is a three-way and not `repeat ? red : green`
 *
 * It was that, and flow v1's third mark turned the fallback into a liar: a
 * SKIPPED row printed green — the sheet telling Chuck she answered a question
 * she had set aside. The backend's `mark_cell` had the identical defect and was
 * fixed in the same change; this is the screen's half of it.
 *
 * The comparison is against the STORED words, not against literals, because the
 * cell already holds the rendered word and the three vocabularies are editable.
 * An unrecognised mark gets no colour at all rather than a wrong one.
 */
const markStyle = (mark: string, w: (key: string) => string) => {
  if (mark === w("mark_repeat")) return s.markRepeat;
  if (mark === w("mark_skipped")) return s.markSkipped;
  if (mark === w("mark_fine")) return s.markFine;
  return undefined;
};

const PracticeSheet: React.FC<Props> = ({ sheet, wording, onPracticeAgain }) => {
  const w = (key: string) => wordingOf(wording, key);

  const columns = [
    w("sheet_col_number"),
    w("sheet_col_from"),
    w("sheet_col_tactic"),
    w("sheet_col_question"),
    w("sheet_col_answer"),
    w("sheet_col_mark"),
    w("sheet_col_help"),
  ];

  return (
    // `data-practice-print` is what the print stylesheet keeps; everything else
    // on the page is hidden, so the printed page is the table and nothing else.
    <section style={s.card} data-practice-print>
      <div style={s.kicker}>{sheet.kicker}</div>
      <h2 style={s.h2}>{sheet.heading}</h2>
      <p style={s.sub}>
        {w("sheet_sub_prefix")}{" "}
        <span style={s.markRepeat}>{w("mark_repeat")}</span> {w("sheet_sub_suffix")}
      </p>

      <table style={s.table}>
        <thead>
          <tr>
            {columns.map((label, i) => (
              // The index is the key because two columns may legitimately share
              // a label one day (both dashes, for instance) and the position is
              // what actually identifies a column here.
              <th key={i} style={s.headerCell}>
                {label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {sheet.rows.map((row) => (
            <tr key={row.number}>
              <td style={s.cell}>{row.number}</td>
              <td style={s.cell}>{row.from}</td>
              <td style={s.cell}>{row.tactic}</td>
              <td style={s.cell}>{row.question}</td>
              <td style={s.cell}>
                <i>{row.answer}</i>
              </td>
              <td style={{ ...s.cell, ...markStyle(row.mark, w) }}>
                {row.mark}
              </td>
              <td style={s.cell}>{row.help}</td>
            </tr>
          ))}
        </tbody>
      </table>

      {/* Flagged before the session — the whole DECK's flags, not this
          sitting's. A question Marie flagged AND kept out of tonight is the one
          Roman most needs to see. The block withdraws entirely when nothing is
          flagged: a heading over an empty list reads as a list that failed to
          load. It PRINTS — Chuck gets it with the sheet. */}
      {sheet.flagged.length > 0 && (
        <div style={{ marginTop: 18, fontSize: 15 }}>
          <b>{sheet.flagged_heading}</b> {sheet.flagged_hint}
          <ul>
            {sheet.flagged.map((line) => (
              <li key={line} style={{ margin: "4px 0" }}>
                {line}
              </li>
            ))}
          </ul>
        </div>
      )}

      <div style={{ ...s.row, marginTop: 20 }} data-practice-no-print>
        <button type="button" style={s.buttonPrimary} onClick={onPracticeAgain}>
          {w("sheet_again_button")}
        </button>
        <button type="button" style={s.button} onClick={() => window.print()}>
          {w("print_button")}
        </button>
        <span style={{ ...s.sub, marginLeft: "auto" }}>{w("homelab_line")}</span>
      </div>
    </section>
  );
};

export default PracticeSheet;
