// =============================================================================
// CandidateCard — the RULING WRAPPER around one §7 candidate (1.7C, then R8)
// =============================================================================
//
// The Casey card layout over the §7 payload: C-code · rulings · state chip, then
// `EvidenceCardBody` — the SHARED body, identical to the one the fact wrapper
// renders three inches lower.
//
// ## Why the body left this file (ONE_CARD_GRAMMAR, ruling R8)
//
// This file was 342 non-comment lines against Rule 17's 300, and the one-card
// grammar needs the same body under a second wrapper. One body, two wrappers —
// so the body had to stop being this component's private JSX. The extraction
// shipped move-only first; this file now mounts the shared component over the
// shared `evidenceCardView`, which is what makes "a candidate and a fact are the
// same item" true of the code and not only of the design.
//
// What is left here is the half that is genuinely about RULING: the head with its
// four buttons, the defer prompt, the receipt of the last ruling, and the compact
// collapse. `RulingButtons` made the same cut from the other side in 1.7D.
//
// ## Two shapes, one card (task 1.7E, item 2)
//
// A card the human has already dealt with renders COMPACT — a summary row of
// C-code, the quote, and its state chip — because 148 full cards in a scroll
// region is a wall to scroll past to reach the twenty that need a decision.
// Selecting a compact card expands it: the way back from a ruling is U, and U
// needs the card it applies to on screen with its buttons.
//
// ## The card body no longer carries the not-linked paragraph (item 8)
//
// It lives on the button row, beside the two controls it disables (see
// `RulingButtons.CardHead`). Five stacked metadata lines became one chip row in
// the same ruling, and the quote in context took the space back — §7.1 says the
// quote is the card's first element, and now the layout says so too.
//
// ## Visual language (§2c, binding)
//
// Pure white surfaces, no card borders, regular weight with bold reserved for the
// pinpoint page, one accent, generous line height.

import React, { useMemo, useState } from "react";

import {
  cardStyle,
  chipStyle,
  compactCardStyle,
  compactQuoteStyle,
  selectedCardStyle,
} from "./candidateCardStyles";
// Re-exported because `CardQueue` and the facts section import them from here —
// the move is a file split, not an API change.
export { cardStyle, chipStyle };

import EvidenceCardBody from "./EvidenceCardBody";
import AllegationTypeahead from "./AllegationTypeahead";
import { HumanLinkSection } from "./HumanLinkSection";
import { needsLinking } from "./cardLinking";
import { evidenceCardView, type ChipFilter } from "./evidenceCardModel";
import { candidateState, stateChip } from "./candidateFilters";
import { DEFER_QUICK_REASONS, type RulingKey } from "./cardTriage";
import { CardHead, StateChipView, codeBadgeStyle } from "./RulingButtons";
import type { RulingReceipt } from "./rulingAcknowledgment";
import type { ScenarioCard } from "../services/scenarioCards";
import type { AllegationOptions, LinkCut } from "../services/evidenceLinks";

/** The receipt on a card: quiet for a landing, loud for a refusal. */
const cardReceiptStyle: React.CSSProperties = {
  border: "1px solid var(--border-default)",
  borderRadius: "8px",
  padding: "0.5rem 0.7rem",
  marginBottom: "10px",
  fontSize: "0.82rem",
  color: "var(--text-secondary)",
  background: "var(--v3-chrome)",
};

const cardReceiptFailedStyle: React.CSSProperties = {
  ...cardReceiptStyle,
  borderColor: "var(--state-danger-strong)",
  color: "var(--state-danger-strong)",
  background: "var(--bg-surface)",
};

/**
 * The defer reason, collected ON the card (architect ruling R1, 2026-08-08).
 *
 * ## Why it moved here
 *
 * It rendered at the bottom of the queue, after a `maxHeight: 70vh` scroll
 * window — so pressing Defer on a card near the top of that window opened a
 * prompt the human could be a full viewport away from. §7's contract is that a
 * card is rulable from the card alone; the one ruling that needs a word from the
 * human was collecting it somewhere else entirely.
 *
 * Behaviour is unchanged and still lives in the reducer: quick picks, free text,
 * Enter commits, Esc cancels. This is placement, not a new flow.
 */
