// =============================================================================
// CandidateCard — one §7 candidate, rendered (extracted from CardQueue, 1.7C)
// =============================================================================
//
// The Casey card layout over the §7 payload: C-code · rulings · state chip · the
// quote in its context · pinpoint · stance · bears-on · one row of metadata chips.
//
// ## Why this is its own file
//
// `CardQueue` was 380 non-comment lines before task 1.7C and every §2.3 addition
// made it longer, so the file that already broke the 300-line limit was about to
// break it harder. The seam is real rather than convenient: this file is THE CARD
// (what one candidate looks like) and `CardQueue` is THE QUEUE (which card is
// selected, what a key does, what a ruling posts). Rule 17 forced the split; the
// split is the one a reader would have drawn anyway.
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
// ## This component renders and does nothing else
//
// Every string on screen comes from the 1.2 payload, and which rows a card shows
// is decided by the pure `cardRows`. This file is the JSX that walks the
// descriptor. It chooses no words — a `switch` composing prose here would be the
// frontend inventing vocabulary, which the language law forbids. The one class of
// word it owns is the name of its own controls (Include, Defer, "Not ruled"),
// which is the same class the ruling buttons have always owned.
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
  contextPanelStyle,
  contextStyle,
  edgeStyle,
  markStyle,
  metaChipStyle,
  warningChipStyle,
  selectedCardStyle,
} from "./candidateCardStyles";
// Re-exported because `CardQueue` and the facts section import them from here —
// the move is a file split, not an API change.
export { cardStyle, chipStyle };

import { cardRows, metaChips, type CardRow } from "./cardRows";
import { needsLinking } from "./cardLinking";
import LinkToAccusationPanel from "./LinkToAccusationPanel";
import { HumanLinkSection } from "./HumanLinkSection";
import { candidateState, stateChip } from "./candidateFilters";
import type { RulingKey } from "./cardTriage";
import QuestionLine from "./QuestionLine";
import { CardHead, StateChipView, codeBadgeStyle } from "./RulingButtons";
import { openViewerWindow } from "./viewerWindow";
import type { ScenarioCard } from "../services/scenarioCards";
import type { AllegationOptions, LinkCut } from "../services/evidenceLinks";

// ─── Row rendering ──────────────────────────────────────────────────────────

/**
 * The pinpoint row — the only interactive row, and so the only one with state.
 *
 * `href` is optional on the descriptor. It is always present in practice (the
 * backend composes `viewer_href` for every card, page or no page), but a chip with
 * nothing to open must not pretend to be a link — it renders as the plain pinpoint
 * text instead of a dead anchor.
 */
const PinpointRow: React.FC<{ row: CardRow }> = ({ row }) => {
  // A refused popup has to be SAID, not swallowed (Standing Rule 1). Local to the
  // chip so the message appears where the human just clicked.
  const [blocked, setBlocked] = useState<string | null>(null);

  if (!row.href) {
    return (
      <div>
        <span style={{ ...chipStyle, color: "var(--text-primary)" }}>{row.value}</span>
      </div>
    );
  }
  const href = row.href;
  return (
    <div>
      {/* An anchor with an href, driven by onClick — so it still looks and
          middle-clicks like a link, and a human who wants a tab can have one,
          while a plain click gets the sized viewer WINDOW (D5).
          `preventDefault` stops the browser also navigating this page away. */}
      <a
        href={href}
        onClick={(event) => {
          event.preventDefault();
          // …and `stopPropagation` stops the click ALSO selecting the card behind
          // it: reading a page and aiming the keyboard are two different intents.
          event.stopPropagation();
          const result = openViewerWindow(href);
          setBlocked(result.opened ? null : result.message);
        }}
        style={{ ...chipStyle, color: "var(--accent-primary)", textDecoration: "none" }}
      >
        {row.value} ↗
      </a>
      {blocked && (
        <div
          role="alert"
          style={{ marginTop: "0.3rem", fontSize: "0.78rem", color: "var(--state-danger-strong)" }}
        >
          {blocked}
        </div>
      )}
    </div>
  );
};

