// =============================================================================
// ForYouRow.tsx — one row on the "For you" page (CC_TASK_FOR_YOU_v1 L1)
// =============================================================================
//
// Three lines and a time, all of them composed by the server. This component
// decides exactly two things: where the row goes when it is clicked, and
// whether it is drawn as already read.
//
// ## The WHOLE row is the link
//
// The mockup's rows have no separate control, and that is right: a row is one
// thing waiting, and hunting for the clickable part of it is friction on the
// page whose whole job is to remove friction. A real `<a>` rather than an
// onClick div, so the keyboard, the middle button and "open in new tab" all
// work without this file knowing about any of them.

import React from "react";
import { Link } from "react-router-dom";

import type { ForYouRow as Row } from "../../services/forYou";
import { practicePath, practiceQuestionPath, practiceReviewPath } from "../../utils/routePaths";
import * as s from "./forYouStyles";

type Props = {
  row: Row;
  slug: string;
};

/**
 * Where this row opens.
 *
 * A row about a question opens that question, carrying `from=for-you` — which
 * is what tells the question page to mark the question read (ruling Q4). A note
 * about a whole scenario has no question to open, so it opens the deck; it does
 * not clear, and L2's sweep is what will clear it.
 *
 * A DECK row opens the deck's REVIEW page (L3): it stands for several answers
 * on several questions, which is precisely the page built to read them together
 * and clear them together with Done reviewing.
 */
export function rowHref(row: Row, slug: string): string {
  // STRUCTURAL: `deck` is the wire kind the server gives a grouped row —
  // `RowVoice::deck_row` in `backend/src/services/for_you_rows.rs`. Renaming it
  // on either side changes the format, and this branch must change with it.
  if (row.kind === "deck") {
    return practiceReviewPath(slug, row.scenario_id);
  }
  return row.question_id === undefined
    ? practicePath(slug, row.scenario_id)
    : practiceQuestionPath(slug, row.scenario_id, row.question_id, "for-you");
}

const ForYouRow: React.FC<Props> = ({ row, slug }) => (
  <Link
    to={rowHref(row, slug)}
    style={row.read ? s.rowRead : s.row}
    data-for-you-row={row.kind}
    data-read={row.read ? "yes" : "no"}
  >
    <div style={s.deckLine}>{row.deck_line}</div>
    <div style={s.body}>{row.body}</div>
    <div style={s.foot}>
      <span style={s.byline}>{row.byline}</span>
      <span style={s.when}>{row.when}</span>
    </div>
  </Link>
);

export default ForYouRow;
