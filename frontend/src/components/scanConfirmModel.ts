// =============================================================================
// scanConfirmModel — clicking Scan must not start a scan
// =============================================================================
//
// Until 2026-09-11 the header's `Scan again` button was wired straight to
// `scan.onRun`: one click, and 313 metered calls were under way with nothing
// asked and nothing to cancel from. This module is the gate that replaced it,
// and it is a PURE state machine rather than a `useState` inside the component
// for one reason — it is the thing that has to be proved.
//
// ## Rust Learning: this is the reducer shape, in TypeScript
//
// Roman will recognise the pattern from `PromoteOutcome` and the pipeline steps:
// a function from (state, action) to a new state plus a DESCRIBED effect, where
// the effect is returned as data and performed by the caller. The component
// below it does the running; this decides whether running is allowed, and a test
// can enumerate every action without a browser, a DOM or a click.
//
// That matters here because the property worth asserting is a NEGATIVE — "no
// other action starts a scan" — and a negative over a component is only as good
// as the list of interactions someone remembered to simulate. Over a closed
// union of actions it is exhaustive by construction.
//
// There is no RTL and no jsdom in this tree (CLAUDE.md rule 30), so a fired
// click is not available to assert this. This shape is what makes the assertion
// possible anyway, and it is stronger than the click would have been.

import type { ScanModel, ScanWording } from "../services/themeScan";

/** Where the confirmation is. */
export type ScanConfirmState =
  | { phase: "idle" }
  /** The bar is open, holding the model it would run with if confirmed. */
  | { phase: "confirming"; modelId: string };

/**
 * Everything a human can do to the header that the confirmation cares about.
 *
 * The last two are in this union deliberately, even though neither changes the
 * confirmation's state. They are here so the exhaustive test below has to name
 * them — a control added to this header later is a control someone must decide
 * about, and an action that silently fell outside the machine would be a
 * control the mutation proof never covered.
 */
export type ScanConfirmAction =
  | { type: "openConfirm" }
  | { type: "chooseModel"; modelId: string }
  | { type: "cancel" }
  | { type: "confirmRun" }
  | { type: "toggleHistory" }
  | { type: "toggleCollapse" };

/** The new state, and the run to start — `null` for "start nothing". */
export interface ScanConfirmStep {
  state: ScanConfirmState;
  /**
   * The model id to scan with, or `null`.
   *
   * A model ID rather than a boolean so the caller cannot start a run with a
   * model other than the one the sentence named. The confirmation says "Run a
   * theme scan with Qwen3.8 27B?" and this is the answer to that question, not
   * a permission to read the picker again afterwards.
   */
  run: string | null;
}

/** The starting state: nothing asked, nothing running. */
export const idleConfirm: ScanConfirmState = { phase: "idle" };

/**
 * Advance the confirmation.
 *
 * `selectedModel` is the picker's current choice, used only when the bar OPENS
 * — from that moment the machine holds its own copy, so a dropdown left open
 * behind the bar cannot change what a click on Run scan does.
 */
export function scanConfirmStep(
  state: ScanConfirmState,
  action: ScanConfirmAction,
  selectedModel: string | null,
): ScanConfirmStep {
  switch (action.type) {
    case "openConfirm":
      // No model, no question to ask. Staying idle rather than opening an
      // unanswerable bar: the pill is already disabled in that state, and a
      // second guard costs nothing next to the failure it prevents.
      return selectedModel === null
        ? { state: { phase: "idle" }, run: null }
        : { state: { phase: "confirming", modelId: selectedModel }, run: null };

    case "chooseModel":
      // Choosing from the dropdown re-aims the open confirmation; it does not
      // open one, and it certainly does not run one.
      return state.phase === "confirming"
        ? { state: { phase: "confirming", modelId: action.modelId }, run: null }
        : { state, run: null };

    case "cancel":
      return { state: { phase: "idle" }, run: null };

    case "confirmRun":
      // The ONE arm that runs. Guarded on `confirming` rather than trusting the
      // caller: a Run scan button can only exist while the bar is open, but
      // "can only" is the kind of claim that survives exactly until the next
      // refactor moves the button.
      return state.phase === "confirming"
        ? { state: { phase: "idle" }, run: state.modelId }
        : { state, run: null };

    case "toggleHistory":
    case "toggleCollapse":
      // The confirmation SURVIVES both. Closing it because a human opened the
      // history would discard a question they are part-way through answering —
      // and the history is exactly where someone checks how long the last scan
      // took before agreeing to another.
      return { state, run: null };
  }
}

