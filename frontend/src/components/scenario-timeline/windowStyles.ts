// =============================================================================
// windowStyles.ts — the floating timeline window
// =============================================================================
//
// TIMELINE_SUBSET_MOCKUP_v1_2026-08-30.html Screen 1, approved as drawn —
// Roman: "the window's layout and colors ... are approved exactly". The sibling
// of `dockStyles.ts`: that is the button and the row in the header, this is the
// window they open.
//
// CONST: geometry and rhythm, not settings. There is no frontend config surface
// for a window's furniture and these are not per-deployment values — they are
// one approved drawing, transcribed. The mockup's own pixel values are in the
// comments beside each rule so a screenshot can be diffed against the drawing
// without opening the HTML.
//
// ## ⚑ THE PALETTE, ROLE BY ROLE — and what was WRONG about the T3 note here
//
// The note this replaces said "app tokens play each role" and left it there.
// That claim shipped D3: the date came out `--text-muted` grey, because a role
// nobody named is a role nobody checks. Every colour below is now matched to
// the mockup's own token by NAME and by VALUE, and the v2 mockup's light frame
// is the reference:
//
//   mockup role     mockup light   app token                value      band
//   --ink           #111827        --text-primary           #101828    same
//   --ink2          #374151        --text-secondary         #475467    same
//   --muted         #6b7280        --text-muted             #667085    same
//   --card          #ffffff        --bg-surface             #ffffff    same
//   --soft          #fafafa        --bg-page                #f4f5f7    same
//   --line          #e5e7eb        --border-default         #d0d5dd    same
//   --indigo-line   #c7d2fe        --border-default         #d0d5dd    same
//   --indigo-bg     #eef2ff        --state-info-bg-soft     #e0e7ff    same
//   --indigo-ink    #3730a3        --accent-primary         #1570ef    same
//   --amber         #b45309        --burden-warning-text    #b54708    same
//   --amber-bg      #fef3c7        --burden-warning-bg      #fef0c7    same
//   --green/--blue/--violet/--slate/--orange: NOT tokens — the tag's own stored
//     colour, read off `chronology_tags`. Measured on DEV 2026-08-31, those five
//     rows hold #059669 / #2563eb / #7c3aed / #64748b / #d97706, which are the
//     mockup's five hexes exactly. Hard-coding the mockup's would have put five
//     domain colour names into shared code for values the database already
//     serves; reading them reproduces the drawing AND obeys the standing rule.
//
// ## ⚑ EVERY LINE IN THIS FILE IS `--border-default`. THAT IS A CORRECTION.
//
// The first cut of this file drew the window's outline, the title bar's bottom
// rule and the selector's border in `--accent-primary` (#1570ef) and defended it
// in this comment as "furniture, no contrast requirement rides on it". Roman
// rejected that on 2026-08-31, and the rejection is the right reading of the
// drawing: the mockup's `--indigo-line` (#c7d2fe) is a PALE hairline, and a
// saturated blue outline is not a pale one whatever the contrast maths says.
// `--border-default` (#d0d5dd) is the token this same table already calls the
// mockup's `--line`, and it is what a hairline is for.
//
// `--accent-primary` survives in this file ONLY as `color:` — the bar's count,
// its buttons, the story note, the footer links. That is the mockup's
// `--indigo-ink`, which is a different role and correctly saturated. If a
// `border` or `borderBottom` in this file ever reads `--accent-primary` again,
// it is this defect coming back.
//
// The bar's GROUND stays `--state-info-bg-soft`: the pale indigo strip is what
// makes the title bar read as a title bar, and it was never the thing at issue.
//
// (Light theme only — ruled 2026-08-31. `tokens.css` carries one palette and
// this file matches it; there is no dark half to reconcile.)

import type { CSSProperties } from "react";

// ─── the floating window (mockup `.fw`) ─────────────────────────────────────

