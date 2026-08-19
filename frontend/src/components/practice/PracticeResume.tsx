// =============================================================================
// PracticeResume.tsx — the sitting she walked out of (mockup v3, item A5)
// =============================================================================
//
// The blue box on the start card: `Unfinished session · today 09:57 · George's
// side · 1 of 5 answered.` · Resume · Start over · and the sub-line saying that
// starting over is not destructive.
//
// ## Why the detail sentence arrives composed
//
// The practice payload's law: the browser holds no templates and no date format.
// Everything after the bold label is one string the server built, so how that
// line reads is a Settings edit rather than a build.
//
// ## Why "Start over" needs a sentence under it
//
// It is the only control on this screen whose NAME sounds destructive. It is
// not: the sitting is closed, its rows stay, and it gets a Chuck's sheet of its
// own. A witness who reads "Start over" as "lose what I did" simply will not
// press it, and will practise around a stale session all week.

import React from "react";

import type { OpenSession, PracticeWording } from "../../services/practice";
import { wordingOf } from "../../services/practice";
import * as d from "./practiceDeckStyles";
import * as s from "./practiceStyles";

interface Props {
  wording: PracticeWording;
  session: OpenSession;
  onResume: () => void;
  onStartOver: () => void;
  /** True while either write is in flight. */
  busy: boolean;
  /** The failure sentence from the last attempt, or null. Never swallowed. */
  error: string | null;
}

const PracticeResume: React.FC<Props> = ({
  wording,
  session,
  onResume,
  onStartOver,
  busy,
  error,
}) => {
  const w = (key: string) => wordingOf(wording, key);

  return (
    <div style={d.resume}>
      <span>
        <b style={{ color: "var(--practice-navy)" }}>{w("unfinished_label")}</b>{" "}
        {session.detail}
      </span>
      <button type="button" style={s.buttonPrimary} onClick={onResume} disabled={busy}>
        {w("resume_label")}
      </button>
      <button type="button" style={s.button} onClick={onStartOver} disabled={busy}>
        {w("start_over_label")}
      </button>
      <span style={{ ...s.sub, fontSize: 14 }}>{w("start_over_hint")}</span>
      {/* Standing Rule 1: a failed write says so, beside the controls it failed
          for. The open sitting is untouched either way — nothing here can lose
          an answer — so this is a notice and not an alarm over the page. */}
      {error !== null && (
        <div style={{ ...s.feedback, marginTop: 0 }} role="alert">
          {error}
        </div>
      )}
    </div>
  );
};

export default PracticeResume;
