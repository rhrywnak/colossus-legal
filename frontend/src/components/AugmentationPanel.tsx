// =============================================================================
// AugmentationPanel — where human content is authored (task 1.4, v2 §8)
// =============================================================================
//
// The three components no system process may touch: C1 identity, C4 human facts,
// C5 Marie's talking points. One modal, everything on one screen — the Casefleet
// Edit-Fact pattern (study §1.6), not a wizard and not a second page.
//
// ## Human content is visibly tagged
//
// Every fact and point renders with its `authored_tag` ("Added by Roman") and no
// citation, because that is what it IS: knowledge with a person behind it rather
// than a document. §8 requires the distinction be visible, and the tag arrives
// composed from the backend — the browser does not decide how provenance reads.
//
// ## The cap comes from the payload
//
// `talking_points_cap` is served, not hardcoded here. It is a tunable that task
// 1.6 moves to the settings store, and a browser that baked in "3" would show
// the wrong limit the day it changes. The backend enforces it regardless — this
// only stops the human typing a fourth point they cannot save.

import React, { useCallback, useEffect, useState } from "react";

import {
  addHumanFact,
  deleteHumanFact,
  fetchAugmentationPanel,
  setTalkingPoints,
  type AugmentationPanelDto,
} from "../services/scenarioAugmentation";
import { updateScenario } from "../services/scenarioCrud";
import WatchListBlock from "./WatchListBlock";

const SURFACE = "var(--bg-surface)";
const HAIRLINE = "1px solid var(--border-default)";

const boxStyle: React.CSSProperties = {
  background: SURFACE,
  border: HAIRLINE,
  borderRadius: "8px",
  padding: "1rem",
  display: "flex",
  flexDirection: "column",
  gap: "0.75rem",
  fontWeight: 400,
};

const fieldStyle: React.CSSProperties = {
  border: HAIRLINE,
  borderRadius: "6px",
  padding: "0.4rem 0.6rem",
  fontWeight: 400,
  width: "100%",
};

const labelStyle: React.CSSProperties = {
  fontSize: "0.75rem",
  color: "var(--text-muted)",
};

const tagStyle: React.CSSProperties = {
  fontSize: "0.72rem",
  color: "var(--text-muted)",
  fontStyle: "italic",
};

interface Props {
  slug: string;
  scenarioId: string;
  /** Bumped by the parent when the scenario changed elsewhere. */
  externalRefresh?: number;
}

