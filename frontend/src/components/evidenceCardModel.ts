// =============================================================================
// evidenceCardModel.ts — ONE evidence card, as data (ONE_CARD_GRAMMAR)
// =============================================================================
//
// The law this module IS: **a candidate and an included fact are the same item at
// two lifecycle stages, and they render as the same card.** Only the wrapper
// differs — ruling controls on one, weight/order/Remove on the other.
//
// ## The defect it replaces
//
// Until 2026-08-09 there were two descriptors over one payload. `cardRows` built
// the candidate's rows; `factsTable.includedRows` built a much thinner
// `WorkingRow` for the fact card, and in the projection it threw away the
// speaker's provenance, every element chip, the count chip, the anchor
// highlight, the surrounding context, the confidence band and the scan's reason.
// So the same piece of evidence answered "who said what, and why does it matter
// here?" in the queue and answered almost none of it three inches lower.
//
// The fix is not a third descriptor. It is that BOTH wrappers now call
// `evidenceCardView` on the SAME `ScenarioCard`, so the two cards cannot show
// different fields — not because two lists were kept in step, but because there
// is one list.
//
// ## Why this is pure data and not JSX
//
// CLAUDE.md rule 30 records that component-test infrastructure (RTL, jsdom) is
// deliberately not set up. The task's first acceptance test —
// "a candidate and a fact serve the same card payload fields" — is only a real
// test if the view is a value something can compare. It is, here. What the tests
// cannot reach is that the JSX walks the view faithfully, which is what Roman's
// DEV verification covers; the same division `cardRows` documented in 1.3.
//
// ## The frontend still composes NOTHING
//
// Every string below is a payload string or a stored word. This module chooses
// which pieces exist and where they fold; it never writes a sentence, maps a
// vocabulary, formats a number or builds a URL (v2 §7 item 2).

import type { CardBearsOn, CardHumanLink, ScenarioCard } from "../services/scenarioCards";
import type { CardGrammarWording } from "../services/evidenceLinks";
import { fillSlots } from "../services/evidenceLinks";

// ─── The view ───────────────────────────────────────────────────────────────

/** One header chip, and what clicking it filters by (Piece 7). */
export type CardChip = {
  /** The words on the chip — a payload string or a stored word, never composed. */
  text: string;
  /** Which facet a click narrows the list to. `null` = not a filter, just a label. */
  filter: ChipFilter | null;
  /** Bad news gets a different treatment; the words still say what is wrong. */
  warning?: boolean;
  /** A provenance tag riding beside the chip before it ("extracted"). */
  tag?: string;
};

/** What a clickable chip filters by. The value is the RAW payload string. */
export type ChipFilter = { kind: "speaker" | "statement_kind"; value: string };

/** The question, folded to one line with the rest one click away (Piece 2a). */
export type CardQuestionView = {
  /** What shows before a click — the full text when it already fits. */
  short: string;
  /** The whole question. Equal to `short` when nothing was folded. */
  full: string;
  /** Whether there is anything behind the fold. Drives the control's presence. */
  truncated: boolean;
  /** Who transcribed it, and the icon class the wrapper draws. */
  authorship: { label: string; human: boolean } | null;
};

/** The answer: lead text with the anchor marked inside it (Piece 2a). */
export type CardAnswerView = {
  /** Text before the anchor. Empty when the anchor starts the quote. */
  before: string;
  /** The anchor itself — verbatim, never groomed (§12). */
  anchor: string;
  /** Text after the anchor. */
  after: string;
};

/** One allegation on one line, with its element chips compressed (Piece 2d). */
export type CardBearsOnView = {
  /** "A-45 — misrepresented the cause of the $30,000 costs", pre-composed. */
  accusation: string;
  /** The count chip ("Count 1 — Breach of Fiduciary Duty"), or `null`. */
  count: string | null;
  /** The first K element chips — always on screen. */
  visibleElements: string[];
  /** The rest. Non-empty means the "+N more" control renders. */
  hiddenElements: string[];
  /** "+3 more", pre-filled from the stored template. `null` when nothing folds. */
  moreLabel: string | null;
  /** Whether a HUMAN made this link, rather than the extraction. */
  human: boolean;
};

/** The surrounding source text, behind one click (Piece 2f). */
export type CardContextView = {
  beforeNotice: string | null;
  before: string;
  after: string;
  afterNotice: string | null;
};

/** One evidence card, identical under both wrappers. */
export type EvidenceCardView = {
  /** "C-14", or `null` for a candidate nothing has numbered yet. */
  code: string | null;
  graphNodeId: string;
  /** Speaker, kind, band — the header row, in display order. */
  chips: CardChip[];
  /** `null` on a statement that is not a Q&A pair (transcript speech, letters). */
  question: CardQuestionView | null;
  answer: CardAnswerView;
  pinpointLabel: string;
  pinpointHref: string;
  bearsOn: CardBearsOnView[];
  /** The scan's own sentence, and the stored word that attributes it. */
  scanReason: { label: string; text: string } | null;
  /** `null` when the payload carries no surrounding text at all. */
  context: CardContextView | null;
};

