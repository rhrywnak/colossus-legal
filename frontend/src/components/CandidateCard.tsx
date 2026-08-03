// =============================================================================
// CandidateCard — one §7 candidate, rendered (extracted from CardQueue, 1.7C)
// =============================================================================
//
// The Casey card layout over the §7 payload: C-code · state · question · the quote
// in its context · speaker · pinpoint · stance · bears-on · the defer notice.
//
// ## Why this is its own file
//
// `CardQueue` was 380 non-comment lines before task 1.7C and every §2.3 addition
// made it longer, so the file that already broke the 300-line limit was about to
// break it harder. The seam is real rather than convenient: this file is THE CARD
// (what one candidate looks like) and `CardQueue` is THE QUEUE (which card is
// focused, what a key does, what a ruling posts). Rule 17 forced the split; the
// split is the one a reader would have drawn anyway.
//
// ## This component renders and does nothing else
//
// Every string on screen comes from the 1.2 payload, and which rows a card shows
// is decided by the pure `cardTriage.cardRows`. This file is the JSX that walks
// the descriptor. It chooses no words — a `switch` composing prose here would be
// the frontend inventing vocabulary, which the language law forbids.
//
// ## Visual language (§2c, binding)
//
// Pure white surfaces, hairline borders, regular weight with bold reserved for the
// pinpoint page, one accent, generous line height.

import React, { useMemo, useState } from "react";

import { cardRows, type CardRow } from "./cardTriage";
import { CardHead, codeBadgeStyle, type RulingKey } from "./RulingButtons";
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
  // The unfocused card still lifts off the page, just less. A flat card on a
  // tinted canvas reads as a hole rather than a surface.
  boxShadow: "var(--shadow-card)",
};

/**
 * The FOCUSED card — the one the keyboard is aimed at.
 *
 * v3 marks focus with ELEVATION (`--shadow-raised`) rather than 1.7C's accent
 * border. That is not only a style change: the card is the thing a human is about
 * to rule on, and lifting it above its siblings says "this one" in a way a
 * coloured outline competes with the ruling buttons to say.
 */
const focusedCardStyle: React.CSSProperties = {
  ...cardStyle,
  boxShadow: "var(--shadow-raised)",
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





// ─── Row rendering ──────────────────────────────────────────────────────────

/**
 * Render one descriptor row.
 *
 * The `element` decides the presentation; the `value` is displayed verbatim.
 * This function chooses no words — a `switch` that composed prose here would be
 * the frontend inventing vocabulary, which the language law forbids.
 */
const Row: React.FC<{ row: CardRow }> = ({ row }) => {
  // A refused popup has to be SAID, not swallowed (Standing Rule 1). Local to the
  // chip so the message appears where the human just clicked.
  const [blocked, setBlocked] = useState<string | null>(null);

  if (row.element === "pinpoint") {
    // `href` is optional on the descriptor. It is always present in practice (the
    // backend composes `viewer_href` for every card, page or no page), but a chip
    // with nothing to open must not pretend to be a link — it renders as the plain
    // pinpoint text instead of a dead anchor.
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
  }

  return (
    <div style={{ display: "flex", gap: "0.4rem", alignItems: "center", flexWrap: "wrap" }}>
      {/* The `quote` element no longer reaches this branch: the anchor is rendered
          inside the context panel above, marked. Everything else here — speaker,
          statement kind, stance, bears-on, grounding — is a 13px card line. */}
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
};

/**
 * One candidate card, v3 layout.
 *
 * `onRule` is optional so a card can render read-only — the DEFERRED TRAY shows
 * parked cards for reference, and putting live ruling buttons on a card that is not
 * the focused one would let a human rule something the keyboard is not aimed at.
 */
export const CandidateCard: React.FC<{
  card: ScenarioCard;
  focused: boolean;
  onRule?: (key: RulingKey) => void;
}> = ({ card, focused, onRule }) => {
  const rows = useMemo(() => cardRows(card), [card]);
  const code = rows.find((r) => r.element === "code");
  const status = rows.find((r) => r.element === "status");
  const body = rows.filter((r) => r.element !== "code" && r.element !== "status");

  return (
    <div style={focused ? focusedCardStyle : cardStyle}>
      {onRule ? (
        <CardHead code={code?.value ?? "—"} state={status?.value} onRule={onRule} />
      ) : (
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            marginBottom: "14px",
          }}
        >
          <span style={codeBadgeStyle}>{code?.value ?? "—"}</span>
          <span style={{ fontSize: "12.5px", color: "var(--text-muted)" }}>{status?.value}</span>
        </div>
      )}

      {/* §7.1 — the question that makes a bare answer interpretable, then the
          quote in its surrounding text.

          Task 1.7C (defect D6): the context is now sentence-complete at both ends
          and its transcript gutter numerals are stripped, both server-side. Where a
          flank ran out of PAGE instead of reaching a sentence boundary, the payload
          carries a composed notice and it is rendered — a card must not imply a
          completeness the data does not have (Standing Rule 1). Measured on DEV,
          that is 154 of 499 cards on the left and 65 on the right.

          The `<mark>` is the anchor: verbatim, highlighted, never groomed (§12). */}
      {card.quote.question && <div style={contextStyle}>{card.quote.question}</div>}

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

      {body
        .filter((r) => r.element !== "quote")
        .map((row, i) => (
          <Row key={`${row.element}-${i}`} row={row} />
        ))}

      {/* A reason a HUMAN gave, distinct from the system's defer notice above. */}
      {card.defer_reason && (
        <div style={{ ...contextStyle, fontStyle: "italic" }}>Parked: {card.defer_reason}</div>
      )}
    </div>
  );
};
