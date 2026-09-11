// =============================================================================
// scanHeaderApi — what `ThemeScanPanel` lends a caller that draws its controls
// =============================================================================
//
// Lifted out of `ThemeScanPanel.tsx` on 2026-09-11. The type had to GROW — the
// header gained a model picker and a confirmation bar, so the panel now lends
// out the catalogue and the candidate count as well as the three controls — and
// that file is pre-existing debt at 853 non-comment lines against a 300-line
// limit (ruling R6 puts its remediation in task 3.14). Growing it to make room
// for a type is the one thing that task may not do, so the type moved instead
// and the panel came out smaller than it went in.
//
// ## Why the panel STAYS MOUNTED and lends its controls out
//
// (Carried over verbatim in substance from the panel, because this is where a
// reader now meets the question.)
//
// The scenario page has no scan CARD any more: the mockup puts the last-scan
// line, the scan control and the history on the Scenario facts header row. The
// obvious implementation — draw them in `ScenarioFactsSection` and delete this
// panel's chrome — would have to re-fetch the model catalogue and the run
// history to know what to draw, which is two readers of one payload and the
// exact way two surfaces come to disagree about which run was last.
//
// Worse, it would UNMOUNT the panel, and architect ruling R3 is explicit about
// what that costs: the panel's mount effect calls `gatherCandidates`, and gather
// is the ONE place candidate ordinals are minted — every card's `C-14` handle.
// A proposed card would arrive with `code: null`, which §2a forbids ("pull up
// C-14" must be speakable).
//
// So the panel keeps its state, its fetches and its mount effect, and lends what
// the header needs through this shape. Nothing below is derived by the caller.

import type React from "react";

import type { ScanModel, ScanWording } from "../services/themeScan";

/** The scan, as a header may borrow it. */
export interface ScanHeaderApi {
  /**
   * The ONE sentence beneath the heading, or `null` for no sentence.
   *
   * Composed by the panel through `factsHeaderLine`, from the run history it
   * already holds. It is a single string and not a pair of fields on purpose:
   * the defect this replaced was a header that could render a last-scan summary
   * and "No scan has run yet" as two independent decisions, and did. A caller
   * that is handed one string cannot show two.
   */
  headerLine: string | null;
  /** A run is in flight. */
  running: boolean;
  /** A model is selected and a run could start. False while the catalogue loads. */
  canRun: boolean;
  /**
   * Start a scan with an explicitly named model.
   *
   * Takes the id rather than reading the panel's selection, so the run that
   * starts is the one the confirmation sentence named. A confirmation that says
   * "Run a theme scan with Qwen3.8 27B?" and then re-reads a dropdown is a
   * confirmation about nothing.
   */
  onRun: (modelId: string) => void;
  /** The run-history disclosure, ready to mount wherever the caller puts it. */
  history: React.ReactNode;
  /**
   * The scan-eligible catalogue, already loaded, already ordered local-first by
   * the SERVER, with the billing labels already composed.
   *
   * Lent rather than re-fetched: `fetchScanModels` is called once, by the panel,
   * on mount. A header calling it again would be the second reader of one
   * payload that the module header above exists to prevent.
   */
  models: ScanModel[];
  /** The picker's current choice, or `null` while the catalogue loads. */
  selectedModel: string | null;
  /** Change the choice. The panel owns the state; the header owns the control. */
  onSelect: (modelId: string) => void;
  /**
   * Candidates in the pool right now, or `null` when the count is unknown —
   * which includes the case where the read FAILED.
   *
   * The confirmation drops both the count and the estimate when this is `null`,
   * rather than showing a zero. A scan offered as "0 candidates" reads as a
   * scan with nothing to do.
   */
  candidateCount: number | null;
  /** Every stored string the header renders, or `null` until they load. */
  wording: ScanWording | null;
}
