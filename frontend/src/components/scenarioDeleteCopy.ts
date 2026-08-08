// =============================================================================
// scenarioDeleteCopy.ts — the one wording for "are you sure you want to delete
// this scenario?", wherever it is asked (2026-08-07)
// =============================================================================
//
// Two surfaces now ask it: the scenario page's header kebab, and each card on
// the Trial Prep dashboard. The words below are the scenario page's, moved here
// verbatim rather than retyped at the second call site.
//
// ## Why this is a function and not two copies of three strings
//
// A confirm dialog is the last thing a human reads before something becomes
// irreversible. Two copies of that sentence are two sentences free to drift —
// and the drift would be invisible, because nobody sees both dialogs at once.
// The instruction for this change put it plainly: the detail page's existing
// delete wording IS the vocabulary; reuse it, do not duplicate it.
//
// ## Why these strings are in code and not `app_settings` rows
//
// Every NEW user-facing string in this codebase is a stored row (v2 §2b). These
// are not new — they are the existing dialog's words, relocated. Moving them to
// rows is a worthwhile change and a separate one: it needs a migration, a
// wording block, and a delivery channel to two different payloads. Doing it
// inside a change that adds a delete control would put untested string plumbing
// in the same commit as a destructive action. Recorded here rather than passed
// off as done.

/** What the confirm dialog says about deleting one scenario. */
export interface ScenarioDeleteCopy {
  title: string;
  message: string;
  confirmLabel: string;
}

/**
 * The dialog's words for the scenario named by `attack`.
 *
 * The message names the scenario, states what else goes with it, says the act
 * is irreversible, and — the part that stops a reasonable person hesitating
 * forever — says what is NOT affected. A scenario is a curation artifact; the
 * evidence it points at lives in the case graph and survives.
 *
 * @param attack the scenario's title as it reads on screen (`ScenarioSummary.attack`
 *   on the dashboard, `ScenarioDetail.attack` on the page — the same string).
 */
export function scenarioDeleteCopy(attack: string): ScenarioDeleteCopy {
  return {
    title: "Delete this scenario?",
    message:
      `“${attack}” and its curated facts and responses will be ` +
      `permanently deleted. This cannot be undone. (The underlying evidence ` +
      `in the case graph is not affected.)`,
    confirmLabel: "Delete scenario",
  };
}
