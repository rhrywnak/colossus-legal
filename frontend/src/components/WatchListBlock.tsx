// =============================================================================
// WatchListBlock — C6 authoring inside the augmentation panel (task 1.5)
// =============================================================================
//
// v2 §10's fourth rehearsal block: what the other side will wave around. These
// are HUMAN-FLAGGED notes — the same storage and the same §8 protections as a
// human fact, distinguished by `kind`.
//
// ## Why this is a second mode of the fact form, not a second form
//
// A watch-list note is structurally a human fact: text, an author, no citation.
// One write path means §8's invariants (no scan writes it; editing it never
// triggers a gather) are enforced once, for both. Phase 2 adds a COMPUTED
// watch-list from the evidence cut and merges it onto this panel — with one
// table that is a filter, and it lands beside these notes rather than beneath a
// second heading.
//
// A separate component rather than more lines in `AugmentationPanel` because
// that file already carries three components' worth of form.

import React, { useState } from "react";

import { addHumanFact, deleteHumanFact, type HumanFactDto } from "../services/scenarioAugmentation";
import { addButtonStyle } from "./scenarioSectionStyles";

interface Props {
  slug: string;
  scenarioId: string;
  notes: HumanFactDto[];
  /** Runs a write, surfaces failures, and re-reads — the panel's shared runner. */
  run: (work: () => Promise<void>) => void;
  boxStyle: React.CSSProperties;
  labelStyle: React.CSSProperties;
  fieldStyle: React.CSSProperties;
  tagStyle: React.CSSProperties;
  hairline: string;
}

const WatchListBlock: React.FC<Props> = ({
  slug,
  scenarioId,
  notes,
  run,
  boxStyle,
  labelStyle,
  fieldStyle,
  tagStyle,
  hairline,
}) => {
  const [text, setText] = useState("");
  // The mockup shows notes plus a "+ Add watch-list note" BUTTON — not an open
  // textarea. An always-expanded form made the section look like a data-entry
  // screen rather than a list of things to expect at trial, and it pushed the notes
  // themselves above the fold's edge.
  const [adding, setAdding] = useState(false);

  return (
    <div style={boxStyle}>

      {notes.map((note) => (
        <div key={note.id} style={{ borderBottom: hairline, paddingBottom: "0.5rem" }}>
          <div style={{ fontSize: "0.9rem", lineHeight: 1.6 }}>{note.text}</div>
          <div style={{ display: "flex", gap: "0.5rem", alignItems: "baseline" }}>
            <span style={tagStyle}>
              {note.authored_tag}
              {note.edited && " · edited since written"}
            </span>
            <button
              type="button"
              onClick={() =>
                run(async () => {
                  await deleteHumanFact(slug, scenarioId, note.id);
                })
              }
              style={{ marginLeft: "auto" }}
            >
              Remove
            </button>
          </div>
        </div>
      ))}

      {adding ? (
        <>
          <label style={labelStyle} htmlFor="watch-text">
            Flag something to watch for
          </label>
          <textarea
            id="watch-text"
            value={text}
            onChange={(e) => setText(e.target.value)}
            rows={2}
            style={fieldStyle}
          />
          <div style={{ display: "flex", gap: "0.5rem" }}>
            <button
              type="button"
              style={addButtonStyle}
              disabled={!text.trim()}
              onClick={() =>
                run(async () => {
                  // No date fields: a watch-list note is a thing to EXPECT, not a
                  // dated event. Offering a date qualifier here would invite a
                  // precision claim about something that has not happened yet.
                  await addHumanFact(slug, scenarioId, { text, kind: "watch_list" });
                  setText("");
                  setAdding(false);
                })
              }
            >
              Add note
            </button>
            <button
              type="button"
              style={{
                ...addButtonStyle,
                color: "var(--text-secondary)",
                boxShadow: "none",
              }}
              onClick={() => {
                setAdding(false);
                setText("");
              }}
            >
              Cancel
            </button>
          </div>
        </>
      ) : (
        <button
          type="button"
          style={{ ...addButtonStyle, alignSelf: "flex-start" }}
          onClick={() => setAdding(true)}
        >
          + Add watch-list note
        </button>
      )}
    </div>
  );
};

export default WatchListBlock;
