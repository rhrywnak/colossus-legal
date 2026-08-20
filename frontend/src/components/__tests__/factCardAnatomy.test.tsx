/**
 * The card anatomy, checked against RENDERED MARKUP (2.13b, then ONE_CARD).
 *
 * ## Why this file can exist when component testing "is not set up"
 *
 * CLAUDE.md Rule 30 records that RTL + jsdom are not configured here, and that
 * remains true — nothing below mounts a component, fires an event or reads a
 * layout. `renderToStaticMarkup` is a pure function from React elements to an
 * HTML string; it needs no DOM at all. So the checks that genuinely require
 * rendered output — Roman's "the Candidate numbers appear in different places
 * cards", and now the one-card law itself — can be made without standing up the
 * infrastructure Rule 30 is about.
 *
 * `evidenceCardView` pins what a card SHOWS as data. This pins that the two
 * RENDERERS honour it, which is the half a data test cannot reach: a component
 * is free to ignore the view it was handed, and only its output shows whether it
 * did.
 */
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import FactRow from "../FactRow";
import { CandidateCard } from "../CandidateCard";
import type { WorkingRow } from "../factsTable";
import { cardFixture, optionsFixture } from "./cardFixtures";

const options = optionsFixture();
const wording = options.wording;

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
    card: cardFixture({ code: "C-1" }),
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
  renderToStaticMarkup(
    <FactRow row={r} wording={wording} options={options} onSetTier={() => {}} />,
  );

/** Where the C-code element starts, as a fraction of the card's markup. */
function codeOffset(html: string): number {
  const at = html.indexOf("data-fact-code");
  expect(at, "every fact card must carry a marked C-code element").toBeGreaterThan(-1);
  return at;
}

describe("the rendered card puts its landmark in the same place, always", () => {
  /**
   * Cards of deliberately different shapes — the combinations that used to move
   * the code around: bare, with a question, fully dressed, documentary (no
   * speaker), and a human fact with no card payload at all.
   */
  const shapes: [string, WorkingRow][] = [
    ["bare", row()],
    [
      "with a question",
      row({ card: cardFixture({ question: "State the basis for your contention." }) }),
    ],
    [
      "fully dressed",
      row({
        card: cardFixture({
          question: "A".repeat(400),
          speaker: "R. Phillips",
          statementKind: "partial admission",
          bearsOn: [
            {
              accusation: `A-41 — ${"B".repeat(180)}`,
              elements: ["Duty", "Breach", "Reliance", "Damages"],
              count: "Count 2 — Fraud",
            },
          ],
        }),
      }),
    ],
    [
      "documentary, no speaker",
      row({ card: cardFixture({ speaker: null, statementKind: "correspondence" }) }),
    ],
    [
      "human fact",
      row({
        isHuman: true,
        code: null,
        tier: null,
        card: null,
        pinpointLabel: "",
        pinpointHref: "",
      }),
    ],
  ];

  it("renders the C-code as the first element of the header on every card", () => {
    // The defect: the code shared a wrapping row with the kind and the quote, so
    // its position moved with the length of whatever sat beside it. It must now
    // precede the chips, the question and the quote on every shape.
    for (const [name, r] of shapes) {
      if (r.code === null) continue; // a human fact has no candidate number
      const html = markup(r);
      const code = codeOffset(html);

      for (const later of ["Q:", "A:", r.card?.quote.text ?? r.text]) {
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

  it("renders the header, the exchange and the source in one order on every card", () => {
    for (const [name, r] of shapes) {
      const html = markup(r);
      const quote = r.card?.quote.text ?? r.text;

      // PRESENCE first, and BEFORE any filtering. An earlier version of this
      // test filtered out the -1s and then asserted the array did not contain
      // -1, which is unconditionally true — dropping the quote from the renderer
      // entirely would have left this green.
      expect(html.indexOf(quote), `${name}: the quote must render`).toBeGreaterThan(-1);

      // The source row's landmark differs by provenance: an evidence card is
      // identified by its PINPOINT, while a human fact keeps its provenance line.
      const sourceMark = r.isHuman ? r.statusLabel : r.pinpointLabel;
      expect(
        html.indexOf(sourceMark),
        `${name}: the source row must render`,
      ).toBeGreaterThan(-1);

      if (r.card?.quote.question) {
        expect(html.indexOf("Q:"), `${name}: the question must render`).toBeGreaterThan(-1);
      }

      // ORDER second. Only the sections this shape actually has take part, which
      // is what "an absent element's row collapses" means — but each of them was
      // just proven present above, so nothing drops out silently.
      const marks = [
        r.card?.quote.question ? html.indexOf("Q:") : null,
        html.indexOf(quote),
        html.indexOf(sourceMark),
      ].filter((i): i is number => i !== null);

      expect(marks, `${name}: sections appear out of the canonical order`).toEqual(
        [...marks].sort((a, b) => a - b),
      );
    }
  });

  it("draws the spine full height and the edge from the card token", () => {
    const html = markup(row());
    expect(html).toContain("align-self:stretch");
    expect(html).toContain("var(--border-card)");
  });
});

// ── Piece 3: both bars PIN ───────────────────────────────────────────────────

describe("the controls do not scroll away (Piece 3)", () => {
  it("the fact card's header row is sticky", () => {
    // The acceptance surface: on a MacBook-sized window the weight picker and
    // the C-code stay on screen while the card's own body scrolls under them.
    // Asserted on the markup because the geometry is inline style, and a sticky
    // row is the one layout property whose absence looks like nothing at all.
    const html = markup(row());
    expect(html).toContain("position:sticky");
  });

  it("the candidate card's ruling bar is sticky", () => {
    const html = renderToStaticMarkup(
      <CandidateCard
        card={cardFixture({ code: "C-73" })}
        selected
        compact={false}
        onSelect={() => {}}
        onRule={() => {}}
        linkOptions={options}
        onSaveLinks={async () => {}}
        onUnlink={() => {}}
      />,
    );
    expect(html).toContain("position:sticky");
  });
});

// ── Piece 5a: the weight PICKER, not a cycling star ──────────────────────────

describe("the weight control names all three tiers before you choose", () => {
  it("offers every tier by its stored name", () => {
    // The star that cycled is gone. Its defence was that the CURRENT tier was
    // written out beside the glyph — true, and not the problem: a cycling control
    // discloses where you are and never where a click takes you, so the only way
    // to learn that one more click buries the card was to bury it. Roman was
    // moved to the background pile by a control he had signed off three days
    // earlier.
    const html = markup(row());

    for (const label of [
      wording.fact_tier_carries_label,
      wording.fact_tier_backup_label,
      wording.fact_tier_background_label,
    ]) {
      expect(html, `the picker must offer "${label}" without a click`).toContain(label);
    }
  });

  it("shows the tier this fact currently carries as the selected one", () => {
    const html = markup(row({ tier: "carries" }));
    // `renderToStaticMarkup` renders a controlled <select>'s value as `selected`
    // on the matching option, which is the one property that says which of the
    // three names is this card's rather than merely offered by it.
    const at = html.indexOf(wording.fact_tier_carries_label);
    const tagStart = html.lastIndexOf("<option", at);
    expect(html.slice(tagStart, at)).toContain("selected");
  });

  it("gives a human fact no weight control at all", () => {
    // §8: a human-authored fact is not evidence and carries no weight in the
    // scenario. A disabled picker would imply it could have one.
    const html = markup(
      row({ isHuman: true, code: null, tier: null, card: null }),
    );
    expect(html).not.toContain(wording.fact_tier_carries_label);
  });
});
