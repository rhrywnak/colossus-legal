// =============================================================================
// RehearsalPage — where Marie rehearses (task 1.5, v2 §10)
// =============================================================================
//
// One READY scenario per screen: our theme, their attack, her talking points,
// and the watch-list — plus the standing card, always visible.
//
// ## Why the type is bigger here than anywhere else in the app
//
// This is a rehearsal surface, not a data table. It is read aloud, from a
// distance, under stress, by someone who is not looking for a row. §2c's visual
// language still applies (one accent, hairline rules, generous whitespace); the
// scale is the one place this page departs from the rest of the app, and it does
// so deliberately.
//
// ## What is NOT on this page, and why that is not an omission
//
// No motivation or strategy, no confidence, no verdicts, no page citations, no
// internal vocabulary (v2 §10). Those exclusions are enforced by the PAYLOAD —
// the backend DTO has no fields for them — so this page cannot show them even by
// mistake. Marie's general access is unchanged: the full working view is still
// hers. The mode is slim; her rights are not.
//
// ## Phase 2 is absent, not faked
//
// Verdicts and the computed hazard/ammunition list ship later. Nothing here
// pretends they exist.

import React, { useCallback, useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";

import Breadcrumb from "../components/Breadcrumb";
import RehearsalScenarioBlocks from "../components/RehearsalScenarioBlocks";
import {
  fetchRehearsal,
  type RehearsalPayload,
  type RehearsalScenario,
} from "../services/rehearsal";
import { getTrialPrepDashboard } from "../services/trialPrep";
import { positionLabel, stepForKey, stepTo } from "./rehearsalNav";

const pageStyle: React.CSSProperties = {
  padding: "28px 40px 80px",
  maxWidth: "56rem",
  margin: "0 auto",
  // The generous line height is part of the point: this text is read aloud.
  lineHeight: 1.7,
};

/**
 * The purpose line (task 1.7D, item 9b).
 *
 * Roman's 2026-08-03 session: he opened this view and could not tell what it was
 * FOR. A rehearsal surface with no self-description reads as a stripped-down
 * scenario page — the exclusions (§10) look like missing data rather than a
 * deliberate slim.
 *
 * One sentence, naming what is here AND the rule that decides what appears. The
 * second half matters as much as the first: an empty view is otherwise
 * indistinguishable from a broken one.
 */
// CONST: user-facing copy, ordered verbatim in the instruction.
const PURPOSE =
  "Marie's testimony-prep view — the theme, the attack, her talking points, and " +
  "what the other side will wave around. Only scenarios marked Ready appear here.";

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

const bodyStyle: React.CSSProperties = { fontSize: "1.35rem" };

const RehearsalPage: React.FC = () => {
  const { slug } = useParams<{ slug: string }>();

  const [payload, setPayload] = useState<RehearsalPayload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [index, setIndex] = useState(0);

  /**
   * `code` → `scenarioId`, so each screen can link back to the scenario it shows
   * (item 9a).
   *
   * ## Why a second read and not a payload field
   *
   * The rehearsal DTO carries `code` ("S-2") but no id, and the back link needs
   * `/cases/:slug/trial-prep/:scenarioId`. Adding the id to the DTO would have been
   * one line — and a backend diff, which this task's authorization put at ZERO. The
   * Trial Prep dashboard already serves `id` and `code` together, so the map is
   * resolvable entirely on this side for the cost of one cached-ish GET.
   *
   * It degrades to nothing rather than to something wrong: if this read fails the
   * per-screen deep link is OMITTED and the Trial Prep crumb (also required by 9a)
   * still gets Marie out. A link labelled "Back to S-2" that did not go to S-2
   * would be worse than no link.
   */
  const [idByCode, setIdByCode] = useState<Record<string, string>>({});
  /**
   * Set when the code→id read failed, so the missing back links are SAID.
   *
   * Standing Rule 1 has no best-effort carve-out for a network read — the
   * localStorage exemption covers cosmetic browser-storage preferences and
   * explicitly not `fetch`/`authFetch`. A `console.warn` alone left the links
   * quietly absent, which is indistinguishable from "this build has no back
   * links". The notice is muted and non-blocking because the failure costs a
   * convenience, not the view: the breadcrumb below is unconditional.
   */
  const [backLinksUnavailable, setBackLinksUnavailable] = useState(false);

  const load = useCallback(async () => {
    if (!slug) return;
    setLoading(true);
    try {
      const loaded = await fetchRehearsal(slug);
      setPayload(loaded);

      // Best-effort, and deliberately NOT awaited into the gating path: the
      // rehearsal view must open even if the dashboard read fails, because its job
      // is to be read aloud and a missing back link does not stop that.
      getTrialPrepDashboard(slug)
        .then((dashboard) => {
          setIdByCode(
            Object.fromEntries(dashboard.scenarios.map((row) => [row.code, row.id])),
          );
          setBackLinksUnavailable(false);
        })
        .catch((e: unknown) => {
          // The deep links are omitted rather than rendered wrong — but the absence
          // is SAID, not swallowed (Standing Rule 1). Muted and inline rather than a
          // banner: the view still works and the breadcrumb still gets Marie out, so
          // an alarm would overstate a lost convenience.
          setBackLinksUnavailable(true);
          console.warn(
            "could not resolve scenario ids for the rehearsal back links; the " +
              "breadcrumb remains the way back",
            e,
          );
        });
      // Clamp rather than reset: a reload after one scenario was demoted should
      // keep Marie roughly where she was, not send her back to the start.
      setIndex((current) => stepTo(current, loaded.scenarios.length, null));
      setError(null);
    } catch (e: unknown) {
      // Explicit error UI, never a swallowed rejection (Standing Rule 1).
      setError(e instanceof Error ? e.message : "Rehearsal mode did not load.");
    } finally {
      setLoading(false);
    }
  }, [slug]);

  useEffect(() => {
    void load();
  }, [load]);

  const total = payload?.scenarios.length ?? 0;

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const step = stepForKey(event.key);
      if (step === null) return;
      // Space would otherwise scroll the page out from under the reader.
      event.preventDefault();
      setIndex((current) => stepTo(current, total, step));
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [total]);

  if (loading) return <div style={pageStyle} data-surface="v3">Loading rehearsal mode…</div>;

  if (error) {
    return (
      <div style={pageStyle} data-surface="v3">
        <div role="alert" style={{ color: "var(--state-danger-strong)" }}>
          {error}
        </div>
        <button type="button" onClick={() => void load()} style={{ marginTop: "1rem" }}>
          Retry
        </button>
      </div>
    );
  }

  if (!payload) return <div style={pageStyle} data-surface="v3">Rehearsal mode is unavailable.</div>;

  const scenario = payload.scenarios[index];

  return (
    <div style={pageStyle} data-surface="v3">
      {/* Item 9a: the browser back button must never be the only way out. */}
      <Breadcrumb
        items={[
          { label: "Dashboard", to: "/" },
          { label: "Trial Prep", to: `/cases/${slug}/trial-prep` },
          { label: "Rehearsal" },
        ]}
      />

      <div style={{ display: "flex", alignItems: "baseline", gap: "1rem" }}>
        <h1 style={{ fontSize: "1.1rem", color: "var(--text-muted)", fontWeight: 500 }}>
          Rehearsal
        </h1>
        <span style={{ color: "var(--text-muted)" }}>{positionLabel(index, total)}</span>
      </div>

      {backLinksUnavailable && (
        <p
          role="status"
          style={{
            fontSize: "0.8rem",
            color: "var(--text-muted)",
            margin: "0.35rem 0 0",
          }}
        >
          Links back to individual scenarios are unavailable right now — use Trial
          Prep above.
        </p>
      )}

      {/* Item 9b: one line saying what this view is and what decides its contents. */}
      <p
        style={{
          fontSize: "0.95rem",
          lineHeight: 1.6,
          color: "var(--text-secondary)",
          margin: "0.5rem 0 0",
          maxWidth: "44rem",
        }}
      >
        {PURPOSE}
      </p>

      {/* An empty rehearsal is a REAL state, not a failure: nobody has declared a
          scenario ready yet. It says so, and says what to do about it. */}
      {!scenario ? (
        <p style={{ ...bodyStyle, marginTop: "2rem" }}>
          Nothing is ready to rehearse yet. A scenario appears here once someone
          switches it to Ready on its page —{" "}
          <Link to={`/cases/${slug}/trial-prep`} style={{ color: "var(--accent-primary)" }}>
            open Trial Prep
          </Link>{" "}
          to pick one.
        </p>
      ) : (
        <>
          <RehearsalScenarioBlocks
            scenario={scenario}
            slug={slug}
            scenarioId={idByCode[scenario.code]}
          />

          <div style={{ display: "flex", gap: "0.75rem", marginTop: "2rem" }}>
            <button
              type="button"
              onClick={() => setIndex(stepTo(index, total, "previous"))}
              disabled={index === 0}
            >
              ← Previous
            </button>
            <button
              type="button"
              onClick={() => setIndex(stepTo(index, total, "next"))}
              disabled={index >= total - 1}
            >
              Next →
            </button>
          </div>
        </>
      )}

      {/* The standing card. Always visible, on every screen, including the empty
          one — §10 makes it the one thing that is never scrolled away from. Its
          lines are backend-composed; this renders them verbatim. */}
      <div
        style={{
          ...blockStyle,
          marginTop: "2.5rem",
          background: "var(--bg-surface)",
          // v3: a card is carried by its shadow, never a border. This one only
          // became a v3 surface when task 1.7D scoped ALL of this page's render
          // paths — before that it sat outside the palette and its hairline was
          // correct. Inside the scope the border resolves to #eef0f3 and would read
          // as a faint double edge against the shadow, which is precisely what the
          // ruling removed. `borderTop` from `blockStyle` is cleared for the same
          // reason: a card is not a section divider.
          borderTop: "none",
          borderRadius: "var(--radius-card)",
          boxShadow: "var(--shadow-card)",
          padding: "1rem 1.25rem",
        }}
      >
        <div style={blockLabelStyle}>Always</div>
        <ul style={{ fontSize: "1.15rem", paddingLeft: "1.4rem" }}>
          {payload.standing_card.map((line, i) => (
            <li key={i}>{line}</li>
          ))}
        </ul>
      </div>
    </div>
  );
};

export default RehearsalPage;
