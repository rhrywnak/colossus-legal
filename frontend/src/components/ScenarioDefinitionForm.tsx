// =============================================================================
// ScenarioDefinitionForm — author a scenario's `definition` (D1 rebuild).
// =============================================================================
//
// The authoring form on the scenario detail page. It writes the typed v2
// `ScenarioDefinition` (plus the top-level `anchor_allegation_ids` column) via
// B1's `PUT /cases/:slug/scenarios/:scenarioId` (`updateScenario`).
//
// D1's whole reason to exist: a non-technical author (Marie, Roman) authors a
// scenario in PLAIN LANGUAGE with ZERO knowledge of graph internals. So:
//   - Every label/help line comes from `scenarioFormLabels.ts` (no schema jargon).
//   - `target` and each wielder are PICKERS over the live bias vocabulary — a user
//     can never type a party the graph doesn't know.
//   - The allegation picker is sourced from the live allegations (¶ + summary).
//   - The retired fields (seed / anti-seed / notes) are gone.
//
// Refresh model (unchanged): on a successful save the form calls `onSaved`, which
// bumps the detail page's `refreshKey` and re-loads the scenario. The page's
// loading gate unmounts this form during the re-fetch, so it remounts pre-filled
// from the PERSISTED definition — re-fetch is the source of truth.

import React, { useEffect, useMemo, useState } from "react";

import { getAllegations, type AllegationDto } from "../services/allegations";
import { getAvailableFilters, type ActorOption } from "../services/bias";
import { updateScenario } from "../services/scenarioCrud";
import {
  CURRENT_SCHEMA_V,
  type ScenarioDefinition,
  type Wielder,
} from "../pages/trialPrepData";
import { parseScenarioDefinition } from "./scenarioDefinitionGuard";
import {
  ACTOR_ROLE_OPTIONS,
  DEFAULT_ACTOR_ROLE,
  SCENARIO_FORM_LABELS as L,
} from "./scenarioFormLabels";

interface Props {
  slug: string;
  scenarioId: string;
  /** The current persisted definition (pre-fills the form). Anything that is not
   *  a clean v2 body — `undefined`, `{}`, a retired v1 shape — opens the form
   *  blank (handled by `parseScenarioDefinition`). */
  definition?: ScenarioDefinition;
  /** The scenario's current complaint-paragraph anchors (pre-fills the picker). */
  anchorAllegationIds: string[];
  /** Called after a successful save so the page can re-fetch + re-fill. */
  onSaved: () => void;
}

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
  margin: "0.9rem 0 0.25rem",
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
const selectStyle: React.CSSProperties = { ...inputStyle, width: "auto", minWidth: "10rem" };
const hintStyle: React.CSSProperties = {
  fontSize: "0.72rem",
  color: "var(--text-muted)",
  marginTop: "0.2rem",
};
const guidanceStyle: React.CSSProperties = {
  ...hintStyle,
  fontStyle: "italic",
};
const wielderRowStyle: React.CSSProperties = {
  display: "flex",
  flexWrap: "wrap",
  alignItems: "center",
  gap: "0.5rem",
  marginBottom: "0.4rem",
};
const smallBtnStyle: React.CSSProperties = {
  padding: "0.3rem 0.6rem",
  fontSize: "0.76rem",
  fontWeight: 600,
  border: "1px solid var(--border-default)",
  borderRadius: "5px",
  backgroundColor: "var(--bg-surface)",
  color: "var(--text-secondary)",
  cursor: "pointer",
};
const allegationListStyle: React.CSSProperties = {
  maxHeight: "10rem",
  overflowY: "auto",
  border: "1px solid var(--border-default)",
  borderRadius: "5px",
  padding: "0.4rem 0.55rem",
  backgroundColor: "var(--bg-surface)",
};
const allegationRowStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "flex-start",
  gap: "0.45rem",
  fontSize: "0.82rem",
  color: "var(--text-primary)",
  margin: "0.15rem 0",
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
const warnStyle: React.CSSProperties = {
  ...errorStyle,
  backgroundColor: "var(--burden-warning-bg)",
  border: "1px solid var(--burden-warning-text)",
  color: "var(--burden-warning-text)",
};

