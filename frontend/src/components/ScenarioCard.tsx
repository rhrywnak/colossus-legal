// =============================================================================
// ScenarioCard.tsx — one scenario on the Trial Prep dashboard grid
// =============================================================================
//
// Split out of `TrialPrepViews.tsx` on 2026-08-07, when the card gained its ⋯
// kebab: that file was already over the module limit (Rule 17) and this card is
// the largest thing in it. The kebab is gone (2026-08-08 — Roman overruled D7 for
// Delete and this card carries a visible Delete instead), but the split stays:
// the size argument was the durable half, and moving the card back would put
// `TrialPrepViews` over the limit again for no gain.
//
// This card is props-in → JSX-out: no fetch, no state. The delete REQUEST is
// raised to the page, which owns the confirm dialog and the write.
//
// ## ⚑ "Go to Practice →" is a LITERAL, and one row is owed
//
// It replaces "Open scenario →", which was a literal too — checked: that string
// appeared nowhere but this file, and nothing in `backend/src` or the migrations
// carried it. The task's rule is to treat the new label the same way, so it is a
// literal and the debt is named rather than hidden.
//
// There IS a served row for a practice label — `scenario_practice_link_label`,
// seeded "Practice" — and it is deliberately NOT reused. Two reasons, either
// alone sufficient: its value is "Practice", not this sentence, and it rides the
// scenario AUGMENTATION payload, which this card never fetches (the trial-prep
// payload carries `create_wording` and `war_room_wording` and no authoring
// block). Reusing it would mean a second fetch on a component rendered twelve
// times, to get a string that is not the one asked for.
//
// Owed row, to go on one of the blocks the trial-prep payload already serves:
//
//   scenario_card_practice_link_label = "Go to Practice →"
//
// It joins the ten the v2.1 and READY reports list, so one migration settles
// them all (rule 25 — the migration is Roman's).

import React from "react";
import { Link } from "react-router-dom";

import { scenarioCardStyle } from "./trialPrepCardStyles";
import type { ScenarioSummary } from "../pages/trialPrepData";
import { statusMeta } from "../pages/trialPrepHelpers";
import { practicePath, scenarioPagePath } from "../utils/routePaths";

/**
 * The line box BOTH bottom controls share.
 *
 * Delete and Practice are absolutely positioned against opposite corners at the
 * same `bottom`, and they render at different font sizes — 0.78rem and 0.82rem,
 * each inherited from what stood in its corner before. Different font sizes give
 * different line boxes, which put the two texts on visibly different lines even
 * though their boxes agree: measured 3px apart in the browser before this.
 *
 * Pinning ONE line height in rem makes the two boxes the same height without
 * touching either font size, so each control keeps the size it is supposed to
 * have and they still read as one row. A px value would not survive a root
 * font-size change; a unitless multiplier would re-introduce the difference,
 * because it multiplies each control's own size.
 */
// STRUCTURAL: a layout constant, not a setting. It exists to make two boxes the
// same height and cannot legitimately vary by deployment, case or environment —
// a deployment that changed it would simply mis-align the two controls again.
const CONTROL_LINE_HEIGHT = "1rem";

/**
 * Delete on the card face.
 *
 * Danger-coloured text, no fill: it has to be findable on a grid of cards without
 * competing with the card's own title. Sized down from the header's copy — this
 * one sits on twelve cards at once, and twelve red buttons would make the
 * dashboard look like a list of problems.
 */
const cardDeleteButtonStyle: React.CSSProperties = {
  border: "none",
  background: "none",
  padding: "4px 6px",
  borderRadius: "6px",
  cursor: "pointer",
  color: "var(--state-danger-strong)",
  fontFamily: "inherit",
  fontSize: "0.78rem",
  // ⚑ Shared with `cardPracticeLinkStyle` — see `CONTROL_LINE_HEIGHT`.
  lineHeight: CONTROL_LINE_HEIGHT,
};