/** Mockup `.fw`: rounded, PALE-bordered, its own shadow, column layout. */
export const shell: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  height: "100%",
  width: "100%",
  background: "var(--bg-surface)",
  border: "1px solid var(--border-default)",
  borderRadius: "12px",
  boxShadow: "0 12px 32px rgba(17, 24, 39, 0.18)",
  overflow: "hidden",
  boxSizing: "border-box",
};

/** Mockup `.fwbar`: pale indigo, `cursor:move` — the ONLY drag handle. */
export const bar: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "0.5rem",
  padding: "0.55rem 0.75rem",
  background: "var(--state-info-bg-soft)",
  borderBottom: "1px solid var(--border-default)",
  cursor: "move",
  userSelect: "none",
};

export const barTitle: CSSProperties = {
  fontWeight: 800,
  fontSize: "0.86rem",
  color: "var(--text-primary)",
  whiteSpace: "nowrap",
  overflow: "hidden",
  textOverflow: "ellipsis",
};

export const barCount: CSSProperties = {
  color: "var(--accent-primary)",
  fontSize: "0.72rem",
  fontWeight: 600,
  whiteSpace: "nowrap",
};

export const barSelect: CSSProperties = {
  marginLeft: "0.25rem",
  fontSize: "0.72rem",
  border: "1px solid var(--border-default)",
  borderRadius: "6px",
  background: "var(--bg-surface)",
  color: "var(--text-primary)",
  padding: "0.05rem 0.25rem",
  maxWidth: "9rem",
};

export const barActions: CSSProperties = { marginLeft: "auto", display: "flex", gap: "0.25rem" };

export const barButton: CSSProperties = {
  border: "none",
  background: "transparent",
  fontSize: "0.9rem",
  cursor: "pointer",
  color: "var(--accent-primary)",
  width: "24px",
  height: "24px",
  borderRadius: "6px",
  fontFamily: "inherit",
  lineHeight: 1,
};

/** Mockup `.fwdesc`: the subset's description, serif, on a muted strip. */
export const description: CSSProperties = {
  fontFamily: "Georgia, 'Times New Roman', serif",
  fontSize: "0.81rem",
  color: "var(--text-secondary)",
  padding: "0.5rem 0.875rem",
  borderBottom: "1px solid var(--border-default)",
  background: "var(--bg-page)",
};

/** Mockup `.fwbody`: the ONLY thing that scrolls. The page beneath never moves. */
export const body: CSSProperties = {
  flex: 1,
  overflowY: "auto",
  padding: "0.625rem 0.75rem",
};

/**
 * Mockup `.ydiv`: the YEAR rule — letter-spaced caps between hairlines.
 *
 * ⚑ This replaces T3's `phaseDivider`, which took a phase colour and drew a
 * coloured phase name with no year on it. A story told in dates is organised by
 * years; the phase is the extra fact on the one rule where it changes, and
 * `subsetRows.dividerFor` decides when that is. The rule is `--ink2` in both
 * cases — one colour, so a phase change cannot read as a different KIND of
 * divider from a year change.
 */
export const yearDivider: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "0.5rem",
  // 11.5px at the app's 16px root.
  fontSize: "0.72rem",
  fontWeight: 800,
  letterSpacing: "0.12em",
  textTransform: "uppercase",
  color: "var(--text-secondary)",
  margin: "0.5rem 0 0.375rem",
};

export const dividerRule: CSSProperties = {
  flex: 1,
  height: "1px",
  background: "var(--border-default)",
};

/** Mockup `.fev`: `grid-template-columns:96px 1fr; gap:10px`. */
export function eventRow(removed: boolean): CSSProperties {
  return {
    display: "grid",
    gridTemplateColumns: "96px 1fr",
    gap: "0.625rem",
    padding: "0.5rem 0.625rem",
    border: removed ? "1px dashed var(--burden-warning-text)" : "1px solid var(--border-default)",
    borderRadius: "8px",
    marginBottom: "0.375rem",
    background: removed ? "var(--burden-warning-bg)" : "var(--bg-surface)",
    textAlign: "left",
    width: "100%",
    boxSizing: "border-box",
    cursor: "pointer",
    fontFamily: "inherit",
  };
}

