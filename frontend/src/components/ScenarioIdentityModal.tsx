// =============================================================================
// ScenarioIdentityModal — the one place a scenario's identity is edited (1.7B)
// =============================================================================
//
// ONE modal, everything on one screen — the Casefleet Edit-Fact pattern (study
// §1.6). It replaces the permanent full-width definition form that used to sit
// between the reader and the evidence, and the scroll-box allegation picker that
// came with it. Nothing here opens a second page.
//
// ## What it edits, and what it deliberately does not
//
// Edits: name · the three texts · the TARGET · anchor allegations (as chips) ·
// the definition body's carried-through fields.
//
// The target joined on 2026-08-07. Until then this modal carried it through
// untouched, on the assumption that something else authored it — and nothing
// did, so every UI-created scenario had none and silently gathered evidence over
// the case-default subject. This control is how an existing scenario is
// completed without a hand-written SQL update.
//
// Does NOT edit `direction`. A scenario's offense/defense stance is its
// identity, not an attribute — the backend refuses it on the update route, and a
// wrongly-created direction is cured by archive-and-recreate (task 3.6). The
// chip states it, with the reason on hover.
//
// Does NOT edit `status`. Declaring a scenario ready is a recorded human act
// with an actor against it (v2 §5/§6); it has its own route and its own control.
//
// ## Why the identity is fetched here rather than passed in
//
// `theme_statement`, `motivation` and `direction` are not on the scenario-detail
// payload this page loads; they arrive on the augmentation payload. Rather than
// widen a DTO for one dialog, the modal reads what it needs when it OPENS — the
// coldest path in the product, one small request, and the page keeps its
// existing contract.

import React, { useCallback, useEffect, useState } from "react";

import Modal from "./Modal";
import { directionChip } from "./scenarioHeader";
import {
  canSave,
  draftFrom,
  patchFrom,
  definitionWouldBeLost,
  withAllegation,
  withoutAllegation,
  type IdentityDraft,
} from "./scenarioIdentity";
import { getAllegations, type AllegationDto } from "../services/allegations";
import { getAvailableFilters, type ActorOption } from "../services/bias";
import {
  fetchAugmentationPanel,
  type ScenarioIdentityWording,
} from "../services/scenarioAugmentation";
import { updateScenario } from "../services/scenarioCrud";
import type { ScenarioDefinition } from "../pages/trialPrepData";

const HAIRLINE = "1px solid var(--border-default)";

const S = {
  label: {
    fontSize: "0.75rem",
    color: "var(--text-muted)",
    marginBottom: "0.2rem",
  } as React.CSSProperties,
  help: {
    fontSize: "0.72rem",
    color: "var(--text-muted)",
    marginTop: "0.15rem",
  } as React.CSSProperties,
  field: {
    border: HAIRLINE,
    borderRadius: "6px",
    padding: "0.4rem 0.6rem",
    fontWeight: 400,
    fontFamily: "inherit",
    fontSize: "0.88rem",
    width: "100%",
    background: "var(--bg-surface)",
  } as React.CSSProperties,
  block: {
    display: "flex",
    flexDirection: "column",
    marginBottom: "0.9rem",
  } as React.CSSProperties,
  chipRow: {
    display: "flex",
    flexWrap: "wrap",
    gap: "0.35rem",
    alignItems: "center",
  } as React.CSSProperties,
  chip: {
    display: "inline-flex",
    alignItems: "center",
    gap: "0.35rem",
    border: HAIRLINE,
    borderRadius: "999px",
    padding: "0.15rem 0.6rem",
    fontSize: "0.76rem",
    color: "var(--text-secondary)",
    background: "var(--bg-surface)",
  } as React.CSSProperties,
  chipRemove: {
    border: "none",
    background: "none",
    color: "var(--text-muted)",
    cursor: "pointer",
    fontSize: "0.9rem",
    lineHeight: 1,
    padding: 0,
    fontFamily: "inherit",
  } as React.CSSProperties,
  readonlyChip: {
    display: "inline-flex",
    border: HAIRLINE,
    borderRadius: "999px",
    padding: "0.15rem 0.6rem",
    fontSize: "0.76rem",
  } as React.CSSProperties,
  buttons: {
    display: "flex",
    justifyContent: "flex-end",
    gap: "0.5rem",
    marginTop: "0.5rem",
  } as React.CSSProperties,
  button: {
    padding: "0.4rem 1rem",
    fontSize: "0.84rem",
    fontWeight: 500,
    border: HAIRLINE,
    borderRadius: "4px",
    background: "var(--bg-surface)",
    color: "var(--text-secondary)",
    cursor: "pointer",
    fontFamily: "inherit",
  } as React.CSSProperties,
  save: {
    padding: "0.4rem 1rem",
    fontSize: "0.84rem",
    fontWeight: 500,
    border: "1px solid var(--accent-primary)",
    borderRadius: "4px",
    background: "var(--accent-primary)",
    // The surface token is what reads as white ON the accent — the same pairing
    // `ScenarioDeleteConfirm` uses for its primary button. No literal hex.
    color: "var(--bg-surface)",
    cursor: "pointer",
    fontFamily: "inherit",
  } as React.CSSProperties,
  disabled: { opacity: 0.5, cursor: "not-allowed" } as React.CSSProperties,
  errorBox: {
    margin: "0 0 0.75rem",
    padding: "0.55rem 0.75rem",
    background: "var(--state-danger-bg-soft)",
    border: "1px solid var(--state-danger-border)",
    borderRadius: "6px",
    color: "var(--state-danger-strong)",
    fontSize: "0.82rem",
  } as React.CSSProperties,
};

