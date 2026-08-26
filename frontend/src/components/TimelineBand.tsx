// =============================================================================
// TimelineBand.tsx — compact case-timeline band for the Home page
// -----------------------------------------------------------------------------
// Reads `GET /api/timeline` through services/caseTimeline (Phase B — the static
// JSON it used to read is gone), renders one pill per phase linking to that
// phase's filtered view on the Timeline page, plus a "View Full Timeline →"
// link. The fetch failure is surfaced visibly, never swallowed (Rule 1).
//
// ## ⚑ AND IT NO LONGER DROPS AN EVENT IN SILENCE
//
// The band groups events by phase. An event whose phase had no pill used to be
// counted NOWHERE and shown nothing — the last silent path on this surface. It
// now renders a stored marker saying how many of the total it can account for
// (design B6). An event nobody can see is an event nobody can fix.
// =============================================================================

import React, { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import {
  buildPhaseSummaries,
  ChronologyWording,
  cw,
  fill,
  getCaseTimeline,
  PhaseSummary,
} from "../services/caseTimeline";

// ─── Styles (inline + tokens) ────────────────────────────────────────────────

const BAND_STYLE: React.CSSProperties = { marginTop: "32px" };

const PILL_ROW_STYLE: React.CSSProperties = {
  display: "flex",
  gap: "8px",
  flexWrap: "wrap",
  marginTop: "12px",
};

const PILL_LABEL_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "14px",
  fontWeight: 600,
  color: "var(--text-primary)",
};

const PILL_SUBLINE_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "13px",
  color: "var(--text-secondary)",
  marginTop: "2px",
};

const MISMATCH_STYLE: React.CSSProperties = {
  marginTop: "10px",
  fontSize: "13px",
  fontWeight: 600,
  // The SAME token the timeline's "no document yet" mark uses, deliberately:
  // both say "this needs a human's attention", and a reader who learns the
  // colour on one surface should not have to learn it again on the other.
  // (It was a var() naming a token that is declared NOWHERE, with a hex literal
  // in the fallback slot quietly doing all the work. The literal is not repeated
  // here: a scanner hunting for hex codes finds documentation before code, and
  // this codebase documents its rules next to its rules.)
  color: "var(--burden-warning-text)",
};

const VIEW_FULL_LINK_STYLE: React.CSSProperties = {
  display: "inline-block",
  marginTop: "12px",
  fontSize: "14px",
  fontWeight: 500,
  color: "var(--accent-primary)",
  textDecoration: "none",
};

// ─── Phase pill ───────────────────────────────────────────────────────────────

/**
 * One clickable phase pill linking to `/timeline#phase-{id}`. Its accent comes
 * from the phase's own `color` (a value from the data file, not a hardcoded
 * code constant) used as a left border and a faint tinted fill on hover. The
 * alpha-hex suffixes (`0d`/`1a`) tint the data color without introducing a new
 * named color into the code.
 */
const PhasePill: React.FC<{ phase: PhaseSummary }> = ({ phase }) => {
  const [hovered, setHovered] = useState(false);
  const eventLabel = `${phase.eventCount} event${phase.eventCount === 1 ? "" : "s"}`;

  return (
    <Link
      to={`/timeline#phase-${phase.id}`}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      style={{
        flex: 1,
        minWidth: "140px",
        padding: "12px 16px",
        borderRadius: "8px",
        textDecoration: "none",
        borderLeft: `3px solid ${phase.color}`,
        backgroundColor: `${phase.color}${hovered ? "1a" : "0d"}`,
        transition: "background-color 0.15s ease",
      }}
    >
      <div style={PILL_LABEL_STYLE}>{phase.label}</div>
      <div style={PILL_SUBLINE_STYLE}>
        {phase.date_range} · {eventLabel}
      </div>
    </Link>
  );
};

// ─── Component ────────────────────────────────────────────────────────────────

/**
 * Compact Timeline band.
 *
 * ## React Learning: fetch-on-mount with a cancel flag
 * Same idiom as Home/CaseSummaryCard — the cleanup sets `cancelled` so a late
 * response after unmount never setStates.
 */
type BandState = {
  phases: PhaseSummary[];
  matched: number;
  unmatched: number;
  total: number;
  wording: ChronologyWording;
};

const TimelineBand: React.FC = () => {
  const [band, setBand] = useState<BandState | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    getCaseTimeline()
      .then((data) => {
        if (cancelled) return;
        const rolled = buildPhaseSummaries(data);
        setBand({
          phases: rolled.phases,
          matched: rolled.matched,
          unmatched: rolled.unmatched,
          total: data.events.length,
          wording: data.wording,
        });
      })
      .catch((err: unknown) => {
        // No silent failure (Rule 1) — the original effect swallowed this.
        if (!cancelled) {
          setError(
            err instanceof Error ? err.message : "Failed to load the case timeline.",
          );
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // Loading: render nothing (the band is supplementary; no spinner needed).
  if (!error && band === null) return null;

  return (
    <div style={BAND_STYLE}>
      <div className="h2-section-header">Case Timeline</div>
      {error ? (
        <div style={{ marginTop: "12px", color: "var(--status-dropped-text)", fontSize: "14px" }}>
          {error}
        </div>
      ) : band && band.phases.length > 0 ? (
        <>
          <div style={PILL_ROW_STYLE}>
            {band.phases.map((phase) => (
              <PhasePill key={phase.id} phase={phase} />
            ))}
          </div>
          {band.unmatched > 0 && (
            // The marker, not a silent drop (design B6). Amber rather than red:
            // the data is readable, it is the grouping that cannot place it.
            <div style={MISMATCH_STYLE}>
              {fill(cw(band.wording, "band_mismatch_template"), {
                shown: band.matched,
                total: band.total,
              })}
            </div>
          )}
          <Link to="/timeline" style={VIEW_FULL_LINK_STYLE}>
            View Full Timeline →
          </Link>
        </>
      ) : (
        <div style={{ marginTop: "12px", color: "var(--text-muted)", fontSize: "14px" }}>
          No timeline phases available.
        </div>
      )}
    </div>
  );
};

export default TimelineBand;