/**
 * Mockup `.fev .d`: THE DATE, and it is the first and boldest thing in the row.
 *
 * ## ⚑ THIS IS DEFECT D3, BY NAME
 *
 * T3 shipped this at `0.7rem` in `--text-muted` — a grey caption beside a black
 * title. Roman's words on 08-31 were that the dates were grey. They are the
 * thing the window exists to show: 13.5px, weight 800, `--text-primary`, the
 * SAME ink the row's title uses, and never `--text-muted`. The only muted text
 * in this column is the year under it.
 *
 * `tagColor` draws the mockup's 3-px right rule — the tag's OWN stored colour,
 * so the row is colour-coded without a single domain name in this file.
 * `approximate` turns the date amber, which is a claim about the DATE and is
 * why it colours the date and nothing else in the row.
 */
export function eventDate(tagColor: string, approximate: boolean): CSSProperties {
  return {
    fontSize: "0.84rem",
    fontWeight: 800,
    color: approximate ? "var(--burden-warning-text)" : "var(--text-primary)",
    whiteSpace: "nowrap",
    lineHeight: 1.25,
    paddingTop: "1px",
    paddingRight: "0.5rem",
    borderRight: `3px solid ${tagColor}`,
  };
}

/**
 * Mockup `.fev .d small`: the year, and the ONLY muted text in the column.
 *
 * ## ⚑ IT WRAPS, AND IT USED TO PAINT OVER THE EVENT TITLE
 *
 * This line inherits `nowrap` from [`eventDate`] and, until 2026-08-31, also
 * declared its own. On a plain row that is right — "2009" is 85px of box
 * holding 30px of text and nothing can overflow. On a month- or
 * year-precision approximate row the line becomes "2009 · month · approx.",
 * which needs 130px in an 85px content box, and because the column also has
 * `overflow: visible` the surplus 45px was PAINTED ACROSS the event title.
 * Measured on "The $50,000": one row of fifteen, `scrollWidth 130` against
 * `clientWidth 85`.
 *
 * `white-space: normal` lets it break at the middle dots it already contains,
 * so the caption takes a second line inside its own column instead of leaving
 * it. The column stays 96px — the mockup drew 96 — and the row simply gets
 * taller, which rows here already do: the fact paragraph beside them is one to
 * eight lines depending on the event.
 *
 * `overflow-wrap: break-word` is the guard for a token that cannot fit even
 * alone. Nothing in the store needs it today ("approx." is 40px), but a
 * reworded caption is one migration away and a single long word would put the
 * overflow straight back.
 *
 * `line-height: 1.15` is tighter than the default so the second line costs the
 * row 13px rather than 20. The DAY line above keeps `nowrap` from
 * [`eventDate`]: "~ May 3" broken across two lines would be a different and
 * worse defect than the one this fixes.
 */
export const eventDateCaption: CSSProperties = {
  display: "block",
  fontSize: "0.69rem",
  fontWeight: 600,
  color: "var(--text-muted)",
  letterSpacing: "0.02em",
  whiteSpace: "normal",
  overflowWrap: "break-word",
  lineHeight: 1.15,
};

/**
 * Mockup `.tag`: the tag pill, tinted from the tag's own stored colour.
 *
 * The mockup pairs a saturated `--green` with a pale `--green-bg`; the database
 * serves ONE colour per tag, so the pale half is derived from it by
 * [`softTint`] rather than invented as a second stored value nobody maintains.
 * One source of truth, and a tag added tomorrow gets a matching pill for free.
 */
export function tagPill(color: string): CSSProperties {
  return {
    display: "inline-block",
    fontSize: "0.66rem",
    fontWeight: 600,
    borderRadius: "999px",
    padding: "0.05rem 0.5rem",
    marginLeft: "0.375rem",
    verticalAlign: "1px",
    color,
    background: softTint(color, 0.14),
  };
}

