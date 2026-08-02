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
import { useParams } from "react-router-dom";

import { fetchRehearsal, type RehearsalPayload } from "../services/rehearsal";
import { positionLabel, stepForKey, stepTo } from "./rehearsalNav";

const pageStyle: React.CSSProperties = {
  padding: "2rem",
  maxWidth: "56rem",
  margin: "0 auto",
  // The generous line height is part of the point: this text is read aloud.
  lineHeight: 1.7,
};

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

  const load = useCallback(async () => {
    if (!slug) return;
    setLoading(true);
    try {
      const loaded = await fetchRehearsal(slug);
      setPayload(loaded);
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

  if (loading) return <div style={pageStyle}>Loading rehearsal mode…</div>;

  if (error) {
    return (
      <div style={pageStyle}>
        <div role="alert" style={{ color: "var(--state-danger-strong)" }}>
          {error}
        </div>
        <button type="button" onClick={() => void load()} style={{ marginTop: "1rem" }}>
          Retry
        </button>
      </div>
    );
  }

  if (!payload) return <div style={pageStyle}>Rehearsal mode is unavailable.</div>;

  const scenario = payload.scenarios[index];

  return (
    <div style={pageStyle}>
      <div style={{ display: "flex", alignItems: "baseline", gap: "1rem" }}>
        <h1 style={{ fontSize: "1.1rem", color: "var(--text-muted)", fontWeight: 500 }}>
          Rehearsal
        </h1>
        <span style={{ color: "var(--text-muted)" }}>{positionLabel(index, total)}</span>
      </div>

      {/* An empty rehearsal is a REAL state, not a failure: nobody has declared a
          scenario ready yet. It says so, and says what to do about it. */}
      {!scenario ? (
        <p style={{ ...bodyStyle, marginTop: "2rem" }}>
          Nothing is ready to rehearse yet. A scenario appears here once someone
          marks it ready on its page.
        </p>
      ) : (
        <>
          <div style={{ ...blockStyle, borderTop: "none", marginTop: "1rem" }}>
            <div style={blockLabelStyle}>{scenario.code} · What they say</div>
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
              <div style={{ ...bodyStyle, color: "var(--text-muted)" }}>
                No points written yet.
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
              <div style={{ ...bodyStyle, color: "var(--text-muted)" }}>
                Nothing flagged.
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
          border: "1px solid var(--border-default)",
          borderRadius: "8px",
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
