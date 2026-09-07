// =============================================================================
// ScenarioFactsHeader.tsx — the Scenario facts header row (v2.1, change D)
// =============================================================================
//
// Left: the heading, the last-scan sentence, `Scan again`, `history`.
// Right: `Reset order` and the section's fold.
//
// ## ⚑ Why the SCAN controls live on the FACTS header
//
// Ruling R35 retired the "Scan & candidates" section. A scan is not a thing a
// human does for its own sake — it produces candidates, and candidates become
// facts — so the control belongs on the list it feeds, not in a card of its own
// above it. What was a section with a card, a model picker, a candidate count
// and a run report is now four words and two links.
//
// The engine did NOT move: `ThemeScanPanel` still owns the model catalogue, the
// run history, the poll and the mount-time `gatherCandidates` that mints every
// card's `C-14` handle (architect ruling R3). It lends this header its three
// controls through `ScanHeaderApi` and nothing here is derived. See that type.
//
// ## What the last-scan sentence says, and what it does NOT say
//
// It is the stored `scan_card_collapsed_summary_template`, filled by the panel:
// "Last scan {when} · {model} · {count} proposed". The mockup draws the first
// two clauses only, because the proposed count now has its own pill an inch
// below. Trimming a served sentence in code would be the browser composing the
// words, which the language law forbids, and a new row is a migration this task
// did not authorise — so the sentence renders whole and the duplication is
// REPORTED. One `scan_header_summary_template` row retires it.
//
// A scenario nothing has scanned has no sentence at all: the panel returns
// `null` rather than "Last scan never", and the served never-scanned notice is
// what the section shows in its place.

import React from "react";

import SectionFold from "./SectionFold";
import type { ScanHeaderApi } from "./ThemeScanPanel";
import type { CardGrammarWording } from "../services/evidenceLinks";
import {
  sectionHeaderStyle,
  sectionMetaStyle,
  sectionTitleStyle,
} from "./scenarioSectionStyles";

// ⚑ CONST: FIVE user-facing literals, and all five are DEBT, not decisions.
//
// This control is new and no settings rows exist for it; a migration is Roman's
// to author (rule 25) and this task did not authorise one. `HEADING` is the
// oldest of the five — "Scenario facts" has been a literal in
// `ScenarioFactsSection` since 1.7C and merely MOVED here, so it is two releases
// of debt rather than new debt, but it is counted with the rest.
//
// The two refusal tooltips are on this list too. They were the ones that fell
// through the first accounting pass — flagged by the architecture review — and
// they are here rather than deleted for the reason this repo states elsewhere:
// a control that refuses without saying why reads as a broken one. Their two
// reasons are genuinely different, so they say different things.
//
// Owed rows, named the way the review suggested:
//   scenario_facts_heading             = "Scenario facts"
//   scenario_facts_scan_again_label    = "Scan again"
//   scenario_facts_history_label       = "history"
//   scenario_facts_scan_running_notice = "A scan is running — its progress is below."
//   scenario_facts_no_model_notice     = "No scan-eligible model is available."
const HEADING = "Scenario facts";
const SCAN_AGAIN_LABEL = "Scan again";
const HISTORY_LABEL = "history";
const SCAN_RUNNING_NOTICE = "A scan is running — its progress is below.";
const NO_MODEL_NOTICE = "No scan-eligible model is available.";

/** A quiet inline link. Mockup: accent-coloured text, no button chrome. */
const linkStyle: React.CSSProperties = {
  border: "none",
  background: "none",
  padding: 0,
  color: "var(--accent-primary)",
  cursor: "pointer",
  fontFamily: "inherit",
  fontSize: "0.8rem",
};

/** The same link, refused. A scan cannot start while one is already running. */
const linkDisabledStyle: React.CSSProperties = {
  ...linkStyle,
  color: "var(--text-muted)",
  cursor: "not-allowed",
};

interface Props {
  /** The scan controls, lent by `ThemeScanPanel`. */
  scan: ScanHeaderApi;
  /**
   * The served sentence for a scenario NO scan has ever touched, or `null`.
   *
   * Served, never inferred: the browser cannot tell "nothing has been scanned"
   * from "nothing scanned has been merged", and on 2026-08-07 it guessed wrong
   * on a page showing both answers at once. It stands where the last-scan
   * sentence would be, because those are the same slot answering the same
   * question.
   */
  neverScannedNotice: string | null;
  /** Whether the run history table is open. */
  historyOpen: boolean;
  onToggleHistory: () => void;
  /** The Reset-order control's stored words, or `null` until they load. */
  grammar: CardGrammarWording | null;
  onResetOrder: () => void;
  /** The section fold. */
  open: boolean;
  onToggleOpen: () => void;
}

/**
 * The header row.
 *
 * ## Why `Reset order` is withheld and `Scan again` is only disabled
 *
 * Two different absences, for two different reasons. Reset order DISCARDS work
 * across the whole list and its confirmation is a stored sentence — a
 * destructive control that cannot state what it does must not be offered at all
 * (ruling R4). `Scan again` has its words in this file and is refused only while
 * a run is in flight, which is a temporary state a human should be able to see
 * the reason for; a control that vanished mid-run would read as a bug.
 */
const ScenarioFactsHeader: React.FC<Props> = ({
  scan,
  neverScannedNotice,
  historyOpen,
  onToggleHistory,
  grammar,
  onResetOrder,
  open,
  onToggleOpen,
}) => (
  <div style={sectionHeaderStyle}>
    <h2 style={sectionTitleStyle}>{HEADING}</h2>

    {/* The last-scan sentence, or the served never-scanned notice in its place.
        Never both, and never a fabricated third state: when the panel has not
        loaded its history yet there is simply no sentence, which is honest —
        the alternative is a line that says something about a scan before
        anything has been read about it. */}
    {neverScannedNotice !== null ? (
      <span style={sectionMetaStyle}>{neverScannedNotice}</span>
    ) : (
      scan.summary !== null && <span style={sectionMetaStyle}>{scan.summary}</span>
    )}

    <button
      type="button"
      style={scan.canRun ? linkStyle : linkDisabledStyle}
      disabled={!scan.canRun}
      // A control that refuses without saying why reads as a broken one. The two
      // reasons it can refuse are genuinely different and say so.
      title={
        scan.running
          ? SCAN_RUNNING_NOTICE
          : scan.canRun
            ? undefined
            : NO_MODEL_NOTICE
      }
      onClick={scan.onRun}
    >
      {SCAN_AGAIN_LABEL}
    </button>

    <button
      type="button"
      style={linkStyle}
      aria-expanded={historyOpen}
      onClick={onToggleHistory}
    >
      {HISTORY_LABEL}
    </button>

    {/* The right-hand controls. The WRAPPER carries the `auto` margin, not the
        Reset button, because the fold must sit at the far edge whether or not
        Reset's words have loaded — a control that moves depending on someone
        else's fetch is a control a human has to look for twice. */}
    <span
      style={{ marginLeft: "auto", display: "flex", alignItems: "center", gap: "0.5rem" }}
    >
      {grammar && (
        <button type="button" onClick={onResetOrder} style={linkStyle}>
          {grammar.reset_order_label}
        </button>
      )}
      {/* The filter pills stay visible when this closes, so a folded section
          still says how many facts are in it. */}
      <SectionFold open={open} onToggle={onToggleOpen} names="the scenario facts" />
    </span>
  </div>
);

export default ScenarioFactsHeader;
