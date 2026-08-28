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

import CardQueue, { type QueueFrame } from "./CardQueue";
import { facetLabel } from "./candidateFilters";
import { fillCount, fillSlots, type AllegationOptions } from "../services/evidenceLinks";
import ThemeScanPanel from "./ThemeScanPanel";
import type { ProposalSource, ScenarioCard } from "../services/scenarioCards";
import { anyScanScored, progressFromCards, proposedCount, queueRegion } from "./queueRegion";
import { useSectionOpen } from "./sectionCollapse";

import {
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

// REMOVED (task R1): `progressTroughStyle`. The pool-wide progress bar it
// belonged to was retired by ONE_CARD_GRAMMAR piece 1c — progress follows the
// active filter now and lives on the chip row.

interface Props {
  slug: string;
  scenarioId: string;
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
  /**
   * Which completed run is proposing the candidates below, or `null` when
   * nothing is proposed (2026-08-08).
   *
   * Served, never inferred — the same discipline as `neverScannedNotice`. It
   * feeds two surfaces that must agree: the queue's heading ("30 proposed by the
   * Aug 7 scan") and the collapsed scan card's one-liner. One value, read once,
   * passed to both.
   */
  proposalSource: ProposalSource | null;
}

const ScanSection: React.FC<Props> = ({
  slug,
  scenarioId,
  externalRefresh,
  onFactsChanged,
  onRulingSaved,
  linkOptions,
  cards,
  neverScannedNotice,
  proposalSource,
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
  // The date the attribution names, formatted in the READER's locale — the same
  // division of labour the scan-history delete confirmation already uses: the
  // server owns the sentence, the browser owns the date format.
  /**
   * What the queue says about the list it is showing (task R4, P4).
   *
   * `null` until the queue reports, which is a real state and not a zero: the
   * heading renders its served zero-state sentence until then rather than
   * "undefined — 0". Nothing is DERIVED from this — see the reporting effect in
   * `CardQueue` for why that matters and why this is not the upward reporting
   * task 2.13c removed.
   */
  const [frame, setFrame] = useState<QueueFrame | null>(null);

  const proposals =
    proposalSource === null
      ? null
      : {
          count: proposedCount(cards),
          when: formatScanDate(proposalSource.started_at),
        };

  const region = queueRegion(
    progress,
    linkOptions
      ? {
          emptyPool: linkOptions.wording.queue_empty_pool_summary,
          allRuled: linkOptions.wording.queue_all_ruled_summary,
          counting: linkOptions.wording.queue_counting_summary,
          proposedHeading: linkOptions.wording.queue_proposed_heading_template,
        }
      : null,
    proposals,
    // The pool's own labelling clause is still earned by the DATA — it applies to
    // a queue led by the POOL, which is what a scanned-and-fully-ruled scenario
    // shows (piece 3b, unchanged).
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

  // The queue arrives COLLAPSED and remembers what the human last chose for THIS
  // scenario (ruled 2026-08-28) — see `sectionCollapse` for what that supersedes
  // and why R7's objection no longer holds.
  //
  // This replaced a session-only override seeded from `region.open`, which
  // opened the queue whenever anything was unruled and re-seeded itself every
  // time that default flipped. Two things made it wrong under the new ruling:
  // the default is now "collapsed" whatever the counts say, and a stored
  // preference that a drain-to-zero could silently wipe is not remembered at
  // all. `region.open` is consequently no longer read here — see its own doc.
  //
  // The head row below, with its heading, its count and its `{ruled} of {total}`,
  // sits OUTSIDE the fold and always has. That is what makes collapsing-by-
  // default honest rather than a hiding place: a closed queue still says how
  // many candidates are in it and how far through them you are.
  const [open, toggleOpen] = useSectionOpen(scenarioId, "candidates");

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
        {/* MUST STAY MOUNTED, collapsed or not (architect ruling R3).
            The panel's mount effect calls `gatherCandidates`, and gather is the
            ONE place candidate ordinals are minted — every card's `C-14` handle.
            Unmounting the panel when the scan card collapses would stop new
            candidates being numbered, and a proposed card would arrive with
            `code: null`, which §2a forbids ("pull up C-14" must be speakable).
            The collapse is INSIDE the panel and hides its body only. Do not
            "optimize" this into a conditional render. */}
        <div>
          <ThemeScanPanel
            slug={slug}
            scenarioId={scenarioId}
            onFactsChanged={onFactsChanged}
            proposalSource={proposalSource}
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
                  onClick={toggleOpen}
                  aria-expanded={open}
                >
                  {open ? "▾" : "▸"} {rawPoolLabel}
                </button>
              )}
            </>
          ) : (
            <>
              {/* THE HEADING (task R4, P4): the name of the list you are looking
                  at and how many are in it — "Included — 21" — with how much of
                  the proposed bucket is addressed at the far end.

                  It replaced "Candidates awaiting ruling — 145", which counted
                  the whole unruled pool above a list showing one filter's 21.
                  The two numbers on one screen disagreed by construction, and
                  the bigger one was the one in bold.

                  `region.summary` still wins when it has something to say: the
                  served proposal heading (which names its scan) and the two
                  served zero-state sentences. Those are facts about the pool
                  that the filter's name cannot carry. */}
              <b style={{ fontSize: "14px" }}>
                {region.summary ??
                  (frame && linkOptions
                    ? `${facetLabel(frame.facet, linkOptions.card_grammar)} — ${frame.count}`
                    : "")}
              </b>
              {region.scope && (
                <span style={{ fontSize: "13px", color: "var(--text-secondary)" }}>
                  {region.scope}
                </span>
              )}
              {/* The addressed count, at the opposite end of the same line. It
                  measures the PROPOSED bucket and does not move when a filter is
                  clicked — see `filterProgress`. Sits left of the fold arrow,
                  which keeps its own `marginLeft: auto`. */}
              {frame && linkOptions && (
                <span
                  style={{
                    marginLeft: "auto",
                    fontSize: "13px",
                    color: "var(--text-secondary)",
                    whiteSpace: "nowrap",
                  }}
                >
                  {fillSlots(linkOptions.card_grammar.filter_progress_template, {
                    ruled: String(frame.progress.ruled),
                    total: String(frame.progress.total),
                  })}
                </span>
              )}
              <button
                type="button"
                style={chevronStyle}
                onClick={toggleOpen}
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
            {/* Piece 1c: the pool-wide bar is GONE. It read "23 of 148 ruled"
                over a bar of 125 nobody owes, while rule-the-promising is the
                ratified triage model — so it reported a debt the method denies.
                Progress now follows the active filter and lives under the chips,
                beside the list it describes.

                What stays here is the one state the chips cannot show: nothing
                has measured the pool yet, which is different from a pool of
                zero (Standing Rule 1). */}
            {/* THE KEY LEGEND IS GONE (Roman's cleanup ruling, 2026-08-10).
                The letters live on the buttons that do the same thing — Include I,
                Exclude E, Defer D, ↩ U — so the legend taught nothing the controls
                were not already saying, on every scroll past it. The keys still
                work; only the text went.

                The counting notice stays: it is the one state the chips cannot
                show, and it is a fact rather than a hint. */}
            {region.countingNotice && (
              <div style={queueMetaStyle}>
                <b style={{ color: "var(--state-success-strong)" }}>{region.countingNotice}</b>
              </div>
            )}

            <CardQueue
              linkOptions={linkOptions}
              slug={slug}
              scenarioId={scenarioId}
              externalRefresh={externalRefresh}
              onFrameChanged={setFrame}
            keyboardActive={open}
              onRulingSaved={onRulingSaved}
            />
          </div>
        )}
      </div>
    </section>
  );
};

/**
 * The run's start time, as the attribution says it: "Aug 7".
 *
 * ## Why the BROWSER formats this and the server does not
 *
 * The sentence is the server's (the language law); the DATE FORMAT is the
 * reader's, and the server does not know their locale. This is the same split the
 * scan-history delete confirmation already makes with its `{run}` placeholder, so
 * the two dates on this page read alike.
 *
 * An unparseable timestamp yields the raw string rather than "Invalid Date": a
 * heading that names a nonsense date is worse than one naming an ugly one, and
 * both are better than a heading that silently drops which scan it means.
 */
function formatScanDate(iso: string): string {
  const at = new Date(iso);
  if (Number.isNaN(at.getTime())) return iso;
  return at.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

export default ScanSection;
