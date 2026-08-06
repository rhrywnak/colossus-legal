// =============================================================================
// RehearsalPage — where Marie rehearses (task 2.11 B2, v2 §10)
// =============================================================================
//
// One READY scenario per screen: what it is, the accusation and every time they
// made it, our answer under each one, the timeline, her points, what to watch for
// — plus the standing card, always visible and never collapsible.
//
// ## Why the type is bigger here than anywhere else in the app
//
// This is a rehearsal surface, not a data table. It is read aloud, from a
// distance, under stress, by someone who is not looking for a row. §2c's visual
// language still applies; the scale is the one deliberate departure.
//
// ## Every word on this page is a settings row
//
// Task 2.11 B2 moved eighteen literals off this file — the purpose line, all four
// block labels, the position sentence, both empty states, and the standing card
// itself. Roman edits this page's language without a build.
//
// Three strings remain in code and are marked as such below: the loading line,
// the load-failure line, and Retry. They describe the state where NO PAYLOAD
// EXISTS, so they cannot be served by the payload they are reporting the failure
// of. Recorded as an exception rather than quietly excused.
//
// ## The per-scenario address
//
// `/cases/:slug/rehearsal/:code` selects within the payload this page already
// loaded. A code that is not in it — because nobody declared that scenario ready
// — gets the stored not-ready sentence. NOT a 404: the address is right and the
// scenario simply is not ready. NOT a leak: the payload never contained it, so
// this page can say nothing about it beyond the code the reader typed.

