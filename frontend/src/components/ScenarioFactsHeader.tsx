// =============================================================================
// ScenarioFactsHeader.tsx — the Scenario facts header row (rebuilt 2026-09-11)
// =============================================================================
//
// Mockup `FACTS_HEADER_MOCKUP_v1_2026-09-11`, drawn on live PROD S-13.
//
// Left, stacked: the heading, and beneath it the honest last-scan line.
// Right: the split `Scan │ {model} ▾` pill, `History`, a `⋯` overflow, and the
// fold chevron. Under the row, when it has been asked for, the confirmation bar.
//
// ## ⚑ Why the SCAN controls live on the FACTS header
//
// Ruling R35 retired the "Scan & candidates" section. A scan is not a thing a
// human does for its own sake — it produces candidates, and candidates become
// facts — so the control belongs on the list it feeds, not in a card of its own
// above it.
//
// The engine did NOT move: `ThemeScanPanel` still owns the model catalogue, the
// run history, the poll and the mount-time `gatherCandidates` that mints every
// card's `C-14` handle (architect ruling R3). It lends this header everything
// below through `ScanHeaderApi`, and nothing here is derived from a second read.
//
// ## The three things this rebuild changed, and why
//
// **1. `Scan again` no longer scans.** It was one click from 313 metered calls
// with nothing asked. It is now `Scan`, and it opens `ScanConfirmBar`. The
// machine that guarantees no other control can start a run is `scanConfirmModel`
// — a pure reducer, so the guarantee is asserted over a closed set of actions
// rather than over whichever clicks a test remembered to fire.
//
// **2. The last-scan line cannot lie.** It used to be two independent decisions:
// a summary composed from the run history, and the served "No scan has run yet"
// notice shown whenever nothing had COMPLETED. On PROD S-13 a scenario whose only
// run was cancelled rendered the notice above a history table listing that run.
// This component is now handed ONE string (`scan.headerLine`, composed by
// `factsHeaderLine`) and there is no longer a pair to contradict.
//
// **3. The whole row folds, and the chevron is bare.** Task 394 P6 gave the fold
// a word because a bare arrow "read as furniture: nobody found it". That reason
// does not survive the row becoming the target — the chevron is now a state
// indicator beside a 48px-tall control the size of the header, not the only way
// in. `SectionFold` is untouched for its other consumer; this draws its own.
//
// ## What is still a literal here, and why that is allowed
//
// `HEADING` alone. Every other word on this row is a settings row as of the
// 2026-09-11 migration — including the two refusal tooltips, which used to be
// listed here as debt. "Scenario facts" is unchanged by this task in both
// wording and meaning, and Law 20 binds new and CHANGED wording; converting a
// string nobody touched would be a second migration riding a defect fix.

import React from "react";

import FactsHeaderOverflow from "./FactsHeaderOverflow";
import ScanConfirmBar from "./ScanConfirmBar";
import ScanSplitPill from "./ScanSplitPill";
import { confirmSentence, idleConfirm, scanConfirmStep } from "./scanConfirmModel";
import type { ScanConfirmAction, ScanConfirmState } from "./scanConfirmModel";
import type { ScanHeaderApi } from "./scanHeaderApi";
import type { CardGrammarWording } from "../services/evidenceLinks";
import {
  sectionHeaderStyle,
  sectionMetaStyle,
  sectionTitleStyle,
} from "./scenarioSectionStyles";

// ⚑ CONST: ONE user-facing literal, and it is pre-existing.
//
// "Scenario facts" has been a literal since 1.7C and merely moved here. The four
// that used to keep it company — "Scan again", "history" and the two refusal
// tooltips — are `scan_header_*` rows as of migration 20260911144911, which is
// what the "Owed rows" note in the previous version of this file was asking for.
const HEADING = "Scenario facts";

/**
 * The row. Mockup: 48px minimum, the whole of it a collapse target.
 *
 * SPREAD from `sectionHeaderStyle` rather than rebuilt, so the page's own 32px
 * gap between sections is inherited and not re-typed here. That is a standing
 * rule with a test behind it (`scenarioPageStructure`): a section that hand-rolls
 * its margin is how one gap on a page starts to differ from the others.
 *
 * What is added on top is what makes the row a control rather than a label —
 * the hit height, the padding the hover fill needs, and the cursor.
 */
const rowStyle: React.CSSProperties = {
  ...sectionHeaderStyle,
  gap: "12px",
  minHeight: "48px",
  padding: "8px 4px",
  borderRadius: "10px",
  cursor: "pointer",
};

/** The heading and its sentence, stacked. The mockup's two-line left block. */
const titleBlockStyle: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: "2px",
  marginRight: "auto",
};

/**
 * 15px/800, not the shared 15px/600.
 *
 * The mockup weights this heading harder than a section title elsewhere, and
 * the reason is on the page: it now sits above a second line of its own, and at
 * 600 the two read as one paragraph rather than as a title and its subtitle.
 */
const headingStyle: React.CSSProperties = { ...sectionTitleStyle, fontWeight: 800 };

/** The last-scan line. A step down in size and colour, per the mockup. */
const lastScanStyle: React.CSSProperties = {
  ...sectionMetaStyle,
  fontSize: "13px",
  color: "var(--text-secondary)",
};

