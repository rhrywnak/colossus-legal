// =============================================================================
// HumanLinkSection — what a card shows once a human has linked it (2.10)
// =============================================================================
//
// The server-composed sentence, then one chip per accusation with its cut and a
// one-click Unlink. This is what stands in place of the stance the extraction
// never found — and it is deliberately NOT a stance: the machine said nothing
// about this statement, so the card reports an act a person took (ruling R2).
//
// ## Why this is its own file (task 2.12)
//
// `LinkToAccusationPanel` was at the 300-line limit (Rule 17) and the seam is
// real: that file is the CONTROL a human uses on a stuck card, this is the
// ANSWER shown once they have used it. The two never appear together.

import React from "react";

import type { AllegationOptions } from "../services/evidenceLinks";
import type { CardHumanLink } from "../services/scenarioCards";

/** Mockup-matched chip metrics, shared by the two pieces below. */
const countChipStyle: React.CSSProperties = {
  fontSize: "11.5px",
  color: "var(--text-muted)",
  whiteSpace: "nowrap",
};

const quietActionStyle: React.CSSProperties = {
  fontFamily: "inherit",
  fontSize: "12.5px",
  fontWeight: 500,
  border: "none",
  borderRadius: "8px",
  padding: "6px 14px",
  background: "transparent",
  color: "var(--text-secondary)",
  cursor: "pointer",
};

/**
 * What a linked card shows in place of the stance it does not have.
 *
 * The server-composed sentence, then one chip per accusation with its cut and a
 * one-click Unlink. Renders nothing at all for a card nobody has linked, which is
 * why the card can call it unconditionally — the alternative was a second
 * `card.human_links.length > 0` test in `CandidateCard`, whose job is to walk the
 * §7 descriptor rather than to decide what a link looks like.
 *
 * `options` is `null` while the panel wording is loading; the chips still render
 * (they carry their own composed labels) but the Unlink control does not, because
 * its word is a stored one and there is no literal to fall back to (R4).
 */
export const HumanLinkSection: React.FC<{
  summary: string | null;
  links: CardHumanLink[];
  options: AllegationOptions | null;
  onUnlink: (allegationId: string) => void;
}> = ({ summary, links, options, onUnlink }) => {
  if (links.length === 0) return null;
  return (
    <>
      {summary && (
        <div style={{ fontSize: "13px", color: "var(--text-secondary)" }}>{summary}</div>
      )}
      <HumanLinkChips
        links={links}
        unlinkLabel={options?.wording.unlink_label ?? null}
        onUnlink={onUnlink}
      />
    </>
  );
};

/** The chips a linked card wears, each with its own one-click Unlink. */
const HumanLinkChips: React.FC<{
  links: CardHumanLink[];
  /** `null` while the stored word has not loaded — no control is rendered then. */
  unlinkLabel: string | null;
  onUnlink: (allegationId: string) => void;
}> = ({ links, unlinkLabel, onUnlink }) => (
  <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
    {links.map((link) => (
      <div
        key={link.allegation_id}
        style={{ display: "flex", gap: "8px", alignItems: "baseline", flexWrap: "wrap" }}
      >
        <span
          style={{
            fontSize: "12px",
            fontWeight: 500,
            borderRadius: "6px",
            padding: "3px 10px",
            background: "var(--v3-chip-alleg-bg)",
            color: "var(--v3-chip-alleg-text)",
          }}
        >
          <span aria-hidden="true">👤</span> {link.label}
        </span>
        <span style={countChipStyle}>{link.cut_label}</span>
        {unlinkLabel && (
          <button
            type="button"
            style={{ ...quietActionStyle, padding: "2px 6px" }}
            // The accessible name has to say WHICH link this takes back: four
            // buttons all reading "Unlink" tell a screen reader nothing.
            aria-label={`${unlinkLabel} — ${link.label}`}
            onClick={(event) => {
              // Not also a selection: taking a link back is not aiming the
              // keyboard.
              event.stopPropagation();
              onUnlink(link.allegation_id);
            }}
          >
            {unlinkLabel}
          </button>
        )}
      </div>
    ))}
  </div>
);

export default HumanLinkSection;
