// =============================================================================
// PracticeTopBar.tsx — the marked exit from a sitting (mockup v3, item B6)
// =============================================================================
//
// Left: `◂ Back to start` and the grey sentence saying what it costs. Right, on
// the question screen only: `Skip this one — doesn't fit`; then `End session ▸`
// on both.
//
// ## Why this exists at all (the measured defect)
//
// .401 shipped the drill with no way out of it. Roman answered question 1, left
// the page by the only means available — the browser — and came back to question
// 1 with no sign his answer had been kept. It HAD been kept; the screen simply
// could not say so. NN/g heuristic 3 is the general form: a clearly marked exit
// from every state, no dead ends, undo cheap.
//
// ## Why the hint is DIFFERENT on the two screens
//
// Because the fact is different. On the question screen nothing has been written
// for THIS question yet and the hint says her earlier answers are kept; on the
// reveal the row already exists, and the hint says so. One sentence for both
// would have to be vague enough to cover a lie.
//
// ## Every string here comes from the payload
//
// Not one literal sentence. `w()` reads the store and THROWS by name on a
// missing key rather than rendering a blank exit.

import React from "react";

import type { PracticeWording } from "../../services/practice";
import { wordingOf } from "../../services/practice";
import * as f from "./practiceFlowStyles";

interface Props {
  wording: PracticeWording;
  /**
   * Which screen this bar is on. The question screen offers the mid-sitting
   * skip; the reveal does not — she has already answered, and "skip" there would
   * mean discarding a row that is on Chuck's sheet.
   */
  screen: "question" | "reveal";
  onBack: () => void;
  onSkip: () => void;
  onEnd: () => void;
  /** True while a write these controls depend on is in flight. */
  busy: boolean;
}

/** The dot the mockup draws between two controls. */
const Separator: React.FC = () => <span style={f.topBarSeparator}>·</span>;

const PracticeTopBar: React.FC<Props> = ({ wording, screen, onBack, onSkip, onEnd, busy }) => {
  const w = (key: string) => wordingOf(wording, key);

  return (
    <div style={f.topBar}>
      <span>
        <button type="button" style={f.topBarLink} onClick={onBack} disabled={busy}>
          {w("back_label")}
        </button>
        <Separator />
        <span style={f.topBarHint}>
          {screen === "question" ? w("back_hint_question") : w("back_hint_reveal")}
        </span>
      </span>
      <span>
        {screen === "question" && (
          <>
            <button type="button" style={f.topBarLink} onClick={onSkip} disabled={busy}>
              {w("skip_question_label")}
            </button>
            <Separator />
          </>
        )}
        <button type="button" style={f.topBarLink} onClick={onEnd} disabled={busy}>
          {w("end_session_label")}
        </button>
      </span>
    </div>
  );
};

export default PracticeTopBar;
