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
import { openViewerWindow } from "./viewerWindow";
import type { ScenarioCard } from "../services/scenarioCards";

const SURFACE = "var(--bg-surface)"; // #ffffff — pure white, per §2c
const HAIRLINE = "1px solid var(--border-default)";

export const cardStyle: React.CSSProperties = {
  background: SURFACE,
  border: HAIRLINE,
  borderRadius: "8px",
  padding: "1rem 1.15rem",
  display: "flex",
  flexDirection: "column",
  gap: "0.6rem",
  fontWeight: 400, // §2c: regular weight; bold is reserved
};

const focusedCardStyle: React.CSSProperties = {
  ...cardStyle,
  borderColor: "var(--accent-primary)",
};

export const chipStyle: React.CSSProperties = {
  border: HAIRLINE,
  borderRadius: "999px",
  padding: "0.1rem 0.55rem",
  fontSize: "0.75rem",
  color: "var(--text-muted)",
  whiteSpace: "nowrap",
};

const quoteStyle: React.CSSProperties = {
  fontSize: "1rem",
  lineHeight: 1.7, // §2c: generous line height
  color: "var(--text-primary)",
};

const contextStyle: React.CSSProperties = {
  color: "var(--text-muted)",
  fontSize: "0.85rem",
  lineHeight: 1.7,
};

const markStyle: React.CSSProperties = {
  background: "var(--highlight-quote-soft)",
  color: "var(--text-primary)",
  padding: "0.05rem 0.1rem",
  borderRadius: "2px",
  fontSize: "1rem",
  lineHeight: 1.7,
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
    <div style={{ display: "flex", gap: "0.4rem", alignItems: "baseline", flexWrap: "wrap" }}>
      <span
        style={
          row.element === "quote"
            ? quoteStyle
            : { fontSize: "0.85rem", color: "var(--text-primary)" }
        }
      >
        {row.value}
      </span>
      {row.chips?.map((chip) => (
        <span key={chip} style={chipStyle}>
          {chip}
        </span>
      ))}
    </div>
  );
};

/** One candidate card in the Casey layout. */
export const CandidateCard: React.FC<{ card: ScenarioCard; focused: boolean }> = ({ card, focused }) => {
  const rows = useMemo(() => cardRows(card), [card]);
  const code = rows.find((r) => r.element === "code");
  const status = rows.find((r) => r.element === "status");
  const body = rows.filter((r) => r.element !== "code" && r.element !== "status");

  return (
    <div style={focused ? focusedCardStyle : cardStyle}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline" }}>
        <span style={{ ...chipStyle, color: "var(--text-primary)" }}>{code?.value ?? "—"}</span>
        <span style={{ fontSize: "0.75rem", color: "var(--text-muted)" }}>{status?.value}</span>
      </div>

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
      <div>
        {card.quote.context_before_notice && (
          <div style={{ ...contextStyle, fontStyle: "italic" }}>
            {card.quote.context_before_notice}
          </div>
        )}
        {card.quote.context_before && (
          <span style={contextStyle}>{card.quote.context_before} </span>
        )}
        <mark style={markStyle}>{card.quote.text}</mark>
        {card.quote.context_after && <span style={contextStyle}> {card.quote.context_after}</span>}
        {card.quote.context_after_notice && (
          <div style={{ ...contextStyle, fontStyle: "italic" }}>
            {card.quote.context_after_notice}
          </div>
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
