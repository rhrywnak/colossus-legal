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
import { fillCount, type AllegationOptions } from "../services/evidenceLinks";
import ThemeScanPanel from "./ThemeScanPanel";
import type { ScenarioCard } from "../services/scenarioCards";
import { anyScanScored, progressFromCards, queueRegion } from "./queueRegion";

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

/**
 * The never-scanned opt-in (piece 3a): the chevron's job, wearing its words.
 *
 * Right-aligned like the chevron it replaces, and deliberately quiet — it opens a
 * pool nobody has judged, which is browsing, not work waiting to be done.
 */
const rawPoolToggleStyle: React.CSSProperties = {
  marginLeft: "auto",
  padding: "6px 12px",
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
  /**
   * The page's card pool, which the queue's summary counts are derived from
   * (task 2.13c).
   *
   * `null` until the page's own read lands — and that nullability is the whole
   * point: it is the ONE state that must stay distinguishable from a pool that
   * is genuinely empty. The queue used to report `{0, 0}` before its fetch
   * resolved, which collapsed the two and produced a header claiming no
   * candidates over a pool of 148.
   */
  cards: ScenarioCard[] | null;
  /**
   * Present ONLY when no scan run has completed for this scenario (task 2.15,
   * piece 3) — the sentence the section leads with instead of a queue.
   *
   * Served, never inferred: the browser cannot tell "nothing has been scanned"
   * from "nothing scanned has been merged", and on 2026-08-07 it guessed wrong on
   * a page that showed both answers at once. See the payload field's own note.
   */
  neverScannedNotice: string | null;
}

const ScanSection: React.FC<Props> = ({
  slug,
  scenarioId,
  scenarioTitle,
  externalRefresh,
  onFactsChanged,
  onRulingSaved,
  linkOptions,
  cards,
  neverScannedNotice,
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
  // Task 2.13c: the counts come from the PAGE's pool, not from the queue. See
  // `progressFromCards` for the latch this breaks. `CardQueue` no longer reports
  // anything upward, so a collapsed region is as well-informed as an open one.
  const progress = progressFromCards(cards);
  // The two zero-state summaries come from the store (the configuration law's
  // text half). `null` until the panel wording loads, which the descriptor
  // renders as an empty summary rather than a compiled-in fallback — R4's rule:
  // there is no literal to fall back to, and a line that invents its own words is
  // exactly what this task removed.
  // `progress` is passed WHOLE, nullability and all. Collapsing it to
  // `?? 0` here is what made "not counted yet" indistinguishable from "empty"
  // and put two false sentences on DEV in successive builds.
  const region = queueRegion(
    progress,
    linkOptions
      ? {
          emptyPool: linkOptions.wording.queue_empty_pool_summary,
          allRuled: linkOptions.wording.queue_all_ruled_summary,
          counting: linkOptions.wording.queue_counting_summary,
        }
      : null,
    // The labelling clause is earned by the DATA: "from all scans" only when
    // something in this pool actually carries a scan's score (piece 3b).
    anyScanScored(cards),
  );

  // ── The never-scanned state (piece 3a) ─────────────────────────────────────
  //
  // The workflow is create → scan → rule, and until the first scan runs these
  // rows are not candidates at all — they are the raw evidence pool. So the
  // section leads with the served notice, and the pool moves behind an explicit
  // control that names its size. Nothing is removed: the same cards, the same
  // queue, the same keys, one click away.
  //
  // Presence of the notice is the whole condition. Deriving it from the counts
  // here is exactly the guess the payload field exists to replace.
  const neverScanned = neverScannedNotice !== null;
  const rawPoolLabel =
    linkOptions && cards
      ? fillCount(linkOptions.wording.queue_raw_pool_toggle_template, cards.length)
      : null;

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
  // The descriptor already returns `open: true` while the counts are unknown, so
  // this no longer special-cases `null` — one place decides, not two.
  //
  // A never-scanned scenario starts CLOSED regardless of what the descriptor
  // computed: the descriptor opens on unruled candidates, and on this scenario
  // every row is unruled precisely because nothing has judged any of them. The
  // human's own click still wins (`openOverride`) — this changes what leads the
  // page, not what is reachable.
  const open = openOverride ?? (neverScanned ? false : region.open);

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
        {/* The head row says one of two things, and which one is a fact the
            SERVER established (piece 3a):

            - nothing has ever scanned this scenario → the served notice, with
              the raw pool behind a named opt-in;
            - a scan has run → the queue exactly as before.

            The chevron is the same control in both cases, so the body below needs
            no second code path and the keyboard guard keeps working unchanged. */}
        <div style={queueHeadStyle}>
          {neverScanned ? (
            <>
              <b style={{ fontSize: "14px" }}>{neverScannedNotice}</b>
              {/* The opt-in IS the chevron's label here: a bare arrow beside a
                  sentence saying "no scan has run" gives no hint that 148 rows are
                  behind it. `rawPoolLabel` is null only while the wording or the
                  pool is still loading, and a control with no words is not
                  rendered at all (the absent-not-fake law). */}
              {rawPoolLabel && (
                <button
                  type="button"
                  style={rawPoolToggleStyle}
                  onClick={() => setOpenOverride(!open)}
                  aria-expanded={open}
                >
                  {open ? "▾" : "▸"} {rawPoolLabel}
                </button>
              )}
            </>
          ) : (
            <>
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
            </>
          )}
        </div>

        {open && (
          <div style={{ padding: "4px 24px 22px" }}>
            {/* Progress: the label, the bar, what is left, the deferred tray, and
                the keys. Mockup `.q-meta`. */}
            <div style={queueMetaStyle}>
              <b style={{ color: "var(--state-success-strong)" }}>
                {/* "0 of 0 ruled" before the fetch lands is a claim about the pool
                    that nothing has measured yet (Standing Rule 1). The descriptor
                    now carries the counting text itself, from the stored row —
                    which also retires the literal this line used to hold. */}
                {region.progressLabel}
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
              onRulingSaved={onRulingSaved}
            />
          </div>
        )}
      </div>
    </section>
  );
};

export default ScanSection;
