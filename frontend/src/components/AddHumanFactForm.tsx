// =============================================================================
// AddHumanFactForm — the C4 add form, on its own (task 1.7C, §2.4)
// =============================================================================
//
// Lifted out of `AugmentationPanel`, whose three-blocks-in-a-stack shape §2 splits
// into three sections. The form is the same one, with the same write path and the
// same date-qualifier rules; what changed is where it opens from — the Scenario
// facts section's "+ Add human fact", beside the facts it creates.
//
// ## The date qualifier only travels WITH a date
//
// The backend refuses a qualifier with nothing to qualify, so the request omits
// both fields together. That is not defensive coding around a quirk: "around
// 2009" and "no date recorded" are different claims about what is known, and a
// stray `date_type` on an undated fact would be the form asserting the first when
// the human meant the second.

import React, { useState } from "react";

import { addHumanFact } from "../services/scenarioAugmentation";
import { addButtonStyle, HAIRLINE, sectionMetaStyle } from "./scenarioSectionStyles";

const fieldStyle: React.CSSProperties = {
  border: HAIRLINE,
  borderRadius: "6px",
  padding: "0.4rem 0.6rem",
  fontWeight: 400,
  width: "100%",
  fontFamily: "inherit",
};

interface Props {
  slug: string;
  scenarioId: string;
  /** The write landed; the caller closes the form and re-reads. */
  onSaved: () => void;
  onCancel: () => void;
}

const AddHumanFactForm: React.FC<Props> = ({ slug, scenarioId, onSaved, onCancel }) => {
  const [text, setText] = useState("");
  const [occurredOn, setOccurredOn] = useState("");
  const [dateType, setDateType] = useState("exact");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = () => {
    setSaving(true);
    setError(null);
    addHumanFact(slug, scenarioId, {
      text,
      // Both fields or neither — see the module header.
      ...(occurredOn ? { occurred_on: occurredOn, date_type: dateType } : {}),
    })
      .then(() => {
        setSaving(false);
        onSaved();
      })
      .catch((e: unknown) => {
        // The typed text STAYS on screen. Standing Rule 1, and simple decency: the
        // human wrote it and only they can act on the refusal.
        setError(e instanceof Error ? e.message : "That fact did not save. Try again.");
        setSaving(false);
      });
  };

  return (
    <div
      style={{
        border: HAIRLINE,
        borderRadius: "8px",
        padding: "0.9rem 1rem",
        marginTop: "0.9rem",
        background: "var(--bg-surface)",
        display: "flex",
        flexDirection: "column",
        gap: "0.6rem",
      }}
    >
      {error && (
        <div role="alert" style={{ color: "var(--state-danger-strong)", fontSize: "0.82rem" }}>
          {error}
        </div>
      )}

      <label style={sectionMetaStyle} htmlFor="human-fact-text">
        A fact you know that no document says
      </label>
      <textarea
        id="human-fact-text"
        value={text}
        onChange={(e) => setText(e.target.value)}
        rows={2}
        style={fieldStyle}
      />

      <div style={{ display: "flex", gap: "0.5rem", alignItems: "center", flexWrap: "wrap" }}>
        <input
          type="date"
          value={occurredOn}
          onChange={(e) => setOccurredOn(e.target.value)}
          aria-label="Date (optional)"
          style={{ ...fieldStyle, width: "auto" }}
        />
        <select
          value={dateType}
          onChange={(e) => setDateType(e.target.value)}
          aria-label="How precise is that date?"
          style={{ ...fieldStyle, width: "auto" }}
        >
          <option value="exact">exact</option>
          <option value="around">around</option>
          <option value="range">from</option>
          <option value="ordered">ordered</option>
        </select>

        <button
          type="button"
          style={addButtonStyle}
          disabled={saving || !text.trim()}
          onClick={submit}
        >
          {saving ? "Saving…" : "Add fact"}
        </button>
        <button
          type="button"
          style={{
            ...addButtonStyle,
            color: "var(--text-secondary)",
            borderColor: "var(--border-default)",
          }}
          disabled={saving}
          onClick={onCancel}
        >
          Cancel
        </button>
      </div>
    </div>
  );
};

export default AddHumanFactForm;
