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
// ## ⚑ THERE IS NO GATED CONTROL LEFT (v2.1, ruling R35)
//
// This module existed for ONE branch: rehearsal was the surface a witness is
// TAKEN TO, so its control was inert unless the scenario was Ready. Before .390
// it looked alive on every scenario and, clicked on a Draft one, silently
// delivered a DIFFERENT scenario's rehearsal. That page is retired and its
// control is gone from the strip, so `rehearsalEnabled` goes with it — a rule
// nothing reads is a rule that quietly stops being true.
//
// What survives is the record of the asymmetry, because it is the answer to the
// question a reader will ask next. Practice was deliberately NOT gated: the
// drill is where a deck is found to be no good, on scenarios still being built,
// and the page it opens is the one that reports "this scenario has no deck yet".
// Gating it would hide the only screen able to say so. That reasoning is why the
// three remaining flags are unconditional — and why `ScenarioStatusControl`'s
// Ready tooltip cannot simply be re-pointed at Practice: NOTHING gates on Ready
// on this surface any more (v2.1 STOP, reported to Roman).

/** The stored status column. `needs_evidence` is permitted by ruling 6. */
export type ScenarioStripStatus = string;

/** What row 2 of the strip is allowed to do for one status. */
export type StripControls = {
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
 * NO branch left, and writing the three constants out is still the point: it
 * makes "is anything on this strip gated by status?" a question the code answers
 * — with "no, and here is why" — instead of one a reader has to reconstruct from
 * three call sites.
 *
 * ## Rust Learning parallel: why keep a function that returns a constant?
 *
 * The same reason a Rust `impl` keeps a method that always returns `true`: the
 * SHAPE is the contract. Three surfaces ask this module what they may draw. When
 * the next gate arrives it arrives here, once, rather than as a status
 * comparison inlined into whichever page needed it first — which is exactly how
 * the five headers this module replaced came to disagree.
 *
 * `status` is consequently unused today and is kept in the signature for that
 * contract. It is named with a leading underscore so the linter's
 * unused-parameter rule stays ON for every other function in the tree.
 */
export function stripControls(_status: ScenarioStripStatus): StripControls {
  return {
    practiceEnabled: true,
    editEnabled: true,
    deleteEnabled: true,
  };
}

/**
 * The two directions this build knows how to name.
 *
 * Mirrors `directionChip`'s own map, and exists so the CHIP'S COLOUR is a
 * decision with a test rather than a string comparison against another
 * component's return value. A recognised direction gets the drawing's red; an
 * unrecognised token gets amber, because "Offensive" on a scenario the database
 * calls something else would be the page inventing a posture.
 */
export function isKnownDirection(direction: string): boolean {
  return direction === "offense" || direction === "defense";
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