const DeferReasonForm: React.FC<{
  draft: string;
  inputRef?: React.RefObject<HTMLInputElement>;
  onDraft: (draft: string) => void;
}> = ({ draft, inputRef, onDraft }) => (
  <div
    style={{
      border: "1px solid var(--state-warning-strong)",
      borderRadius: "8px",
      padding: "10px 12px",
      marginBottom: "12px",
      display: "flex",
      flexDirection: "column",
      gap: "0.5rem",
      background: "var(--state-warning-bg-soft)",
    }}
  >
    <div style={{ fontSize: "0.8rem", color: "var(--v3-amber-text)" }}>
      Why defer this? Enter commits · Esc cancels
    </div>
    <div style={{ display: "flex", gap: "0.4rem", flexWrap: "wrap" }}>
      {DEFER_QUICK_REASONS.map((reason, i) => (
        <button
          key={reason}
          type="button"
          // The card's own click selects it; a quick pick is not a selection.
          onClick={(event) => {
            event.stopPropagation();
            onDraft(reason);
          }}
          style={{ ...chipStyle, cursor: "pointer", background: "var(--bg-surface)" }}
        >
          {i + 1}. {reason}
        </button>
      ))}
    </div>
    <input
      ref={inputRef}
      value={draft}
      onChange={(e) => onDraft(e.target.value)}
      onClick={(e) => e.stopPropagation()}
      placeholder="or type a reason"
      style={{
        border: "1px solid var(--border-default)",
        borderRadius: "6px",
        padding: "0.4rem 0.6rem",
        fontWeight: 400,
        fontFamily: "inherit",
      }}
    />
  </div>
);

/**
 * One candidate card.
 *
 * ## `onRule` is required, and it is THIS card's (1.7G, ruling R1)
 *
 * It used to be optional, because the ruling buttons belonged to the selected card
 * alone — the argument being that live buttons everywhere would let a human rule
 * one card while the keyboard was aimed at another. That argument was answered the
 * wrong way round: the two input paths do not disagree about which card "I" means,
 * because they no longer share a target. The keyboard rules the card it is aimed
 * at (the selected one) and a button rules the card it is printed on, and the
 * caller binds this callback to that card's id. So every card carries its own
 * controls and none of them can reach another card.
 */
