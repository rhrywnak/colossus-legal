// =============================================================================
// WatchListBlock — C6 authoring (task 1.5; per-row edit added in 2.11 C)
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
// triggers a gather) are enforced once, for both.
//
// ## Editing, added by ruling C4b
//
// Until task 2.11 C a wrong word could only be fixed by REMOVING the note and
// writing it again — which threw away who wrote it and when, and re-stamped the
// editor as its author. `PUT …/human-facts/:fact_id` is an UPDATE in place:
// `authored_by` and `created_at` survive, `updated_at` moves, and the "edited
// since written" tag stays true.
//
// The row editor is `AuthoredLineEditor`, shared with the rehearsal page and
// with the talking-points section. One behaviour, four call sites.
//
// ## Not one visible word lives in this file
//
// They arrive as a prop. The component is shared with a surface whose standing
// law is that every visible word is a settings row, and a component holding a
// literal cannot be reused there.

import React, { useState } from "react";

import AuthoredLineEditor from "./AuthoredLineEditor";
import {
  addHumanFact,
  deleteHumanFact,
  editWatchItem,
  type AuthoringWordingDto,
  type HumanFactDto,
} from "../services/scenarioAugmentation";
import { addButtonStyle } from "./scenarioSectionStyles";

interface Props {
  slug: string;
  scenarioId: string;
  notes: HumanFactDto[];
  wording: AuthoringWordingDto;
  /** Runs a write, surfaces failures, and re-reads — the panel's shared runner. */
  run: (work: () => Promise<void>) => void;
  boxStyle: React.CSSProperties;
  labelStyle: React.CSSProperties;
  fieldStyle: React.CSSProperties;
  tagStyle: React.CSSProperties;
  hairline: string;
}

const noteTextStyle: React.CSSProperties = { fontSize: "0.9rem", lineHeight: 1.6 };

const rowErrorStyle: React.CSSProperties = {
  color: "var(--state-danger-strong)",
  fontSize: "0.78rem",
  marginTop: "0.2rem",
};

const WatchListBlock: React.FC<Props> = ({
  slug,
  scenarioId,
  notes,
  wording,
  run,
  boxStyle,
  labelStyle,
  fieldStyle,
  tagStyle,
  hairline,
}) => {
  const [text, setText] = useState("");
  // The mockup shows notes plus an add BUTTON — not an open textarea. An
  // always-expanded form made the section look like a data-entry screen rather
  // than a list of things to expect at trial, and it pushed the notes themselves
  // above the fold's edge.
  const [adding, setAdding] = useState(false);

  return (
    <div style={boxStyle}>
      {notes.map((note) => (
        <div key={note.id} style={{ borderBottom: hairline, paddingBottom: "0.5rem" }}>
          <AuthoredLineEditor
            text={note.text}
            wording={{
              editLabel: wording.watch_edit_label,
              saveLabel: wording.watch_save_label,
              cancelLabel: wording.watch_cancel_label,
            }}
            onSave={(next) =>
              editWatchItem(slug, scenarioId, note.id, next).then(() =>
                // Re-read through the panel's runner, so the screen matches the
                // database rather than the optimistic guess a local edit would be.
                run(async () => undefined),
              )
            }
            saveFailedNotice={wording.watch_save_failed_notice}
            fieldLabel={wording.watch_field_label}
            textStyle={noteTextStyle}
            fieldStyle={fieldStyle}
            buttonStyle={addButtonStyle}
            errorStyle={rowErrorStyle}
          >
            <div style={{ display: "flex", gap: "0.5rem", alignItems: "baseline" }}>
              <span style={tagStyle}>
                {note.authored_tag}
                {/* Provenance survives an edit: the tag says who wrote it, and
                    this says the words have changed since. */}
                {note.edited && ` · ${wording.watch_edited_suffix}`}
              </span>
              <button
                type="button"
                style={{ ...addButtonStyle, marginLeft: "auto" }}
                onClick={() =>
                  run(async () => {
                    await deleteHumanFact(slug, scenarioId, note.id);
                  })
                }
              >
                {wording.watch_remove_label}
              </button>
            </div>
          </AuthoredLineEditor>
        </div>
      ))}

      {adding ? (
        <>
          <label style={labelStyle} htmlFor="watch-text">
            {wording.watch_field_label}
          </label>
          <textarea
            id="watch-text"
            value={text}
            onChange={(e) => setText(e.target.value)}
            rows={2}
            style={fieldStyle}
            spellCheck
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
              {wording.watch_save_label}
            </button>
            <button
              type="button"
              style={{ ...addButtonStyle, color: "var(--text-secondary)", boxShadow: "none" }}
              onClick={() => {
                setAdding(false);
                setText("");
              }}
            >
              {wording.watch_cancel_label}
            </button>
          </div>
        </>
      ) : (
        <button
          type="button"
          style={{ ...addButtonStyle, alignSelf: "flex-start" }}
          onClick={() => setAdding(true)}
        >
          {wording.watch_add_label}
        </button>
      )}
    </div>
  );
};

export default WatchListBlock;