/**
 * "Go to Practice →" on the card face (Roman's ruling, 2026-09-07).
 *
 * ## What it replaces, and why the replacement is a real link
 *
 * A plain `<span>` reading "Open scenario →" sat here, inside the card's own
 * `<Link>`. Its comment called it a "visual affordance only — the whole card
 * navigates, so this is plain text, not a separate link", and that was true: it
 * pointed at the page the card already opens, so it was an arrow telling you
 * about a click you were already making. It is gone.
 *
 * Practice is a DIFFERENT destination, so this one is a real `<Link>` — and
 * therefore a sibling of the card's link rather than a child of it, for the same
 * reason Delete is: an `<a>` inside an `<a>` is invalid HTML, and every click on
 * the inner one would also navigate to the outer one's target.
 *
 * ## Why it wears the span's clothes and the Delete button's box
 *
 * The TEXT is the span's — accent colour, 0.82rem — because that is what a human
 * has been looking at in this corner. The BOX is `cardDeleteButtonStyle`'s
 * padding and radius, because it now has a sibling at the opposite corner and two
 * controls on one row that sit at different heights read as a mistake.
 *
 * ⚑ The label is a LITERAL, and one row is owed — see the module header.
 */
const cardPracticeLinkStyle: React.CSSProperties = {
  padding: "4px 6px",
  borderRadius: "6px",
  color: "var(--accent-primary)",
  fontFamily: "inherit",
  fontSize: "0.82rem",
  textDecoration: "none",
  // An `<a>` is inline by default, so without this it takes its height from the
  // line box rather than from its own padding and sits 3px above the button —
  // measured in the browser on 2026-09-07, which is the only way this class of
  // defect shows up.
  display: "inline-block",
  lineHeight: CONTROL_LINE_HEIGHT,
};