/** A prose row with its chips — the stance line and each bears-on line. */
const TextRow: React.FC<{ row: CardRow }> = ({ row }) => (
  <div style={{ display: "flex", gap: "0.4rem", alignItems: "center", flexWrap: "wrap" }}>
    <span style={{ fontSize: "13px", color: "var(--text-secondary)" }}>{row.value}</span>
    {row.chips?.map((chip) => (
      <span
        key={chip}
        style={{
          ...chipStyle,
          background: "var(--v3-chip-alleg-bg)",
          color: "var(--v3-chip-alleg-text)",
        }}
      >
        {chip}
      </span>
    ))}
  </div>
);

/**
 * The §7.3 / §7.7 / §7.8 metadata as ONE row of chips (item 8).
 *
 * Roman's ruling, 2026-08-03: the five stacked lines are dead. They cost the card
 * five vertical inches to say four short things, and the quote — the thing being
 * ruled on — was pushed down the card by its own metadata.
 */
const MetaRow: React.FC<{ card: ScenarioCard }> = ({ card }) => {
  // Which chips exist is decided by the pure `metaChips` (task 2.12, item C) —
  // this walks the list, exactly as the card walks `cardRows`. Every entry is a
  // payload string, rendered verbatim and carrying itself as its own tooltip so
  // nothing is lost to the truncation.
  const chips = metaChips(card);

  // Every chip can be absent at once (an unscored, grounded, speakerless
  // documentary item), and an empty flex row would still take its parent's gap.
  if (chips.length === 0) return null;

  return (
    <div style={{ display: "flex", gap: "6px", flexWrap: "wrap", alignItems: "center" }}>
      {chips.map((chip, i) => (
        <span
          key={`${chip.text}-${i}`}
          style={chip.warning ? { ...metaChipStyle, ...warningChipStyle } : metaChipStyle}
          title={chip.text}
        >
          {chip.text}
        </span>
      ))}
    </div>
  );
};

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
  /**
   * The accusations THIS card's link panel offers, and its words (task 2.10).
   *
   * `null` while the scenario's options are loading, or if that read failed. The
   * panel is then not rendered at all — there is deliberately no fallback set of
   * words to render it with (R4), and a control with no labels would be worse
   * than the dead end it replaces.
   */
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
}> = ({
  card,
  selected,
  compact,
  onSelect,
  onRule,
  keyboardRefused = false,
  onCorrectQuestion,
  onRevertQuestion,
  linkOptions,
  onSaveLinks,
  onUnlink,
  proposedAttribution = null,
}) => {
  const rows = useMemo(() => cardRows(card), [card]);
  const code = rows.find((r) => r.element === "code");
  const pinpoint = rows.find((r) => r.element === "pinpoint");
  const bearsOn = rows.filter((r) => r.element === "bears_on");
  // The stance row renders ONLY when the card has a real stance. When it has none,
  // `cardRows` fills the §7.5 slot with the defer reason (the contract test asserts
  // exactly one of the two is present) — and that sentence now belongs to the
  // button row rather than the card body (item 8).
  const stance = card.stance ? rows.find((r) => r.element === "stance") : undefined;
  const chip = stateChip(candidateState(card));

  // Task 2.10: the panel appears on a card the extraction never linked and nobody
  // has linked since — and only when its words have loaded. A card that HAS been
  // linked shows its chips instead: the work is done, and re-offering the control
  // beneath the answer would read as though it had not been.
  const linkPanel = linkOptions !== null && needsLinking(card) && card.human_links.length === 0;

  // Item B: while the panel holds ticks or a cut that have not been saved, the
  // greyed Include and Exclude say why. Roman filled a panel in, saw Include
  // still greyed, and reported it as broken — it was correct behaviour with no
  // observable, which is the same defect class as 1.7C's silent keypress.
  const [linkDraftDirty, setLinkDraftDirty] = useState(false);

  if (compact) {
    return (
      <div style={compactCardStyle} onClick={onSelect}>
        <span style={codeBadgeStyle}>{code?.value ?? "—"}</span>
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
        code={code?.value ?? "—"}
        chip={chip}
        chipTitle={card.defer_reason ?? undefined}
        deferOnly={card.defer_required}
        deferOnlyReason={card.defer_required_reason}
        keyboardRefused={keyboardRefused}
        // Q4: the panel below carries the explanation, so the button row does not
        // print it a second time three inches away. A card with no panel — one
        // with no quote — keeps its sentence exactly where 1.7E-a put it.
        reasonShownElsewhere={linkPanel}
        unsavedLinkReason={
          linkDraftDirty && linkOptions ? linkOptions.wording.save_blocks_ruling : null
        }
        onRule={onRule}
      />

      {/* §7.1 — the question that makes a bare answer interpretable, then the
          quote in its surrounding text.

          Task 1.7C (defect D6): the context is now sentence-complete at both ends
          and its transcript gutter numerals are stripped, both server-side. Where a
          flank ran out of PAGE instead of reaching a sentence boundary, the payload
          carries a composed notice and it is rendered — a card must not imply a
          completeness the data does not have (Standing Rule 1). Measured on DEV,
          that is 154 of 499 cards on the left and 65 on the right.

          The `<mark>` is the anchor: verbatim, highlighted, never groomed (§12). */}
      {card.quote.question && (
        <QuestionLine
          question={card.quote.question}
          authorship={card.quote.question_authorship}
          onSave={(text) => onCorrectQuestion(text)}
          onRevert={onRevertQuestion}
          revertLabel={linkOptions?.wording.question_revert_label}
        />
      )}

      {/* The context panel (mockup `.ctx`) — the source page's own words on their
          own surface, with the anchor marked inside them. */}
      <div style={contextPanelStyle}>
        {card.quote.context_before_notice && (
          <>
            <span style={edgeStyle}>{card.quote.context_before_notice}</span>
            <br />
          </>
        )}
        {card.quote.context_before && <span>{card.quote.context_before} </span>}
        <mark style={markStyle}>{card.quote.text}</mark>
        {card.quote.context_after && <span> {card.quote.context_after}</span>}
        {card.quote.context_after_notice && (
          <>
            <br />
            <span style={edgeStyle}>{card.quote.context_after_notice}</span>
          </>
        )}
      </div>

      {pinpoint && <PinpointRow row={pinpoint} />}
      {stance && <TextRow row={stance} />}

      {/* §7.5, the human's half: what a person said this bears on, and one click
          to take each back. Rendered where the machine's stance would be, because
          it answers the same question — differently sourced, and the 👤 glyph
          beside every chip says so. */}
      <HumanLinkSection
        summary={card.human_link_summary}
        links={card.human_links}
        options={linkOptions}
        onUnlink={onUnlink}
      />

      {linkPanel && linkOptions && (
        <LinkToAccusationPanel
          options={linkOptions}
          onSave={onSaveLinks}
          onDraftDirty={setLinkDraftDirty}
        />
      )}
      {bearsOn.map((row, i) => (
        <TextRow key={`bears_on-${i}`} row={row} />
      ))}

      <MetaRow card={card} />

      {/* What the SCAN said, in its own words and marked as its own (2026-08-08).
          The judge's reason is the thing the human is being asked to weigh, and a
          card that showed a role and a band without it would be the C-222 shape
          again — a verdict with no argument behind it.

          Rendered verbatim: `reason` is the model's sentence, not something to
          compose, and the attribution beneath it is a stored template the browser
          fills with the run's date in the reader's locale. */}
      {card.proposed?.reason && (
        <div style={{ ...contextStyle, fontStyle: "italic" }}>{card.proposed.reason}</div>
      )}
      {card.proposed && proposedAttribution && (
        <div style={{ ...contextStyle, color: "var(--text-muted)" }}>{proposedAttribution}</div>
      )}

      {/* A reason a HUMAN gave, distinct from the system's defer notice above. */}
      {card.defer_reason && (
        <div style={{ ...contextStyle, fontStyle: "italic" }}>Parked: {card.defer_reason}</div>
      )}
    </div>
  );
};
