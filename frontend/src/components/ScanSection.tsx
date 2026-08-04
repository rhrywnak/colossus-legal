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
import type { AllegationOptions } from "../services/evidenceLinks";
import ThemeScanPanel from "./ThemeScanPanel";
import { queueRegion } from "./queueRegion";

import {
  kbdStyle,
  sectionHeaderStyle,
  sectionMetaStyle,
  sectionPanelStyle,
  sectionTitleStyle,
} from "./scenarioSectionStyles";

/** Mockup `.q-head`: the queue's head row, divided from the scan row above it. */
const queueHeadStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "12px",
  padding: "14px 24px",
  borderTop: "1px solid var(--border-default)",
};

/**
 * Mockup `.chev`: a 30x30 chrome square, radius 8, NO border.
 *
 * `margin-left: auto` puts it hard right, away from the text — which is the point
 * of item 4. The only way to collapse the queue is to aim at this button.
 */
const chevronStyle: React.CSSProperties = {
  marginLeft: "auto",
  width: "30px",
  height: "30px",
  borderRadius: "8px",
  background: "var(--v3-chrome)",
  border: "none",
  cursor: "pointer",
  color: "var(--text-secondary)",
  fontSize: "13px",
  fontFamily: "inherit",
  flexShrink: 0,
};

/** Mockup `.q-meta`: 16px gaps, 13px, --text-2. */
const queueMetaStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "16px",
  margin: "4px 0 14px",
  fontSize: "13px",
  color: "var(--text-secondary)",
  flexWrap: "wrap",
};

/** Mockup `.q-progress`: 6px tall, chrome trough, fully rounded. */
const progressTroughStyle: React.CSSProperties = {
  flex: 1,
  minWidth: "180px",
  height: "6px",
  background: "var(--v3-chrome-strong)",
  borderRadius: "99px",
  overflow: "hidden",
};

interface Props {
  slug: string;
  scenarioId: string;
  scenarioTitle: string;
  /** Bumped when a scan merge writes candidate facts; relayed to both children. */
  externalRefresh: number;
  onFactsChanged: () => void;
  /** Relayed to the queue: a ruling the server confirmed (task 1.7F Part A). */
  onRulingSaved: () => void;
  /** Passed straight through to the queue — the page owns this read (2.12). */
  linkOptions: AllegationOptions | null;
}

const ScanSection: React.FC<Props> = ({
  slug,
  scenarioId,
  scenarioTitle,
  externalRefresh,
  onFactsChanged,
  onRulingSaved,
  linkOptions,
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
        <h2 style={sectionTitleStyle}>Scan &amp; candidates</h2>
        {/* §2.3's labelling law, said out loud: a human who thinks the queue is
            "the last scan's results" will rerun a scan expecting the pile to
            reset, and it will not. */}
        <span style={sectionMetaStyle}>
          scans add candidates; your rulings drain them — rerunning never removes
          anything
        </span>
      </div>

      <div style={sectionPanelStyle}>
        {/* 1 + 2: the scan control line, the last-run meta, and the history
            disclosure. Behaviour unchanged from 1.7B. */}
        {/* No wrapper padding: the scan row carries the mockup's own 14px/24px. */}
        <div>
          <ThemeScanPanel
            slug={slug}
            scenarioId={scenarioId}
            scenarioTitle={scenarioTitle}
            onFactsChanged={onFactsChanged}
          />
        </div>

        {/* 3: the ruling queue.

            Item 4 (Roman's 2026-08-03 session): collapse happens via the EXPLICIT
            CHEVRON and nothing else. 1.7C used a native disclosure element, whose
            summary makes the WHOLE head row a toggle — so clicking the count, the
            scope text, or the empty space between them folded the queue away
            mid-triage. That is the defect, and a native disclosure cannot be made
            partly clickable, so the region is now a plain head row plus a
            conditional body.

            The keyboard legend moved OUT of the head and INTO the meta row inside
            the body, with the mockup's "— or use the buttons" — where it sits beside
            the buttons it is describing, and where it disappears with the body it
            applies to rather than advertising keys that are paused. */}
        <div style={queueHeadStyle}>
          <b style={{ fontSize: "14px" }}>{region.summary}</b>
          {region.scope && (
            <span style={{ fontSize: "13px", color: "var(--text-secondary)" }}>
              {region.scope}
            </span>
          )}
          <button
            type="button"
            style={chevronStyle}
            onClick={() => setOpenOverride(!open)}
            aria-expanded={open}
            aria-label={region.chevronLabel}
            title={region.chevronLabel}
          >
            {open ? "▾" : "▸"}
          </button>
        </div>

        {open && (
          <div style={{ padding: "4px 24px 22px" }}>
            {/* Progress: the label, the bar, what is left, the deferred tray, and
                the keys. Mockup `.q-meta`. */}
            <div style={queueMetaStyle}>
              <b style={{ color: "var(--state-success-strong)" }}>
                {/* "0 of 0 ruled" before the fetch lands is a claim about the pool
                    that nothing has measured yet (Standing Rule 1). */}
                {progress === null ? "Counting candidates…" : region.progressLabel}
              </b>
              <div
                role="progressbar"
                aria-valuenow={region.progressPercent}
                aria-valuemin={0}
                aria-valuemax={100}
                aria-label="Candidates ruled"
                style={progressTroughStyle}
              >
                <div
                  style={{
                    width: `${region.progressPercent}%`,
                    height: "100%",
                    background: "var(--state-success-strong)",
                  }}
                />
              </div>
              {region.remainingLabel && <span>{region.remainingLabel}</span>}
              <span style={{ color: "var(--text-muted)" }}>
                Keys: <kbd style={kbdStyle}>I</kbd> <kbd style={kbdStyle}>E</kbd>{" "}
                <kbd style={kbdStyle}>D</kbd> <kbd style={kbdStyle}>U</kbd> — or use the
                buttons
              </span>
            </div>

            <CardQueue
              linkOptions={linkOptions}
              slug={slug}
              scenarioId={scenarioId}
              externalRefresh={externalRefresh}
              keyboardActive={open}
              onProgress={setProgress}
              onRulingSaved={onRulingSaved}
            />
          </div>
        )}
      </div>
    </section>
  );
};

export default ScanSection;