/** The bare chevron. Decoration — the row it sits on carries the behaviour. */
const chevronStyle = (open: boolean): React.CSSProperties => ({
  // 13px / secondary, not 11px / muted. At the smaller size it disappeared into
  // the card against the mockup's, which is the exact failure task 394 P6
  // recorded for the bare arrow — and the whole-row target only excuses a
  // chevron a reader can still SEE.
  fontSize: "13px",
  color: "var(--text-secondary)",
  flexShrink: 0,
  padding: "0 4px",
  // Rotated rather than swapped for a second glyph: the turn is what reads as
  // "this one control moved", where ▸ becoming ▾ reads as two different marks.
  transform: open ? "rotate(0deg)" : "rotate(-90deg)",
  transition: "transform 120ms ease",
  display: "inline-block",
});

interface Props {
  /** The scan, lent by `ThemeScanPanel`. */
  scan: ScanHeaderApi;
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
 * The header row, and the confirmation it can open.
 *
 * ## Why the confirmation's state lives HERE and not in the section
 *
 * `ScenarioFactsSection` is at 281 non-comment lines against a 300-line limit,
 * and more to the point the confirmation is not its business: nothing outside
 * this row opens it, reads it or is affected by it. The state is as local as
 * the control.
 */
const ScenarioFactsHeader: React.FC<Props> = ({
  scan,
  historyOpen,
  onToggleHistory,
  grammar,
  onResetOrder,
  open,
  onToggleOpen,
}) => {
  const [confirm, setConfirm] = React.useState<ScanConfirmState>(idleConfirm);

  /**
   * Every interaction on this row goes through the machine.
   *
   * Including the two that do not change it (history, collapse) — routing them
   * here is what makes "no action but Run scan starts a scan" a property of a
   * closed union rather than a claim about this file. The effect is performed
   * in one place, immediately after the transition, and it is the only call to
   * `scan.onRun` in the application.
   */
  const dispatch = (action: ScanConfirmAction) => {
    const next = scanConfirmStep(confirm, action, scan.selectedModel);
    setConfirm(next.state);
    if (next.run !== null) scan.onRun(next.run);
  };

  const words = scan.wording;
  const sentence =
    words === null || confirm.phase !== "confirming"
      ? null
      : confirmSentence({
          wording: words,
          model: scan.models.find((m) => m.model_id === confirm.modelId),
          candidateCount: scan.candidateCount,
        });

  return (
    <>
      <div
        style={rowStyle}
        role="button"
        tabIndex={0}
        aria-expanded={open}
        // The row's accessible NAME is the heading it contains, so a screen
        // reader hears "Scenario facts, button, expanded" rather than a control
        // needing a second sentence invented to describe it.
        aria-labelledby="scenario-facts-heading"
        onClick={() => {
          dispatch({ type: "toggleCollapse" });
          onToggleOpen();
        }}
        onKeyDown={(e) => {
          // A div with a role is not a button: Enter and Space are the two keys
          // the platform would have given us for free, and they are supplied
          // here rather than left missing.
          if (e.key !== "Enter" && e.key !== " ") return;
          e.preventDefault();
          dispatch({ type: "toggleCollapse" });
          onToggleOpen();
        }}
      >
        <span style={titleBlockStyle}>
          <h2 id="scenario-facts-heading" style={headingStyle}>
            {HEADING}
          </h2>
          {/* ONE sentence or none. See `factsHeaderLine` for why this is a
              single string and not a summary plus a notice. */}
          {scan.headerLine !== null && <span style={lastScanStyle}>{scan.headerLine}</span>}
        </span>

        {words !== null && (
          <>
            <ScanSplitPill
              label={words.header_scan_label}
              models={scan.models}
              selectedModel={scan.selectedModel}
              canRun={scan.canRun}
              refusal={
                scan.running
                  ? words.header_running_notice
                  : scan.canRun
                    ? undefined
                    : words.header_no_model_notice
              }
              onOpenConfirm={() => dispatch({ type: "openConfirm" })}
              onSelect={(modelId) => {
                dispatch({ type: "chooseModel", modelId });
                scan.onSelect(modelId);
              }}
            />

            <button
              type="button"
              style={historyPillStyle}
              aria-expanded={historyOpen}
              onClick={(e) => {
                e.stopPropagation();
                dispatch({ type: "toggleHistory" });
                onToggleHistory();
              }}
            >
              {words.header_history_label}
            </button>
          </>
        )}

        {grammar && (
          <FactsHeaderOverflow
            label={grammar.more_actions_label}
            resetOrderLabel={grammar.reset_order_label}
            onResetOrder={onResetOrder}
          />
        )}

        <span style={chevronStyle(open)} aria-hidden="true">
          ▾
        </span>
      </div>

      {sentence !== null && words !== null && (
        <ScanConfirmBar
          sentence={sentence}
          runLabel={words.header_confirm_run_label}
          cancelLabel={words.header_confirm_cancel_label}
          onRun={() => dispatch({ type: "confirmRun" })}
          onCancel={() => dispatch({ type: "cancel" })}
        />
      )}
    </>
  );
};

/** The History pill. Same shell as the split pill's, one word wide. */
const historyPillStyle: React.CSSProperties = {
  fontFamily: "inherit",
  fontSize: "13px",
  fontWeight: 600,
  color: "var(--text-primary)",
  padding: "7px 14px",
  borderRadius: "999px",
  border: "1px solid var(--border-default)",
  background: "var(--bg-surface)",
  cursor: "pointer",
  flexShrink: 0,
};

export default ScenarioFactsHeader;
