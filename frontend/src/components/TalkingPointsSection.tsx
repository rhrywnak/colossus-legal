// =============================================================================
// TalkingPointsSection — C5 on the scenario working page (§2.5)
// =============================================================================
//
// The 1.7B augmentation panel bundled human facts, the watch-list and Marie's
// talking points into one stack behind an "Add" click. §2 splits that stack: human
// facts join the Scenario facts table, and these two get sections of their own,
// always present. C5 is the answer Marie gives under pressure — it is not a
// sub-panel of a form.
//
// ## Numbered, capped from Settings, and in her own words
//
// `cap` is SERVED (`talking_points_cap`, a stored parameter since task 1.6). A
// browser that baked in "3" would show the wrong limit the day it changes, and the
// backend enforces it regardless — this only stops a human typing a fourth point
// they cannot save.
//
// ## Task 2.11 C changed two things (ruling C4b)
//
// **Editing is per row.** The whole list used to go into edit mode together, and
// saving rewrote every row — which destroyed each one's `authored_by` and
// `created_at` and re-stamped them with the editor and today. One point is now
// one `PUT …/talking-points/:position`, an UPDATE in place. The whole-list write
// survives for what it is actually for: adding, reordering, dropping one.
//
// **Not one visible word lives in this file.** They are stored rows now, arriving
// on the panel — because the row editor below is SHARED with the rehearsal page,
// whose standing law is that every visible word is a settings row, and a
// component holding a literal cannot be reused on a surface that forbids one.
//
// ## The paired exhibit is SKIPPED, not stubbed (ruling R9)
//
// §2.5 asks each point to render its backing exhibit when the pairing data
// exists. No pairing field exists on the wire and no pairing data exists in the
// store, so the paired branch would be dead code — worse than an honest absence.
// Every point says so, and task 3.9 brings the field and the branch together.

import React, { useState } from "react";

import AuthoredLineEditor from "./AuthoredLineEditor";
import {
  absentStyle,
  addButtonStyle,
  DIVIDER,
  sectionHeaderStyle,
  sectionMetaStyle,
  sectionPaddedPanelStyle,
  sectionTitleStyle,
} from "./scenarioSectionStyles";
import {
  editTalkingPoint,
  fillCap,
  fillN,
  setTalkingPoints,
  type AuthoringWordingDto,
  type TalkingPointDto,
} from "../services/scenarioAugmentation";

interface Props {
  slug: string;
  scenarioId: string;
  points: TalkingPointDto[];
  /** `talking_points_cap` from the payload — served, never hardcoded. */
  cap: number;
  /** Every word this section speaks, from the store. */
  wording: AuthoringWordingDto;
  /** Re-read the payload after a successful write. */
  onChanged: () => void;
}

const rowTextStyle: React.CSSProperties = { fontSize: "0.9rem", lineHeight: 1.6 };

const fieldStyle: React.CSSProperties = {
  border: DIVIDER,
  borderRadius: "6px",
  padding: "0.35rem 0.55rem",
  width: "100%",
  fontFamily: "inherit",
  fontWeight: 400,
};

const rowErrorStyle: React.CSSProperties = {
  color: "var(--state-danger-strong)",
  fontSize: "0.78rem",
  marginTop: "0.2rem",
};

/** The accent pill carrying the point's number — and its address. */
const numberPillStyle: React.CSSProperties = {
  fontSize: "12px",
  fontWeight: 700,
  color: "var(--v3-on-fill)",
  background: "var(--accent-primary)",
  borderRadius: "999px",
  width: "20px",
  height: "20px",
  display: "inline-flex",
  alignItems: "center",
  justifyContent: "center",
  flexShrink: 0,
};