/** Short one-line label for an allegation option: "¶54 — summary". */
function allegationLabel(a: AllegationDto): string {
  const para = a.paragraph ? `¶${a.paragraph}` : a.id;
  const summary = a.allegation ?? a.title;
  return summary ? `${para} — ${summary}` : para;
}

const ScenarioDefinitionForm: React.FC<Props> = ({
  slug,
  scenarioId,
  definition,
  anchorAllegationIds,
  onSaved,
}) => {
  // Pre-fill from the persisted definition — but ONLY if it is a clean v2 body.
  // A `{}` sentinel, a retired v1 shape, or a malformed body all yield `undefined`
  // here and the form opens blank (the "not yet authored under v2" state).
  const prefill = useMemo(
    () => parseScenarioDefinition(definition),
    [definition],
  );

  const [attackText, setAttackText] = useState(prefill?.attack_text ?? "");
  const [attackMeaning, setAttackMeaning] = useState(prefill?.attack_meaning ?? "");
  const [targetId, setTargetId] = useState(prefill?.target ?? "");
  const [wielders, setWielders] = useState<Wielder[]>(prefill?.wielders ?? []);
  const [anchorIds, setAnchorIds] = useState<string[]>(anchorAllegationIds);

  // Live vocabulary for the pickers (loaded once on mount).
  const [subjects, setSubjects] = useState<ActorOption[]>([]);
  const [actors, setActors] = useState<ActorOption[]>([]);
  const [allegations, setAllegations] = useState<AllegationDto[]>([]);
  const [vocabError, setVocabError] = useState<string | null>(null);

  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Load the picker vocabularies once. Both calls go through `authFetch`, which
  // enforces a 30s timeout via AbortController (Rule 13); the `cancelled` guard
  // prevents a state update after unmount. A failure is a VISIBLE advisory — the
  // author can still save the accusation text, so it is a warning, not a blocker
  // (Standing Rule 1: the failure is observable, not swallowed).
  useEffect(() => {
    let cancelled = false;
    Promise.all([getAvailableFilters(), getAllegations()])
      .then(([filters, alleg]) => {
        if (cancelled) return;
        setSubjects(filters.subjects);
        setActors(filters.actors);
        setAllegations(alleg.allegations);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        // Surface the underlying message for the logs/console, but show the user
        // the recovery-oriented copy from the label config.
        console.warn("ScenarioDefinitionForm: vocabulary load failed", err);
        setVocabError(L.vocabError);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // `attack_text` is the one field the backend rejects when absent — Save stays
  // disabled until it carries non-whitespace content.
  const canSave = attackText.trim().length > 0 && !saving;

  // ── Wielder row helpers ────────────────────────────────────────────────────
  const addWielder = () =>
    setWielders((prev) => [...prev, { party_id: "", actor_role: DEFAULT_ACTOR_ROLE }]);
  const removeWielder = (index: number) =>
    setWielders((prev) => prev.filter((_, i) => i !== index));
  const updateWielder = (index: number, patch: Partial<Wielder>) =>
    setWielders((prev) =>
      prev.map((w, i) => (i === index ? { ...w, ...patch } : w)),
    );

  // ── Allegation toggle ──────────────────────────────────────────────────────
  const toggleAnchor = (id: string) =>
    setAnchorIds((prev) =>
      prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id],
    );

  const handleSave = () => {
    const trimmedAttack = attackText.trim();
    if (!trimmedAttack) return; // Guard even though the button is disabled.

    setSaving(true);
    setError(null);

    // Drop half-filled wielder rows (a party never chosen). Optional scalars are
    // OMITTED when blank (not sent as ""), matching the backend
    // `skip_serializing_if` shape. `schema_v` is the mirrored current version.
    const cleanWielders = wielders.filter((w) => w.party_id.trim().length > 0);
    const nextDefinition: ScenarioDefinition = {
      attack_text: trimmedAttack,
      wielders: cleanWielders,
      schema_v: CURRENT_SCHEMA_V,
      ...(attackMeaning.trim() ? { attack_meaning: attackMeaning.trim() } : {}),
      ...(targetId ? { target: targetId } : {}),
    };

    // Send the anchors too — the picker was pre-filled from the current value, so
    // sending it back is a faithful round-trip (COALESCE replaces the column). An
    // empty array correctly clears anchors the user un-ticked.
    updateScenario(slug, scenarioId, {
      definition: nextDefinition,
      anchor_allegation_ids: anchorIds,
    })
      .then(() => {
        // Success unmounts this form (the page re-loads through its loading gate),
        // so there is no success-state to set here — the re-fetch is the feedback.
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
      <div style={headerStyle}>{L.header}</div>

      {vocabError && <div style={warnStyle}>{vocabError}</div>}

      {/* Accusation (required) */}
      <label style={fieldLabel} htmlFor="def-attack-text">
        {L.attackText.label}
      </label>
      <textarea
        id="def-attack-text"
        style={textareaStyle}
        value={attackText}
        onChange={(e) => setAttackText(e.target.value)}
        placeholder={L.attackText.placeholder}
      />
      <div style={guidanceStyle}>{L.attackText.guidance}</div>

      {/* What it means */}
      <label style={fieldLabel} htmlFor="def-attack-meaning">
        {L.attackMeaning.label}
      </label>
      <textarea
        id="def-attack-meaning"
        style={textareaStyle}
        value={attackMeaning}
        onChange={(e) => setAttackMeaning(e.target.value)}
        placeholder={L.attackMeaning.placeholder}
      />

      {/* Target — a picker over the live subjects vocabulary. */}
      <label style={fieldLabel} htmlFor="def-target">
        {L.target.label}
      </label>
      {subjects.length === 0 ? (
        <div style={hintStyle}>{L.target.empty}</div>
      ) : (
        <select
          id="def-target"
          style={selectStyle}
          value={targetId}
          onChange={(e) => setTargetId(e.target.value)}
        >
          <option value="">{L.target.placeholder}</option>
          {subjects.map((s) => (
            <option key={s.id} value={s.id}>
              {s.name}
            </option>
          ))}
        </select>
      )}

      {/* Wielders — a list of {party, role} pickers. */}
      <label style={fieldLabel}>{L.wielders.label}</label>
      <div style={hintStyle}>{L.wielders.help}</div>
      {wielders.length === 0 && <div style={hintStyle}>{L.wielders.empty}</div>}
      {wielders.map((w, i) => (
        <div style={wielderRowStyle} key={i}>
          <select
            style={selectStyle}
            aria-label="Party"
            value={w.party_id}
            onChange={(e) => updateWielder(i, { party_id: e.target.value })}
          >
            <option value="">{L.wielders.partyPlaceholder}</option>
            {actors.map((a) => (
              <option key={a.id} value={a.id}>
                {a.name}
              </option>
            ))}
          </select>
          <select
            style={selectStyle}
            aria-label="Role"
            value={w.actor_role}
            onChange={(e) =>
              updateWielder(i, { actor_role: e.target.value as Wielder["actor_role"] })
            }
          >
            {ACTOR_ROLE_OPTIONS.map((r) => (
              <option key={r.code} value={r.code}>
                {r.label}
              </option>
            ))}
          </select>
          <button type="button" style={smallBtnStyle} onClick={() => removeWielder(i)}>
            {L.wielders.removeButton}
          </button>
        </div>
      ))}
      <button
        type="button"
        style={smallBtnStyle}
        onClick={addWielder}
        disabled={actors.length === 0}
      >
        {L.wielders.addButton}
      </button>

      {/* Anchor allegations — a picker over the live allegations. */}
      <label style={fieldLabel}>{L.anchorAllegations.label}</label>
      <div style={hintStyle}>{L.anchorAllegations.help}</div>
      {allegations.length === 0 ? (
        <div style={hintStyle}>{L.anchorAllegations.empty}</div>
      ) : (
        <div style={allegationListStyle}>
          {allegations.map((a) => (
            <label style={allegationRowStyle} key={a.id}>
              <input
                type="checkbox"
                checked={anchorIds.includes(a.id)}
                onChange={() => toggleAnchor(a.id)}
              />
              <span>{allegationLabel(a)}</span>
            </label>
          ))}
        </div>
      )}

      {error && <div style={errorStyle}>{error}</div>}

      <button
        type="button"
        style={canSave ? saveBtnStyle : saveBtnDisabled}
        onClick={handleSave}
        disabled={!canSave}
      >
        {saving ? L.saving : L.save}
      </button>
    </div>
  );
};

export default ScenarioDefinitionForm;
