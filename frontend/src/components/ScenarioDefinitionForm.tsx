// =============================================================================
// ScenarioDefinitionForm — author a scenario's `definition` (B2a write path).
// =============================================================================
//
// A small authoring form on the scenario detail page (between the attack box and
// the curated-facts binder). It writes the typed `ScenarioDefinition` via B1's
// `PUT /cases/:slug/scenarios/:scenarioId` (the `updateScenario` client). The
// definition is what B2b will use to seed the candidate-facts panel to THIS
// scenario's theme; B2a only authors + persists it.
//
// Refresh model: on a successful save the form calls `onSaved`, which bumps the
// detail page's `refreshKey` and re-loads the scenario. The page's loading gate
// unmounts this form during the re-fetch, so it remounts pre-filled from the
// PERSISTED definition — re-fetch is the source of truth, not a hand-merged
// response (mirrors the ScenarioCurationPanel refresh idiom).

import React, { useState } from "react";

import { CURRENT_SCHEMA_V, type ScenarioDefinition } from "../pages/trialPrepData";
import { updateScenario } from "../services/scenarioCrud";

interface Props {
  slug: string;
  scenarioId: string;
  /** The current persisted definition (pre-fills the form). Undefined — or a
   *  runtime `{}` with no `attack_text` — means "not yet defined": the form
   *  opens blank. */
  definition?: ScenarioDefinition;
  /** Called after a successful save so the page can re-fetch + re-fill. */
  onSaved: () => void;
}

// ── Multi-value helpers ──────────────────────────────────────────────────────
// `wielders` / `seed_phrases` / `anti_seed_phrases` are authored one-per-line in
// a textarea (plain — this is an internal authoring tool, not a public form).

/** Split a textarea's text into a trimmed, empty-line-free list. */
const linesToList = (text: string): string[] =>
  text
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0);

/** Join a list back to one-per-line text for pre-filling a textarea. */
const listToLines = (list?: string[]): string => (list ?? []).join("\n");

// ── Styles (tokens only — Rule 2) ────────────────────────────────────────────

const wrapStyle: React.CSSProperties = {
  border: "1px solid var(--border-default)",
  borderRadius: "8px",
  padding: "1rem 1.1rem",
  marginTop: "0.75rem",
  backgroundColor: "var(--bg-page)",
};
const headerStyle: React.CSSProperties = {
  fontSize: "0.74rem",
  fontWeight: 600,
  letterSpacing: "0.05em",
  textTransform: "uppercase",
  color: "var(--text-muted)",
  marginBottom: "0.75rem",
};
const fieldLabel: React.CSSProperties = {
  display: "block",
  fontSize: "0.78rem",
  fontWeight: 600,
  color: "var(--text-secondary)",
  margin: "0.7rem 0 0.25rem",
};
const inputStyle: React.CSSProperties = {
  width: "100%",
  boxSizing: "border-box",
  padding: "0.4rem 0.55rem",
  fontSize: "0.85rem",
  border: "1px solid var(--border-default)",
  borderRadius: "5px",
  backgroundColor: "var(--bg-surface)",
  color: "var(--text-primary)",
};
const textareaStyle: React.CSSProperties = {
  ...inputStyle,
  minHeight: "3.5rem",
  resize: "vertical",
  fontFamily: "inherit",
};
const hintStyle: React.CSSProperties = {
  fontSize: "0.72rem",
  color: "var(--text-muted)",
  marginTop: "0.2rem",
};
const saveBtnStyle: React.CSSProperties = {
  marginTop: "0.9rem",
  padding: "0.4rem 0.9rem",
  fontSize: "0.82rem",
  fontWeight: 600,
  border: "1px solid var(--accent-primary)",
  borderRadius: "5px",
  backgroundColor: "var(--accent-bg-soft)",
  color: "var(--accent-primary)",
  cursor: "pointer",
};
const saveBtnDisabled: React.CSSProperties = {
  ...saveBtnStyle,
  opacity: 0.5,
  cursor: "not-allowed",
};
const errorStyle: React.CSSProperties = {
  margin: "0.75rem 0 0",
  padding: "0.6rem 0.8rem",
  backgroundColor: "var(--state-danger-bg-soft)",
  border: "1px solid var(--state-danger-border)",
  borderRadius: "6px",
  color: "var(--state-danger-strong)",
  fontSize: "0.82rem",
};