/**
 * A hex colour as a translucent `rgba()` — the pale ground under a tag pill.
 *
 * Pure, and exported so a test can reach it. Returns the input UNCHANGED when
 * it is not a `#rrggbb` or `#rgb` string: a tag whose stored colour is a CSS
 * keyword, a `var()`, or a typo still renders a pill, and the pill is simply
 * the flat colour rather than the component throwing on a value the database
 * was always free to hold.
 */
export function softTint(hex: string, alpha: number): string {
  const short = /^#([0-9a-f])([0-9a-f])([0-9a-f])$/i.exec(hex);
  const long = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(hex);
  const parts = short
    ? [short[1] + short[1], short[2] + short[2], short[3] + short[3]]
    : long
      ? [long[1], long[2], long[3]]
      : null;
  if (parts === null) return hex;
  const [r, g, b] = parts.map((part) => parseInt(part, 16));
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

/** Mockup `.fev h4`: 13.5px / 800 / ink, inline so the tag pill sits beside it. */
export const eventTitle: CSSProperties = {
  margin: 0,
  fontSize: "0.84rem",
  fontWeight: 800,
  color: "var(--text-primary)",
  display: "inline",
};

export const removedTitle: CSSProperties = {
  ...eventTitle,
  color: "var(--text-muted)",
  textDecoration: "line-through",
};

/**
 * Mockup `.fev p`: the fact, in the serif, at `--ink2`.
 *
 * SERIF and not the UI face, matching the mockup and the timeline page: the
 * fact is quoted material — somebody's words about what happened — and the
 * change of face is what separates it from the app's own labels around it.
 * `--text-secondary` is the mockup's `--ink2`: readable body text a shade below
 * the title, NOT `--text-muted`, which is the caption grey and would put the
 * substance of every row into the same colour as the year.
 */
export const eventFact: CSSProperties = {
  margin: "0.2rem 0 0",
  fontFamily: "Georgia, 'Times New Roman', serif",
  fontSize: "0.78rem",
  color: "var(--text-secondary)",
};

// ⚑ `dateFlag` — the mockup's `.fev .flag`, the amber "date to confirm" pill —
// was here and is REMOVED with the badge it dressed (Roman's ruling,
// 2026-08-31). The style went rather than being left unused: unlike
// `isDateToConfirm`, which he instructed be kept because a real column will
// call it, a pill nothing renders is just a pill nothing renders. `gapBadge`
// below is the one that remains, and it now has the row to itself — the two
// used to sit side by side and were deliberately drawn unalike, one filled and
// one outlined, so nobody read "the date is unsettled" as "the event is gone".

/** Mockup `.gap .badge`: the amber "not on the chronology" pill. */
export const gapBadge: CSSProperties = {
  display: "inline-block",
  fontSize: "0.62rem",
  fontWeight: 700,
  color: "var(--burden-warning-text)",
  background: "var(--bg-surface)",
  border: "1px solid var(--burden-warning-text)",
  borderRadius: "999px",
  padding: "0.05rem 0.45rem",
  marginLeft: "0.35rem",
};

/** Mockup `.fev .n`: the author's story note — italic, indigo. */
export const storyNote: CSSProperties = {
  marginTop: "0.2rem",
  fontSize: "0.72rem",
  fontStyle: "italic",
  color: "var(--accent-primary)",
};

/**
 * Mockup `.fwbar button.pop`: the ⧉ control.
 *
 * Wider than its siblings and set in text weight rather than glyph size — the
 * mockup gives it `font-size:12px; font-weight:700` where – and × are 15px.
 * It is the one control in the bar that does something the reader has to
 * choose, so it reads as a small button rather than a window ornament.
 */
export const barPopButton: CSSProperties = {
  border: "none",
  background: "transparent",
  cursor: "pointer",
  color: "var(--accent-primary)",
  minWidth: "24px",
  height: "24px",
  borderRadius: "6px",
  fontFamily: "inherit",
  fontSize: "0.75rem",
  fontWeight: 700,
  letterSpacing: "0.02em",
  lineHeight: 1,
  padding: "0 0.25rem",
};

/** Mockup `.fwfoot`. */
export const foot: CSSProperties = {
  borderTop: "1px solid var(--border-default)",
  padding: "0.4rem 0.875rem",
  fontSize: "0.7rem",
  display: "flex",
  gap: "0.875rem",
  alignItems: "center",
  background: "var(--bg-page)",
};

export const footLink: CSSProperties = {
  color: "var(--accent-primary)",
  textDecoration: "none",
  fontWeight: 600,
  background: "none",
  border: "none",
  cursor: "pointer",
  fontFamily: "inherit",
  fontSize: "0.7rem",
  padding: 0,
};

export const footCount: CSSProperties = { marginLeft: "auto", color: "var(--text-muted)" };

/**
 * Where the floating window sits in the page's stack.
 *
 * STRUCTURAL: a layer ORDER, not a setting. A deployment that could change this
 * could put the window over the app's navigation or under a modal, which is a
 * design decision and not a per-environment one.
 *
 * 40 is chosen against the layers this app already uses, surveyed 2026-08-31:
 *
 *   9999  modals and confirms (`Modal`, `ScenarioDeleteConfirm`, the pipeline
 *         dialogs) — a modal must cover the window; it is asking a question.
 *   1000  popovers and overlays (`AuthorityPopover`, `UploadDialog`, …)
 *    200  the nav dropdown, and the header's own inner strip
 *    100  the app HEADER — it must stay above the window, which is why the
 *         window opens 20px BELOW it rather than under it
 *     60  the candidate filter bar
 *     50  the subset picker modal — it edits what this window shows
 *     45  `dockStyles.attachList`, this feature's own chooser
 *  →  40  THE FLOATING WINDOW: above every page's own content and stacking
 *         context, below everything that is entitled to interrupt it
 *
 * Named here rather than left inline in `SubsetFloatingWindow` so the next
 * person choosing a layer can read the whole ladder in one place.
 */
export const WINDOW_Z_INDEX = 40;

/**
 * Mockup Screen 5 `.pip`: the popped-out window's own root.
 *
 * No border, no radius and no shadow, where the in-page `shell` has all three —
 * this IS an OS window now and the operating system draws its own frame. A
 * rounded card inside a square window is the tell of a page pretending to be a
 * window, which is the one thing Screen 5 is not.
 */
export const popoutShell: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  height: "100vh",
  width: "100%",
  background: "var(--bg-surface)",
  overflow: "hidden",
  boxSizing: "border-box",
};

