// =============================================================================
// subsetSaveModel.ts — what Save sends, and what the banner says when half of it
// lands
// =============================================================================
//
// T6.0 and T6.4, defect D2. Two pure decisions the modal used to make inside a
// callback, where no test could reach them.
//
// ## ⚑ WHY A SAVE IS TWO CALLS AT ALL
//
// A subset's name and description live on `chronology_subsets`; its events live
// on `chronology_subset_events`, replaced as a whole set by their own endpoint
// (T1's write contract). One Save is therefore up to two writes, and the second
// can fail after the first has already committed. Before T6.4 the modal said
// "That change was not saved" in that case. Half of it HAD been saved: the name
// was on the row, and a reader who believed the banner would type it again.
//
// So the banner is built here, in halves, from the store's own rows.

import { cw, fill, type ChronologyWording } from "../../services/caseTimeline";
import type { SubsetDetail } from "../../services/caseTimelineSubsets";
import { initialPicks, type Pick, toSubsetPayload } from "./subsetPicker";

/**
 * Has the picked list changed since the modal opened?
 *
 * ## Why this exists even though the 422 was not caused by a clean save
 *
 * T6.0 named the real cause — the handler took a bare JSON array while the modal
 * sent `{"events": […]}` — and it was not "the backend rejects an unchanged
 * list". But sending the events PUT after a rename that touched no event is
 * still a write nobody performed: it puts an `events_replaced` row in
 * `chronology_subset_history`, which is the audit trail somebody will one day
 * read to answer "when did this story last change". The name/description call
 * already skips itself when unchanged, for exactly that reason. This makes the
 * events call behave the same way.
 *
 * ## Both sides go through `toSubsetPayload`, deliberately
 *
 * Comparing `picks` against `detail.events` directly would mean re-implementing
 * the trim and the 1-based numbering that the payload builder applies — and the
 * question being asked is precisely "would the wire bytes differ", so the wire
 * builder is the right authority for both sides. A note the author typed and
 * then blanked back to spaces is NOT a change, and this is why.
 *
 * Order matters: the same fifteen events in a different order is a different
 * story, and must be saved.
 */
export function eventsAreDirty(original: SubsetDetail | null, picks: Pick[]): boolean {
  // Creating: there is no original, so everything about it is new. The caller
  // does not consult this on the create path, but a function that answered
  // "clean" for a brand-new subset would be a trap for the next caller.
  if (original === null) return true;

  const before = toSubsetPayload(initialPicks(original));
  const after = toSubsetPayload(picks);
  if (before.length !== after.length) return true;
  return before.some((ref, index) => {
    const now = after[index];
    // `note` is absent rather than empty when there is none — `toSubsetPayload`
    // omits it — so `undefined` on both sides compares equal here without any
    // normalising of its own.
    return (
      ref.event_id !== now.event_id || ref.position !== now.position || ref.note !== now.note
    );
  });
}

/**
 * A save that did not fully land, in the terms the banner needs.
 *
 * `sentence` is the whole named sentence the service threw — "That subset's
 * events were not saved (HTTP 422: …)." It is carried alongside the parts
 * because there are failures the split cannot describe (a timeout has no status
 * and no server reason) and the honest thing to render then is the sentence
 * that was always rendered.
 */
export type SaveFailure = {
  /** Did the name/description call commit before this failure? */
  nameSaved: boolean;
  /** The HTTP status, or null when no server answered. */
  status: number | null;
  /** The server's own message. Empty when the body carried none. */
  reason: string;
  /** The service's complete named sentence. */
  sentence: string;
};

/** The banner, in halves. `saved` is the green line; null when nothing saved. */
export type BannerModel = { saved: string | null; failed: string };

/**
 * What the banner says.
 *
 * Three outcomes, and only two of them reach here — both calls succeeding closes
 * the modal (T6.4).
 *
 * 1. The FIRST call failed. Nothing was attempted after it and nothing
 *    committed, so the banner is the existing named sentence and there is no
 *    green half to draw. Inventing one would be the old lie with the colours
 *    reversed.
 * 2. The first SUCCEEDED and the events call failed. Green: what saved. Red:
 *    the events template, carrying the status and the SERVER's own reason —
 *    T1 answers 400/409/422 naming the offending field and value, and replacing
 *    that with our own words would throw away the only part that says what to
 *    fix.
 *
 * ## The empty reason, and why the fix is a cut rather than a word
 *
 * A body with no readable message would render "…(HTTP 422: )" — a colon in
 * front of nothing. The separator is cut instead of being filled with something
 * like "no reason given", because that phrasing would be a user-visible string
 * written in code, which is the one thing this app does not do. The cut is
 * conservative: it only fires when the reason is genuinely empty, and if the
 * template is ever reworded so the reason is not the last thing before a
 * parenthesis, the text passes through untouched rather than being mangled.
 *
 * A failure with no status at all (a timeout, a dropped connection) cannot fill
 * `{status}` with anything true, so it renders the service's sentence instead.
 */
export function bannerModel(wording: ChronologyWording, failure: SaveFailure): BannerModel {
  if (!failure.nameSaved) return { saved: null, failed: failure.sentence };

  const saved = cw(wording, "subsets_saved_name_only_banner");
  if (failure.status === null) return { saved, failed: failure.sentence };

  const reason = failure.reason.trim();
  const text = fill(cw(wording, "subsets_events_not_saved_banner_template"), {
    status: String(failure.status),
    reason,
  });
  return { saved, failed: reason === "" ? text.replace(/:\s*\)/, ")") : text };
}
