// =============================================================================
// RehearsalScenarioBlocks — one ready scenario, as the signed mockup lays it out
// =============================================================================
//
// Five foldable sections, in the mockup's order:
//
//   1  What this is                 (editable — the theme sentence)
//   2  The accusation + instances   (editable — the plain-words sentence)
//   3  The timeline, drawn
//   4  Your points, in your words   (editable — per row, and add)
//   5  Watch for                    (editable — per row, and add)
//
// The Always strip is the page's, not this component's: it never folds, and §10
// makes it the one thing never scrolled away from.
//
// ## What changed in 2.11 C, and what did not
//
// The RENDER was rebuilt to the signed mockup. The laws did not move: every word
// still arrives from the store, every absence is still a NAMED gap, every count
// still travels composed beside the block it counts, and "What this is" gained a
// caret because the mockup gives it one — B2 had it fixed open on the argument
// that folding one sentence saves nothing, and the mockup is the spec.
//
// ## Every word is the store's
//
// There is not one user-facing literal below. A block with nothing in it renders
// its NAMED gap, never a blank — the honest-gap law, which is what the page's
// whole credibility rests on.

import React from "react";

import RehearsalAccusationBlock from "./RehearsalAccusationBlock";
import RehearsalPointsBlock from "./RehearsalPointsBlock";
import RehearsalSection from "./RehearsalSection";
import RehearsalTimelineBlock from "./RehearsalTimelineBlock";
import RehearsalWatchBlock from "./RehearsalWatchBlock";
import SentenceEditor from "./SentenceEditor";
import {
  attributionStyle,
  editButtonStyle,
  editorFieldStyle,
  whatLineStyle,
} from "./rehearsalStyles";
import type { RehearsalPayload, RehearsalScenario } from "../services/rehearsal";
import type { OpenSections } from "../pages/rehearsalSections";

/** Everything the five blocks need to WRITE, in one bundle. */
export interface RehearsalEdits {
  /** The theme sentence. Never withdrawable — the mockup offers no such control. */
  onSaveWhat: (text: string | null) => void;
  /** The plain-words accusation. `null` withdraws it. */
  onSaveAccusation: (text: string | null) => void;
  onEditPoint: (position: number, text: string) => Promise<void>;
  onAddPoint: (text: string) => Promise<void>;
  onEditWatchItem: (id: string, text: string) => Promise<void>;
  onAddWatchItem: (text: string) => Promise<void>;
  /** True while a sentence write is in flight, so a double-click cannot send twice. */
  busy: boolean;
}

interface Props {
  scenario: RehearsalScenario;
  wording: RehearsalPayload["wording"];
  open: OpenSections;
  onToggle: (section: keyof OpenSections) => void;
  /** Which instance rows are open, by printed position. */
  openRows: ReadonlySet<number>;
  onToggleRow: (position: number) => void;
  onJumpToRow: (position: number) => void;
  edits: RehearsalEdits;
}

const RehearsalScenarioBlocks: React.FC<Props> = ({
  scenario,
  wording,
  open,
  onToggle,
  openRows,
  onToggleRow,
  onJumpToRow,
  edits,
}) => (
  <>
    <RehearsalSection
      heading={wording.block_what_heading}
      count={null}
      open={open.what}
      onToggle={() => onToggle("what")}
    >
      <SentenceEditor
        text={scenario.what_this_is}
        missingNotice={scenario.what_this_is_gap ?? ""}
        wording={{
          editLabel: wording.editor.edit_label,
          saveLabel: wording.editor.save_label,
          cancelLabel: wording.editor.cancel_label,
          // No Withdraw: the mockup offers none here, and a scenario with no
          // "what this is" is a gap to fill rather than a sentence to retract.
          placeholder: wording.editor.what_placeholder,
        }}
        onSave={edits.onSaveWhat}
        busy={edits.busy}
        sentenceStyle={whatLineStyle}
        buttonStyle={editButtonStyle}
        fieldStyle={editorFieldStyle}
      >
        {scenario.what_this_is_attribution && (
          <div style={attributionStyle}>{scenario.what_this_is_attribution}</div>
        )}
      </SentenceEditor>
    </RehearsalSection>

    <RehearsalSection
      heading={wording.block_accusation_heading}
      count={scenario.headers.accusation}
      // Read from its own number, never counted off a list this client could
      // have filtered — the folded header must not disagree with the body.
      gapCount={scenario.accusation.gap_count}
      open={open.accusation}
      onToggle={() => onToggle("accusation")}
    >
      <RehearsalAccusationBlock
        accusation={scenario.accusation}
        wording={wording}
        openRows={openRows}
        onToggleRow={onToggleRow}
        onJumpToRow={onJumpToRow}
        onSaveText={edits.onSaveAccusation}
        busy={edits.busy}
        anchorPrefix={scenario.code}
      />
    </RehearsalSection>

    <RehearsalSection
      heading={wording.block_timeline_heading}
      count={scenario.headers.timeline}
      open={open.timeline}
      onToggle={() => onToggle("timeline")}
    >
      <RehearsalTimelineBlock timeline={scenario.timeline} />
    </RehearsalSection>

    <RehearsalSection
      heading={wording.block_points_heading}
      count={scenario.headers.points}
      open={open.points}
      onToggle={() => onToggle("points")}
    >
      <RehearsalPointsBlock
        points={scenario.points}
        pointsGap={scenario.points_gap}
        wording={wording}
        onEdit={edits.onEditPoint}
        onAdd={edits.onAddPoint}
      />
    </RehearsalSection>

    <RehearsalSection
      heading={wording.block_watch_heading}
      count={scenario.headers.watch_for}
      open={open.watchFor}
      onToggle={() => onToggle("watchFor")}
    >
      <RehearsalWatchBlock
        items={scenario.watch_for}
        watchGap={scenario.watch_for_gap}
        wording={wording}
        onEdit={edits.onEditWatchItem}
        onAdd={edits.onAddWatchItem}
      />
    </RehearsalSection>
  </>
);

export default RehearsalScenarioBlocks;