interface Props {
  slug: string;
  scenarioId: string;
  /** The definition body as last saved — carried through the patch untouched
   *  where this modal does not edit it (`target`, `wielders`). */
  definition?: ScenarioDefinition;
  anchorAllegationIds: string[];
  /** Called after a save that actually persisted; the page re-fetches. */
  onSaved: () => void;
  onClose: () => void;
}

/** "¶54 — summary", the same one-line form the retired picker used. */
function allegationLabel(a: AllegationDto): string {
  const paragraph = a.paragraph ? `¶${a.paragraph}` : a.id;
  const summary = a.allegation ?? a.title;
  return summary ? `${paragraph} — ${summary}` : paragraph;
}

const ScenarioIdentityModal: React.FC<Props> = ({
  slug,
  scenarioId,
  definition,
  anchorAllegationIds,
  onSaved,
  onClose,
}) => {
  const [draft, setDraft] = useState<IdentityDraft | null>(null);
  const [direction, setDirection] = useState<string | null>(null);
  const [allegations, setAllegations] = useState<AllegationDto[]>([]);
  const [wording, setWording] = useState<ScenarioIdentityWording | null>(null);
  // `null` = still loading. `[]` after a failure, with `error` set — never an
  // empty list presented as "this case has no people in it".
  const [subjects, setSubjects] = useState<ActorOption[] | null>(null);
  const [picker, setPicker] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // One load on open: the identity fields this page's payload does not carry,
  // and the allegations the chip picker offers.
  useEffect(() => {
    let live = true;
    Promise.all([fetchAugmentationPanel(slug, scenarioId), getAllegations()])
      .then(([panel, alleg]) => {
        if (!live) return;
        setDirection(panel.identity.direction);
        setWording(panel.identity_wording);
        setDraft(
          draftFrom({
            name: panel.identity.name,
            themeStatement: panel.identity.theme_statement,
            motivation: panel.identity.motivation,
            definition,
            anchorAllegationIds,
          }),
        );
        setAllegations(alleg.allegations);
      })
      .catch((e: unknown) => {
        if (!live) return;
        // Standing Rule 1: the modal stays open with the failure on screen. An
        // empty form would invite someone to retype an identity that is
        // already stored and then overwrite it with a partial one.
        setError(
          e instanceof Error
            ? e.message
            : "Could not load this scenario's identity. Close and try again.",
        );
      });
    return () => {
      live = false;
    };
  }, [slug, scenarioId, definition, anchorAllegationIds]);

  // The party vocabulary the target list offers — the same endpoint the create
  // form and the Bias Explorer's "About" filter read, so all three agree on who
  // this case knows about.
  //
  // Its own effect rather than a third promise in the one above: that effect
  // re-runs whenever the definition prop changes, and re-fetching a 120-row
  // catalogue on every save round-trip would be waste. This one runs once.
  //
  // It closes over `slug` and `scenarioId` for its failure log, and the empty
  // dependency list is still correct: the page mounts this modal only while it is
  // open and unmounts it on close, so neither value can change during this
  // component's life. Adding them as dependencies would re-fetch the catalogue
  // for no behavioural gain.
  useEffect(() => {
    let live = true;
    getAvailableFilters()
      .then((filters) => {
        if (live) setSubjects(filters.subjects);
      })
      .catch((e: unknown) => {
        if (!live) return;
        // The human-facing half is `subjectsFailed` below, which renders the
        // stored "do not save" sentence — the only honest thing to say before the
        // wording arrives, since there is no literal to fall back to.
        //
        // The OPERATOR-facing half used to be nothing at all: this catch took the
        // error object and dropped it, so a failed target-vocabulary load left no
        // trace anywhere a diagnosis could start from (audit defect 20). Rule 1's
        // best-effort carve-out covers cosmetic browser storage and explicitly
        // not an `authFetch`, so the cause is logged even though the state is
        // already visible.
        // NAMES the scenario. Two identity modals can be open in two tabs, and a
        // console line that cannot say which one it came from sends an operator
        // to the URL bar to find out.
        console.warn(
          `Scenario identity: the party vocabulary failed to load for ${slug}/${scenarioId}:`,
          e,
        );
        setSubjects([]);
      });
    return () => {
      live = false;
    };
  }, []);

  const edit = useCallback(
    (patch: Partial<IdentityDraft>) =>
      setDraft((current) => (current ? { ...current, ...patch } : current)),
    [],
  );

  const handleSave = () => {
    if (!draft || saving) return;
    // The refusals this dialog owns: a definition field typed while "what they
    // say" is blank cannot be stored, and saving anyway would drop it without a
    // word. Said out loud rather than expressed as a dead button.
    //
    // Each answer gets its OWN sentence. A human who picked a person and a human
    // who wrote a gloss made different edits and need different instructions,
    // and one message covering both would have to name neither field.
    const lost = definitionWouldBeLost(draft);
    if (lost !== null) {
      setError(
        (lost === "target"
          ? wording?.target_needs_attack_text
          : wording?.meaning_needs_attack_text) ?? null,
      );
      return;
    }
    if (!canSave(draft)) return;
    setSaving(true);
    setError(null);
    updateScenario(slug, scenarioId, patchFrom(draft, definition))
      .then(() => {
        setSaving(false);
        // Close only AFTER the write resolved: the modal closing is never proof
        // the save happened. The page re-fetches rather than trusting the draft,
        // because the backend trims and normalises what it stores.
        onSaved();
        onClose();
      })
      .catch((e: unknown) => {
        setSaving(false);
        setError(
          e instanceof Error ? e.message : "Failed to save. Your text is still here.",
        );
      });
  };

  const chosen = draft?.anchorAllegationIds ?? [];
  const unchosen = allegations.filter((a) => !chosen.includes(a.id));
  // An empty list AFTER loading means the read failed (a case with no parties
  // cannot have a scenario), and the stored sentence for it says not to save.
  const subjectsFailed = subjects !== null && subjects.length === 0;

  return (
    <Modal title="Scenario identity" busy={saving} onClose={onClose}>
      {error && <div style={S.errorBox}>{error}</div>}
      {subjectsFailed && wording && (
        <div style={S.errorBox}>{wording.target_options_failed_notice}</div>
      )}

      {!draft ? (
        <div style={{ color: "var(--text-muted)", fontSize: "0.85rem" }}>
          {error ? null : "Loading…"}
        </div>
      ) : (
        <>
          <div style={S.block}>
            <label style={S.label} htmlFor="identity-name">
              Name
            </label>
            <input
              id="identity-name"
              autoFocus
              style={S.field}
              value={draft.name}
              disabled={saving}
              onChange={(e) => edit({ name: e.target.value })}
            />
          </div>

          {/* The three texts, in the order a human thinks about them: what they
              said, what we answer, why they said it. Each labelled in plain
              language — the labels are what stop the three collapsing into one
              in someone's head. */}
          <div style={S.block}>
            <label style={S.label} htmlFor="identity-attack">
              {wording?.attack_label}
            </label>
            <textarea
              id="identity-attack"
              style={{ ...S.field, minHeight: "4.5rem" }}
              value={draft.attackText}
              disabled={saving}
              onChange={(e) => edit({ attackText: e.target.value })}
            />

          </div>

          {/* ONE ATTACK BOX (Roman, 2026-08-10). "What that is meant to imply"
              is GONE from this dialog.

              It asked the same question as the box above it in different words,
              and the create form seeded both from one answer — so every UI-made
              scenario arrived with two identical texts, one of which no read
              surface rendered. The scan judged the copy. Two boxes for one idea
              is how they drift apart later.

              The COLUMN survives and nothing stored is destroyed: a legacy
              scenario that has a gloss and no attack text still scans against it
              (`theme_scan_validate`'s fallback). This modal simply stops asking
              for a second one, and `patchFrom` carries whatever is already there
              through untouched. */}
          {/* The target sits after the two attack texts and before our answer:
              it completes the description of what THEY are doing, and it is the
              field that decides what evidence this scenario can even see. Its
              words come from stored rows, unlike the labels above it, which
              predate the language law. */}
          {wording && (
            <div style={S.block}>
              <label style={S.label} htmlFor="identity-target">
                {wording.target_label}
              </label>
              <select
                id="identity-target"
                style={S.field}
                value={draft.target}
                disabled={saving || subjects === null}
                onChange={(e) => edit({ target: e.target.value })}
              >
                <option value="">{wording.target_unset_option}</option>
                {/* A target already stored but absent from the vocabulary would
                    otherwise vanish from the list, and saving would then clear
                    it. It is offered by its raw id so the human can SEE that
                    something is set and that the graph no longer knows it. */}
                {draft.target.length > 0 &&
                  !(subjects ?? []).some((s) => s.id === draft.target) && (
                    <option value={draft.target}>{draft.target}</option>
                  )}
                {(subjects ?? []).map((s) => (
                  <option key={s.id} value={s.id}>
                    {s.name}
                  </option>
                ))}
              </select>
              <span style={S.help}>{wording.target_helper}</span>
            </div>
          )}

          <div style={S.block}>
            <label style={S.label} htmlFor="identity-theme">
              {wording?.theme_label}
            </label>
            <input
              id="identity-theme"
              style={S.field}
              value={draft.themeStatement}
              disabled={saving}
              onChange={(e) => edit({ themeStatement: e.target.value })}
            />
            <span style={S.help}>{wording?.theme_helper}</span>
          </div>

          <div style={S.block}>
            <label style={S.label} htmlFor="identity-motivation">
              {wording?.motivation_label}
            </label>
            <input
              id="identity-motivation"
              style={S.field}
              value={draft.motivation}
              disabled={saving}
              onChange={(e) => edit({ motivation: e.target.value })}
            />
          </div>

          {/* Direction: stated, not offered. See the module header. */}
          {direction && (
            <div style={S.block}>
              <span style={S.label}>Direction</span>
              <span>
                <span
                  style={{ ...S.readonlyChip, color: directionChip(direction).color }}
                  title={directionChip(direction).title ?? undefined}
                >
                  {directionChip(direction).label}
                </span>
              </span>
              <span style={S.help}>{directionChip(direction).title}</span>
            </div>
          )}

          {/* Allegation CHIPS with an inline picker — the scroll-box of
              checkboxes is retired (study §1.4: chips are the cross-reference
              currency). */}
          <div style={S.block}>
            <span style={S.label}>{wording?.bears_on_label}</span>
            <div style={S.chipRow}>
              {chosen.map((id) => {
                const found = allegations.find((a) => a.id === id);
                return (
                  <span key={id} style={S.chip}>
                    {found ? allegationLabel(found) : id}
                    <button
                      type="button"
                      style={S.chipRemove}
                      aria-label={`Remove ${found ? allegationLabel(found) : id}`}
                      disabled={saving}
                      onClick={() => setDraft(withoutAllegation(draft, id))}
                    >
                      ×
                    </button>
                  </span>
                );
              })}
              {chosen.length === 0 && (
                <span style={{ ...S.help, marginTop: 0 }}>None yet.</span>
              )}
            </div>
            <select
              aria-label="Add a complaint paragraph"
              style={{ ...S.field, marginTop: "0.4rem" }}
              value={picker}
              disabled={saving || unchosen.length === 0}
              onChange={(e) => {
                if (!e.target.value) return;
                setDraft(withAllegation(draft, e.target.value));
                setPicker("");
              }}
            >
              <option value="">
                {unchosen.length === 0 ? "All paragraphs added" : "Add a paragraph…"}
              </option>
              {unchosen.map((a) => (
                <option key={a.id} value={a.id}>
                  {allegationLabel(a)}
                </option>
              ))}
            </select>
          </div>

          <div style={S.buttons}>
            <button
              type="button"
              style={saving ? { ...S.button, ...S.disabled } : S.button}
              disabled={saving}
              onClick={onClose}
            >
              Cancel
            </button>
            <button
              type="button"
              style={
                canSave(draft) && !saving ? S.save : { ...S.save, ...S.disabled }
              }
              disabled={!canSave(draft) || saving}
              onClick={handleSave}
            >
              {saving ? "Saving…" : "Save"}
            </button>
          </div>
        </>
      )}
    </Modal>
  );
};

export default ScenarioIdentityModal;