// ─── Building it ────────────────────────────────────────────────────────────

/** The two stored thresholds this view folds by (§2b). */
export type CardLimits = {
  /** Characters of question shown before the fold. */
  questionChars: number;
  /** Element chips shown before "+N more". */
  elementK: number;
};

/**
 * Fold a question to one line, keeping the whole of it for the unfold.
 *
 * ## Why the cut is at a word boundary and the ellipsis is a real character
 *
 * Cutting mid-word produces "conversations or conta…", which reads as a typo
 * before it reads as a truncation. Walking back to the last space costs one
 * `lastIndexOf` and makes the fold look deliberate. `…` rather than "..." so the
 * three dots cannot be mistaken for the source document's own elision — legal
 * quotations use "..." to mean text was omitted from the RECORD, and this is a
 * display fold that omits nothing.
 */
function foldQuestion(text: string, limit: number): { short: string; truncated: boolean } {
  const trimmed = text.trim();
  if (trimmed.length <= limit) return { short: trimmed, truncated: false };

  const cut = trimmed.slice(0, limit);
  const lastSpace = cut.lastIndexOf(" ");
  // A limit that lands inside the very first word leaves no space to back up to;
  // the hard cut is then the honest one rather than an empty line.
  const body = lastSpace > 0 ? cut.slice(0, lastSpace) : cut;
  return { short: `${body}…`, truncated: true };
}

/**
 * Split the quote around its anchor.
 *
 * ## Domain note: the anchor is the WHOLE quote today, and this is not redundant
 *
 * `CardQuote.text` IS the anchor — the verbatim words §12 makes sacred — and the
 * flanks are `context_before`/`context_after`, which belong behind the "Show
 * context" fold rather than in the answer. So `before` and `after` are empty in
 * every case this build produces, and the shape exists because Piece 6 (the
 * marked line, riding .388) puts a SECOND highlight inside this same block. When
 * it lands, the wrapper already has a structure to render two marks into.
 *
 * Written as a function rather than inlined so that arrival is one edit here and
 * none in the JSX.
 */
function splitAnchor(quoteText: string): CardAnswerView {
  return { before: "", anchor: quoteText, after: "" };
}

/**
 * Compress one allegation's element chips to the first K, keeping the rest.
 *
 * Roman's ruling: element chips COMPRESS, never vanish — they are the quick
 * "what harm was done" indicator, and dropping them is what left the fact card
 * unable to say anything about damages.
 */
function compressElements(
  elements: string[],
  limit: number,
  wording: CardGrammarWording,
): Pick<CardBearsOnView, "visibleElements" | "hiddenElements" | "moreLabel"> {
  const visibleElements = elements.slice(0, limit);
  const hiddenElements = elements.slice(limit);
  return {
    visibleElements,
    hiddenElements,
    moreLabel:
      hiddenElements.length > 0
        ? fillSlots(wording.elements_more_template, {
            count: String(hiddenElements.length),
          })
        : null,
  };
}

/** The machine's allegations, then the human's — ordered, never merged. */
function bearsOnViews(
  machine: CardBearsOn[],
  human: CardHumanLink[],
  limit: number,
  wording: CardGrammarWording,
): CardBearsOnView[] {
  const fromMachine = machine.map((b) => ({
    accusation: b.accusation,
    count: b.count,
    human: false,
    ...compressElements(b.elements, limit, wording),
  }));

  // A human's link is a different CLAIM about the world from the extraction's
  // edge — the payload keeps the two lists apart for exactly that reason — so
  // this orders them rather than merging them, and carries the flag that lets
  // the card say which is which. A human link has no elements and no count: the
  // person supplied the accusation, not a reading of the complaint's structure.
  const fromHuman = human.map((l) => ({
    accusation: l.label,
    count: null,
    human: true,
    visibleElements: [],
    hiddenElements: [],
    moreLabel: null,
  }));

  return [...fromMachine, ...fromHuman];
}

/**
 * The header chips, in display order: WHO, then WHAT KIND, then the band.
 *
 * ## Why the speaker leads (ruling R2)
 *
 * The card has to answer "who said this" before it says anything else. It did
 * not: the question led, with the transcription badge under it, and Roman read
 * that badge as the speaker. Putting the real speaker first is half the fix and
 * folding the question is the other half.
 *
 * ## Why an ABSENT speaker still renders a chip
 *
 * Documentary evidence genuinely has no speaker, and silence there is
 * indistinguishable from a card whose speaker chip failed to render. The stored
 * "speaker not extracted" says which — never a guessed name (§2b's absent-not-
 * fake law).
 */
