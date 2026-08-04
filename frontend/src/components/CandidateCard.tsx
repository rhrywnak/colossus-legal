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

import { cardRows, type CardRow } from "./cardRows";
import { candidateState, stateChip } from "./candidateFilters";
import type { RulingKey } from "./cardTriage";
import QuestionLine from "./QuestionLine";
import { CardHead, StateChipView, codeBadgeStyle } from "./RulingButtons";
import { openViewerWindow } from "./viewerWindow";
import type { ScenarioCard } from "../services/scenarioCards";

const SURFACE = "var(--bg-surface)"; // --card #ffffff

/**
 * The card. Mockup `.card`: white, radius 12, `--shadow-raised`, padding
 * 18px 24px 20px. NO BORDER — v3 removed every card border (see the
 * `--bg-canvas` comment in tokens.css for why the canvas moved with them).
 */
export const cardStyle: React.CSSProperties = {
  background: SURFACE,
  borderRadius: "var(--radius-card)",
  padding: "18px 24px 20px",
  display: "flex",
  flexDirection: "column",
  gap: "0.6rem",
  fontWeight: 400,
  // The unselected card still lifts off the page, just less. A flat card on a
  // tinted canvas reads as a hole rather than a surface.
  boxShadow: "var(--shadow-card)",
};

/**
 * The SELECTED card — the one the keyboard is aimed at.
 *
 * v3 marks selection with ELEVATION (`--shadow-raised`) rather than 1.7C's accent
 * border. That is not only a style change: the card is the thing a human is about
 * to rule on, and lifting it above its siblings says "this one" in a way a
 * coloured outline competes with the ruling buttons to say.
 */
const selectedCardStyle: React.CSSProperties = {
  ...cardStyle,
  boxShadow: "var(--shadow-raised)",
};

/** The compact row a ruled card collapses to. Same surface, one line of it. */
const compactCardStyle: React.CSSProperties = {
  ...cardStyle,
  flexDirection: "row",
  alignItems: "center",
  gap: "12px",
  padding: "10px 16px",
  cursor: "pointer",
};

/** Mockup `.pin` / neutral chips inside the card body. */
export const chipStyle: React.CSSProperties = {
  borderRadius: "6px",
  padding: "3px 10px",
  fontSize: "12px",
  fontWeight: 500,
  color: "var(--text-secondary)",
  whiteSpace: "nowrap",
};

/**
 * A metadata chip in the one-row metadata strip (task 1.7E, item 8).
 *
 * ## Why it truncates instead of being shortened
 *
 * The payload's words are the payload's words: "Grounded — found on the page" is
 * the sentence the backend composed, and rewriting it to "✓ grounded" in the
 * browser would be this file inventing vocabulary. So the chip renders the
 * sentence verbatim and lets CSS clip what does not fit, with the full text on the
 * element's `title` — truncation is presentation, paraphrase would be authorship.
 */
const metaChipStyle: React.CSSProperties = {
  ...chipStyle,
  background: "var(--v3-chrome)",
  maxWidth: "24ch",
  overflow: "hidden",
  textOverflow: "ellipsis",
  display: "inline-block",
  verticalAlign: "middle",
};

/**
 * The quote-in-context panel. Mockup `.ctx`: its OWN soft surface (#fafbfc,
 * radius 10, padding 14px 18px) inset within the white card.
 *
 * 1.7C rendered the context as bare text. Giving it a panel does a job: it makes
 * visually obvious where the SOURCE PAGE's words start and stop, so the reader can
 * tell the document's voice from the card's own labels above and below it.
 */
const contextPanelStyle: React.CSSProperties = {
  background: "var(--v3-context-panel)",
  borderRadius: "10px",
  padding: "14px 18px",
  color: "var(--text-secondary)",
  lineHeight: 1.7,
  fontSize: "13.5px",
};

const contextStyle: React.CSSProperties = {
  color: "var(--text-secondary)",
  fontSize: "13.5px",
  lineHeight: 1.7,
};

/** Mockup `.edge`: the honest page-edge marker — italic, muted, 12.5px. */
const edgeStyle: React.CSSProperties = {
  color: "var(--text-muted)",
  fontStyle: "italic",
  fontSize: "12.5px",
};

/** Mockup `.ctx mark`: #ffec9e, radius 3, padding 1px 3px, weight 500. */
const markStyle: React.CSSProperties = {
  background: "var(--highlight-quote-soft)",
  color: "var(--text-primary)",
  padding: "1px 3px",
  borderRadius: "3px",
  fontWeight: 500,
};

/** The one-line quote in a compact row: whatever fits, then an ellipsis. */
const compactQuoteStyle: React.CSSProperties = {
  flex: 1,
  minWidth: 0,
  fontSize: "13px",
  color: "var(--text-secondary)",
  whiteSpace: "nowrap",
  overflow: "hidden",
  textOverflow: "ellipsis",
};

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
  // Every entry is a payload string, rendered verbatim and carrying itself as its
  // own tooltip so nothing is lost to the truncation.
  const chips: string[] = [
    ...(card.speaker.name ? [card.speaker.name, card.speaker.attribution] : []),
    ...(card.statement_kind ? [card.statement_kind] : []),
    ...(card.grounding ? [card.grounding.label] : []),
    card.confidence.label,
  ];
  return (
    <div style={{ display: "flex", gap: "6px", flexWrap: "wrap", alignItems: "center" }}>
      {chips.map((chip, i) => (
        <span key={`${chip}-${i}`} style={metaChipStyle} title={chip}>
          {chip}
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
}> = ({
  card,
  selected,
  compact,
  onSelect,
  onRule,
  keyboardRefused = false,
  onCorrectQuestion,
  onRevertQuestion,
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
      {bearsOn.map((row, i) => (
        <TextRow key={`bears_on-${i}`} row={row} />
      ))}

      <MetaRow card={card} />

      {/* A reason a HUMAN gave, distinct from the system's defer notice above. */}
      {card.defer_reason && (
        <div style={{ ...contextStyle, fontStyle: "italic" }}>Parked: {card.defer_reason}</div>
      )}
    </div>
  );
};