const ScenarioDefinitionForm: React.FC<Props> = ({
  slug,
  scenarioId,
  definition,
  onSaved,
}) => {
  // Pre-fill from the persisted definition. `?? ""` / `?? []` tolerate both an
  // absent definition and a runtime `{}` (whose `attack_text` is undefined even
  // though the type says string) — the "not yet defined" state opens blank.
  const [attackText, setAttackText] = useState(definition?.attack_text ?? "");
  const [attackMeaning, setAttackMeaning] = useState(
    definition?.attack_meaning ?? "",
  );
  const [target, setTarget] = useState(definition?.target ?? "");
  const [notes, setNotes] = useState(definition?.notes ?? "");
  const [wieldersText, setWieldersText] = useState(
    listToLines(definition?.wielders),
  );
  const [seedText, setSeedText] = useState(listToLines(definition?.seed_phrases));
  const [antiSeedText, setAntiSeedText] = useState(
    listToLines(definition?.anti_seed_phrases),
  );

  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // `attack_text` is the one field B1 rejects when absent — Save stays disabled
  // until it carries non-whitespace content.
  const canSave = attackText.trim().length > 0 && !saving;

  const handleSave = () => {
    const trimmedAttack = attackText.trim();
    if (!trimmedAttack) return; // Guard even though the button is disabled.

    setSaving(true);
    setError(null);

    // Build the typed definition. Optional scalars are OMITTED when blank (not
    // sent as ""), matching the backend `skip_serializing_if` shape. `schema_v`
    // is set to the mirrored current version, never a user field.
    const nextDefinition: ScenarioDefinition = {
      attack_text: trimmedAttack,
      wielders: linesToList(wieldersText),
      seed_phrases: linesToList(seedText),
      anti_seed_phrases: linesToList(antiSeedText),
      schema_v: CURRENT_SCHEMA_V,
      ...(attackMeaning.trim() ? { attack_meaning: attackMeaning.trim() } : {}),
      ...(target.trim() ? { target: target.trim() } : {}),
      ...(notes.trim() ? { notes: notes.trim() } : {}),
    };

    updateScenario(slug, scenarioId, { definition: nextDefinition })
      .then(() => {
        // Success unmounts this form (the page re-loads through its loading
        // gate), so no success-state to set here — the re-fetch is the feedback.
        onSaved();
      })
      .catch((err: unknown) => {
        // Standing Rule 1: a failed PUT is a visible, contextual error, never a
        // silent no-op. The form stays mounted with the user's input intact.
        setError(
          err instanceof Error
            ? err.message
            : "Failed to save the definition. Try again.",
        );
        setSaving(false);
      });
  };

  return (
    <div style={wrapStyle}>
      <div style={headerStyle}>Define this scenario</div>

      <label style={fieldLabel} htmlFor="def-attack-text">
        Attack (required)
      </label>
      <textarea
        id="def-attack-text"
        style={textareaStyle}
        value={attackText}
        onChange={(e) => setAttackText(e.target.value)}
        placeholder="The attack this scenario answers, in the wielder's words"
      />

      <label style={fieldLabel} htmlFor="def-attack-meaning">
        What it means
      </label>
      <input
        id="def-attack-meaning"
        style={inputStyle}
        value={attackMeaning}
        onChange={(e) => setAttackMeaning(e.target.value)}
        placeholder="Plain-language gloss (optional)"
      />

      <label style={fieldLabel} htmlFor="def-target">
        Target
      </label>
      <input
        id="def-target"
        style={inputStyle}
        value={target}
        onChange={(e) => setTarget(e.target.value)}
        placeholder="Who the attack targets (optional)"
      />

      <label style={fieldLabel} htmlFor="def-wielders">
        Wielders
      </label>
      <textarea
        id="def-wielders"
        style={textareaStyle}
        value={wieldersText}
        onChange={(e) => setWieldersText(e.target.value)}
        placeholder="Who wields this attack"
      />
      <div style={hintStyle}>One per line.</div>

      <label style={fieldLabel} htmlFor="def-seed">
        Seed phrases
      </label>
      <textarea
        id="def-seed"
        style={textareaStyle}
        value={seedText}
        onChange={(e) => setSeedText(e.target.value)}
        placeholder="Phrases that steer the candidate search toward this theme"
      />
      <div style={hintStyle}>One per line.</div>

      <label style={fieldLabel} htmlFor="def-anti-seed">
        Anti-seed phrases
      </label>
      <textarea
        id="def-anti-seed"
        style={textareaStyle}
        value={antiSeedText}
        onChange={(e) => setAntiSeedText(e.target.value)}
        placeholder="Phrases that steer AWAY (known false positives)"
      />
      <div style={hintStyle}>One per line.</div>

      <label style={fieldLabel} htmlFor="def-notes">
        Notes
      </label>
      <textarea
        id="def-notes"
        style={textareaStyle}
        value={notes}
        onChange={(e) => setNotes(e.target.value)}
        placeholder="Free-form authoring notes (optional)"
      />

      {error && <div style={errorStyle}>{error}</div>}

      <button
        type="button"
        style={canSave ? saveBtnStyle : saveBtnDisabled}
        onClick={handleSave}
        disabled={!canSave}
      >
        {saving ? "Saving…" : "Save definition"}
      </button>
    </div>
  );
};

export default ScenarioDefinitionForm;
