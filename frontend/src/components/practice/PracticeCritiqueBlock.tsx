// PracticeCritiqueBlock.tsx — the block under the answer box, in all five states.
//
// Named `…Block` and not `PracticeCritique` because `practiceCritique.ts` — the
// pure decisions this file draws — already sits beside it, and TypeScript
// refuses two files whose names differ only in casing. The same collision cost
// a rename in the print task (`printSheets.ts` / `PrintSheets.tsx`).
//
// ## The working state is the point of this component
//
// It renders from the moment she presses Answer, BEFORE anything comes back.
// Roman's defect #1 of 2026-08-20 was that nothing appeared until the read
// returned, so the page looked inert while it worked and she pressed again. A
// state not drawn is a state not built — and a state not TESTED WHILE PENDING is
// a state that will quietly stop existing, which is why `practiceCritique.test`
// holds the promise unresolved.
//
// ## What each state draws is decided elsewhere
//
// `practiceCritique.critiqueFor` owns the branching, pure and tested. This file
// draws what it is told. The one rule worth repeating here: PARTS IF PRESENT,
// ELSE TEXT, NEVER BOTH — `read_text` is a lossy projection of the parts, so
// showing both prints the call and first pointer twice.

import React from "react";

import type { PracticeWording } from "../../services/practice";
import { wordingOf } from "../../services/practice";
import { citedSources, type CritiqueView } from "./practiceCritique";
import { waitingLineKey, LONG_WAIT_MS } from "./practiceAnswerPhase";
import * as c from "./practiceCritiqueStyles";

/** The bars that stand in for text that has not arrived. */
const Shimmer: React.FC = () => (
  <>
    <div style={{ ...c.shimmer, width: "62%" }} data-critique-shimmer />
    <div style={{ ...c.shimmer, width: "94%" }} data-critique-shimmer />
    <div style={{ ...c.shimmer, width: "78%" }} data-critique-shimmer />
  </>
);

const Critique: React.FC<{
  view: CritiqueView;
  wording: PracticeWording;
}> = ({ view, wording }) => {
  const w = (key: string) => wordingOf(wording, key);

  // NOTHING, not an empty box. She stopped waiting, or the read failed or
  // abstained — her answer is saved either way, and a bordered empty box would
  // say "something should be here" and invite her to wait for what is not coming.
  if (view.kind === "idle" || view.kind === "none") return null;

  if (view.kind === "working") {
    return (
      <div style={c.working} data-critique="working" role="status" aria-live="polite">
        <p style={{ ...c.call, color: "var(--practice-blue)", display: "flex", gap: 9 }}>
          <span style={c.spinner} data-critique-spinner aria-hidden="true" />
          {w("read_working_label")}
        </p>
        <Shimmer />
        {/* At ten seconds the line changes to say her answer is safe REGARDLESS.
            That is the fact she needs while waiting, and it is true from the
            moment the row was written — the read is the second write. */}
        {/* The line is chosen by the same pure rule the test asserts, so the
            threshold cannot drift between the two. */}
        <p style={c.foot}>{w(waitingLineKey(view.longWait ? LONG_WAIT_MS : 0))}</p>
      </div>
    );
  }

  if (view.kind === "sentence") {
    return (
      <div
        style={view.ok === false ? c.fault : c.fine}
        data-critique="sentence"
      >
        <p style={c.call}>{view.text}</p>
        {/* ARCHITECT'S ADDITION, 2026-08-23 — Roman may strike it. Delete this
            block and say so in the report. Measured: 12 of 14 stored answers
            carry only a sentence, so without a word of explanation one answer
            showing three parts and the next showing one reads as breakage. */}
        <p style={c.plainHint}>{w("read_plain_hint")}</p>
      </div>
    );
  }

  const parts = view.result.read_parts;
  if (parts === null) return null;
  const cited = citedSources(view.result);

  return (
    <div
      style={view.result.read_ok === false ? c.fault : c.fine}
      data-critique="parts"
    >
      <p style={c.call}>{parts.call}</p>

      {/* An empty `why` is LEGITIMATE — the model may have nothing to add — and
          an empty labelled block would read as a part that failed to load. */}
      {parts.why !== "" && (
        <div style={c.part}>
          <span style={c.partLabel}>{w("read_why_label")}</span>
          {parts.why}
        </div>
      )}

      {parts.pointers.length > 0 && (
        <div style={c.part}>
          <span style={c.partLabel}>{w("read_pointers_label")}</span>
          <ul style={{ margin: 0, padding: "0 0 0 19px" }}>
            {parts.pointers.map((pointer, i) => (
              <li key={i} style={{ margin: "0 0 4px" }}>
                {pointer}
              </li>
            ))}
          </ul>
        </div>
      )}

      {/* ⚑ THE SOURCE LIST IS THE ONE PLACE A BAD READ CAN BE CAUGHT.
          These are the words that were SENT to the model, never words it sent
          back — so if it cites S2 for a claim S2 does not support, Marie can
          see S2 say so. Built from the reply instead, a hallucinated citation
          would render its own supporting evidence. */}
      {cited.length > 0 && (
        <div style={c.sources}>
          {cited.map((source) => (
            <div key={source.key} style={c.sourceRow}>
              <span style={c.sourceKey}>{source.key}</span>
              {source.text === null ? (
                // Shown, never dropped: a cited key with no source should be
                // impossible, and hiding it would hide the failure this list
                // exists to expose.
                <span style={c.sourceMissing}>{w("read_source_missing")}</span>
              ) : (
                <span>{source.text}</span>
              )}
            </div>
          ))}
        </div>
      )}

      {/* ⚑ "This is wrong →" IS DELIBERATELY NOT A CONTROL HERE.
          Measured 2026-08-23 BEFORE building it: it would have written
          `practice_questions.flag_note`, whose only reader is the retired
          end-of-sitting sheet, and which holds 0 rows across 46 questions on
          DEV. It is also the WRONG OBJECT — the flag hangs off the QUESTION,
          while "this is wrong" is about the READ, so a perfect question with a
          garbage read would be filed as a bad question.

          A button that swallows her objection silently is a false promise at
          the exact moment she should be telling a person, and this is the one
          screen where the model may have got something wrong about her own
          case. Awaiting Roman's ruling on replacing it with a line naming
          Chuck. Until then the block says who has not reviewed this, and
          nothing that pretends to record a complaint. */}
      <p style={c.foot}>{w("read_unreviewed")}</p>
      {/* ⚑ RENDERS ONLY HERE — not on the one-sentence fallback, not while the
          read is running. Roman's ruling of 2026-08-23. A three-part critique
          with citations underneath is the one shape on this product that reads
          as authoritative, and silence beside it reads as agreement. The
          fallback sentence does not make the same claim, and the working state
          has made no claim yet. */}
      <p style={c.plainHint}>{w("read_fallible")}</p>
    </div>
  );
};

export default Critique;
