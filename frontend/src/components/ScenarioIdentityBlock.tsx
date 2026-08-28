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
import type { ScenarioIdentityWording } from "../services/scenarioAugmentation";

// Mockup `.panel .identity`: margin-top 20px, padding 20px 24px 16px, borderless
// white on --shadow-card. There is NO border — v3 removed every card border, and a
// hairline here would read as a double edge against the shadow.
const cardStyle: React.CSSProperties = {
  ...sectionPanelStyle,
  position: "relative",
  marginTop: "20px",
  // Bottom padding rose with the section gaps: at 16px the Bears-on chips sat
  // closer to the card's edge than the sections sit to each other, which reads as
  // the card running out rather than ending.
  padding: "20px 24px 20px",
  // The identity block's own content sets the radius clip; `overflow: hidden` from
  // the shared panel would clip the absolutely-positioned pencil.
  overflow: "visible",
};

// The card's vertical rhythm, named once because the whole point of it is the
// RATIO between the two (Roman, 2026-08-28, from live use: "the card reads
// cramped" and the sections "run together").
//
// Space above a label is now roughly four times the space below it, so each
// label sits with the text it introduces rather than floating equidistant
// between its own section and the one before. That asymmetry is what makes a
// section look like it owns its content; before this, `SECTION_GAP` was 14.4px
// against a 3px label gap in the same card, which is close enough to even that
// four labelled blocks read as one undifferentiated column.
//
// CONST: a visual rhythm, not a setting. There is no frontend config surface and
// these are not per-deployment values; naming them keeps the ratio in one place
// instead of five literals that drift apart at the next adjustment.
const SECTION_GAP = "22px";
const LABEL_GAP = "6px";

// Mockup `.id-grid`: two equal columns, 32px column gap. `auto-fit` keeps it from
// squeezing two columns into a narrow window. The ROW gap is `SECTION_GAP`: it
// only applies when the two columns wrap to one, and when they do these are two
// stacked sections like any other and must be separated like them.
const gridStyle: React.CSSProperties = {
  display: "grid",
  gridTemplateColumns: "repeat(auto-fit, minmax(18rem, 1fr))",
  gap: `${SECTION_GAP} 32px`,
};

// Was the mockup's `.lbl`: 11px, .08em, uppercase, --text-3, 3px bottom.
//
// ## Bolded 2026-08-25, then made legible 2026-08-28 (Roman, both from live use)
//
// The four names this style renders — the attack, our theme, their motivation,
// and what the scenario bears on — are the card's only wayfinding. The first
// pass bolted weight onto them and deliberately changed nothing else, on the
// reasoning that size, tracking, casing and colour were the mockup's. Three days
// of real use said that was not enough: at 11px in --text-muted they still read
// as "small light-gray letterspaced caps" and were hard to discern at all.
//
// So the two properties that were held back have moved, and only those two:
//
//   colour  --text-muted (#667085, 4.97:1) → --text-primary (#1a202c, 16.32:1)
//   size    11px → 12px
//
// Colour is the change that does the work. --text-primary is the app's darkest
// text token and is deliberately not pure black — a near-black with a blue cast,
// so it pairs with the accent — which is what was asked for. Size moves ONE step
// on this card's own scale (11 / 11.5 / 12 / 12.5 / 13); a bigger jump would
// start competing with the 0.9rem body text these labels introduce.
//
// The uppercase and the .08em tracking STAY, and that is what keeps them reading
// as labels rather than as headings: at the same colour as the body text below,
// casing and tracking are now the whole of the distinction, so removing either
// would leave a label indistinguishable from the sentence under it.
//
// ## Nothing else changes appearance because of this
//
// This object is PRIVATE to this file and is applied in exactly four places (the
// three `Field`s and the Bears-on heading below). There is no shared label style
// on this page to change instead, so the card moves alone:
//
//   `ScenarioHeaderTiers.tsx`  — the SCENARIO eyebrow, its own object, and the
//                                one thing in the same visual family (11px,
//                                uppercase, tracked, muted). Untouched.
//   `ScenarioIdentityModal.tsx` — the identity editor's field labels, its own.
//   `SentenceEditor.tsx`        — what renders the accusation card's "In plain
//                                 words". NOT the same family at all: 12px,
//                                 muted, sentence case, no tracking, no bold.
//                                 It was never going to inherit this and does
//                                 not.
const labelStyle: React.CSSProperties = {
  fontSize: "12px",
  fontWeight: 700,
  letterSpacing: "0.08em",
  textTransform: "uppercase",
  color: "var(--text-primary)",
  marginBottom: LABEL_GAP,
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
  /**
   * The four names and their stated absences, from the store.
   *
   * `null` while the augmentation payload is unloaded, which renders the whole
   * block as nothing rather than as unlabelled prose — the honest-gap law, and
   * the same rule `AccusationSection` follows. Four texts under no headings are
   * worse than an absent card: a reader cannot tell the attack from the answer.
   */
  wording: ScenarioIdentityWording | null;
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
  wording,
}) =>
  wording === null ? null : (
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
    <div style={{ marginBottom: SECTION_GAP, paddingRight: "1.5rem" }}>
      <Field
        label={wording.attack_label}
        value={attackText}
        absent={wording.attack_absent}
      />
    </div>

    <div style={gridStyle}>
      <Field
        label={wording.theme_label}
        value={themeStatement}
        absent={wording.theme_absent}
      />
      <Field
        label={wording.motivation_label}
        value={motivation}
        absent={wording.motivation_absent}
      />
    </div>

    <div style={{ marginTop: SECTION_GAP }}>
      <div style={labelStyle}>{wording.bears_on_label}</div>
      {anchorAllegationIds.length === 0 ? (
        <p style={absentStyle}>{wording.bears_on_absent}</p>
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
