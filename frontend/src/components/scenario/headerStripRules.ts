// =============================================================================
// headerStripRules.ts — what the header strip's controls are allowed to do
// =============================================================================
//
// TIMELINE_SUBSET_MOCKUP_v2_2026-08-31.html Screen 1, approved as drawn, and
// design §11 item 1. The pure half of `ScenarioHeaderStrip.tsx`, split for the
// reason this project splits every such pair: there is no component-testing tier
// here (CLAUDE.md rule 30), so a rule decided inside a component is a rule no
// test can reach.
//
// ⚑ NAMED `headerStripRules` and not `scenarioHeaderStrip`, which is what T5.4
// asked for: macOS is case-insensitive, so `scenarioHeaderStrip.ts` and
// `ScenarioHeaderStrip.tsx` are the SAME PATH to the filesystem and TypeScript
// refuses the pair outright (TS1261). The task named both files; only one of
// the two names can survive, and the component keeps the one a reader greps for.
//
// ## ⚑ WHAT THE STRIP IS FOR, IN ROMAN'S WORDS
//
// "It looks like crap … very chaotic." Five surfaces each owned a piece of
// header: the detail page had `ScenarioHeaderTiers`, rehearsal had
// `RehearsalPageHeader`, practice built a bare `<h1>` from its deck payload, the
// dashboard had no per-scenario header at all, and the questions page had
// nothing. Each grew its own controls. This module holds the ONE rule they now
// share, so the four surfaces that render the strip cannot disagree about when a
// control is live.
//
// ## The one rule, and the defect behind it
//
// Rehearsal is the surface a witness is TAKEN TO. Before .390 the control looked
// alive on every scenario and, clicked on a Draft one, silently delivered a
// DIFFERENT scenario's rehearsal — one missing argument, two silent
// substitutions. So the control is inert unless the scenario is actually Ready,
// and it says why on hover.
//
// Practice is deliberately NOT gated, and that asymmetry is the interesting
// part: the drill is where a deck is found to be no good, on scenarios still
// being built, and the page it opens is the one that reports "this scenario has
// no deck yet". Gating it would hide the only screen able to say so.

/** The stored status column. `needs_evidence` is permitted by ruling 6. */
export type ScenarioStripStatus = string;

/** What row 2 of the strip is allowed to do for one status. */
export type StripControls = {
  /** The Rehearsal view control is a live link rather than an inert span. */
  rehearsalEnabled: boolean;
  /** Practice is never gated — see the module header. */
  practiceEnabled: boolean;
  /** Editing identity is never gated: a half-authored scenario is the normal case. */
  editEnabled: boolean;
  /** Delete is never gated; the confirm dialog is the guard (Roman, 2026-08-07). */
  deleteEnabled: boolean;
};

/**
 * Which of the strip's controls are live for a given status.
 *
 * ONE branch, and the other three constants are the point: writing them out
 * makes "why is Practice not gated when Rehearsal is?" a question the code
 * answers instead of one a reader has to reconstruct from four call sites.
 *
 * `"ready"` and nothing else enables rehearsal. Not `!== "draft"`: the status
 * column also permits `needs_evidence`, and a scenario that needs evidence is
 * exactly the kind nobody should be taken into a rehearsal on.
 */
export function stripControls(status: ScenarioStripStatus): StripControls {
  return {
    rehearsalEnabled: status === "ready",
    practiceEnabled: true,
    editEnabled: true,
    deleteEnabled: true,
  };
}

/**
 * Is the View Timeline button drawn at all?
 *
 * ABSENT and not disabled, per Screen 1: "when no subset is attached the button
 * is simply absent — nothing else shifts". A disabled button here would be an
 * offer of something that does not exist — there is no timeline story to view
 * until somebody attaches one, and the place to do that is the Timeline subsets
 * section, not this strip.
 */
export function showsViewTimeline(attachedCount: number): boolean {
  return attachedCount > 0;
}
