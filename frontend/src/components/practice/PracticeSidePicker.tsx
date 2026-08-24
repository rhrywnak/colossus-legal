// =============================================================================
// PracticeSidePicker.tsx — one side at a time (mockup v8, 2026-08-24)
// =============================================================================
//
// Two buttons above the question list: the defense's side and Chuck's, each
// carrying how many questions are behind it. One is chosen; the list below shows
// that side and nothing else.
//
// ## What it replaced, and why
//
// The list dealt `mixed` — each defense trap followed immediately by the
// redirect that repairs it. That order describes a courtroom MOMENT correctly
// and describes neither person's job: Marie answers one side at a time, Chuck
// reads one side at a time, and a list that alternates is a list in which
// neither of them can find their place.
//
// ## Why buttons and not the select the practice bar uses
//
// Both counts have to be readable WITHOUT opening anything. "How much is on the
// other side" is half of what this control is for, and a collapsed select hides
// exactly that.
//
// ## Every string comes from the payload
//
// Including the two side names, which are the practice bar's OWN rows —
// `who_george_title` and `who_chuck_title`. No second pair was seeded: two rows
// holding the same two words are two places to edit, and one of them eventually
// does not get edited.

import React from "react";

import type { PracticeWording } from "../../services/practice";
import { wordingOf } from "../../services/practice";
import * as d from "./practiceDeckStyles";

/** How many questions each side holds. Counted from the whole deck, above. */
export interface SideCounts {
  george: number;
  chuck: number;
}

const PracticeSidePicker: React.FC<{
  side: "george" | "chuck";
  onSide: (side: "george" | "chuck") => void;
  counts: SideCounts;
  wording: PracticeWording;
}> = ({ side, onSide, counts, wording }) => {
  const w = (key: string) => wordingOf(wording, key);

  /** `{side} · {n}` — the side's own title, and its count. */
  const tab = (title: string, n: number) =>
    w("deck_side_tab_template").replace("{side}", title).replace("{n}", String(n));

  const choices: { value: "george" | "chuck"; label: string }[] = [
    { value: "george", label: tab(w("who_george_title"), counts.george) },
    { value: "chuck", label: tab(w("who_chuck_title"), counts.chuck) },
  ];

  return (
    <div style={d.picker} role="tablist" aria-label={w("deck_heading")}>
      {choices.map((choice) => (
        <button
          key={choice.value}
          type="button"
          role="tab"
          // The chosen side is filled, but fill is a COLOUR — `aria-selected` is
          // what says the same thing to a screen reader, and to anyone whose
          // display flattens the two.
          aria-selected={side === choice.value}
          style={{
            ...d.pickerButton,
            ...(side === choice.value ? d.pickerButtonOn : {}),
          }}
          onClick={() => onSide(choice.value)}
        >
          {choice.label}
        </button>
      ))}
    </div>
  );
};

export default PracticeSidePicker;