/**
 * Mockup Screen 5 `.desk .pip .fwbar{cursor:default}`: the popped-out bar.
 *
 * The one difference from the in-page `bar`, and it matters: `cursor:move` on a
 * window the reader moves by its OS title bar promises a drag that does nothing
 * when they try it.
 */
export const popoutBar: CSSProperties = {
  ...bar,
  cursor: "default",
};

/**
 * The minimized bar, pinned bottom-right (§5C).
 *
 * ⚑ A FOURTH declaration in the same role as the three the follow-up named, and
 * it is changed with them. This IS the window's outline — the whole window, when
 * it is collapsed — so leaving it saturated while the open window went pale
 * would have shipped a bar that outlines itself differently from the box it
 * collapses out of. Reported so it can be objected to.
 */
export const minimizedBar: CSSProperties = {
  ...bar,
  borderRadius: "10px",
  border: "1px solid var(--border-default)",
  boxShadow: "0 8px 20px rgba(17, 24, 39, 0.16)",
  height: "100%",
  boxSizing: "border-box",
};

export const state: CSSProperties = {
  padding: "0.875rem",
  fontSize: "0.78rem",
  color: "var(--text-muted)",
};

export const errorState: CSSProperties = {
  ...state,
  color: "var(--burden-warning-text)",
  fontWeight: 600,
};