const AugmentationPanel: React.FC<Props> = ({ slug, scenarioId, externalRefresh }) => {
  const [panel, setPanel] = useState<AugmentationPanelDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const [factText, setFactText] = useState("");
  const [factDate, setFactDate] = useState("");
  const [factDateType, setFactDateType] = useState("exact");
  const [points, setPoints] = useState<string[]>([]);
  const [theme, setTheme] = useState("");
  const [motivation, setMotivation] = useState("");

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const loaded = await fetchAugmentationPanel(slug, scenarioId);
      setPanel(loaded);
      setPoints(loaded.talking_points.map((p) => p.text));
      setTheme(loaded.identity.theme_statement ?? "");
      setMotivation(loaded.identity.motivation ?? "");
      setError(null);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Failed to load the panel.");
    } finally {
      setLoading(false);
    }
  }, [slug, scenarioId]);

  useEffect(() => {
    void load();
  }, [load, externalRefresh]);

  /** Run a write, surface any failure, and re-read so the screen matches the DB. */
  const run = useCallback(
    async (work: () => Promise<void>) => {
      try {
        await work();
        setError(null);
        await load();
      } catch (e: unknown) {
        // Explicit error UI, never a swallowed rejection: the human just typed
        // this, and they are the only one who can act on the refusal.
        setError(e instanceof Error ? e.message : "That did not save.");
      }
    },
    [load],
  );

  if (loading) return <div style={{ padding: "1rem" }}>Loading the panel…</div>;
  if (!panel) {
    return (
      <div style={{ padding: "1rem", color: "var(--state-danger-strong)" }}>
        <div>{error ?? "The panel is unavailable."}</div>
        <button type="button" onClick={() => void load()} style={{ marginTop: "0.5rem" }}>
          Retry
        </button>
      </div>
    );
  }

  const capReached = points.filter((p) => p.trim()).length >= panel.talking_points_cap;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
      {error && (
        <div role="alert" style={{ ...boxStyle, color: "var(--state-danger-strong)" }}>
          {error}
        </div>
      )}

      {/* ── C1 identity ────────────────────────────────────────────────── */}
      <div style={boxStyle}>
        <div style={labelStyle}>
          {panel.identity.code} · {panel.identity.direction}
        </div>

        {/* Their framing sits above ours, unlabelled as editable, so the two are
            never mistaken for each other. */}
        {panel.identity.attack_text && (
          <div style={{ fontSize: "0.85rem", color: "var(--text-muted)" }}>
            They say: {panel.identity.attack_text}
          </div>
        )}

        <label style={labelStyle} htmlFor="theme">
          Our answer, in one sentence
        </label>
        <input
          id="theme"
          value={theme}
          onChange={(e) => setTheme(e.target.value)}
          style={fieldStyle}
        />

        <label style={labelStyle} htmlFor="motivation">
          What they want the jury to believe
        </label>
        <input
          id="motivation"
          value={motivation}
          onChange={(e) => setMotivation(e.target.value)}
          style={fieldStyle}
        />

        <button
          type="button"
          onClick={() =>
            void run(async () => {
              await updateScenario(slug, scenarioId, {
                theme_statement: theme,
                motivation,
              });
            })
          }
          style={{ alignSelf: "flex-start" }}
        >
          Save identity
        </button>
      </div>

      {/* ── C4 human facts ─────────────────────────────────────────────── */}
      <div style={boxStyle}>
        <div style={labelStyle}>Human facts — knowledge in no document</div>

        {panel.human_facts.map((fact) => (
          <div key={fact.id} style={{ borderBottom: HAIRLINE, paddingBottom: "0.5rem" }}>
            <div style={{ fontSize: "0.9rem", lineHeight: 1.6 }}>{fact.text}</div>
            <div style={{ display: "flex", gap: "0.5rem", alignItems: "baseline" }}>
              {fact.date_label && <span style={labelStyle}>{fact.date_label}</span>}
              {fact.person_refs.length > 0 && (
                <span style={labelStyle}>
                  {fact.person_refs.join(", ")}
                  {/* Said plainly: these are names someone typed, not resolved
                      entities. Task B0 is what makes them links. */}
                  {!fact.person_refs_are_linked && " (typed, not linked)"}
                </span>
              )}
              <span style={tagStyle}>
                {fact.authored_tag}
                {/* A human fact has no citation to check it against, so "this
                    was changed after it was first written" is the only signal a
                    reviewer gets that the text has moved. */}
                {fact.edited && " · edited since written"}
              </span>
              <button
                type="button"
                onClick={() =>
                  void run(async () => {
                    await deleteHumanFact(slug, scenarioId, fact.id);
                  })
                }
                style={{ marginLeft: "auto" }}
              >
                Remove
              </button>
            </div>
          </div>
        ))}

        <label style={labelStyle} htmlFor="fact-text">
          Add a fact
        </label>
        <textarea
          id="fact-text"
          value={factText}
          onChange={(e) => setFactText(e.target.value)}
          rows={2}
          style={fieldStyle}
        />
        <div style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
          <input
            type="date"
            value={factDate}
            onChange={(e) => setFactDate(e.target.value)}
            aria-label="Date (optional)"
            style={{ ...fieldStyle, width: "auto" }}
          />
          <select
            value={factDateType}
            onChange={(e) => setFactDateType(e.target.value)}
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
            onClick={() =>
              void run(async () => {
                await addHumanFact(slug, scenarioId, {
                  text: factText,
                  // The date type only travels WITH a date — the backend refuses
                  // a qualifier with nothing to qualify.
                  ...(factDate ? { occurred_on: factDate, date_type: factDateType } : {}),
                });
                setFactText("");
                setFactDate("");
              })
            }
          >
            Add
          </button>
        </div>
      </div>

      {/* ── C6 watch-list ──────────────────────────────────────────────── */}
      <WatchListBlock
        slug={slug}
        scenarioId={scenarioId}
        notes={panel.watch_list}
        run={(work) => void run(work)}
        boxStyle={boxStyle}
        labelStyle={labelStyle}
        fieldStyle={fieldStyle}
        tagStyle={tagStyle}
        hairline={HAIRLINE}
      />

      {/* ── C5 talking points ──────────────────────────────────────────── */}
      <div style={boxStyle}>
        <div style={labelStyle}>
          Marie&rsquo;s talking points — up to {panel.talking_points_cap}, in her words
        </div>

        {points.map((point, index) => (
          <input
            key={index}
            value={point}
            onChange={(e) => {
              const next = [...points];
              next[index] = e.target.value;
              setPoints(next);
            }}
            aria-label={`Talking point ${index + 1}`}
            style={fieldStyle}
          />
        ))}

        <div style={{ display: "flex", gap: "0.5rem" }}>
          <button
            type="button"
            disabled={capReached}
            onClick={() => setPoints([...points, ""])}
            title={capReached ? `That is already ${panel.talking_points_cap} points` : undefined}
          >
            Add a point
          </button>
          <button
            type="button"
            onClick={() =>
              void run(async () => {
                await setTalkingPoints(slug, scenarioId, points);
              })
            }
          >
            Save points
          </button>
        </div>

        {panel.talking_points.some((p) => p.authored_tag) && (
          <div style={tagStyle}>
            {panel.talking_points.find((p) => p.authored_tag)?.authored_tag}
          </div>
        )}
      </div>
    </div>
  );
};

export default AugmentationPanel;
