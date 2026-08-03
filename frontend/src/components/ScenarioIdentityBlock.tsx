// =============================================================================
// ScenarioIdentityBlock — C1 back on the page, read-only (defect D8, §2.2)
// =============================================================================
//
// 1.7B moved identity editing into one modal, which was right, and in doing so took
// the identity off the PAGE entirely — defect D8. A human opening S-2 could no
// longer see what the other side claims, what our answer is, or which paragraphs
// the scenario bears on without opening a dialog.
//
// §2.2 puts it back as a read-only card with a pencil that opens the SAME modal as
// the header's Edit button. One editor, two doors (§2.1's one-modal law).
//
// ## The three texts are never collapsed
//
//   attack_text     — what THEY say         ("Marie is obstructive")
//   theme_statement — how WE answer it       (the one-line tagline)
//   motivation      — what they want the jury to believe by saying it
//
// Three sentences, three points of view. The C1 migration says in terms that
// merging any two destroys what rehearsal mode reads.
//
// ## The definition stays modal-only
//
// Long prose (1.7B's ruling, unchanged in §2.2). It would dominate this card and
// push the three short texts below the fold, which is how the attack ends up
// unread.
//
// ## Absent is absent
//
// A text nobody has written yet renders as a stated absence, not as an empty
// paragraph that looks like a rendering fault and not as invented filler.

import React from "react";

import { labelForAllegationId } from "./allegationLabel";
import { allegationChipStyle, sectionPanelStyle } from "./scenarioSectionStyles";
import type { AllegationDto } from "../services/allegations";

// Mockup `.panel .identity`: margin-top 20px, padding 20px 24px 16px, borderless
// white on --shadow-card. There is NO border — v3 removed every card border, and a
// hairline here would read as a double edge against the shadow.
const cardStyle: React.CSSProperties = {
  ...sectionPanelStyle,
  position: "relative",
  marginTop: "20px",
  padding: "20px 24px 16px",
  // The identity block's own content sets the radius clip; `overflow: hidden` from
  // the shared panel would clip the absolutely-positioned pencil.
  overflow: "visible",
};

// Mockup `.id-grid`: two equal columns, 16px/32px gaps. `auto-fit` keeps it from
// squeezing two columns into a narrow window.
const gridStyle: React.CSSProperties = {
  display: "grid",
  gridTemplateColumns: "repeat(auto-fit, minmax(18rem, 1fr))",
  gap: "16px 32px",
};

// Mockup `.lbl`: 11px/600, .08em, uppercase, --text-3, 3px bottom.
const labelStyle: React.CSSProperties = {
  fontSize: "11px",
  fontWeight: 600,
  letterSpacing: "0.08em",
  textTransform: "uppercase",
  color: "var(--text-muted)",
  marginBottom: "3px",
};

const textStyle: React.CSSProperties = {
  margin: 0,
  fontSize: "0.9rem",
  lineHeight: 1.6,
  color: "var(--text-primary)",
};

const absentStyle: React.CSSProperties = {
  ...textStyle,
  color: "var(--text-muted)",
  fontStyle: "italic",
};



interface Props {
  /** THEIR framing — `definition.attack_text` / the augmentation identity. */
  attackText: string | null;
  /** OUR one-line answer — the `theme_statement` column. */
  themeStatement: string | null;
  /** What they want the jury to believe — the `motivation` column. */
  motivation: string | null;
  /** Complaint paragraphs this scenario touches. Ids; labelled below. */
  anchorAllegationIds: string[];
  /** The allegation catalogue, for the chip labels. */
  allegations: AllegationDto[];
  /** Opens the ONE identity modal — the same one the header's Edit opens. */
  onEdit: () => void;
}

/** One labelled text, or a stated absence. */
const Field: React.FC<{ label: string; value: string | null; absent: string }> = ({
  label,
  value,
  absent,
}) => (
  <div>
    <div style={labelStyle}>{label}</div>
    {value && value.trim() ? (
      <p style={textStyle}>{value}</p>
    ) : (
      // Says what is missing and implies where to fix it (the pencil is one
      // glance away). An empty <p> would read as a broken render.
      <p style={absentStyle}>{absent}</p>
    )}
  </div>
);

const ScenarioIdentityBlock: React.FC<Props> = ({
  attackText,
  themeStatement,
  motivation,
  anchorAllegationIds,
  allegations,
  onEdit,
}) => (
  <div style={cardStyle}>
    <button
      type="button"
      onClick={onEdit}
      title="Edit — the same modal as the header button"
      aria-label="Edit scenario identity"
      style={{
        position: "absolute",
        top: "0.7rem",
        right: "0.8rem",
        border: "none",
        background: "none",
        color: "var(--accent-primary)",
        cursor: "pointer",
        fontSize: "0.9rem",
        fontFamily: "inherit",
      }}
    >
      ✎
    </button>

    {/* The attack is full width and first: it is the thing being answered, and
        everything else on this page is a response to it. */}
    <div style={{ marginBottom: "0.9rem", paddingRight: "1.5rem" }}>
      <Field
        label="The attack — what they claim"
        value={attackText}
        absent="No attack text written yet — the pencil opens the editor."
      />
    </div>

    <div style={gridStyle}>
      <Field
        label="Our theme — one sentence"
        value={themeStatement}
        absent="No theme written yet."
      />
      <Field
        label="Their motivation"
        value={motivation}
        absent="No motivation written yet."
      />
    </div>

    <div style={{ marginTop: "0.9rem" }}>
      <div style={labelStyle}>Bears on</div>
      {anchorAllegationIds.length === 0 ? (
        <p style={absentStyle}>No allegations linked yet.</p>
      ) : (
        <div style={{ display: "flex", gap: "0.4rem", flexWrap: "wrap" }}>
          {anchorAllegationIds.map((id) => (
            <span key={id} style={allegationChipStyle}>
              {/* One composer, shared with the modal's picker (ruling R11). An id
                  the catalogue does not know renders AS the id rather than being
                  dropped — a silently shorter chip row would disagree with the
                  scenario's actual anchors. */}
              {labelForAllegationId(id, allegations)}
            </span>
          ))}
        </div>
      )}
    </div>

    {/* C9 related-scenarios strip: placement RESERVED at the block bottom (§2.2)
        and rendering nothing until task 3.2 derives it. Absent, not fake. */}
  </div>
);

export default ScenarioIdentityBlock;
