// =============================================================================
// PrepTopBlock — what the prep page says before it says anything else (task R3)
// =============================================================================
//
// The page's opening, in the order a witness needs it:
//
//   S-5 · name · direction        (small — the handle, not the message)
//   OUR THEME                      (big; the page's first sentence)
//   THE ATTACK                     (large, them-tinted — what she is answering)
//   ▸ The attack in full           (folded; their words, names and dates)
//   A-41  A-46                     (bears-on chips, small, beside the fold)
//   They said it 5 times, in 3 documents, …
//
// ## Why the theme is the first thing and carries no heading
//
// It is the sentence Marie says out loud. A heading above it would make her read
// a label before she reads her own line, and the identity row directly above
// already says what this scenario is. The mockup puts it here for that reason.
//
// ## THEIR MOTIVATION is deliberately not on this page
//
// Ratified §10 exclusion, re-confirmed 2026-08-10. It is a claim about the other
// side's intent, it is one click away on the working page, and a witness who
// reads it aloud under cross has volunteered a theory nobody asked her for.
//
// ## Nothing here is editable
//
// This whole surface is read-only; every edit routes to the working page through
// the one link in the header. The old page put a `SentenceEditor` on the theme
// and an authorship line under it — "Written in plain words by roman · Aug 7" —
// and both are gone. A prep surface that invites editing invites editing under
// pressure, minutes before a deposition.

import React, { useState } from "react";

import { allegationChipStyle } from "./scenarioSectionStyles";
import type { RehearsalScenario, RehearsalWording } from "../services/rehearsal";

/** The theme: the page's first sentence, and the biggest thing on it. */
const themeStyle: React.CSSProperties = {
  margin: "10px 0 0",
  fontSize: "27px",
  lineHeight: 1.3,
  fontWeight: 600,
  letterSpacing: "-0.01em",
  color: "var(--text-primary)",
};

const themeGapStyle: React.CSSProperties = {
  ...themeStyle,
  fontSize: "20px",
  fontWeight: 400,
  fontStyle: "italic",
  color: "var(--text-muted)",
};

/** The attack, in the other side's colour. */
const attackCardStyle: React.CSSProperties = {
  marginTop: "22px",
  padding: "16px 20px",
  borderRadius: "10px",
  background: "var(--state-danger-bg-soft)",
  borderLeft: "4px solid var(--state-danger-strong)",
};

// The rehearsal page's half of the 2026-08-25 label-weight ruling. The scenario
// card's four identity labels went 600 -> 700 in .411; this is the same family
// (11px / .08em / uppercase) on the page Marie actually reads from, and leaving
// it at 600 would have made "THE ATTACK" render bold on one page and semibold on
// the other. Weight ONLY — size, tracking, casing and colour are untouched.
//
// Third member of the family, for the next reader: `rehearsalStyles.alwaysLabelStyle`
// was already 700, so this is the family's existing setting, not a new one.
const attackLabelStyle: React.CSSProperties = {
  fontSize: "11px",
  fontWeight: 700,
  letterSpacing: "0.08em",
  textTransform: "uppercase",
  color: "var(--v3-red-text)",
  marginBottom: "6px",
};

const attackStyle: React.CSSProperties = {
  margin: 0,
  fontSize: "19px",
  lineHeight: 1.45,
  color: "var(--text-primary)",
};

const foldRowStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "10px",
  flexWrap: "wrap",
  marginTop: "12px",
};

const foldButtonStyle: React.CSSProperties = {
  border: "none",
  background: "none",
  padding: 0,
  cursor: "pointer",
  fontFamily: "inherit",
  fontSize: "13px",
  color: "var(--accent-primary)",
  textAlign: "left",
};

/** Their words verbatim — the full paragraph, once opened. */
const fullAttackStyle: React.CSSProperties = {
  margin: "10px 0 0",
  padding: "12px 16px",
  borderRadius: "8px",
  background: "var(--bg-surface)",
  fontSize: "14.5px",
  lineHeight: 1.6,
  color: "var(--text-secondary)",
  whiteSpace: "pre-wrap",
};

const countLineStyle: React.CSSProperties = {
  margin: "18px 0 0",
  fontSize: "15px",
  lineHeight: 1.5,
  color: "var(--text-secondary)",
};

interface Props {
  scenario: RehearsalScenario;
  wording: RehearsalWording;
}

const PrepTopBlock: React.FC<Props> = ({ scenario, wording }) => {
  // The fold starts CLOSED. The plain-words accusation above it is what a
  // witness rehearses; the verbatim paragraph is what she opens when Chuck
  // quotes it at her, which is a different moment.
  const [showFull, setShowFull] = useState(false);
  const accusation = scenario.accusation;

  return (
    <div>
      {/* ⚑ THE IDENTITY LINE IS GONE (T5, Roman's ruling 5).
          It read `S-11 · The $50,000 · Offensive` and was the THIRD naming of
          the same scenario on this page — the breadcrumb says it, the header
          strip above says it with the role chip and the status, and this said
          it again in plain text. Three identities on one page is the chaos the
          strip was built to end. What this block is for starts below. */}
      {/* The theme. A scenario nobody has framed shows its stated gap here
          rather than an empty space where the page's first sentence goes. */}
      {scenario.what_this_is ? (
        <p style={themeStyle}>{scenario.what_this_is}</p>
      ) : (
        <p style={themeGapStyle}>{scenario.what_this_is_gap}</p>
      )}

      <div style={attackCardStyle}>
        <div style={attackLabelStyle}>{wording.block_accusation_heading}</div>
        {accusation.text ? (
          <p style={attackStyle}>{accusation.text}</p>
        ) : (
          <p style={{ ...attackStyle, fontStyle: "italic", color: "var(--text-muted)" }}>
            {accusation.text_gap}
          </p>
        )}

        {/* The full paragraph, and the chips that say which complaint
            paragraphs this bears on. Both sit under the plain words because
            both are reference material — reached for, not read first. */}
        <div style={foldRowStyle}>
          {scenario.attack_text && (
            <button
              type="button"
              style={foldButtonStyle}
              aria-expanded={showFull}
              onClick={() => setShowFull(!showFull)}
            >
              {showFull ? "▾" : "▸"} {wording.attack_full_label}
            </button>
          )}
          {/* A-41, composed server-side from the paragraph the Allegation node
              carries (task R4, P6a). They used to read `A-<hash>`, because the
              backend built them out of the anchor id — and the paragraph is not
              in the id.

              Rendered verbatim: this page composes nothing, which is why the fix
              went to the assembly rather than putting the case-wide allegation
              catalogue in a browser that is open in front of opposing counsel. */}
          {scenario.bears_on.map((chip) => (
            <span key={chip} style={allegationChipStyle}>
              {chip}
            </span>
          ))}
        </div>

        {showFull && scenario.attack_text && (
          <p style={fullAttackStyle}>{scenario.attack_text}</p>
        )}
      </div>

      {/* Composed server-side, plural-correct, and with the date span this
          scenario actually has — or without one, when nothing is dated. */}
      {accusation.plain_count_line && (
        <p style={countLineStyle}>{accusation.plain_count_line}</p>
      )}
    </div>
  );
};

export default PrepTopBlock;