function headerChips(card: ScenarioCard, wording: CardGrammarWording): CardChip[] {
  const chips: CardChip[] = [];

  if (card.speaker.name) {
    chips.push({
      text: card.speaker.name,
      // The RAW extracted string is the filter value, and the tag says so.
      // Canonical person filtering waits for B0 — one man, not two strings — and
      // pretending otherwise here would silently under-count a witness.
      filter: { kind: "speaker", value: card.speaker.name },
      tag: wording.speaker_extracted_label,
    });
  } else {
    chips.push({ text: wording.speaker_absent_label, filter: null });
  }

  if (card.statement_kind) {
    chips.push({
      text: card.statement_kind,
      filter: { kind: "statement_kind", value: card.statement_kind },
    });
  }

  // The scan's proposed role, still attributed to the scan ("Scan: supports").
  if (card.proposed?.role_label) {
    chips.push({ text: card.proposed.role_label, filter: null });
  }
  if (card.proposed?.duplicate_label) {
    chips.push({ text: card.proposed.duplicate_label, filter: null });
  }
  // §7.8: banded words, never a naked percentage. "Unscored" is the ABSENCE of
  // information taking up space, so it earns no chip.
  if (card.confidence.band !== "unscored") {
    chips.push({ text: card.confidence.label, filter: null });
  }
  // §7.7 makes grounding a WARNING — it speaks only when the quote could not be
  // located. A chip announcing that it WAS found spends a line saying nothing.
  if (card.grounding && GROUNDING_WARNING_STATES.includes(card.grounding.state)) {
    chips.push({ text: card.grounding.label, filter: null, warning: true });
  }

  return chips;
}

/**
 * Grounding states that are NOT good news, and so earn a chip.
 *
 * `pending` sits with `not_found` because "not yet checked" is not good news
 * either — it is the absence of verification, which is precisely what a human
 * about to rule needs told.
 */
export const GROUNDING_WARNING_STATES = ["not_found", "pending"];

/**
 * ONE evidence card, from ONE payload, for BOTH wrappers.
 *
 * This is the function the whole task turns on. `CandidateCard` and `FactRow`
 * both call it on the same `ScenarioCard`, so the two surfaces cannot show
 * different fields — there is nothing to keep in step.
 *
 * @param card    the payload, identical whichever list it came from
 * @param wording the stored words (ruling R6); the caller withholds the card
 *                entirely until they load, so this is never optional here
 * @param limits  the two stored fold thresholds (§2b)
 */
export function evidenceCardView(
  card: ScenarioCard,
  wording: CardGrammarWording,
  limits: CardLimits,
): EvidenceCardView {
  const questionText = card.quote.question?.trim();
  const authorship = card.quote.question_authorship;

  const hasContext =
    card.quote.context_before.length > 0 ||
    card.quote.context_after.length > 0 ||
    card.quote.context_before_notice !== null ||
    card.quote.context_after_notice !== null;

  return {
    code: card.code,
    graphNodeId: card.graph_node_id,
    chips: headerChips(card, wording),
    question: questionText
      ? {
          ...foldQuestion(questionText, limits.questionChars),
          full: questionText,
          authorship: authorship
            ? {
                // A human's badge is composed server-side and names them; the
                // machine's is the stored sentence saying the text was
                // transcribed from the document. Neither is ever a speaker (R2).
                label:
                  authorship.source === "human"
                    ? authorship.label
                    : wording.question_machine_authorship_label,
                human: authorship.source === "human",
              }
            : null,
        }
      : null,
    answer: splitAnchor(card.quote.text),
    pinpointLabel: card.pinpoint.label,
    pinpointHref: card.pinpoint.viewer_href,
    bearsOn: bearsOnViews(card.bears_on, card.human_links, limits.elementK, wording),
    // Ruling R3: present on a ruled card exactly as on a proposed one. The
    // backend recovers it from the run the ruling cites; this only renders it.
    scanReason: card.scan_reason
      ? { label: wording.scan_reason_label, text: card.scan_reason }
      : null,
    context: hasContext
      ? {
          beforeNotice: card.quote.context_before_notice,
          before: card.quote.context_before,
          after: card.quote.context_after,
          afterNotice: card.quote.context_after_notice,
        }
      : null,
  };
}

/**
 * Whether a card matches an active chip filter (Piece 7).
 *
 * Matched on the RAW payload value rather than on the chip's displayed text,
 * because those differ the moment a chip carries a provenance tag — and because
 * a filter that compared prose would break the day the wording changed, which is
 * the same argument `candidateState` makes for reading `status` over
 * `status_label`.
 */
export function matchesChip(card: ScenarioCard, filter: ChipFilter | null): boolean {
  if (!filter) return true;
  if (filter.kind === "speaker") return card.speaker.name === filter.value;
  return card.statement_kind === filter.value;
}