const TalkingPointsSection: React.FC<Props> = ({
  slug,
  scenarioId,
  points,
  cap,
  wording,
  onChanged,
}) => {
  const [adding, setAdding] = useState(false);
  const [draft, setDraft] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  /** Adding is the WHOLE-list write: it changes the list, not one sentence. */
  const add = () => {
    const trimmed = draft.trim();
    if (trimmed.length === 0) return;

    setSaving(true);
    setError(null);
    setTalkingPoints(slug, scenarioId, [...points.map((p) => p.text), trimmed])
      .then(() => {
        setDraft("");
        setAdding(false);
        onChanged();
      })
      .catch((e: unknown) => {
        // Standing Rule 1: the draft STAYS on screen and the failure is named.
        // The human just wrote these words; discarding them on a failed save
        // would be the worst possible response.
        setError(e instanceof Error ? e.message : wording.points_save_failed_notice);
      })
      .finally(() => setSaving(false));
  };

  const atCap = points.length >= cap;

  return (
    <section>
      <div style={sectionHeaderStyle}>
        <h2 style={sectionTitleStyle}>{wording.points_section_heading}</h2>
        <span style={sectionMetaStyle}>
          {fillCap(wording.points_section_meta_template, cap)}
        </span>
      </div>

      <div style={sectionPaddedPanelStyle}>
        {error && (
          <div role="alert" style={{ ...rowErrorStyle, marginBottom: "0.6rem" }}>
            {error}
          </div>
        )}

        {points.length === 0 && !adding ? (
          <p style={absentStyle}>{wording.points_empty_notice}</p>
        ) : (
          points.map((point, index) => (
            <div
              key={point.position}
              style={{
                display: "flex",
                gap: "0.7rem",
                alignItems: "flex-start",
                // Tighter since task R4 (P5): the row is one line now, so the
                // breathing room a three-line block needed is space between
                // points that no longer exist as blocks.
                padding: "0.35rem 0",
                borderBottom: index === points.length - 1 ? "none" : DIVIDER,
              }}
            >
              {/* The number a human reads AND the point's address on the write
                  route — one number, or editing point 2 lands on point 1. */}
              <span style={numberPillStyle}>{point.position}</span>
              <div style={{ flex: 1 }}>
                <AuthoredLineEditor
                  text={point.text}
                  wording={{
                    editLabel: wording.points_edit_label,
                    saveLabel: wording.points_save_label,
                    cancelLabel: wording.points_cancel_label,
                    savingLabel: wording.points_saving_label,
                  }}
                  onSave={(text) =>
                    editTalkingPoint(slug, scenarioId, point.position, text).then(onChanged)
                  }
                  saveFailedNotice={wording.points_save_failed_notice}
                  fieldLabel={fillN(wording.points_field_label_template, point.position)}
                  textStyle={rowTextStyle}
                  fieldStyle={fieldStyle}
                  buttonStyle={addButtonStyle}
                  errorStyle={rowErrorStyle}
                >
                  {/* Ruling R9: no pairing data exists on the wire yet, so every
                      point says so. Task 3.9 brings the field and the paired
                      branch together — a branch written now would be dead code.

                      INLINE since task R4 (P5): a `span` with a separator rather
                      than a `div` under the text. The dot is punctuation between
                      two things on one line, not vocabulary. */}
                  <span style={{ ...absentStyle, fontSize: "0.75rem" }}>
                    · {wording.points_no_exhibit_notice}
                  </span>
                </AuthoredLineEditor>
              </div>
            </div>
          ))
        )}

        <div
          style={{
            display: "flex",
            gap: "0.7rem",
            alignItems: "center",
            flexWrap: "wrap",
            marginTop: "0.8rem",
          }}
        >
          {adding ? (
            <>
              <input
                value={draft}
                onChange={(e) => setDraft(e.target.value)}
                aria-label={fillN(wording.points_field_label_template, points.length + 1)}
                style={{ ...fieldStyle, flex: 1, minWidth: "16rem" }}
                // The browser advises while you type; storage stays verbatim.
                spellCheck
              />
              <button
                type="button"
                style={addButtonStyle}
                disabled={saving || draft.trim().length === 0}
                onClick={add}
              >
                {saving ? wording.points_saving_label : wording.points_save_label}
              </button>
              <button
                type="button"
                style={{
                  ...addButtonStyle,
                  color: "var(--text-secondary)",
                  borderColor: "var(--border-default)",
                }}
                disabled={saving}
                onClick={() => {
                  setAdding(false);
                  setDraft("");
                  setError(null);
                }}
              >
                {wording.points_cancel_label}
              </button>
            </>
          ) : (
            <button
              type="button"
              style={addButtonStyle}
              disabled={atCap}
              // A control that refuses without saying why reads as a broken one.
              title={atCap ? fillCap(wording.points_cap_reached_notice, cap) : undefined}
              onClick={() => setAdding(true)}
            >
              {wording.points_add_label}
            </button>
          )}
          <span style={{ fontSize: "0.75rem", color: "var(--text-muted)" }}>
            {wording.points_authoring_note}
          </span>
        </div>
      </div>
    </section>
  );
};

export default TalkingPointsSection;
