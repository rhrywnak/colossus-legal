/**
 * The card anatomy, checked against RENDERED MARKUP (task 2.13b).
 *
 * ## Why this file can exist when component testing "is not set up"
 *
 * CLAUDE.md Rule 30 records that RTL + jsdom are not configured here, and that
 * remains true — nothing below mounts a component, fires an event or reads a
 * layout. `renderToStaticMarkup` is a pure function from React elements to an
 * HTML string; it needs no DOM at all. So the one check that genuinely required
 * rendered output — Roman's "the Candidate numbers appear in different places
 * cards" — can be made without standing up the infrastructure Rule 30 is about.
 *
 * `sectionsFor` already pins the anatomy as data. This pins that the RENDERER
 * honours it, which is the half a data test cannot reach: a component is free to
 * ignore the list it was handed, and only its output shows whether it did.
 */
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import FactRow, { nextTier } from "../FactRow";
import type { WorkingRow } from "../factsTable";
import type { LinkPanelWording } from "../../services/evidenceLinks";

/** Only the fields the card reads; the rest of the panel wording is irrelevant. */
const wording = {
  fact_question_label: "Q:",
  fact_statement_kind_label: "Kind:",
  fact_tier_carries_label: "Carries the scenario",
  fact_tier_backup_label: "Backup",
  fact_tier_background_label: "Background",
  fact_tier_prompt: "How much does this fact carry?",
  fact_order_drag_hint: "Drag to reorder",
  fact_remove_label: "Remove",
  fact_remove_confirm_template: "Remove {code}?",
  fact_remove_confirm_yes: "Remove it",
  fact_remove_confirm_cancel: "Keep it",
} as unknown as LinkPanelWording;

function row(over: Partial<WorkingRow> = {}): WorkingRow {
  return {
    code: "C-1",
    graphNodeId: "ev-1",
    text: "Yes.",
    bearsOn: [],
    pinpointLabel: "CFS responses at 26",
    pinpointHref: "/documents/doc-7?page=26",
    statusLabel: "In the scenario",
    isHuman: false,
    question: null,
    statementKind: null,
    tier: "backup",
    sortOrdinal: null,
    speaker: null,
    displayOrdinal: null,
    ...over,
  };
}

const markup = (r: WorkingRow) =>
  renderToStaticMarkup(<FactRow row={r} wording={wording} onSetTier={() => {}} />);

/** Where the C-code element starts, as a fraction of the card's markup. */
function codeOffset(html: string): number {
  const at = html.indexOf("data-fact-code");
  expect(at, "every fact card must carry a marked C-code element").toBeGreaterThan(-1);
  return at;
}

