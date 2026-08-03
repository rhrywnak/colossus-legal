// =============================================================================
// RehearsalScenarioBlocks — one ready scenario, as Marie reads it (task 1.7D)
// =============================================================================
//
// Their claim, our answer, her points, what to watch for. Extracted from
// `RehearsalPage` for the module-size limit (Rule 17): task 1.7D added the
// breadcrumb, the purpose line, the code->id resolution and the guidance links,
// taking that page from 184 to over 300 non-comment lines.
//
// The seam is the obvious one — this file is ONE SCENARIO as it is read aloud, and
// what remains in the page is the shell she navigates with.
//
// ## What is NOT here, and why that is not an omission
//
// No motivation or strategy, no confidence, no verdicts, no page citations, no
// internal vocabulary (v2 §10). Those exclusions are enforced by the PAYLOAD — the
// backend DTO has no fields for them — so this component cannot show them even by
// mistake.

import React from "react";
import { Link } from "react-router-dom";

import type { RehearsalScenario } from "../services/rehearsal";

const blockLabelStyle: React.CSSProperties = {
  fontSize: "0.8rem",
  letterSpacing: "0.06em",
  textTransform: "uppercase",
  color: "var(--text-muted)",
  marginBottom: "0.35rem",
};

const blockStyle: React.CSSProperties = {
  borderTop: "1px solid var(--border-default)",
  paddingTop: "1rem",
  marginTop: "1.5rem",
};

// The scale is the one place the rehearsal surface departs from the rest of the
// app, and it does so deliberately: this text is read aloud, from a distance,
// under stress, by someone who is not looking for a row.
const bodyStyle: React.CSSProperties = { fontSize: "1.35rem" };

/**
 * One ready scenario's four blocks: their claim, our answer, her points, what to
 * watch for.
 *
 * Extracted from the page for the module-size limit (Rule 17) — task 1.7D added the
 * breadcrumb, the purpose line, the code→id map and the guidance links, which took
 * the page from 184 to 332 non-comment lines. The seam is the obvious one: this is
 * ONE SCENARIO as Marie reads it, and what remains in the page is the shell she
 * navigates with.
 *
 * `scenarioId` is optional because the code→id read is best-effort — see the page.
 * Every link built from it is guarded, so a failed resolution omits links rather
 * than rendering dead ones.
 */
const RehearsalScenarioBlocks: React.FC<{
  scenario: RehearsalScenario;
  slug: string | undefined;
  scenarioId: string | undefined;
}> = ({ scenario, slug, scenarioId }) => {
  /** The scenario page, when we could resolve it — otherwise plain text. */
  const link = (text: string) =>
    scenarioId ? (
      <Link
        to={`/cases/${slug}/trial-prep/${scenarioId}`}
        style={{ color: "var(--accent-primary)" }}
      >
        {text}
      </Link>
    ) : (
      text
    );

  return (
    <>
          <div style={{ ...blockStyle, borderTop: "none", marginTop: "1.5rem" }}>
            <div
              style={{
                ...blockLabelStyle,
                display: "flex",
                alignItems: "baseline",
                gap: "0.75rem",
                flexWrap: "wrap",
              }}
            >
              <span>{scenario.code} · What they say</span>
              {/* Item 9a: back to the scenario this screen is showing. Omitted —
                  never rendered as a dead or wrong link — when the id could not be
                  resolved (see `idByCode`). */}
              {scenarioId && (
                <Link
                  to={`/cases/${slug}/trial-prep/${scenarioId}`}
                  style={{
                    fontSize: "0.75rem",
                    textTransform: "none",
                    letterSpacing: "normal",
                    color: "var(--accent-primary)",
                    textDecoration: "none",
                  }}
                >
                  ‹ Back to {scenario.code}
                </Link>
              )}
            </div>
            <div style={bodyStyle}>
              {scenario.attack ?? "Their claim has not been written down yet."}
            </div>
          </div>

          <div style={blockStyle}>
            <div style={blockLabelStyle}>Our answer</div>
            <div style={{ ...bodyStyle, fontWeight: 600 }}>
              {scenario.theme ?? "Our answer has not been framed yet."}
            </div>
          </div>

          <div style={blockStyle}>
            <div style={blockLabelStyle}>Your points</div>
            {scenario.points.length === 0 ? (
              // Item 9c: an empty block says WHERE the fix is, not just that it is
              // empty. A ready scenario with no talking points is a real state — the
              // readiness gate does not require them — so this is guidance, not an
              // error.
              <div style={{ ...bodyStyle, color: "var(--text-muted)" }}>
                No talking points yet —{" "}
                {link("add them on the scenario page")}
                .
              </div>
            ) : (
              <ol style={{ ...bodyStyle, paddingLeft: "1.4rem" }}>
                {scenario.points.map((point, i) => (
                  <li key={i} style={{ marginBottom: "0.6rem" }}>
                    {point.text}
                    {/* The paired exhibit, when one is authored. A plain label —
                        never a page or a line number: §10 excludes pinpoint
                        impeachment sourcing from this surface. */}
                    {point.exhibit && (
                      <span style={{ color: "var(--text-muted)", fontSize: "1rem" }}>
                        {" "}
                        ({point.exhibit})
                      </span>
                    )}
                  </li>
                ))}
              </ol>
            )}
          </div>

          <div style={blockStyle}>
            <div style={blockLabelStyle}>Watch for</div>
            {scenario.watch_list.length === 0 ? (
              // Honest, and deliberately NOT phrased as a warning: nothing flagged
              // is a legitimate and often correct state for a scenario.
              <div style={{ ...bodyStyle, color: "var(--text-muted)" }}>
                Nothing flagged yet —{" "}
                {link("add watch-list notes on the scenario page")}
                .
              </div>
            ) : (
              <ul style={{ ...bodyStyle, paddingLeft: "1.4rem" }}>
                {scenario.watch_list.map((note, i) => (
                  <li key={i} style={{ marginBottom: "0.6rem" }}>
                    {note}
                  </li>
                ))}
              </ul>
            )}
          </div>

    </>
  );
};

export default RehearsalScenarioBlocks;
