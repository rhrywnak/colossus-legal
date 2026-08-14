// =============================================================================
// EvidenceCardParts — the pieces ONE evidence card is built from
// =============================================================================
//
// Split out of `EvidenceCardBody` when the shared body passed the 300-line limit
// (Rule 17), following the `FactRowParts` / `EvidenceChainParts` precedent this
// directory already has — and the ruling R8 discipline that no file this task
// touches ends over the limit.
//
// The seam is the conventional one and it is real here: everything below renders
// ONE piece of a card and decides nothing, while `EvidenceCardBody` is the
// assembly — which pieces appear, in what order, and what is currently unfolded.
// The card's ANATOMY is data in `evidenceCardModel.evidenceCardView`, and the
// body walks it.
//
// ## Visual language (§2c, binding)
//
// Pure white surfaces, hairline borders, regular weight with bold reserved for
// true emphasis, one accent plus chips, generous line height.

import React, { useState } from "react";

import {
  chipStyle,
  metaChipStyle,
  warningChipStyle,
} from "./candidateCardStyles";
import { openViewerWindow } from "./viewerWindow";
import type { CardBearsOnView, CardChip, ChipFilter } from "./evidenceCardModel";
import type { CardGrammarWording } from "../services/evidenceLinks";

/** The provenance tag riding beside a raw extracted name — smaller, quieter. */
const tagStyle: React.CSSProperties = {
  ...chipStyle,
  fontSize: "10px",
  padding: "2px 6px",
  color: "var(--text-muted)",
  border: "1px solid var(--border-card)",
};

/** The 👤 provenance mark: present, quiet, and never the loudest thing on a row. */
const HUMAN_MARKER_STYLE: React.CSSProperties = {
  color: "var(--text-muted)",
};

const clickableChipStyle: React.CSSProperties = {
  ...metaChipStyle,
  cursor: "pointer",
  // A chip that filters has to LOOK different from one that labels, or a human
  // learns the behaviour by accident and only on the chips they happen to click.
  border: "1px solid var(--border-card)",
};

/**
 * The header row: who said it, what kind of statement, and the badges.
 *
 * First class and in the same position on both wrappers — §3.1's whole point.
 * The speaker LEADS (ruling R2): the card answers "who said this" before it says
 * anything else, which is the half of the "System" defect that layout can fix.
 */
export const ChipRow: React.FC<{
  chips: CardChip[];
  wording: CardGrammarWording;
  /** Narrow the list to this chip's value (Piece 7). Absent = chips only label. */
  onFilter?: (filter: ChipFilter) => void;
}> = ({ chips, wording, onFilter }) => {
  if (chips.length === 0) return null;

  return (
    <div style={{ display: "flex", gap: "6px", flexWrap: "wrap", alignItems: "center" }}>
      {chips.map((chip, i) => {
        const filter = chip.filter;
        const interactive = Boolean(filter && onFilter);
        const base = chip.warning
          ? { ...metaChipStyle, ...warningChipStyle }
          : interactive
            ? clickableChipStyle
            : metaChipStyle;

        return (
          <React.Fragment key={`${chip.text}-${i}`}>
            {interactive && filter ? (
              <button
                type="button"
                style={{ ...base, fontFamily: "inherit" }}
                // Chips are cross-reference currency (§2c): the hint says what a
                // click does, from the store, because a control that does not
                // disclose itself is one most humans never find.
                title={wording.chip_filter_hint_template.replace("{value}", chip.text)}
                // The card's own click aims the keyboard at it; narrowing a list
                // is not a selection. Same reasoning as the ruling buttons'.
                onClick={(event) => {
                  event.stopPropagation();
                  onFilter?.(filter);
                }}
              >
                {chip.text}
              </button>
            ) : (
              <span style={base} title={chip.text}>
                {chip.text}
              </span>
            )}
            {/* Provenance rides BESIDE the name rather than inside it, so the
                chip's own text stays the exact string a filter matches on. */}
            {chip.tag && <span style={tagStyle}>{chip.tag}</span>}
          </React.Fragment>
        );
      })}
    </div>
  );
};

