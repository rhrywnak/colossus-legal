// =============================================================================
// pairCardModel.ts — the unified pair card's model, as data (task R4, P3)
// =============================================================================
//
// ONE card renders on two pages: the working page's accusation section, where a
// human marks instances and pairs answers, and the rehearsal page, where Marie
// reads the same statements with no controls at all. This module is the shape
// they share, and the two adapters that build it from the two very different
// payloads those pages hold.
//
// ## Why a model module and not one component with two prop shapes
//
// The card has real rules — which words get highlighted, when a quote is long
// enough to fold, what a missing speaker reads as — and a component is the worst
// place to keep a rule, because the only way to test it there is to render it.
// This repo has no DOM test infrastructure (CLAUDE.md rule 30), so a rule that
// lives in JSX is a rule nothing checks. Every decision the card makes is made
// here, returned as data, and asserted in `__tests__/pairCardModel.test.ts`.
//
// `PairCard.tsx` then does what a component should: it turns this data into
// elements and holds the open/closed state of the fold.
//
// ## What the card closes (the P3 complaints, in Roman's words)
//
//   "Our answer blends in"          → the answer is its own block, green-edged,
//                                     labelled, with its own provenance
//   "no context per entry"          → every card says who, when, what kind, and
//                                     which document it came out of
//   "no C-code on rehearsal"        → the code rides both adapters; codes are
//                                     speakable handles, not internal vocabulary
//   "human has to hunt through the  → the operative words are highlighted and a
//    text"                            long quote folds around them
//
// ## Domain note: the highlight, and the marked line that does not exist yet
//
// The design says the highlight is "the human MARKED LINE when one exists, else
// the scan's anchor quote". There is no marked line in this build — the four
// human-fact kinds are `fact`, `watch_list`, `accusation_instance` and
// `answer_pairing`, and none of them stores a span within a statement. So today
// every highlight resolves to the anchor quote.
//
// [`QuoteFold.source`] names which one it was anyway. That is not decoration: it
// is the field the authoring feature fills in when it lands, and having it here
// from the start means the card, its tests and its two adapters do not move when
// it does. A reader of this code should not have to wonder whether the fallback
// is deliberate.

import type { ScenarioCard } from "../services/scenarioCards";
import type { RehearsalAnswer, RehearsalInstance } from "../services/rehearsal";

/** Where a highlight's words came from. See this module's header. */
export const HIGHLIGHT_MARKED_LINE = "marked_line";
export const HIGHLIGHT_ANCHOR_QUOTE = "anchor_quote";

/**
 * A quote, split into the words around its operative phrase.
 *
 * `before` and `after` are routinely empty — an anchor quote with no surrounding
 * context is the ordinary case on the rehearsal page, where the payload carries
 * the verbatim statement and nothing either side of it.
 */
export type QuoteFold = {
  before: string;
  /** The operative words. Never empty on a card that renders at all. */
  highlight: string;
  after: string;
  /** `marked_line` | `anchor_quote` — which source the highlight came from. */
  source: string;
  /**
   * Whether this quote is long enough to be worth folding.
   *
   * `false` renders the whole thing with no control, which is most cards. A fold
   * control over a one-line quote is a control that costs a click and reveals
   * nothing.
   */
  needsFold: boolean;
};

/** Who said a thing, when, in what, and what we call it. */
export type PairCardProvenance = {
  /** The speaker's name, or the served sentence for an unrecorded one. */
  who: string;
  /** The date as it should READ, or `null` when nothing is dated. */
  when: string | null;
  /** "Deposition", "Letter" — `null` when the extraction recorded no kind. */
  kindLabel: string | null;
  /** "Hearing to approve plan, p. 24" — `null` when the record cannot say. */
  sourceLabel: string | null;
  /** Opens the document at that page. `null` renders no control (never a dead link). */
  sourceHref: string | null;
  /** "C-91" — the handle a human says out loud. `null` when unnumbered. */
  code: string | null;
};

/** One statement and, beneath it, what we say back. */
export type PairCardModel = {
  provenance: PairCardProvenance;
  quote: QuoteFold;
  /** Our answer, or `null` when nobody has paired one. */
  answer: { provenance: PairCardProvenance; quote: QuoteFold } | null;
};