import React, { useCallback, useEffect, useMemo, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";

import Breadcrumb from "../components/Breadcrumb";
import RehearsalScenarioBlocks from "../components/RehearsalScenarioBlocks";
import { ghostButtonStyle } from "../components/scenarioSectionStyles";
import { fetchRehearsal, type RehearsalPayload } from "../services/rehearsal";
import { fillCode, openSectionsFrom, type OpenSections } from "./rehearsalSections";
import { positionAt, stepForKey, stepTo } from "./rehearsalNav";

const pageStyle: React.CSSProperties = {
  padding: "28px 40px 80px",
  maxWidth: "56rem",
  margin: "0 auto",
  // The generous line height is part of the point: this text is read aloud.
  lineHeight: 1.7,
};

// CONST: the three strings that describe the absence of a payload, and therefore
// cannot come from one. Every other word on this page is a settings row.
const LOADING = "Loading rehearsal mode…";
const LOAD_FAILED = "Rehearsal mode did not load.";
const RETRY = "Try again";

const RehearsalPage: React.FC = () => {
  const { slug, code } = useParams<{ slug: string; code?: string }>();
  const navigate = useNavigate();

  const [payload, setPayload] = useState<RehearsalPayload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [index, setIndex] = useState(0);
  const [open, setOpen] = useState<OpenSections | null>(null);

  const load = useCallback(async () => {
    if (!slug) return;
    setLoading(true);
    try {
      const loaded = await fetchRehearsal(slug);
      setPayload(loaded);
      // The default states are the SERVER's, parsed from the store at boot where a
      // typo is a named refusal. This page receives the decided answer.
      setOpen(openSectionsFrom(loaded.collapse));
      // Clamp rather than reset: a reload after one scenario was demoted should
      // keep Marie roughly where she was, not send her back to the start.
      setIndex((current) => stepTo(current, loaded.scenarios.length, null));
      setError(null);
    } catch (e: unknown) {
      // Explicit error UI, never a swallowed rejection (Standing Rule 1).
      setError(e instanceof Error ? e.message : LOAD_FAILED);
    } finally {
      setLoading(false);
    }
  }, [slug]);

  useEffect(() => {
    void load();
  }, [load]);

  const total = payload?.scenarios.length ?? 0;

  /** Where the address points, when it names a scenario. */
  const addressed = useMemo(
    () => payload?.scenarios.findIndex((s) => s.code === code) ?? -1,
    [payload, code],
  );

  // An address that names a READY scenario moves the reader to it. One that names
  // a scenario nobody declared ready falls through to the notice below — the page
  // never navigates somewhere it cannot show.
  useEffect(() => {
    if (addressed >= 0) setIndex(addressed);
  }, [addressed]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const step = stepForKey(event.key);
      if (step === null) return;
      // Space would otherwise scroll the page out from under the reader.
      event.preventDefault();
      setIndex((current) => {
        const next = stepTo(current, total, step);
        // The address follows the reader, so a scenario can be linked to and
        // returned to. `replace` rather than push: a rehearsal is worked through
        // in one pass, and filling the back button with every step would make the
        // browser's Back mean "one scenario ago" instead of "out of here".
        const moved = payload?.scenarios[next];
        if (moved && slug) {
          navigate(`/cases/${slug}/rehearsal/${moved.code}`, { replace: true });
        }
        return next;
      });
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [total, payload, slug, navigate]);

  const toggle = (section: keyof OpenSections) =>
    setOpen((current) => (current ? { ...current, [section]: !current[section] } : current));

  const setAll = (value: boolean) =>
    setOpen({ accusation: value, timeline: value, points: value, watchFor: value });

  if (loading) {
    return (
      <div style={pageStyle} data-surface="v3">
        {LOADING}
      </div>
    );
  }

  if (error || !payload || !open) {
    return (
      <div style={pageStyle} data-surface="v3">
        <div role="alert" style={{ color: "var(--state-danger-strong)" }}>
          {error ?? LOAD_FAILED}
        </div>
        <button
          type="button"
          onClick={() => void load()}
          style={{ ...ghostButtonStyle, marginTop: "1rem" }}
        >
          {RETRY}
        </button>
      </div>
    );
  }

  const w = payload.wording;
  const scenario = payload.scenarios[index];
  // The address named something this page is not showing. It exists or it does
  // not; either way it is not ready, and that is all this page may say about it.
  const notReady = code !== undefined && addressed < 0;

  return (
    <div style={pageStyle} data-surface="v3">
      {/* The browser's Back button must never be the only way out. */}
      <Breadcrumb
        items={[
          { label: "Dashboard", to: "/" },
          { label: "Trial Prep", to: `/cases/${slug}/trial-prep` },
          { label: w.page_heading },
        ]}
      />

      <div style={{ display: "flex", alignItems: "baseline", gap: "1rem", flexWrap: "wrap" }}>
        <h1 style={{ fontSize: "1.1rem", color: "var(--text-muted)", fontWeight: 500 }}>
          {w.page_heading}
        </h1>
        <span style={{ color: "var(--text-muted)" }}>
          {positionAt(index, payload.positions)}
        </span>
        {/* Expand all / Collapse all — the accordion rules require both, because a
            reader who wants the whole page should not click six carets. */}
        {scenario && (
          <span style={{ display: "flex", gap: "0.5rem", marginLeft: "auto" }}>
            <button type="button" onClick={() => setAll(true)} style={ghostButtonStyle}>
              {w.expand_all_label}
            </button>
            <button type="button" onClick={() => setAll(false)} style={ghostButtonStyle}>
              {w.collapse_all_label}
            </button>
          </span>
        )}
      </div>

      <p
        style={{
          fontSize: "0.95rem",
          lineHeight: 1.6,
          color: "var(--text-secondary)",
          margin: "0.5rem 0 0",
          maxWidth: "44rem",
        }}
      >
        {w.purpose_line}
      </p>

      {notReady && (
        <p role="status" style={{ fontSize: "1.1rem", marginTop: "1.5rem" }}>
          {fillCode(w.not_ready_notice, code)}
        </p>
      )}

      {/* An empty rehearsal is a REAL state, not a failure: nobody has declared a
          scenario ready yet. It says so, and says what to do about it. */}
      {!scenario ? (
        /* The stored sentence already says what to do ("a scenario appears here
           once someone switches it to Ready on its page"), and the breadcrumb
           above is the way there. An inline link would need a label, and a label
           compiled in here is exactly what this task removed. */
        <p style={{ fontSize: "1.3rem", marginTop: "2rem" }}>{w.nothing_ready_notice}</p>
      ) : (
        <>
          <RehearsalScenarioBlocks
            scenario={scenario}
            wording={w}
            open={open}
            onToggle={toggle}
          />

          <div style={{ display: "flex", gap: "0.75rem", marginTop: "2rem" }}>
            <button
              type="button"
              onClick={() => setIndex(stepTo(index, total, "previous"))}
              disabled={index === 0}
              style={ghostButtonStyle}
            >
              {w.previous_label}
            </button>
            <button
              type="button"
              onClick={() => setIndex(stepTo(index, total, "next"))}
              disabled={index >= total - 1}
              style={ghostButtonStyle}
            >
              {w.next_label}
            </button>
          </div>
        </>
      )}

      {/* The standing card. Always visible, on every screen including the empty
          one, and the ONE block with no fold to close — §10 makes it the thing
          that is never scrolled away from, and the addendum makes that a rule.
          Its lines are backend-composed; this renders them verbatim. */}
      <div
        style={{
          marginTop: "2.5rem",
          background: "var(--bg-surface)",
          borderRadius: "var(--radius-card)",
          boxShadow: "var(--shadow-card)",
          padding: "1rem 1.25rem",
        }}
      >
        <div
          style={{
            fontSize: "0.8rem",
            letterSpacing: "0.06em",
            textTransform: "uppercase",
            color: "var(--text-muted)",
            marginBottom: "0.35rem",
          }}
        >
          {payload.always.heading}
        </div>
        <ul style={{ fontSize: "1.15rem", paddingLeft: "1.4rem" }}>
          {payload.always.lines.map((line, i) => (
            <li key={i}>{line}</li>
          ))}
        </ul>
      </div>
    </div>
  );
};

export default RehearsalPage;
