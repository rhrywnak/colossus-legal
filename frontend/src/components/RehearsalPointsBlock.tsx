// =============================================================================
// RehearsalPointsBlock — her points, in her words, editable here
// =============================================================================
//
// The mockup's `.point` / `.pt-num` / `.pt-exhibit` / `.addrow`, plus the two
// acts ruling C4b authorised on this page: fix one point, add a new one.
//
// ## Reuse, never fork
//
// The per-row editor is `AuthoredLineEditor`, shared with the watch block and
// with both sections on the scenario working page. The WRITES are the same
// guarded routes the working view calls — `PUT …/talking-points/:position` for
// an edit and `PUT …/talking-points` for an add — reached through the same
// client functions. Nothing here is a second write path.
//
// ## Why adding sends the whole list
//
// The list write is what owns the ORDERING, and ordering is server-owned. An
// append route would have to invent an ordering protocol this endpoint does not
// have. So an add is "the list, plus one", and an edit — which changes no
// ordering and must not re-stamp anybody's authorship — is the per-row route.
//
// ## The exhibit line is an honest absence, not a stub
//
// Every point says "No exhibit paired yet" because no pairing data exists on the
// wire (tracker task 3.9). The mockup shows a "Pair an exhibit" control beside
// it; Roman DEFERRED that control to B3 on 2026-08-06, so the sentence renders
// without it. A control that cannot do anything is worse than a named gap.

import React, { useState } from "react";

import AuthoredLineEditor from "./AuthoredLineEditor";
import {
  editButtonStyle,
  editorFieldStyle,
  errorStyle,
} from "./rehearsalStyles";
import {
  addNoteStyle,
  addRowStyle,
  pointExhibitStyle,
  pointNumberStyle,
  pointStyle,
  pointTextStyle,
} from "./rehearsalRowStyles";
import { absentStyle } from "./scenarioSectionStyles";
import type { RehearsalPoint, RehearsalWording } from "../services/rehearsal";

interface Props {
  points: RehearsalPoint[];
  /** The stored sentence when nobody has written one. */
  pointsGap: string | null;
  wording: RehearsalWording;
  /** Store one edited point, addressed by its printed position. */
  onEdit: (position: number, text: string) => Promise<void>;
  /** Store the list plus one new point. */
  onAdd: (text: string) => Promise<void>;
}

const RehearsalPointsBlock: React.FC<Props> = ({
  points,
  pointsGap,
  wording,
  onEdit,
  onAdd,
}) => {
  const [adding, setAdding] = useState(false);
  const [draft, setDraft] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const w = wording.editor;

  const add = () => {
    const trimmed = draft.trim();
    if (trimmed.length === 0) return;

    setSaving(true);
    onAdd(trimmed)
      .then(() => {
        setDraft("");
        setAdding(false);
        setError(null);
      })
      // The draft stays on screen and the failure is named: the human just typed
      // this, and they are the only one who can act on the refusal.
      .catch((e: unknown) => {
        setError(e instanceof Error ? e.message : w.save_failed_template);
      })
      .finally(() => setSaving(false));
  };

  return (
    <>
      {points.length === 0 && !adding && pointsGap && (
        <p style={absentStyle}>{pointsGap}</p>
      )}

      {points.map((point) => (
        <div key={point.position} style={pointStyle}>
          <span style={pointNumberStyle}>{point.position}</span>
          <div style={{ flex: 1 }}>
            <AuthoredLineEditor
              text={point.text}
              wording={{
                editLabel: w.edit_label,
                saveLabel: w.save_label,
                cancelLabel: w.cancel_label,
              }}
              onSave={(text) => onEdit(point.position, text)}
              saveFailedNotice={w.save_failed_template}
              fieldLabel={`${wording.block_points_heading} ${point.position}`}
              textStyle={pointTextStyle}
              fieldStyle={editorFieldStyle}
              buttonStyle={editButtonStyle}
              errorStyle={errorStyle}
            >
              {/* Named, never blank. Task 3.9 brings the pairing editor and the
                  paired branch together; the control is deferred to B3. */}
              <div style={pointExhibitStyle}>
                {point.exhibit ?? point.exhibit_notice}
              </div>
            </AuthoredLineEditor>
          </div>
        </div>
      ))}

      {adding ? (
        <div style={{ marginTop: "14px" }}>
          <textarea
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            aria-label={wording.add_point_label}
            rows={2}
            style={editorFieldStyle}
            // Advice while typing; storage stays verbatim.
            spellCheck
          />
          {error && (
            <div role="alert" style={errorStyle}>
              {error}
            </div>
          )}
          <div style={{ display: "flex", gap: "6px", marginTop: "6px" }}>
            <button
              type="button"
              style={editButtonStyle}
              disabled={saving || draft.trim().length === 0}
              onClick={add}
            >
              {w.save_label}
            </button>
            <button
              type="button"
              style={editButtonStyle}
              disabled={saving}
              onClick={() => {
                setAdding(false);
                setDraft("");
                setError(null);
              }}
            >
              {w.cancel_label}
            </button>
          </div>
        </div>
      ) : (
        <div style={addRowStyle}>
          <button type="button" style={editButtonStyle} onClick={() => setAdding(true)}>
            {wording.add_point_label}
          </button>
          <span style={addNoteStyle}>{wording.points_authoring_note}</span>
        </div>
      )}
    </>
  );
};

export default RehearsalPointsBlock;