const ScenarioCard: React.FC<{
  scenario: ScenarioSummary;
  slug: string;
  /** Opens the delete confirmation for THIS scenario. The card never deletes
   *  anything itself — it asks the page, which owns the dialog. */
  onRequestDelete: (scenario: ScenarioSummary) => void;
}> = ({ scenario, slug, onRequestDelete }) => {
  const status = statusMeta(scenario.status);
  return (
    // ## Why Delete is a SIBLING of the link, not a child of it
    //
    // The whole card is the navigation target, so the card IS an `<a>`. A
    // `<button>` inside an `<a>` is invalid HTML, and every click on it would
    // also navigate — the "must not hijack the card link" rule, lost. Nesting
    // and then calling `preventDefault`/`stopPropagation` would paper over it:
    // the markup would still be invalid, and keyboard and middle-click paths
    // would still reach the anchor.
    //
    // So the anchor and the button are siblings inside a positioned wrapper, and
    // the button is placed over the card's bottom-right corner. Nothing to
    // suppress, because nothing overlaps: the two controls are genuinely
    // separate. This reasoning survived the kebab it was written for.
    <div style={{ position: "relative", display: "flex" }}>
      <Link
        to={scenarioPagePath(slug, scenario.id)}
        style={{
          ...scenarioCardStyle,
          textDecoration: "none",
          color: "var(--text-primary)",
          flex: 1,
          // ⚑ RESERVES the band the two absolute controls sit in. The removed
          // span held that space in FLOW — it was a real line with `marginTop:
          // auto` pushing it down — and taking it out without replacing it would
          // let a two-line title on a short card run under Delete and Practice,
          // which are positioned against the card's edge and cannot reflow. 14px
          // of the card's own bottom padding plus room for a 0.82rem control at
          // `bottom: 8px`.
          paddingBottom: "2.25rem",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
          <span
            style={{
              width: "9px",
              height: "9px",
              borderRadius: "50%",
              backgroundColor: status.color,
              flexShrink: 0,
            }}
          />
          <span style={{ fontSize: "0.72rem", color: "var(--text-muted)" }}>{status.label}</span>
        </div>
        {/* The scenario's code prefixes its name everywhere the scenario appears
            (§2a). Rendered as plain text in the existing title line rather than as
            a new chip or row: the code is part of how the scenario is NAMED, so it
            reads as "S-3 · Marie is obstructive", and no layout changes. The string
            arrives fully formatted from the backend — the browser never builds it.
            The right padding is kept: it holds the title's line length steady
            against the card's edge. */}
        <div
          style={{
            fontSize: "0.95rem",
            fontWeight: 600,
            color: "var(--text-primary)",
            paddingRight: "1.5rem",
          }}
        >
          <span style={{ color: "var(--text-muted)", fontVariantNumeric: "tabular-nums" }}>
            {scenario.code}
          </span>
          {" · "}
          {scenario.attack}
        </div>
        {/* THE PATTERN-FLAG CHIP IS GONE (ruling R2 §3, 2026-08-10; built .396).
            It read "pattern analysis pending" on every card in every state,
            because `baseless_repeat_count` is a field the backend hardcodes to
            null — the cross-document pass that would populate it is unwired. A
            chip whose text never varies is not a signal; on a page whose whole
            job is to separate what is ready from what is not, it was noise
            wearing a status pill's clothes.

            It comes back when pattern analysis has a real source, exactly as the
            two metric cards removed on 2026-07-27 will. `baseless_repeat_count`
            stays on the payload — it is a field with a defined meaning and an
            honest null; what is retired is rendering it as though it said
            something. */}
        {/* The "N instances · no speakers yet · N responses" line was removed on
            2026-08-07 with the metric that led it (see `MetricsBand`). */}
        {/* "Open scenario →" STOOD HERE and is gone (Roman, 2026-09-07). It was a
            span, not a link, and its own comment said why: "the whole card
            navigates, so this is plain text". That is exactly what made it
            redundant — an arrow announcing the click you were already making, on
            twelve cards at once. Practice took its corner, and Practice is
            somewhere the card does not otherwise go. */}
      </Link>

      {/* DELETE, VISIBLE (Roman overruled D7 for Delete on 2026-08-07).

          It replaces the ⋯ kebab that stood here, which held Delete and nothing
          else — a menu whose only item is the thing you came for is a click tax,
          and Roman asked for a button twice before getting one.

          The mis-click guard is unchanged and is the same one D7 relied on: this
          opens the page's confirm dialog, which names the scenario and stays open
          if the delete fails. What changed is that the control says what it does.

          PLACEMENT: bottom-right, diagonally opposite the card's title and as
          far as this card can put it from the Practice link at the bottom-LEFT
          — which is the corner "Open scenario →" used to hold. */}
      <div style={{ position: "absolute", bottom: "8px", right: "10px" }}>
        <button
          type="button"
          style={cardDeleteButtonStyle}
          title={`Delete ${scenario.code} — asks for confirmation first`}
          aria-label={`Delete scenario ${scenario.code}`}
          onClick={() => onRequestDelete(scenario)}
        >
          Delete
        </button>
      </div>

      {/* GO TO PRACTICE (Roman, 2026-09-07), in the corner the retired
          "Open scenario →" span held.

          A SIBLING of the card's link and not a child of it, exactly as Delete
          is, and for the reason stated at the top of this component: an `<a>`
          inside an `<a>` is invalid markup and the inner click would navigate
          twice. The two controls are the same shape at opposite corners.

          `practicePath` is the guarded builder (Law 8) — this file composes no
          path of its own, so the route-link guard covers this link the same way
          it already covered the card's. */}
      <div style={{ position: "absolute", bottom: "8px", left: "10px" }}>
        <Link
          to={practicePath(slug, scenario.id)}
          style={cardPracticeLinkStyle}
          title={`Practice ${scenario.code}`}
          aria-label={`Go to Practice for scenario ${scenario.code}`}
        >
          Go to Practice →
        </Link>
      </div>
    </div>
  );
};

export default ScenarioCard;