export const CandidateCard: React.FC<{
  card: ScenarioCard;
  /** The card the keyboard is aimed at. Raised and expanded. */
  selected: boolean;
  /** Collapse to the summary row. The list passes `ruled && !selected`. */
  compact: boolean;
  onSelect: () => void;
  /** Already bound to THIS card by the list — see the note above. */
  onRule: (key: RulingKey) => void;
  /** True when I or E was just refused on THIS card (the reducer's `notice`). */
  keyboardRefused?: boolean;
  /** Save a human's correction of THIS card's question (task 1.7F Part B).
   *  Already bound to this card's id by the list, exactly as `onRule` is. */
  onCorrectQuestion: (text: string) => Promise<void>;
  /** Restore the machine's own question by deleting the correction. */
  onRevertQuestion: () => Promise<void>;
  /** The accusations THIS card's link panel offers, and its words (task 2.10). */
  linkOptions: AllegationOptions | null;
  /** Save this card's links. Bound by the list, exactly as `onRule` is. */
  onSaveLinks: (allegationIds: string[], cut: LinkCut) => Promise<void>;
  /** Take one of this card's links back. */
  onUnlink: (allegationId: string) => void;
  /**
   * "Proposed by the Aug 7 scan" — composed by the list from the stored template
   * and the run's date, or `null` when nothing proposed this card (2026-08-08).
   *
   * Passed DOWN rather than composed here: every proposed card in one payload
   * comes from the same run (R-b), so the sentence is built once by the list and
   * not thirty times by thirty cards that could each format the date differently.
   */
  proposedAttribution?: string | null;
  /**
   * What the last ruling did, when it was about THIS card (2026-08-08).
   *
   * The acknowledgment renders where the human's eye already is. `null` on every
   * other card — the list gives it to the one card it names.
   */
  receipt?: RulingReceipt | null;
  /**
   * The open defer prompt, when it is THIS card's (architect ruling R1).
   *
   * The reason input renders under the action row, on the card being deferred.
   * It used to render at the bottom of the queue below a 70vh scroll window,
   * where it could open entirely outside the human's view.
   */
  deferring?: { graphNodeId: string; draft: string } | null;
  deferInputRef?: React.RefObject<HTMLInputElement>;
  onDeferDraft?: (draft: string) => void;
  /**
   * Narrow the queue to a chip's value (Piece 7).
   *
   * Optional: a list with no filter of its own passes nothing, and the chips
   * then render as labels. A chip that looked clickable and did nothing would be
   * worse than one that never claimed to be.
   */
  onFilterChip?: (filter: ChipFilter) => void;
}> = ({
  card,
  selected,
  compact,
  onSelect,
  onRule,
  keyboardRefused = false,
  // RECEIVED AND DELIBERATELY NOT USED — do not "clean these up" (task R1).
  //
  // `QuestionLine` (task 1.7F Part B) is a complete, tested component that edits
  // a card's question, and NOTHING RENDERS IT: the only reference to it outside
  // its own file is a structure test. These two handlers are wired all the way
  // from `CardQueue` through `CandidateList` to here and then stop, which is the
  // same dead-wire shape as `ThemeScanPanel.onFactsChanged` — the defect .390
  // exists to fix, one feature over.
  //
  // Deleting them would silently retire a built feature, which is not a call a
  // dead-binding sweep gets to make. Renamed with the `_` the compiler ignores so
  // the contract stays visible and the finding stays reportable. Filed for the
  // architect; NOT fixed here, because wiring an unreachable editor into a
  // witness-facing card is a design decision, not a lint.
  onCorrectQuestion: _onCorrectQuestion,
  onRevertQuestion: _onRevertQuestion,
  linkOptions,
  onSaveLinks,
  onUnlink,
  proposedAttribution = null,
  receipt = null,
  deferring = null,
  deferInputRef,
  onDeferDraft,
  onFilterChip,
}) => {
  // ONE view, from the shared builder. `null` until the stored words load —
  // there is deliberately no fallback vocabulary to render a card with (R4).
  const view = useMemo(
    () =>
      linkOptions
        ? evidenceCardView(card, linkOptions.card_grammar, {
            questionChars: linkOptions.card_question_truncate_chars,
            elementK: linkOptions.card_element_chips_visible_k,
          })
        : null,
    [card, linkOptions],
  );
  const chip = stateChip(candidateState(card));

  // Task 2.10: the control appears on a card the extraction never linked and
  // nobody has linked since. A card that HAS been linked shows its chips.
  const linkPanel =
    linkOptions !== null && needsLinking(card) && card.human_links.length === 0;

  // Item B: while the panel holds ticks or a cut that have not been saved, the
  // greyed Include and Exclude say why. Roman filled a panel in, saw Include
  // still greyed, and reported it as broken — it was correct behaviour with no
  // observable, which is the same defect class as 1.7C's silent keypress.
  const [linkDraftDirty, setLinkDraftDirty] = useState(false);
  /**
   * Said when a link has landed on THIS card (Piece 4b).
   *
   * Held on the card, not in the type-ahead, because the type-ahead is about to
   * unmount: the link clears `defer_required`, the queue re-reads, and the panel
   * stops being offered. A sentence that vanished with the control that earned
   * it is the silent-defer defect in a new costume.
   */
  const [linkNotice, setLinkNotice] = useState<string | null>(null);

  if (compact) {
    return (
      <div style={compactCardStyle} onClick={onSelect}>
        <span style={codeBadgeStyle}>{card.code ?? "—"}</span>
        <span style={compactQuoteStyle} title={card.quote.text}>
          {card.quote.text}
        </span>
        <StateChipView chip={chip} title={card.defer_reason ?? undefined} />
      </div>
    );
  }

  return (
    <div
      style={selected ? selectedCardStyle : cardStyle}
      // A click anywhere on a card aims the keyboard at it.
      //
      // It does NOT rule anything: the ruling buttons stop their own clicks from
      // reaching here (see `RulingButtons`), so pressing Include is not also a
      // request to aim the keyboard at that card.
      //
      // Deliberately NOT `role="button"` and not focusable: the card already
      // contains a link and four buttons, and interactive roles do not nest —
      // and a focusable card would put a UA focus ring (the OS accent colour,
      // orange on this machine) around a surface whose whole visual language says
      // selection is ELEVATION. The accessible path is not tabbing through 148
      // cards; it is ↑/↓/j/k, which the list handles and the hint bar announces.
      onClick={onSelect}
    >
      <CardHead
        code={card.code ?? "—"}
        chip={chip}
        chipTitle={card.defer_reason ?? undefined}
        deferOnly={card.defer_required}
        deferOnlyReason={card.defer_required_reason}
        keyboardRefused={keyboardRefused}
        // Q4: the panel below carries the explanation, so the button row does not
        // print it a second time three inches away. A card with no panel — one
        // with no quote — keeps its sentence exactly where 1.7E-a put it.
        // D3a: the standing condition is stated on the card's face, from the
        // stored label. `null` until the wording loads, which renders nothing
        // rather than a compiled-in sentence.
        lockedConditionLabel={linkOptions?.wording.card_locked_condition_label ?? null}
        unsavedLinkReason={
          linkDraftDirty && linkOptions ? linkOptions.wording.save_blocks_ruling : null
        }
        onRule={onRule}
      />

      {/* R1: the reason input, on the card, directly under the action row. A
          human who presses Defer types their reason where they are looking —
          §7's "rulable from the card alone", applied to the one ruling that
          needs a word from them. */}
      {deferring && onDeferDraft && (
        <DeferReasonForm
          draft={deferring.draft}
          inputRef={deferInputRef}
          onDraft={onDeferDraft}
        />
      )}

      {/* Piece 4b: linking a locked card WAKES its ruling buttons, and says so.
          A control that silently becomes usable is one the human has to notice
          for themselves — and this one changes what the card is FOR. */}
      {linkNotice && (
        <div role="status" style={cardReceiptStyle}>
          {linkNotice}
        </div>
      )}

      {/* What the last ruling on THIS card did — landed or refused. Every ruling
          leaves one: on beta.385 a defer wrote an anchor, a reference row and its
          provenance, and said nothing at all, so it was reported as a dead
          button. */}
      {receipt && (
        <div
          role={receipt.failed ? "alert" : "status"}
          style={receipt.failed ? cardReceiptFailedStyle : cardReceiptStyle}
        >
          {receipt.text}
        </div>
      )}

      {/* THE SHARED BODY. The fact wrapper mounts this same component over a view
          built by the same function from the same payload, so the two cards
          cannot show different fields. */}
      {view && linkOptions && (
        <EvidenceCardBody
          view={view}
          wording={linkOptions.card_grammar}
          onFilterChip={onFilterChip}
        >
          {/* The wrapper's own extras, between the chips and the exchange.

              A locked card states its condition on its FACE and offers the
              type-ahead right there (Piece 4b) — the 120-checkbox wall it
              replaces was a scroll region on a card that could not be ruled
              until something in it was ticked. */}
          {linkPanel && (
            <div
              style={{
                border: "1px solid var(--state-warning-strong)",
                background: "var(--state-warning-bg-soft)",
                borderRadius: "8px",
                padding: "10px 12px",
                display: "flex",
                flexDirection: "column",
                gap: "0.5rem",
              }}
            >
              <div style={{ fontSize: "12.5px", color: "var(--v3-amber-text)", lineHeight: 1.5 }}>
                {linkOptions.card_grammar.link_typeahead_intro}
              </div>
              <AllegationTypeahead
                options={linkOptions}
                onSave={onSaveLinks}
                onDraftDirty={setLinkDraftDirty}
                onLinked={() =>
                  setLinkNotice(
                    linkOptions.card_grammar.link_woke_ruling_template.replace(
                      "{code}",
                      card.code ?? "",
                    ),
                  )
                }
              />
            </div>
          )}

          {/* §7.5, the human's half: what a person said this bears on, and one
              click to take each back. A card that HAS been linked shows its
              chips instead of the control — the work is done, and re-offering
              it would read as though it had not been. */}
          <HumanLinkSection
            summary={card.human_link_summary}
            links={card.human_links}
            options={linkOptions}
            onUnlink={onUnlink}
          />
        </EvidenceCardBody>
      )}

      {/* "Proposed by the Aug 7 scan" — composed by the list from the stored
          template and the run's date. It attributes the reason ABOVE it, so it
          renders under the body rather than inside it. */}
      {card.proposed && proposedAttribution && (
        <div style={{ fontSize: "12.5px", color: "var(--text-muted)" }}>
          {proposedAttribution}
        </div>
      )}

      {/* A reason a HUMAN gave, distinct from the system's defer notice above. */}
      {card.defer_reason && (
        <div style={{ fontSize: "13px", color: "var(--text-secondary)", fontStyle: "italic" }}>
          Parked: {card.defer_reason}
        </div>
      )}
    </div>
  );
};