/** The `Q:` / `A:` prefix and its text, sharing one column (the hanging indent). */
export const ExchangeRow: React.FC<{
  prefix: string;
  children: React.ReactNode;
  /** Muted for the question, primary for the answer — the answer is the star. */
  muted?: boolean;
}> = ({ prefix, children, muted = false }) => (
  <div style={{ display: "flex", gap: "0.45rem", alignItems: "baseline" }}>
    <span
      style={{
        fontWeight: 600,
        flexShrink: 0,
        minWidth: "1.6em",
        color: "var(--text-muted)",
        fontSize: muted ? "13px" : "14px",
      }}
    >
      {prefix}
    </span>
    <span
      style={{
        color: muted ? "var(--text-secondary)" : "var(--text-primary)",
        fontSize: muted ? "13px" : "14px",
        lineHeight: 1.6,
        minWidth: 0,
        overflowWrap: "anywhere",
      }}
    >
      {children}
    </span>
  </div>
);

/** One allegation on one line, its element chips compressed behind "+N more". */
export const BearsOnRow: React.FC<{ row: CardBearsOnView; wording: CardGrammarWording }> = ({
  row,
  wording,
}) => {
  const [open, setOpen] = useState(false);

  return (
    <div style={{ marginTop: "4px" }}>
      <div style={{ fontSize: "13px", color: "var(--text-secondary)" }}>
        {/* The 👤 says a HUMAN made this link. The two lists are different claims
            about the world and the card must not blur them (task 2.10, R2).
            Ruling (b), 2026-08-12: it renders MUTED, in its own span, so the
            provenance mark stops competing with the accusation it prefixes. The
            distinction stays; only its weight changes. */}
        {row.human && <span aria-hidden="true" style={HUMAN_MARKER_STYLE}>👤 </span>}
        <span style={{ fontWeight: 600, color: "var(--text-primary)" }}>{row.accusation}</span>
        {row.count && (
          <span
            style={{
              ...chipStyle,
              marginLeft: "0.4rem",
              background: "var(--v3-chip-alleg-bg)",
              color: "var(--v3-chip-alleg-text)",
            }}
          >
            {row.count}
          </span>
        )}
      </div>

      {(row.visibleElements.length > 0 || row.hiddenElements.length > 0) && (
        <div
          style={{
            display: "flex",
            gap: "5px",
            flexWrap: "wrap",
            alignItems: "center",
            marginTop: "3px",
          }}
        >
          {row.visibleElements.map((element) => (
            <span key={element} style={metaChipStyle} title={element}>
              {element}
            </span>
          ))}
          {open &&
            row.hiddenElements.map((element) => (
              <span key={element} style={metaChipStyle} title={element}>
                {element}
              </span>
            ))}
          {row.moreLabel && (
            <button
              type="button"
              onClick={(event) => {
                event.stopPropagation();
                setOpen(!open);
              }}
              style={{
                ...chipStyle,
                cursor: "pointer",
                fontFamily: "inherit",
                border: "1px dashed var(--border-default)",
                background: "none",
                color: "var(--text-muted)",
              }}
            >
              {open ? wording.elements_fewer_label : row.moreLabel}
            </button>
          )}
        </div>
      )}
    </div>
  );
};

/** The pinpoint, opening the sized viewer WINDOW rather than a tab (D5). */
export const PinpointRow: React.FC<{ label: string; href: string }> = ({ label, href }) => {
  // A refused popup has to be SAID, not swallowed (Standing Rule 1). Local to the
  // chip so the message appears where the human just clicked.
  const [blocked, setBlocked] = useState<string | null>(null);

  if (!href) {
    return (
      <div>
        <span style={{ ...chipStyle, color: "var(--text-primary)" }}>{label}</span>
      </div>
    );
  }

  return (
    <div>
      {/* An anchor with an href, driven by onClick — so it still looks and
          middle-clicks like a link, while a plain click gets the sized window. */}
      <a
        href={href}
        onClick={(event) => {
          event.preventDefault();
          event.stopPropagation();
          const result = openViewerWindow(href);
          setBlocked(result.opened ? null : result.message);
        }}
        style={{ ...chipStyle, color: "var(--accent-primary)", textDecoration: "none" }}
      >
        {label} ↗
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

