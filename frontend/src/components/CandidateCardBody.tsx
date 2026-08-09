// =============================================================================
// CandidateCardBody — what a candidate card SHOWS, split from its wrapper
// =============================================================================
//
// Extracted from `CandidateCard.tsx` on 2026-08-09 (ONE_CARD_GRAMMAR, ruling R8).
// **This commit is move-only: not one rendered pixel changes.** Every component
// and every line of JSX below came across verbatim; what moved is where they
// live.
//
// ## Why the move, and why it is its own commit
//
// `CandidateCard.tsx` was 342 non-comment lines against Rule 17's 300 — a
// pre-existing overage the one-card-grammar task would have made worse, because
// the whole point of that task is that this body has to serve TWO wrappers (the
// candidate's ruling controls and the fact's weight/order/Remove). Moving the
// body out first is both the size fix and the seam the next commits need, and
// keeping it move-only means the diff that follows is readable as behaviour
// rather than as a re-indent.
//
// ## The seam
//
// This file is WHAT A CARD SHOWS — the quote in its context, the pinpoint, what
// it bears on, the scan's reason. `CandidateCard` keeps WHAT MAY BE DONE TO IT —
// the ruling head, the defer prompt, the receipt, the compact collapse. That is
// the same cut `RulingButtons` made from the other side in 1.7D, and the reason
// the two files read as a pair.
//
// ## This component renders and does nothing else
//
// Unchanged from the header it came from: every string on screen comes from the
// 1.2 payload, and which rows a card shows is decided by the pure `cardRows`.
// This file is the JSX that walks the descriptor. It chooses no words — a
// `switch` composing prose here would be the frontend inventing vocabulary,
// which the language law forbids.

import React, { useMemo, useState } from "react";

import {
  chipStyle,
  contextPanelStyle,
  contextStyle,
  edgeStyle,
  markStyle,
  metaChipStyle,
  warningChipStyle,
} from "./candidateCardStyles";
import { cardRows, metaChips, type CardRow } from "./cardRows";
import { needsLinking } from "./cardLinking";
import LinkToAccusationPanel from "./LinkToAccusationPanel";
import { HumanLinkSection } from "./HumanLinkSection";
import QuestionLine from "./QuestionLine";
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

// ─── The body ───────────────────────────────────────────────────────────────

/**
 * Whether THIS card offers the link panel.
 *
 * Exported because the answer is needed on both sides of the seam: the body
 * renders the panel, and the wrapper's `CardHead` suppresses its own copy of the
 * explanation when the panel below is already giving one (Q4). Two independent
 * derivations of one boolean would eventually disagree and print the sentence
 * twice — the clutter ruling, arrived at by accident.
 *
 * Task 2.10: the panel appears on a card the extraction never linked and nobody
 * has linked since — and only when its words have loaded. A card that HAS been
 * linked shows its chips instead: the work is done, and re-offering the control
 * beneath the answer would read as though it had not been.
 */
export function showsLinkPanel(
  card: ScenarioCard,
  linkOptions: AllegationOptions | null,
): boolean {
  return linkOptions !== null && needsLinking(card) && card.human_links.length === 0;
}

/**
 * One candidate card's body: everything between the ruling head and the card edge.
 *
 * Props are the wrapper's, passed straight down — this component owns no state
 * beyond the pinpoint's blocked-popup message, which belongs to the chip that
 * raised it.
 */
export const CandidateCardBody: React.FC<{
  card: ScenarioCard;
  /**
   * The accusations THIS card's link panel offers, and its words (task 2.10).
   *
   * `null` while the scenario's options are loading, or if that read failed. The
   * panel is then not rendered at all — there is deliberately no fallback set of
   * words to render it with (R4), and a control with no labels would be worse
   * than the dead end it replaces.
   */
  linkOptions: AllegationOptions | null;
  /** Save a human's correction of THIS card's question (task 1.7F Part B). */
  onCorrectQuestion: (text: string) => Promise<void>;
  /** Restore the machine's own question by deleting the correction. */
  onRevertQuestion: () => Promise<void>;
  /** Save this card's links. Bound by the list, exactly as `onRule` is. */
  onSaveLinks: (allegationIds: string[], cut: LinkCut) => Promise<void>;
  /** Take one of this card's links back. */
  onUnlink: (allegationId: string) => void;
  /**
   * "Proposed by the Aug 7 scan" — composed by the list from the stored template
   * and the run's date, or `null` when nothing proposed this card (2026-08-08).
   */
  proposedAttribution?: string | null;
  /**
   * Raised to the wrapper when the panel holds ticks or a cut that have not been
   * saved, so its greyed Include and Exclude can say why (task 2.12, item B).
   *
   * Roman filled a panel in, saw Include still greyed, and reported it as broken —
   * it was correct behaviour with no observable, which is the same defect class as
   * 1.7C's silent keypress. The state lives in the WRAPPER because that is where
   * the buttons are; the panel that changes it lives here.
   */
  onLinkDraftDirty: (dirty: boolean) => void;
}> = ({
  card,
  linkOptions,
  onCorrectQuestion,
  onRevertQuestion,
  onSaveLinks,
  onUnlink,
  proposedAttribution = null,
  onLinkDraftDirty,
}) => {
  const rows = useMemo(() => cardRows(card), [card]);
  const pinpoint = rows.find((r) => r.element === "pinpoint");
  const bearsOn = rows.filter((r) => r.element === "bears_on");
  // The stance row renders ONLY when the card has a real stance. When it has none,
  // `cardRows` fills the §7.5 slot with the defer reason (the contract test asserts
  // exactly one of the two is present) — and that sentence now belongs to the
  // button row rather than the card body (item 8).
  const stance = card.stance ? rows.find((r) => r.element === "stance") : undefined;
  const linkPanel = showsLinkPanel(card, linkOptions);

  return (
    <>
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
          onDraftDirty={onLinkDraftDirty}
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
    </>
  );
};

export default CandidateCardBody;
