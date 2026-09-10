// =============================================================================
// PracticeDeckGapAdd.tsx — "+ Add question here", in the gap between two rows
// =============================================================================
//
// Chuck writes a question while reading the one it belongs after, and until
// tonight the only place to put it was a box at the bottom of the deck — so
// every question added in the middle of a run had to be added at the end and
// then dragged up, past however many rows stood between. This is the same form,
// opened where he is looking.
//
// ## The form is the SAME component, deliberately
//
// `PracticeAddQuestion` knows that a tactic belongs to a cross question and
// nowhere else, that a redirect must name the George question it follows, and
// what may be attached to. A second form with its own fields would be a second
// place all of that could drift, and the drift would not show up until somebody
// filed a cross question with no tactic through one of them.
//
// ## Split out of `PracticeDeckList` for the size limit
//
// That module reached Rule 17's 300 lines with this inline. The seam is real:
// the list decides WHICH gaps exist and what a drop means, and this decides what
// one gap looks like and what pressing it sends.

import React from "react";

import type {
  PracticeAttachOption,
  PracticeWording,
} from "../../services/practice";
import type { NewQuestion } from "../../services/practiceEditor";
import PracticeAddQuestion from "./PracticeAddQuestion";
import * as d from "./practiceDeckStyles";

// The gap control's one word. A LITERAL, and OWED as a settings row — a
// migration is Roman's to author (CLAUDE.md rule 25) and this task did not
// authorise one. Deliberately not routed through `w()`:
// `dto::practice_wording_reach_tests` scans every practice source for a literal
// `w("key")` and requires each to be a field on the backend's mirror, so a key
// with no row behind it would red the suite rather than degrade.
//
// OWED: settings row — practice_editor_add_here_label = "+ Add question here"
const ADD_HERE_LABEL = "+ Add question here";

interface Props {
  /** True while this particular gap has the form open. One at a time. */
  open: boolean;
  onOpen: () => void;
  onCancel: () => void;
  /**
   * The row this gap sits ABOVE, or `null` for the gap above the first row.
   *
   * The row above, and not the row below, because that is the anchor that
   * survives the re-render: the row below the gap may be the one just added.
   */
  anchor: string | null;
  /** Add it — the caller carries it to the server with the position attached. */
  onAdd: (question: NewQuestion) => void;
  wording: PracticeWording;
  attachOptions: PracticeAttachOption[];
  /** True when a write may be attempted — none is in flight. */
  ready: boolean;
}

/**
 * One gap: a thin line, or the add form that line opens.
 *
 * ## Why `at_start` and not `after: null`
 *
 * `after: null` is what the bottom "+ Add a question" box sends, and the server
 * reads it as the END of the side — the behaviour that route has always had. The
 * gap above the first row is the opposite end of the same list, and one absent
 * value cannot mean both. So the top gap sends its own flag, and the two ends
 * stay distinguishable; without it, pressing the topmost line would put the
 * question at the bottom of the deck, which is the one place the person pressing
 * it did not point at.
 */
const PracticeDeckGapAdd: React.FC<Props> = ({
  open,
  onOpen,
  onCancel,
  anchor,
  onAdd,
  wording,
  attachOptions,
  ready,
}) =>
  open ? (
    <PracticeAddQuestion
      wording={wording}
      attachOptions={attachOptions}
      ready={ready}
      onAdd={(question) =>
        onAdd({ ...question, after: anchor, at_start: anchor === null })
      }
      onCancel={onCancel}
    />
  ) : (
    <button type="button" style={d.addGapLine} onClick={onOpen}>
      {ADD_HERE_LABEL}
    </button>
  );

export default PracticeDeckGapAdd;