describe("the rendered card puts its landmark in the same place, always", () => {
  /**
   * Five cards of deliberately different shapes: bare, with a question, with a
   * long question and accusations, documentary (no speaker), and a human fact.
   * These are the combinations that used to move the code around.
   */
  const shapes: [string, WorkingRow][] = [
    ["bare", row()],
    ["with a question", row({ question: "State the basis for your contention." })],
    [
      "fully dressed",
      row({
        question: "A".repeat(400),
        bearsOn: ["¶41 — " + "B".repeat(180)],
        statementKind: "partial admission",
        speaker: "R. Phillips",
        pinpointLabel: "CFS responses at 26",
        pinpointHref: "/documents/doc-7?page=26",
      }),
    ],
    ["documentary, no speaker", row({ statementKind: "correspondence", speaker: null })],
    [
      "human fact",
      row({ isHuman: true, code: null, tier: null, pinpointLabel: "", pinpointHref: "" }),
    ],
  ];

  it("renders the C-code as the first element of the header on every card", () => {
    // The defect: the code shared a wrapping row with the kind and the quote, so
    // its position moved with the length of whatever sat beside it. It must now
    // precede the kind, the speaker, the question and the quote on every shape.
    for (const [name, r] of shapes) {
      if (r.code === null) continue; // a human fact has no candidate number
      const html = markup(r);
      const code = codeOffset(html);

      for (const later of ["Kind:", "Q:", r.text].filter(Boolean) as string[]) {
        const at = html.indexOf(later);
        if (at === -1) continue;
        expect(
          code,
          `${name}: the C-code must precede "${later.slice(0, 12)}" in the markup`,
        ).toBeLessThan(at);
      }
    }
  });

  it("gives the C-code identical styling on every card", () => {
    // "at identical position/class" — with inline styles, the class equivalent is
    // the style attribute. A card that styled its code differently would be the
    // inconsistency Roman reported, in a new form.
    const styles = new Set<string>();
    for (const [, r] of shapes) {
      if (r.code === null) continue;
      const html = markup(r);
      const at = html.indexOf("data-fact-code");
      // The style attribute of the element carrying the marker.
      const tagStart = html.lastIndexOf("<", at);
      const tagEnd = html.indexOf(">", at);
      const tag = html.slice(tagStart, tagEnd);
      styles.add(/style="([^"]*)"/.exec(tag)?.[1] ?? "");
    }
    expect(styles.size, `the C-code is styled ${styles.size} different ways`).toBe(1);

    // ANTI-VACUITY. If `data-fact-code` vanished from every card, `indexOf`
    // returns -1, `lastIndexOf("<", -1)` returns 0 in V8, and the extraction
    // above would pull the outermost CARD container's style — identical on every
    // shape, so `styles.size === 1` would pass while the marker was gone
    // entirely. Pinning a property only the code element carries is what makes
    // the size assertion mean what it says.
    const [style] = [...styles];
    expect(style, "the extracted style must be the C-code's, not the card's").toContain(
      "font-weight:600",
    );
    expect(style).toContain("color:var(--text-primary)");
  });

  it("renders header, question, quote and source in one order on every card", () => {
    // The anatomy as it actually reaches the browser. `sectionsFor` says what the
    // order should be; this proves the renderer did not reorder it.
    for (const [name, r] of shapes) {
      const html = markup(r);

      // PRESENCE first, and BEFORE any filtering. An earlier version of this
      // test filtered out the -1s and then asserted the array did not contain
      // -1, which is unconditionally true — dropping `QuoteRow` from the
      // renderer entirely would have left this green. The two unconditional
      // sections are checked by name, not by surviving a filter.
      expect(html.indexOf(r.text), `${name}: the quote must render`).toBeGreaterThan(-1);
      // The source row's landmark differs by provenance, and task 2.13c is why:
      // an evidence card is identified by its PINPOINT, while a human fact keeps
      // its provenance line. The "In the scenario" tag that used to sit on every
      // evidence row was deleted — every fact on this list is in the scenario, so
      // it carried no information.
      const sourceMark = r.isHuman ? r.statusLabel : r.pinpointLabel;
      expect(
        html.indexOf(sourceMark),
        `${name}: the source row must render`,
      ).toBeGreaterThan(-1);
      if (r.statementKind) {
        expect(html.indexOf("Kind:"), `${name}: the kind must render`).toBeGreaterThan(-1);
      }
      if (r.question) {
        expect(html.indexOf("Q:"), `${name}: the question must render`).toBeGreaterThan(-1);
      }

      // ORDER second. Only the sections this shape actually has take part, which
      // is what "an absent element's row collapses" means — but each of them was
      // just proven present above, so nothing drops out silently.
      const marks = [
        r.statementKind ? html.indexOf("Kind:") : null,
        r.question ? html.indexOf("Q:") : null,
        html.indexOf(r.text),
        html.indexOf(r.isHuman ? r.statusLabel : r.pinpointLabel),
      ].filter((i): i is number => i !== null);

      expect(
        marks,
        `${name}: sections appear out of the canonical order`,
      ).toEqual([...marks].sort((a, b) => a - b));
    }
  });

  it("draws the spine full height and the edge from the card token", () => {
    const html = markup(row());
    expect(html).toContain("align-self:stretch");
    expect(html).toContain("var(--border-card)");
  });
});

// ── Task 2.13c: the weight control, and the drag that did not take ──────────

describe("the weight control cycles through all three states", () => {
  it("goes carries → backup → background → carries", () => {
    // One control now, not three radios. The cycle must be complete and it must
    // come back round: a control you can leave a card stuck in is worse than the
    // icon pile it replaced.
    expect(nextTier("carries")).toBe("backup");
    expect(nextTier("backup")).toBe("background");
    expect(nextTier("background")).toBe("carries");
  });

  it("treats an unweighed fact as backup, so one click means something", () => {
    // A fact with no tier is `backup` by the migration's default; the control
    // must agree, or the first click on a fresh card would appear to do nothing.
    expect(nextTier(null)).toBe(nextTier("backup"));
  });

  it("visits every tier within three clicks from anywhere", () => {
    // The property that makes cycling acceptable: no state is unreachable.
    for (const start of ["carries", "backup", "background"] as const) {
      const seen = new Set<string>([start]);
      let at: ReturnType<typeof nextTier> = start;
      for (let i = 0; i < 3; i += 1) {
        at = nextTier(at);
        seen.add(at);
      }
      expect(seen.size).toBe(3);
    }
  });

  it("renders the current weight as visible text, not only a tooltip", () => {
    // Roman's ruling that a feature discloses itself on screen. The label must be
    // in the markup as text — a `title` or `aria-label` alone leaves a sighted
    // mouse user hovering three glyphs to learn what they mean.
    const html = markup(row({ tier: "carries" }));
    expect(html).toContain("Carries the scenario");
    const background = markup(row({ tier: "background" }));
    expect(background).toContain("Background");
  });
});