/**
 * How many characters of quote earn a fold.
 *
 * ## Why a character count and not a line count
 *
 * The design asks for a clamp of about two lines. A LINE is a property of the
 * rendered box — it depends on the width of the window Marie happens to have
 * open — so a model that counted lines would be computing something it cannot
 * see. Characters are a property of the data, which is what this module is
 * allowed to reason about; the visual clamp itself is CSS, in the component,
 * where the box actually exists.
 *
 * 240 is roughly two lines at the card's width and font size. It decides only
 * whether a CONTROL is offered — being wrong by a line costs a reader one
 * unnecessary click, never a word of evidence.
 */
export const FOLD_THRESHOLD_CHARS = 240;

/**
 * Build the fold for one quote.
 *
 * `context` flanks are optional because only the working page has them: its
 * cards carry sentence-expanded page text either side of the anchor, and the
 * rehearsal payload carries the anchor alone.
 */
export function foldQuote(
  highlight: string,
  before = "",
  after = "",
): QuoteFold {
  const total = before.length + highlight.length + after.length;
  return {
    before,
    highlight,
    after,
    // Always the anchor today. See this module's header.
    source: HIGHLIGHT_ANCHOR_QUOTE,
    needsFold: total > FOLD_THRESHOLD_CHARS,
  };
}

/**
 * The working page's adapter: one candidate card becomes one pair card.
 *
 * ## Why the working page needs no new payload
 *
 * Everything the card header wants is already on `ScenarioCard`, because the
 * queue above renders the same facts: the code, the speaker, the statement kind,
 * the pinpoint label and its page, and the quote with the context either side of
 * it. The accusation section was looking up nothing but the TEXT and throwing
 * the rest away.
 *
 * @param card the included fact this instance points at
 * @param whoUnrecorded the served sentence for a statement with no speaker —
 *        passed in rather than written here, because it is vocabulary
 */
export function pairCardFromScenarioCard(
  card: ScenarioCard,
  whoUnrecorded: string,
): { provenance: PairCardProvenance; quote: QuoteFold } {
  return {
    provenance: {
      // A speaker the record does not name is a real and measured state (one of
      // forty-six on S-2). It reads as the served sentence rather than as a
      // blank, which would look like a card that failed to render.
      who: card.speaker.name ?? whoUnrecorded,
      // The card payload carries no composed date, and this module will not
      // build one out of parts — a date is a claim about precision and the
      // backend owns how it reads. Absent here is honest, not a gap in the work.
      when: null,
      kindLabel: card.statement_kind,
      sourceLabel: card.pinpoint.label,
      // Served, never assembled here — `CardPinpoint.viewer_href` says so in
      // terms ("the browser builds no URLs"). An empty one offers no control
      // rather than a link that goes nowhere: a reader clicks it in front of
      // opposing counsel.
      sourceHref: card.pinpoint.viewer_href || null,
      code: card.code,
    },
    quote: foldQuote(card.quote.text, card.quote.context_before, card.quote.context_after),
  };
}

/**
 * The rehearsal page's adapter: one prep instance becomes one pair card.
 *
 * Everything here arrives COMPOSED — `who`, `when`, the source label and its
 * href are all built server-side, which is that page's standing law. This
 * adapter chooses nothing; it names which composed field goes in which slot.
 */
export function pairCardFromRehearsalInstance(
  instance: RehearsalInstance,
): { provenance: PairCardProvenance; quote: QuoteFold } {
  return {
    provenance: {
      who: instance.who,
      // Exactly one of these is ever present — the backend decides, so this
      // cannot render both or neither.
      when: instance.when ?? instance.when_gap,
      kindLabel: instance.kind_label || null,
      sourceLabel: instance.source.label || null,
      sourceHref: instance.source.href || null,
      code: instance.code ?? null,
    },
    // No context flanks on this payload: the prep page carries the verbatim
    // statement and nothing either side of it, so a long quote folds around a
    // highlight that spans the whole of it. That is still worth doing — C-91's
    // twelve lines clamp to two — it simply has no dimmed surroundings.
    quote: foldQuote(instance.quote),
  };
}

/** The rehearsal page's answer adapter. Same rules, one fewer field. */
export function pairCardFromRehearsalAnswer(
  answer: RehearsalAnswer,
): { provenance: PairCardProvenance; quote: QuoteFold } {
  return {
    provenance: {
      who: answer.who,
      when: answer.when ?? answer.when_gap,
      // An answer's kind is not on this payload and is not invented here.
      kindLabel: null,
      sourceLabel: answer.source.label || null,
      sourceHref: answer.source.href || null,
      code: answer.code ?? null,
    },
    quote: foldQuote(answer.quote),
  };
}
