// =============================================================================
// RehearsalAccusationBlock — the accusation, every instance, and what is missing
// =============================================================================
//
// The page's central block, reproducing the mockup's `.accusation`, its
// `.attribution`, `.countline`, the instance list, and `.preplist`.
//
// ## The duplicate-gap defect, and the shape that ends it (ruling C5)
//
// beta.381 rendered the full "NO ANSWER PREPARED — who, when, where" sentence on
// the row AND in the prep list. It renders here at three deliberately different
// zoom levels, and only one of them is the sentence:
//
//   · the row's meta line       — a three-word tag              (the row)
//   · an OPENED row             — a three-word banner           (the row)
//   · "What still needs preparing" — the sentence, once, with a jump link
//
// All three arrive composed. This component holds no template and could not
// rebuild the sentence in the other two places if it tried.
//
// ## Why the jump link expands the row it points at
//
// The prep list is what a human can act on today. Scrolling them to a compact
// row would land them on a one-line summary of the thing they came to fix, so
// the link opens the row as well as scrolling to it.

import React from "react";

import RehearsalInstanceRow from "./RehearsalInstanceRow";
import SentenceEditor from "./SentenceEditor";
import {
  attributionStyle,
  editButtonStyle,
  editorFieldStyle,
} from "./rehearsalStyles";
import {
  accusationStyle,
  countLineStyle,
  prepHeadingStyle,
  prepItemStyle,
  prepJumpStyle,
  prepListStyle,
} from "./rehearsalRowStyles";
import type {
  RehearsalAccusation,
  RehearsalGap,
  RehearsalWording,
} from "../services/rehearsal";

interface Props {
  accusation: RehearsalAccusation;
  wording: RehearsalWording;
  /** Which instance rows are open, by position. */
  openRows: ReadonlySet<number>;
  onToggleRow: (position: number) => void;
  /** Opens a row and scrolls to it — the prep list's jump. */
  onJumpToRow: (position: number) => void;
  /** Store or withdraw the sentence. `null` withdraws. */
  onSaveText: (text: string | null) => void;
  busy: boolean;
  /** Prefix for the row anchors, so two scenarios cannot collide. */
  anchorPrefix: string;
}

/** One entry in "What still needs preparing". */
const PrepItem: React.FC<{
  gap: RehearsalGap;
  jumpLabel: string;
  onJump: (position: number) => void;
}> = ({ gap, jumpLabel, onJump }) => (
  <div style={prepItemStyle}>
    {gap.message}
    {/* Offered only for the gaps that HAVE a rendered row. A removed accusation
        and an unloadable instance have none, and a link that scrolled somewhere
        would land the reader on somebody else's statement. */}
    {gap.position !== null && (
      <button type="button" style={prepJumpStyle} onClick={() => onJump(gap.position ?? 0)}>
        {jumpLabel} ↑
      </button>
    )}
  </div>
);

const RehearsalAccusationBlock: React.FC<Props> = ({
  accusation,
  wording,
  openRows,
  onToggleRow,
  onJumpToRow,
  onSaveText,
  busy,
  anchorPrefix,
}) => (
  <>
    <SentenceEditor
      text={accusation.text}
      // Never a quote from the record standing in for it — that was the defect
      // task 2.11 B1 ended at the root.
      missingNotice={accusation.text_gap ?? ""}
      wording={{
        editLabel: wording.editor.edit_label,
        saveLabel: wording.editor.save_label,
        cancelLabel: wording.editor.cancel_label,
        withdrawLabel: wording.editor.withdraw_label,
        placeholder: wording.editor.accusation_placeholder,
      }}
      onSave={onSaveText}
      busy={busy}
      sentenceStyle={accusationStyle}
      buttonStyle={editButtonStyle}
      fieldStyle={editorFieldStyle}
    >
      {/* Who wrote it and when — a finished line, or the stored notice saying
          the authorship was never recorded. Absent entirely when there is no
          sentence: an authorship line over a named gap attributes an absence. */}
      {accusation.attribution && (
        <div style={attributionStyle}>{accusation.attribution}</div>
      )}
    </SentenceEditor>

    {/* Exactly one of these is ever present — the backend decides. */}
    <div style={countLineStyle}>
      {accusation.count_line ?? accusation.no_instances_notice}
      {/* Rows open on click, and a feature discoverable only by trying it is a
          feature nobody finds. */}
      {accusation.count_line && ` ${wording.row_open_hint}`}
    </div>

    {accusation.instances.map((instance) => (
      <RehearsalInstanceRow
        key={instance.position}
        instance={instance}
        answerLabel={wording.answer_label}
        open={openRows.has(instance.position)}
        onToggle={() => onToggleRow(instance.position)}
        anchorId={`${anchorPrefix}-row-${instance.position}`}
      />
    ))}

    {accusation.gaps.length > 0 && (
      <div style={prepListStyle}>
        <h4 style={prepHeadingStyle}>{wording.prep_list_heading}</h4>
        {accusation.gaps.map((gap, i) => (
          <PrepItem
            key={`${gap.kind}:${i}`}
            gap={gap}
            jumpLabel={wording.go_to_row_label}
            onJump={onJumpToRow}
          />
        ))}
      </div>
    )}
  </>
);

export default RehearsalAccusationBlock;
