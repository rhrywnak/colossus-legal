// =============================================================================
// TalkingPointsSection — C5 restored to the page (§2.5)
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
// ## The paired exhibit is SKIPPED, not stubbed (ruling R9)
//
// §2.5 asks each point to render its backing exhibit when the pairing data exists.
// Phase A measured that no pairing field exists on the wire and no pairing data
// exists in the store, so the paired branch would be dead code in 1.7C — worse than
// an honest absence. Every point therefore says "No exhibit paired yet", and task
// 3.9 brings the field and the branch together.

import React, { useState } from "react";

import {
  absentStyle,
  addButtonStyle,
  DIVIDER,
  sectionHeaderStyle,
  sectionMetaStyle,
  sectionPaddedPanelStyle,
  sectionTitleStyle,
} from "./scenarioSectionStyles";
import { setTalkingPoints, type TalkingPointDto } from "../services/scenarioAugmentation";

interface Props {
  slug: string;
  scenarioId: string;
  points: TalkingPointDto[];
  /** `talking_points_cap` from the payload — served, never hardcoded. */
  cap: number;
  /** Re-read the payload after a successful write. */
  onChanged: () => void;
}

const TalkingPointsSection: React.FC<Props> = ({
  slug,
  scenarioId,
  points,
  cap,
  onChanged,
}) => {
  // The section holds a DRAFT of the whole list, because C5 is saved as a list
  // (the ordering is server-owned and a per-point PATCH would invent an ordering
  // protocol this endpoint does not have).
  const [draft, setDraft] = useState<string[] | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const editing = draft !== null;
  const values = draft ?? points.map((p) => p.text);
  const filled = values.filter((v) => v.trim()).length;

  const save = (next: string[]) => {
    setSaving(true);
    setError(null);
    setTalkingPoints(slug, scenarioId, next)
      .then(() => {
        setDraft(null);
        setSaving(false);
        onChanged();
      })
      .catch((e: unknown) => {
        // Standing Rule 1: the draft STAYS on screen and the failure is named. The
        // human just wrote these words; discarding them on a failed save would be
        // the worst possible response.
        setError(
          e instanceof Error ? e.message : "Those talking points did not save. Try again.",
        );
        setSaving(false);
      });
  };

  return (
    <section>
      <div style={sectionHeaderStyle}>
        <h2 style={sectionTitleStyle}>Marie&rsquo;s talking points</h2>
        <span style={sectionMetaStyle}>
her own words · up to {cap}
        </span>
      </div>

      <div style={sectionPaddedPanelStyle}>
        {error && (
          <div
            role="alert"
            style={{
              color: "var(--state-danger-strong)",
              fontSize: "0.82rem",
              marginBottom: "0.6rem",
            }}
          >
            {error}
          </div>
        )}

        {values.length === 0 && !editing ? (
          <p style={absentStyle}>
            No talking points yet — these are the sentences Marie says when she is
            pressed on this scenario.
          </p>
        ) : (
          values.map((text, index) => (
            <div
              key={index}
              style={{
                display: "flex",
                gap: "0.7rem",
                alignItems: "baseline",
                padding: "0.55rem 0",
                borderBottom: index === values.length - 1 ? "none" : DIVIDER,
              }}
            >
              {/* Mockup `.tp-n`: a filled accent PILL, not a grey numeral — the
                  points are the answer Marie gives, and v3 gives them the accent. */}
              <span
                style={{
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
                }}
              >
                {index + 1}
              </span>
              <div style={{ flex: 1 }}>
                {editing ? (
                  <input
                    value={text}
                    onChange={(e) => {
                      const next = [...values];
                      next[index] = e.target.value;
                      setDraft(next);
                    }}
                    aria-label={`Talking point ${index + 1}`}
                    style={{
                      border: DIVIDER,
                      borderRadius: "6px",
                      padding: "0.35rem 0.55rem",
                      width: "100%",
                      fontFamily: "inherit",
                      fontWeight: 400,
                    }}
                  />
                ) : (
                  <div style={{ fontSize: "0.9rem", lineHeight: 1.6 }}>{text}</div>
                )}
                {/* Ruling R9: no pairing data exists on the wire yet, so every
                    point says so. Task 3.9 brings the field and the paired branch
                    together — a branch written now would be dead code. */}
                <div style={{ ...absentStyle, fontSize: "0.75rem", marginTop: "0.15rem" }}>
                  No exhibit paired yet
                </div>
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
          {editing ? (
            <>
              <button
                type="button"
                style={addButtonStyle}
                disabled={saving}
                onClick={() => save(values.filter((v) => v.trim()))}
              >
                {saving ? "Saving…" : "Save talking points"}
              </button>
              <button
                type="button"
                style={{ ...addButtonStyle, color: "var(--text-secondary)", borderColor: "var(--border-default)" }}
                disabled={saving}
                onClick={() => {
                  setDraft(null);
                  setError(null);
                }}
              >
                Cancel
              </button>
            </>
          ) : (
            <button
              type="button"
              style={addButtonStyle}
              disabled={filled >= cap}
              title={filled >= cap ? `That is already ${cap} points` : undefined}
              onClick={() => setDraft([...values, ""])}
            >
              + Add talking point
            </button>
          )}
          <span style={{ fontSize: "0.75rem", color: "var(--text-muted)" }}>
Authored by you and Marie — the system never rewrites these.
          </span>
        </div>
      </div>
    </section>
  );
};

export default TalkingPointsSection;