/**
 * The confirmation's sentence, split at the slot the template itself declares.
 *
 * ## Why parts and not one string
 *
 * The mockup sets the model's name in bold inside the question — it is the one
 * word the human has to check before answering. Emphasising it means putting a
 * tag around part of a stored sentence, and the wrong way to do that is to
 * search the rendered text for the model name afterwards: that is the browser
 * parsing prose, and it breaks the day a model is called "Scan" or a template is
 * reworded.
 *
 * Splitting at `{model}` BEFORE substitution is not parsing. The slot is the
 * template's own declaration of where the name goes, so the emphasis follows the
 * stored sentence's structure rather than guessing at it — and a template
 * without the slot simply yields no emphasised part.
 */
export interface ConfirmSentence {
  /** Everything before the model's name. */
  before: string;
  /** The name itself, set in bold by the bar. */
  model: string;
  /** Everything after it. */
  after: string;
}

/**
 * The confirmation's sentence, or `null` when there is nothing to ask.
 *
 * ## Why three templates and not one with optional slots
 *
 * The estimate is MEASURED, per model, from that model's completed runs — so a
 * model nobody has scanned with has no estimate, and a candidate count that
 * failed to read has no estimate either (the estimate being the count times the
 * rate). A single template with an empty `{minutes}` renders "about  minutes"
 * on screen; a defaulted zero promises a 313-card scan will take no time at all.
 * Each state gets a complete sentence instead.
 */
export function confirmSentence(input: {
  wording: ScanWording;
  model: ScanModel | undefined;
  candidateCount: number | null;
}): ConfirmSentence | null {
  const { wording, model, candidateCount } = input;
  if (model === undefined) return null;

  const filled = (template: string): ConfirmSentence => {
    // `split` with a limit of 2 is deliberate: a template that names the model
    // twice emphasises the first and leaves the rest of the sentence — including
    // the second mention — in `after`, rather than silently losing a clause.
    const at = template.indexOf("{model}");
    if (at === -1) return { before: template, model: "", after: "" };
    return {
      before: template.slice(0, at),
      model: model.confirm_label,
      after: template.slice(at + "{model}".length),
    };
  };

  if (candidateCount === null) return filled(wording.header_confirm_no_count_template);

  const minutes = estimatedMinutes(candidateCount, model.measured_seconds_per_candidate);
  const template =
    minutes === null
      ? wording.header_confirm_template.replace("{count}", String(candidateCount))
      : wording.header_confirm_timed_template
          .replace("{count}", String(candidateCount))
          .replace("{minutes}", String(minutes));
  return filled(template);
}

/**
 * The same sentence as one flat string.
 *
 * For the accessible name of the bar, where markup is not available and the
 * emphasis is carried by the `<strong>` in the visible copy instead. One
 * function so the spoken sentence and the read one cannot come apart.
 */
export function flattenConfirmSentence(s: ConfirmSentence): string {
  return `${s.before}${s.model}${s.after}`;
}

/**
 * How long this scan is likely to take, in whole minutes, or `null`.
 *
 * `null` whenever the rate is absent or not a usable positive number. The field
 * is omitted from the payload when nothing has been measured, so `undefined` is
 * the ordinary case on a model's first use — not an error, just an estimate this
 * deployment has not earned the right to make yet.
 *
 * Rounded UP to a minimum of one minute. A twenty-second scan reading "about 1
 * minute" overstates by forty seconds; the alternative rounds to "about 0
 * minutes", which reads as an error and is the only version a human would have
 * to think about.
 */
export function estimatedMinutes(
  candidateCount: number,
  secondsPerCandidate: number | undefined,
): number | null {
  if (secondsPerCandidate === undefined || !Number.isFinite(secondsPerCandidate)) return null;
  if (secondsPerCandidate <= 0) return null;
  if (candidateCount <= 0) return null;
  return Math.max(1, Math.round((candidateCount * secondsPerCandidate) / 60));
}
