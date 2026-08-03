// =============================================================================
// ScanSection — Scan & candidates: one section, three stacked elements (§2.3)
// =============================================================================
//
// Roman's grouping ruling 2026-08-03, and the second stated deviation from the
// study (§5.2): Casefleet renders proposal cards as a standalone list, and this
// page instead GROUPS the ruling queue with the scan that produced it. Candidates
// are scan output; grouping the control with the product makes the page read as
// produce → judge → keep.
//
// One bordered section holding:
//
//   1. the scan control line + last-run meta      (`ThemeScanPanel`, unchanged)
//   2. the scan history disclosure                (inside the panel, §2.3 item 2)
//   3. the ruling queue as a collapsible region   (here, wrapping `CardQueue`)
//
// ## §7 behaviour is byte-identical
//
// The collapsible wrapper is CHROME, not behaviour. One focused card, one-key
// I/E/D, auto-advance, U undo, defer quick-pick, no page navigation to rule —
// all of it lives in `cardTriage` (pure) and `CardQueue`, neither of which this
// file changes. The 31 reducer tests pass unedited.
//
// The one addition is a guard, not a behaviour change: keys are INERT while the
// region is collapsed (ruling R7). It lives in `CardQueue` rather than the reducer
// precisely so the reducer stays a pure state machine that knows nothing about
// chrome.

import React, { useState } from "react";

import CardQueue from "./CardQueue";
import ThemeScanPanel from "./ThemeScanPanel";
import { queueRegion } from "./queueRegion";

const HAIRLINE = "1px solid var(--border-default)";

const sectionHeaderStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "baseline",
  flexWrap: "wrap",
  gap: "0.7rem",
  marginTop: "2rem",
  marginBottom: "0.6rem",
};

const titleStyle: React.CSSProperties = {
  margin: 0,
  fontSize: "0.98rem",
  fontWeight: 600,
  color: "var(--text-primary)",
};

const boxStyle: React.CSSProperties = {
  background: "var(--bg-surface)",
  border: HAIRLINE,
  borderRadius: "10px",
  boxShadow: "0 1px 2px rgba(16,24,40,.05)",
};

const queueSummaryStyle: React.CSSProperties = {
  cursor: "pointer",
  listStyle: "none",
  display: "flex",
  alignItems: "center",
  flexWrap: "wrap",
  gap: "0.8rem",
  padding: "0.75rem 1rem",
  fontSize: "0.85rem",
  color: "var(--text-primary)",
};

const kbdStyle: React.CSSProperties = {
  border: HAIRLINE,
  borderBottomWidth: "2px",
  borderRadius: "4px",
  padding: "0 0.28rem",
  background: "var(--bg-canvas)",
  color: "var(--text-secondary)",
  fontFamily: "inherit",
  fontSize: "0.7rem",
};

interface Props {
  slug: string;
  scenarioId: string;
  scenarioTitle: string;
  /** Bumped when a scan merge writes candidate facts; relayed to both children. */
  externalRefresh: number;
  onFactsChanged: () => void;
}

