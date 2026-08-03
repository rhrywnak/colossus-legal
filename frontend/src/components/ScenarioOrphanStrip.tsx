// =============================================================================
// ScenarioOrphanStrip — the humane orphan strip (defect D9, §2.8)
// =============================================================================
//
// 26 saved references whose content is gone. Measured on DEV, that is scenario
// S-2's entire ruling history from before 2026-07-25, including six human INCLUDE
// decisions — the July re-extraction loss, and the origin of the durability law.
//
// ## What was wrong with the 1.7B version
//
// It hung off the bottom of the ruling queue, always expanded, and listed 26 raw
// `graph_node_id` strings under the words "no longer in the graph". Accurate and
// unusable: a human could not tell WHAT had been lost, and every visit to the page
// re-presented the loss as a wall of hashes.
//
// §2.8's answer is a collapsed disclosure with plain language and counts grouped by
// document title. The point is not to hide it — the summary line still states the
// number every time — it is that "18 rulings from George Phillips's discovery
// response" is a sentence a human can act on and a list of hashes is not.
//
// ## Nothing is invented
//
// The grouping and every label arrive composed from `/facts/orphans`. Two of the 26
// point at a document id that no longer exists (a single-`l` `ruling` vs the live
// double-`l` `rulling` — the OCR spelling, see the backend's service module), and
// those render under an id-derived slug rather than borrowing the live document's
// title. §2.8 is explicit: report honestly, fall back to what the data supports, do
// not invent titles.

import React, { useCallback, useEffect, useState } from "react";

import { fetchScenarioOrphans, type ScenarioOrphansDto } from "../services/scenarioOrphans";
import { DIVIDER } from "./scenarioSectionStyles";

// Mockup `.orphans`: margin-top 24px, padding 14px 20px, 13.5px, --text-2, on its
// own parchment (#fbf7ea) with radius 12 and --shadow-card. Borderless like every
// other v3 surface — the shadow is the edge.
const wrapStyle: React.CSSProperties = {
  marginTop: "24px",
  borderRadius: "var(--radius-card)",
  boxShadow: "var(--shadow-card)",
  padding: "14px 20px",
  fontSize: "13.5px",
  color: "var(--text-secondary)",
  background: "var(--v3-orphan-bg)",
};

interface Props {
  slug: string;
  scenarioId: string;
  /** Bumped when a ruling lands, so a newly-orphaned ref cannot go unnoticed. */
  externalRefresh?: number;
}

const ScenarioOrphanStrip: React.FC<Props> = ({ slug, scenarioId, externalRefresh }) => {
  const [data, setData] = useState<ScenarioOrphansDto | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(() => {
    fetchScenarioOrphans(slug, scenarioId)
      .then((loaded) => {
        setData(loaded);
        setError(null);
      })
      .catch((e: unknown) => {
        // Explicit error UI. An orphan strip that failed silently would render as
        // nothing, and nothing means "nothing was lost" — the one thing this
        // surface must never say by accident (Standing Rule 1).
        setError(
          e instanceof Error
            ? e.message
            : "Could not check this scenario for saved references whose content is unavailable.",
        );
      });
  }, [slug, scenarioId]);

  useEffect(() => {
    load();
    // `externalRefresh`'s VALUE is never used, only its change.
  }, [load, externalRefresh]);

  if (error) {
    return (
      <div style={{ ...wrapStyle, color: "var(--state-danger-strong)" }} role="alert">
        {error}
        <button
          type="button"
          onClick={load}
          style={{
            marginLeft: "0.6rem",
            border: DIVIDER,
            borderRadius: "6px",
            background: "var(--bg-surface)",
            cursor: "pointer",
            fontFamily: "inherit",
            fontSize: "0.8rem",
            padding: "0.15rem 0.5rem",
          }}
        >
          Retry
        </button>
      </div>
    );
  }

  // Nothing orphaned → nothing rendered. This is the one absence on the page that
  // is genuinely good news, and a "0 unavailable references" strip would draw the
  // eye to a problem that does not exist.
  if (!data || data.total === 0) return null;

  return (
    <details style={wrapStyle}>
      {/* `listStyle: none` + the webkit reset removes the native disclosure
          triangle; the mockup's strip opens from its own "show" link. */}
      <summary
        style={{
          cursor: "pointer",
          color: "var(--text-primary)",
          listStyle: "none",
        }}
        className="no-marker"
      >
        ⚠ {data.total} saved reference{data.total === 1 ? "" : "s"} with content
        unavailable · <span style={{ color: "var(--accent-primary)" }}>show</span>
      </summary>

      <div style={{ marginTop: "0.6rem", lineHeight: 1.7 }}>
        <p style={{ margin: "0 0 0.6rem" }}>
          These survive from before rulings carried anchors; a re-processing in July
          destroyed the records they pointed to. They are kept so that nothing is
          lost silently — task 2.5 adds the formal acknowledge-and-clear step.
        </p>
        <ul style={{ margin: 0, paddingLeft: "1.1rem" }}>
          {data.groups.map((group) => (
            <li key={group.document_id || group.label} style={{ marginBottom: "0.2rem" }}>
              <strong style={{ fontWeight: 600 }}>{group.count}</strong> from {group.label}
              {/* The label's trustworthiness is stated, never guessed at from its
                  shape. `resolved` needs no annotation — it IS the document's
                  title. */}
              {group.title_state === "unrecoverable" && (
                <span style={{ color: "var(--text-muted)" }}>
                  {" "}
                  — this document is no longer in the system, so its title cannot be
                  recovered
                </span>
              )}
              {group.title_state === "unparseable" && (
                <span style={{ color: "var(--text-muted)" }}>
                  {" "}
                  — the saved reference does not name a document this build recognises
                </span>
              )}
            </li>
          ))}
        </ul>
      </div>
    </details>
  );
};

export default ScenarioOrphanStrip;
