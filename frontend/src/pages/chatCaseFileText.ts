// chatCaseFileText.ts — the "Chat case file" box's sentences, as Roman wrote
// them (CC_TASK_KEEPWARM_BUTTON_v1, ruled in chat 2026-09-24 13:05–13:07).
//
// Pure functions so the words are tested without a browser. No internal word
// ("cache", "prefix", "tokens") appears in anything returned here.

import type { ChatCaseFileActivity, KeepLoadedResult } from "../services/chatCaseFile";

export const BOX_TITLE = "Chat case file";
export const BOX_INTRO =
  "Keeps the case file loaded so the next chat question doesn't pay to reload it.";
export const BUTTON_LABEL = "Keep loaded for another hour";
export const BUTTON_BUSY_LABEL = "Keeping it loaded…";
/** Not in the ruled copy: the box's one plain sentence for any failure. */
export const ERROR_LINE = "That didn't work. Please try again in a minute.";
/** Not in the ruled copy: the page could not read the activity lines. */
export const READ_ERROR_LINE = "The last chat activity could not be read. Reload the page to try again.";
/** Ruled 2026-09-24 (plan STOP): any model whose prices are not known. */
const COST_UNKNOWN = "Cost not known for this model.";

/** `$2.95` — two decimals. */
export function dollars(amount: number): string {
  return `$${amount.toFixed(2)}`;
}

/** `7¢` for a cost under a dollar; dollars otherwise. */
export function cents(amount: number): string {
  if (amount >= 1) return dollars(amount);
  return `${Math.max(1, Math.round(amount * 100))}¢`;
}

/** "Last chat question: 1:42 pm, by Marie". */
export function lastQuestionLine(a: ChatCaseFileActivity): string {
  if (a.last_question_at === null) return "Last chat question: none yet.";
  const by = a.last_question_by === null ? "" : `, by ${a.last_question_by}`;
  return `Last chat question: ${a.last_question_at}${by}`;
}

/** "Loaded until: 2:42 pm", or the not-loaded sentence with its reload cost. */
export function loadedLine(a: ChatCaseFileActivity): string {
  if (a.loaded && a.loaded_until !== null) return `Loaded until: ${a.loaded_until}`;
  if (a.reload_cost_dollars === null) return "Not loaded. The next question will reload it.";
  return `Not loaded. The next question will reload it (about ${dollars(a.reload_cost_dollars)}).`;
}

/** The line after a tap: read ("Done…") or wrote ("It had run out…"). */
export function resultLine(r: KeepLoadedResult): string {
  if (r.outcome === "read") {
    const cost = r.cost_dollars === null ? COST_UNKNOWN : `Cost ${cents(r.cost_dollars)}.`;
    return `Done. Kept loaded until ${r.loaded_until}. ${cost}`;
  }
  const cost = r.cost_dollars === null ? COST_UNKNOWN : `Cost ${dollars(r.cost_dollars)}.`;
  return `It had run out, so it was reloaded. Kept until ${r.loaded_until}. ${cost}`;
}