const ScanSection: React.FC<Props> = ({
  slug,
  scenarioId,
  scenarioTitle,
  externalRefresh,
  onFactsChanged,
}) => {
  // Queue progress is owned by `CardQueue` (it does the fetching), and this
  // section needs it for the summary line and the progress bar. So the queue
  // reports it upward rather than this section fetching the pool a second time —
  // two reads of one pool is how two surfaces end up disagreeing about the count.
  //
  // `null` until the queue reports, and that distinction is load-bearing: treating
  // "not known yet" as `{0, 0}` would compute "all candidates ruled" for the second
  // before the fetch lands, so the region would render COLLAPSED and then snap open.
  // A queue that hides itself on arrival is the opposite of what §2.3 wants, and a
  // human who looks away for that second sees a page with no work on it.
  const [progress, setProgress] = useState<{ ruled: number; total: number } | null>(null);
  const region = queueRegion(progress?.ruled ?? 0, progress?.total ?? 0);

  // Mirrors the descriptor's computed default, then follows the human's clicks.
  // Keyed on the default so a queue that drains to zero collapses on its own —
  // and re-opens if a merge puts unruled candidates back. NOT persisted
  // (ruling R7): a queue that remembers "collapsed" over 145 unruled candidates is
  // a silent failure wearing a preference's clothes.
  const [openOverride, setOpenOverride] = useState<boolean | null>(null);
  const [lastDefault, setLastDefault] = useState(region.open);
  if (lastDefault !== region.open) {
    // Adjusting state during render, deliberately: this is React's documented
    // derive-from-changed-input pattern, and it is guarded by the inequality so it
    // runs once per change rather than every render.
    setLastDefault(region.open);
    setOpenOverride(null);
  }
  // Open until the counts are known — see the `null` note above.
  const open = progress === null ? true : (openOverride ?? region.open);

  return (
    <section>
      <div style={sectionHeaderStyle}>
        <h2 style={titleStyle}>Scan &amp; candidates</h2>
        {/* §2.3's labelling law, said out loud: a human who thinks the queue is
            "the last scan's results" will rerun a scan expecting the pile to
            reset, and it will not. */}
        <span style={{ fontSize: "0.78rem", color: "var(--text-muted)" }}>
          scans add candidates; your rulings drain them — rerunning never removes
          anything
        </span>
      </div>

      <div style={boxStyle}>
        {/* 1 + 2: the scan control line, the last-run meta, and the history
            disclosure. Behaviour unchanged from 1.7B. */}
        <div style={{ padding: "0.25rem 0.5rem" }}>
          <ThemeScanPanel
            slug={slug}
            scenarioId={scenarioId}
            scenarioTitle={scenarioTitle}
            onFactsChanged={onFactsChanged}
          />
        </div>

        {/* 3: the ruling queue. */}
        <details
          open={open}
          onToggle={(event) => setOpenOverride(event.currentTarget.open)}
          style={{ borderTop: HAIRLINE }}
        >
          <summary style={queueSummaryStyle}>
            <strong style={{ fontWeight: 600 }}>{region.summary}</strong>
            {region.scope && (
              <span style={{ color: "var(--text-muted)" }}>{region.scope}</span>
            )}
            <span style={{ marginLeft: "auto", color: "var(--text-muted)" }}>
              <kbd style={kbdStyle}>I</kbd> include · <kbd style={kbdStyle}>E</kbd> exclude ·{" "}
              <kbd style={kbdStyle}>D</kbd> defer · <kbd style={kbdStyle}>U</kbd> undo
            </span>
          </summary>

          <div style={{ padding: "0 1rem 0.5rem" }}>
            {/* Progress: the label, the bar, and what is left. */}
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: "1rem",
                flexWrap: "wrap",
                fontSize: "0.8rem",
                color: "var(--text-secondary)",
                margin: "0.4rem 0 0.2rem",
              }}
            >
              <strong style={{ fontWeight: 600 }}>
                {/* "0 of 0 ruled" before the fetch lands is a claim about the pool
                    that nothing has measured yet (Standing Rule 1). */}
                {progress === null ? "Counting candidates…" : region.progressLabel}
              </strong>
              <div
                role="progressbar"
                aria-valuenow={region.progressPercent}
                aria-valuemin={0}
                aria-valuemax={100}
                aria-label="Candidates ruled"
                style={{
                  flex: 1,
                  minWidth: "10rem",
                  height: "5px",
                  background: "var(--border-default)",
                  borderRadius: "99px",
                  overflow: "hidden",
                }}
              >
                <div
                  style={{
                    width: `${region.progressPercent}%`,
                    height: "100%",
                    background: "var(--accent-primary)",
                  }}
                />
              </div>
              {region.remainingLabel && <span>{region.remainingLabel}</span>}
            </div>

            <CardQueue
              slug={slug}
              scenarioId={scenarioId}
              externalRefresh={externalRefresh}
              keyboardActive={open}
              onProgress={setProgress}
            />
          </div>
        </details>
      </div>
    </section>
  );
};

export default ScanSection;
