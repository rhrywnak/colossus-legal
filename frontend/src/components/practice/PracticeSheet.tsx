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
              <td
                style={{
                  ...s.cell,
                  ...(row.mark === w("mark_repeat") ? s.markRepeat : s.markFine),
                }}
              >
                {row.mark}
              </td>
              <td style={s.cell}>{row.help}</td>
            </tr>
          ))}
        </tbody>
      </table>

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
