// =============================================================================
// scenarioPageStructure.test.ts — what the scenario page is NOT
// =============================================================================
//
// Rules that decay the moment someone adds "just a small preview" or "just a
// second Delete for convenience". These tests are the fence.
//
// They read the SOURCE and assert what the files DECLARE — they render nothing, so
// they cannot prove no PDF appears on screen; they prove no component on this
// page's tree imports one, which is the mistake that would actually be made.
// Component testing (RTL/jsdom) is not set up in this repo (CLAUDE.md Rule 30), so
// this is the available fence and it has already caught two regressions.

import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import {
  VIEWER_WINDOW_FEATURES_FOR_TEST,
  VIEWER_WINDOW_NAME_FOR_TEST,
} from "../../components/viewerWindow";

const SRC = join(__dirname, "..", "..");
const read = (...parts: string[]) => readFileSync(join(SRC, ...parts), "utf8");

/**
 * Every file the scenario page pulls in, directly or one level down.
 *
 * Rewritten for the seven §2 sections (task 1.7C). The 1.7B tree is gone with the
 * components that formed it: `ScenarioCurationPanel` and `AugmentationPanel` were
 * split into the four §2 sections below, and `RunHistoryList` became
 * `ScanHistoryTable`. All three files are deleted, not merely unmounted.
 */
const SCENARIO_TREE = [
  "pages/ScenarioDetailPage.tsx",
  "components/ScenarioHeaderTiers.tsx",
  "components/ScenarioIdentityBlock.tsx",
  "components/ScanSection.tsx",
  "components/ThemeScanPanel.tsx",
  "components/ScanControlLine.tsx",
  "components/ScanHistoryTable.tsx",
  "components/CardQueue.tsx",
  "components/CandidateCard.tsx",
  "components/RulingButtons.tsx",
  "components/ScenarioStatusControl.tsx",
  "components/ScenarioFactsSection.tsx",
  "components/WorkingView.tsx",
  "components/AddHumanFactForm.tsx",
  "components/TalkingPointsSection.tsx",
  "components/WatchListSection.tsx",
  "components/WatchListBlock.tsx",
  "components/ScenarioOrphanStrip.tsx",
  "components/ScenarioIdentityModal.tsx",
  "components/Modal.tsx",
];

describe("no PDF renders on the scenario page (D2)", () => {
  it("no component in the scenario tree imports a PDF viewer", () => {
    const offenders = SCENARIO_TREE.filter((file) => read(file).includes("PdfViewer"));
    expect(
      offenders,
      `these files import a PDF viewer; the scenario page is popup-only — a ` +
        `pinpoint opens the dedicated viewer in its own window:\n${offenders.join("\n")}`,
    ).toEqual([]);
  });

  /**
   * The viewer component itself is NOT dead — it is what the pinpoints route TO.
   * Asserting where it IS used stops someone "cleaning up" the file this task's
   * change appears to orphan.
   */
  it("keeps the dedicated viewer, which is where pinpoints now go", () => {
    const importers = readdirSync(join(SRC, "pages"))
      .filter((name) => name.endsWith(".tsx"))
      .filter((name) => read("pages", name).includes("PdfViewer"));

    expect(importers.sort()).toEqual(["DocumentWorkspace.tsx", "DocumentWorkspaceTabs.tsx"]);
  });
});

describe("pinpoints open a real WINDOW, not a tab (task 1.7C, defect D5)", () => {
  /**
   * ## This assertion is the INVERSE of the one it replaces
   *
   * Until 1.7C this file asserted `target="_blank"` was present in both surfaces —
   * the 1.7B ruling was "popup-only, in a new tab". Roman's D5 ruling supersedes
   * it: side-by-side reading against the page is the whole point of the surface,
   * and a tab hides the page.
   *
   * The fence still has to fail a bare `window.open(href)`, which opens a TAB in
   * every engine. So it asserts the features string is passed, not merely that
   * `window.open` appears somewhere.
   */
  it("both pinpoint surfaces route through the viewer-window helper", () => {
    // The queue's card and the facts table are the two places a pinpoint appears
    // (§2.7 names both). Both have since moved for Rule 17: the card's rendering
    // to `CandidateCard` when `CardQueue` was split, and the facts ROW to
    // `FactRow` when task 2.13 extracted it out of an over-long `WorkingView`.
    // The fence follows the code — what it guards is that a pinpoint never opens
    // with a bare `window.open`, not which file happens to hold the row today.
    for (const file of ["components/CandidateCard.tsx", "components/FactRow.tsx"]) {
      expect(read(file), `${file} must open its pinpoint via openViewerWindow`).toContain(
        "openViewerWindow",
      );
    }
  });

  it("the helper passes window FEATURES — a bare window.open would be a tab", () => {
    const helper = read("components", "viewerWindow.ts");
    expect(helper).toContain("width=1100");
    expect(helper).toContain("height=1000");
    // Reuse: one named window, re-navigated and re-raised, rather than 148 windows
    // over a 148-card pass.
    expect(helper).toContain("colossus-viewer");
    expect(helper).toContain(".focus()");
  });

  it("noopener is NOT in the features string, because it would break the reuse", () => {
    // Ruling R8: per the WHATWG spec `noopener` puts the new context in a separate
    // browsing-context group, so the named target cannot be found and every click
    // opens a fresh window. Reuse wins; the viewer is our own same-origin route, so
    // the reverse-tabnabbing threat noopener guards against does not apply.
    //
    // Asserted on the FEATURES VALUE, not on the file's text: the module documents
    // at length why noopener is absent, and a source-text check would fail on the
    // explanation. The value is what the browser sees.
    expect(VIEWER_WINDOW_FEATURES_FOR_TEST).not.toContain("noopener");
    expect(VIEWER_WINDOW_NAME_FOR_TEST).toBe("colossus-viewer");
  });
});

describe("the page has one editor for identity (1.7B, carried)", () => {
  it("the retired definition form is mounted nowhere", () => {
    const mounts: string[] = [];
    for (const dir of ["pages", "components"]) {
      for (const name of readdirSync(join(SRC, dir))) {
        if (!name.endsWith(".tsx") || name === "ScenarioDefinitionForm.tsx") continue;
        if (read(dir, name).includes("ScenarioDefinitionForm")) mounts.push(`${dir}/${name}`);
      }
    }
    expect(
      mounts,
      `the permanent definition form is retired; identity is edited in the ` +
        `modal:\n${mounts.join("\n")}`,
    ).toEqual([]);
  });

  it("the identity modal is the only writer of the C1 texts", () => {
    // Two editors for one theme statement is how one of them ends up forgotten.
    const modal = read("components", "ScenarioIdentityModal.tsx");
    expect(modal).toContain("patchFrom");

    // The identity BLOCK (D8) reads those texts and must never write them — it has
    // a pencil that opens the modal, and no form of its own.
    const block = read("components", "ScenarioIdentityBlock.tsx");
    expect(block).not.toContain("theme_statement:");
    expect(block).not.toContain("updateScenario");
  });
});

describe("Delete is a visible button, guarded by the dialog (D7 OVERRULED 2026-08-07)", () => {
  /**
   * ## The history this describes
   *
   * D7 (task 1.7C) moved Delete behind a ⋯ kebab, because a bare Delete had sat
   * one mis-click from "Mark ready to rehearse". Roman asked for a button twice
   * and got a menu twice; on 2026-08-07 he OVERRULED D7 for Delete. The guard is
   * the confirm dialog — it names the scenario and stays open on failure — plus
   * distance, and both are asserted below.
   *
   * These tests still earn their place under the standing law: each one fails
   * when a user-visible promise breaks (a delete with no confirmation, or a
   * destructive control back beside the status one), not when markup moves.
   */
  it("both surfaces route Delete through the page's confirm dialog", () => {
    // The mis-click guard, and the only reason a visible Delete is safe. Neither
    // surface may call the delete service itself.
    // Each surface raises its own request upward — the header's `onDelete`, the
    // card's `onRequestDelete` — and neither may call the delete service itself.
    for (const [file, callback] of [
      ["ScenarioHeaderTiers.tsx", "onDelete"],
      ["ScenarioCard.tsx", "onRequestDelete"],
    ]) {
      const source = read("components", file);
      expect(source, `${file} must raise the request, not perform it`).not.toContain(
        "deleteScenario",
      );
      expect(source, `${file} must ask the page to open the dialog`).toContain(callback);
    }
    // The page owns both the dialog and the write.
    const page = read("pages", "ScenarioDetailPage.tsx");
    expect(page).toContain("ScenarioDeleteConfirm");
    expect(page).toContain("scenarioDeleteCopy");
  });

  it("Delete is not adjacent to the status control on the header", () => {
    // The half of D7's concern that survives the overrule: "Mark ready to
    // rehearse" is the status control, and it must not sit next to the
    // destructive one. It lives in the IDENTITY row; Delete is last in the
    // ACTIONS row, behind a separator.
    const header = read("components", "ScenarioHeaderTiers.tsx");
    const statusAt = header.indexOf("<ScenarioStatusControl");
    const deleteAt = header.indexOf("deleteButtonStyle}");
    expect(statusAt, "the status control is still on the header").toBeGreaterThan(-1);
    expect(deleteAt, "Delete is still on the header").toBeGreaterThan(-1);
    expect(header.slice(statusAt, deleteAt)).toContain("actionsRowStyle");
    expect(header).toContain("actionSeparatorStyle");
  });

  it("the kebab is deleted from the tree, not merely unmounted", () => {
    // It held Delete and nothing else, so nothing consumes it after the two
    // buttons landed. Asserting its ABSENCE FROM DISK is what stops it being
    // restored by someone tidying an import — the same rule the three retired
    // 1.7B panels are held to below.
    const present = readdirSync(join(SRC, "components")).filter(
      (name) => name === "ScenarioKebab.tsx",
    );
    expect(
      present,
      "ScenarioKebab.tsx has no consumer since Roman overruled D7 for Delete",
    ).toEqual([]);
  });
});

describe("the 1.7B panels are gone, not just unmounted (task 1.7C)", () => {
  /**
   * `ScenarioCurationPanel`, `AugmentationPanel` and `RunHistoryList` were split
   * into the §2 sections and deleted. Asserting their ABSENCE FROM DISK — rather
   * than merely that nothing imports them — is what stops the split being undone by
   * someone restoring "the panel that had everything in it".
   */
  it("the three retired panels are deleted from the tree", () => {
    const present = readdirSync(join(SRC, "components")).filter((name) =>
      ["ScenarioCurationPanel.tsx", "AugmentationPanel.tsx", "RunHistoryList.tsx"].includes(
        name,
      ),
    );
    expect(
      present,
      `these files were split into the §2 sections and must stay deleted:\n${present.join("\n")}`,
    ).toEqual([]);
  });

  it("nothing in the tree still imports them", () => {
    const offenders = SCENARIO_TREE.filter((file) => {
      const source = read(file);
      return (
        source.includes('from "./ScenarioCurationPanel"') ||
        source.includes('from "./AugmentationPanel"') ||
        source.includes('from "./RunHistoryList"')
      );
    });
    expect(offenders).toEqual([]);
  });
});

describe("the v3 visual language (task 1.7D)", () => {
  it("the queue title row is NOT a toggle — only the chevron collapses", () => {
    // Item 4, from Roman's first real session: 1.7C used `<details>/<summary>`,
    // which makes the WHOLE head row a toggle, so clicking the count or the empty
    // space beside it folded the queue away mid-triage. A `<details>` cannot be
    // made partly clickable, so the region became a head row plus a conditional
    // body — and this fence stops someone reinstating the shorter version.
    const section = read("components", "ScanSection.tsx");
    expect(section).not.toContain("<details");
    expect(section).not.toContain("<summary");
    expect(section, "the chevron is the only collapse control").toContain("chevronStyle");
  });

  it("the labelling law survives the copy split", () => {
    // `queueRegion` shortened its head-line clause to "from all scans"; the
    // add/drain sentence moved to the section subtitle. Both halves must exist
    // SOMEWHERE, or a human will rerun a scan expecting the pile to reset.
    const section = read("components", "ScanSection.tsx");
    expect(section).toContain("rulings drain them");
    expect(section).toContain("rerunning never removes");
  });

  it("no card in the scenario tree carries a border (v3 is borderless)", () => {
    // The v3 ruling: cards are white surfaces on layered shadows. A hairline
    // border on top of that shadow reads as a double edge, which is the thing the
    // ruling removed. Checks the CARD shells, not inputs or internal dividers.
    const offenders: string[] = [];
    for (const file of ["components/CandidateCard.tsx", "components/ScenarioIdentityBlock.tsx"]) {
      const source = read(file);
      // A card shell declares a radius and a shadow; it must not also declare a
      // border on the same object.
      if (/border:\s*HAIRLINE/.test(source) || /border:\s*DIVIDER/.test(source)) {
        offenders.push(file);
      }
    }
    expect(offenders, `these card shells still carry a border:\n${offenders.join("\n")}`).toEqual(
      [],
    );
  });

  it("a ruling button NAMES its card — no shared index in the click path", () => {
    // THE 1.7G FENCE (ruling R1). This assertion used to require the opposite:
    // `dispatch({ type: "key", key, typing: false })`, a click event with no target
    // that the reducer resolved through `state.cards[state.index]`. That shared
    // index was the beta.369 defect — every button in a 148-card list aimed at
    // whatever happened to be selected. If someone restores the untargeted
    // dispatch, this fails.
    const queue = read("components", "CardQueue.tsx");
    expect(queue).toContain('dispatch({ type: "rule", key, graphNodeId })');
    expect(queue).not.toContain('dispatch({ type: "key", key, typing: false })');

    // …and the id comes from the card's own render scope, not from the queue.
    const list = read("components", "CandidateList.tsx");
    expect(list).toContain("onRule(key, card.graph_node_id)");
  });

  it("a ruling button does not also select its card", () => {
    // The second half of the same defect: the button sits inside a card whose
    // onClick selects it, so one physical click produced two dispatches and the
    // second destroyed the auto-advance. Removing this `stopPropagation` puts the
    // human back at the top of the list after every mouse ruling.
    const buttons = read("components", "RulingButtons.tsx");
    expect(buttons).toContain("event.stopPropagation()");
  });

  it("every card in the list carries its own controls", () => {
    // Ruling R1: the acceptance test clicks the LAST card's Include without
    // selecting it first, which is only possible if that card HAS an Include.
    // `onRule={selected ? onRule : undefined}` is what made 147 of 148 rows inert.
    const list = read("components", "CandidateList.tsx");
    expect(list).not.toContain("selected ? onRule : undefined");
  });

  it("cardTriage.ts knows what a ruling targets, but nothing about the DOM", () => {
    // The reducer had to learn that a ruling names its card (1.7G) — that is state
    // machine business, and its tests cover it. What it must still not learn is
    // chrome: no component callbacks, no click plumbing, no element concepts, or
    // the §7 contract stops being testable without a browser.
    const reducer = read("components", "cardTriage.ts");
    expect(reducer).not.toContain("onRule");
    expect(reducer).not.toContain('type: "click"');
    expect(reducer).not.toContain("document.");
  });

  it("the status toggle button is gone, replaced by the segmented control", () => {
    // Item 3 / ruling R4. A button states the ACTION available, not the state you
    // are in, and the reader had to invert it to learn the status.
    const header = read("components", "ScenarioHeaderTiers.tsx");
    expect(header).not.toContain("Mark ready to rehearse");
    expect(header).not.toContain("Remove from rehearsal");
    expect(header).toContain("ScenarioStatusControl");
  });

  it("colour never stands alone on the ruling buttons or the status control", () => {
    // The mockup's stated accessibility rule: every coloured control carries an
    // icon and (where it decides something) a word, so the meaning survives
    // colourblindness and a greyscale print.
    // The buttons moved to their own module when CandidateCard was split for
    // Rule 17; the fence follows them.
    const buttons = read("components", "RulingButtons.tsx");
    for (const glyph of ["✓", "✕", "⏸", "↩"]) {
      expect(buttons, `the ruling buttons must carry the ${glyph} glyph`).toContain(glyph);
    }
    expect(buttons).toContain('label: "Include"');
    expect(buttons).toContain('label: "Exclude"');

    const control = read("components", "ScenarioStatusControl.tsx");
    expect(control, "the active segment carries a ● glyph, not just a fill").toContain("●");
  });
});

describe("nothing fake is rendered (the Phase-1 law, §1 and §6)", () => {
  /**
   * The design reserves placement for eight future components and requires each to
   * render NOTHING until its task lands. The temptation is a greyed placeholder or
   * a "coming in Phase 2" hint, and both are worse than an absence: they tell a
   * human the feature is here and broken.
   *
   * This checks the phrases a placeholder would carry. It cannot prove the absence
   * of every conceivable stub, but it catches the one that would actually be
   * written — the mockup's own `phase-tag` chips, copied across from the picture
   * into the page.
   */
  it("no phase-tag placeholder copy reached the page", () => {
    const forbidden = ["Phase 2", "Phase 3", "phase-tag", "coming soon", "Coming soon"];
    const offenders: string[] = [];
    for (const file of SCENARIO_TREE) {
      const source = read(file);
      for (const phrase of forbidden) {
        // A COMMENT may legitimately say "task 2.3" or "Phase 2"; a rendered string
        // may not. Only flag the phrase inside JSX text or a quoted string.
        if (source.includes(`>${phrase}`) || source.includes(`"${phrase}`)) {
          offenders.push(`${file}: ${phrase}`);
        }
      }
    }
    expect(
      offenders,
      `future-phase components render NOTHING (absent, not fake):\n${offenders.join("\n")}`,
    ).toEqual([]);
  });

  it("the readiness verdict slot renders nothing until 2.4 computes one", () => {
    // `headerDescriptor` returns `readiness: null` and the header guards on it. A
    // verdict is a claim about whether this scenario can be taken into a courtroom;
    // a grey "Unknown" chip would be the page making that claim with no basis.
    expect(read("components", "scenarioHeader.ts")).toContain("readiness: null");
    expect(read("components", "ScenarioHeaderTiers.tsx")).toContain("header.readiness &&");
  });
});

describe("the live facts update and the summary override (task 1.7F)", () => {
  it("the two refresh keys stay TWO, and the ruling bumps the narrow one", () => {
    // RULING R6. One page-wide key re-reads all four endpoints AND feeds
    // `externalRefresh` into the card queue, which reloads its pool and
    // dispatches `cards_loaded` — mid-triage, on every include. That would
    // disturb the selection task 1.7G spent two builds fixing. So a ruling bumps
    // the CARDS-ONLY key, and the obvious "simplification" of merging them back
    // into one fails here.
    const page = read("pages", "ScenarioDetailPage.tsx");
    expect(page).toContain("pageRefreshKey");
    expect(page).toContain("cardsRefreshKey");
    expect(page, "a confirmed ruling re-reads the cards alone").toContain(
      "onRulingSaved={refreshCards}",
    );
    expect(
      page,
      "the queue's externalRefresh must stay on the PAGE key — pointing it at the " +
        "cards key would reload the queue on every ruling, which is what the split avoids",
    ).toContain("externalRefresh={pageRefreshKey}");
  });

  it("the cards-only re-read goes through a key-bumped effect", () => {
    // So the cancelled-flag protection against out-of-order responses is
    // inherited rather than reimplemented: two rulings in quick succession start
    // two reads, and the slower one must not paint the older pool last.
    const page = read("pages", "ScenarioDetailPage.tsx");
    expect(page).toContain("}, [slug, scenarioId, cardsRefreshKey]);");
    expect(page, "the re-read effect guards against a stale response").toContain(
      "if (cancelled) return;",
    );
  });

  it("NO OPTIMISTIC ROWS: the fact appears only after the server confirms", () => {
    // RULING R3, and the 1.3 law behind it — optimistic rows over a swallowed
    // save failure could have shown fifty rulings that were never recorded. The
    // callback fires from the resolve handler of the write, never beside it.
    //
    // Asserted as ORDER rather than as a literal arrow (2026-08-08): the success
    // handler gained a second call — the ruling's acknowledgment — and pinning
    // its exact formatting made this test fail for a change that left the law
    // untouched. What the law actually says is that `onRulingSaved` appears
    // AFTER the write is issued and inside its resolve handler, never before.
    const hook = read("components", "useQueueReducer.tsx");
    const issued = hook.indexOf("applyFactAction(slug, scenarioId,");
    const confirmed = hook.indexOf("onRulingSaved()", issued);
    expect(issued, "the ruling write is issued in this hook").toBeGreaterThan(-1);
    expect(
      confirmed,
      "onRulingSaved must fire from the write's resolve handler, not beside it",
    ).toBeGreaterThan(issued);
    // Two-argument `then`, so an exception inside the success callback is not
    // reported to the human as a failed ruling.
    expect(hook).not.toContain(".catch(\n");
  });

  it("the override never composes case vocabulary in the browser", () => {
    // The badge's words arrive composed ("System", "roman · 4 Aug 2026"). The
    // ICONS are the list's own control vocabulary, exactly like the state chip's.
    // A date formatted here would read differently per locale, which is not a
    // property a legal record may have.
    const line = read("components", "QuestionLine.tsx");
    expect(line).toContain("authorship.label");
    for (const forbidden of ["toLocaleDateString", "toLocaleString", "Intl.DateTimeFormat"]) {
      expect(line, `${forbidden} would compose the badge's date in the browser`).not.toContain(
        forbidden,
      );
    }
  });

  it("a correction re-reads rather than patching the card in place", () => {
    // The question shown is composed server-side from the graph's sentence and
    // the override table. Rebuilding that composition in the browser would be the
    // client deciding how authorship reads.
    const queue = read("components", "CardQueue.tsx");
    expect(queue).toContain("await saveQuestionOverride(slug, graphNodeId, text);");
    expect(queue).toContain("await load();");
  });

  /**
   * The source between two markers, or a loud failure.
   *
   * `String.indexOf` returns -1 for a marker that is not there, and
   * `slice(-1, n)` then yields a one-character string — on which every
   * `not.toContain` assertion passes trivially. That is a guard that silently
   * stops guarding the day somebody renames a function, which is the one failure
   * mode a structure test must not have.
   */
  function between(source: string, start: string, end: string): string {
    const from = source.indexOf(start);
    const to = source.indexOf(end, from + start.length);
    if (from === -1) throw new Error(`marker not found: ${start}`);
    if (to === -1) throw new Error(`marker not found after ${start}: ${end}`);
    return source.slice(from, to);
  }

  /**
   * One self-closing JSX element with its props, brace-aware.
   *
   * A naive scan to the first `/>` breaks on an inline JSX prop value — 
   * `icon={<X />}` would end the element early, and the assertions below would
   * then pass while inspecting half the props. Tracking `{}` depth means a `/>`
   * inside an expression value is skipped, so this ends at the element's own
   * self-close or throws.
   */
  function jsxElement(source: string, name: string): string {
    const from = source.indexOf(`<${name}`);
    if (from === -1) throw new Error(`element not mounted: <${name}`);
    let depth = 0;
    for (let i = from; i < source.length - 1; i += 1) {
      const c = source[i];
      if (c === "{") depth += 1;
      else if (c === "}") depth -= 1;
      else if (depth === 0 && c === "/" && source[i + 1] === ">") {
        return source.slice(from, i + 2);
      }
    }
    throw new Error(`<${name}> is not self-closing, or never closes`);
  }

  it("removing an EVIDENCE fact uses the cards-only refresh, not the page one", () => {
    // Measured on DEV (beta.374): wiring this to the page-level refresh collapsed
    // the candidate queue region on every removal, throwing the human out of the
    // list they were working. `ScenarioDetailPage` already states the rule — the
    // page refresh is "wrong after a ruling: it would disturb the queue's
    // selection mid-triage" — and a removal IS a ruling, ledgered through
    // `record_removal`.
    const page = read("pages", "ScenarioDetailPage.tsx");
    const section = jsxElement(page, "ScenarioFactsSection");

    expect(section).toContain("onFactRemoved={refreshAfterRemoval}");
    expect(section).toContain("onChanged={refresh}");
    // The defect itself, named: the heavy refresh must never be the one a
    // removal fires.
    expect(section).not.toContain("onFactRemoved={refresh}");

    const facts = read("components", "ScenarioFactsSection.tsx");
    const removeFact = between(facts, "const removeFact", "const included");
    expect(removeFact).toContain("onFactRemoved()");
    expect(removeFact).not.toContain("onChanged()");
  });

  it("…and a HUMAN fact still uses the page-level one, which is the other half", () => {
    // The symmetric guard. Human facts live in the augmentation payload, which
    // ONLY the page-level read fetches — so "simplifying" both callbacks onto the
    // light one would leave additions and deletions apparently accepted and
    // invisible, with nothing failing. Without this assertion the fence the
    // commit describes is only half built.
    const facts = read("components", "ScenarioFactsSection.tsx");

    const removeHumanFact = between(facts, "const removeHumanFact", "const removeFact");
    expect(removeHumanFact).toContain("onChanged()");
    expect(removeHumanFact).not.toContain("onFactRemoved()");

    // The add path is the same contract, reached through the form's onSaved.
    const addForm = jsxElement(facts, "AddHumanFactForm");
    expect(addForm).toContain("onChanged()");
    expect(addForm).not.toContain("onFactRemoved()");
  });

  it("a removal reloads the QUEUE's pool too, or the two surfaces disagree", () => {
    // The second half of the same defect, and the one that is invisible: the
    // facts list is fed by the page's `cards` state, while the queue's counts
    // are fed by `CardQueue`'s OWN fetch, keyed on `externalRefresh`. Bumping
    // only the page's key left the card gone from the facts list and still
    // "included" in the queue, with nothing on screen saying which was right.
    //
    // Reloading the pool is safe: `cards_loaded` preserves the human's place,
    // and a removal changes a card's status without changing pool membership.
    const page = read("pages", "ScenarioDetailPage.tsx");

    const scan = jsxElement(page, "ScanSection");
    expect(scan).toContain("externalRefresh={pageRefreshKey + queueRefreshKey}");

    const removal = between(page, "const refreshAfterRemoval", "}, []);");
    expect(removal).toContain("setCardsRefreshKey");
    expect(removal).toContain("setQueueRefreshKey");

    // And it is still NOT the page-level read, which is what collapsed the
    // queue's region on beta.374.
    expect(removal).not.toContain("setPageRefreshKey");
  });

  it("the two refresh keys are still two, and still say why", () => {
    // The seam that has now produced two defects in two builds. The keys are
    // named apart deliberately; collapsing them is the one-line "simplification"
    // that reintroduces the disturbance, and the comment saying so is what a
    // future reader has to meet before doing it.
    const page = read("pages", "ScenarioDetailPage.tsx");
    expect(page).toContain("const refresh = useCallback(() => setPageRefreshKey");
    expect(page).toContain("const refreshCards = useCallback(() => setCardsRefreshKey");
    expect(page).toMatch(/wrong after a ruling/);
  });
});

// ── Task 2.13: the server is the one source of truth for weight and order ────

describe("weight and order live on the server, not in the page (task 2.13)", () => {
  /**
   * ## Why this fence exists
   *
   * The tempting shortcut is to paint the star immediately and write in the
   * background — it feels faster. It also puts a SECOND copy of the weight in
   * this component, and the two disagree the moment a write fails: a star lit for
   * a judgment that was never stored, with nothing on screen saying so. The same
   * argument applies to order, where the wrong copy is even harder to notice
   * because a list simply looks like a list.
   *
   * So both writes go: call the route, then re-read the cards. These assertions
   * are the fence that keeps it that way, in the two files that could break it.
   */
  it("neither the section nor the view holds its own tier or order state", () => {
    for (const file of ["components/ScenarioFactsSection.tsx", "components/WorkingView.tsx"]) {
      const source = read(file);
      // A `useState` seeded from a row's tier or ordinal is the shape of the
      // defect: a local copy of something the payload already carries.
      expect(source, `${file} must not keep a local copy of a fact's tier`).not.toMatch(
        /useState[^\n]*\b(tier|Tier)\b/,
      );
      expect(source, `${file} must not keep a local copy of a fact's order`).not.toMatch(
        /useState[^\n]*\b(sortOrdinal|sort_ordinal)\b/,
      );
    }
  });

  it("both curation writes are followed by a cards re-read", () => {
    const source = read("components/ScenarioFactsSection.tsx");
    // `onFactRemoved` is the LIGHT refresh (cards only) — the one that updates
    // this list and the queue's counts together without disturbing the queue's
    // selection mid-triage. A write that did not re-read would leave the human
    // looking at the state from before their own edit.
    for (const [write, label] of [
      ["setFactTier(slug", "the weight write"],
      ["setFactOrder(slug", "the order write"],
    ]) {
      // Anchored on the CALL, not the import — the import is the first match and
      // proves nothing about what follows the request.
      const at = source.indexOf(write);
      expect(at, `${label} must be called from the facts section`).toBeGreaterThan(-1);
      // The re-read has to appear inside the promise chain that follows the call,
      // not merely somewhere in the file.
      const chain = source.slice(at, at + 600);
      expect(chain, `${label} must re-read the cards after it lands`).toContain(
        "onFactRemoved()",
      );
    }
  });

  it("the browser never computes a stored ordinal", () => {
    // Rule 12, at its sharpest here: the ORDINAL is the server's, derived from
    // what is stored. The browser names the two neighbours a row was dropped
    // between and nothing else — a page that did the arithmetic would write a
    // position derived from a list that may have changed underneath it.
    for (const file of [
      "components/factsTable.ts",
      "components/WorkingView.tsx",
      "components/ScenarioFactsSection.tsx",
      "services/scenarioFactCuration.ts",
    ]) {
      expect(read(file), `${file} must not carry the ordinal step`).not.toContain("1024");
    }
  });
});

// ── Task 2.13b: the visual rules that decay silently ────────────────────────

describe("the facts cards keep their visual rhythm (task 2.13b)", () => {
  /**
   * Roman's 2026-08-05 note, the part a test can hold: "it is difficult to see
   * where one card ends and the next starts." The ruling is that PROXIMITY does
   * the separating — the inter-card gap decisively larger than any gap inside a
   * card — and the hairline assists. Two numbers three files apart drift back
   * together the first time somebody tidies a layout, so the RATIO is the fence.
   */
  it("separates cards by more than three times the largest gap inside one", () => {
    // The scale moved to `factRowStyles` in 2.13c, when FactRow was split for
    // the 300-line limit. Keeping the whole scale in one module is also what
    // makes "two text sizes, total" checkable by reading a file.
    const source = read("components/factRowStyles.ts");
    const gap = Number(/CARD_GAP_PX = (\d+)/.exec(source)?.[1]);
    const intra = Number(/MAX_INTRA_GAP_PX = (\d+)/.exec(source)?.[1]);

    expect(gap, "CARD_GAP_PX must be declared in factRowStyles").toBeGreaterThan(0);
    expect(intra, "MAX_INTRA_GAP_PX must be declared in factRowStyles").toBeGreaterThan(0);
    expect(gap / intra).toBeGreaterThanOrEqual(3);
  });

  it("draws the card hairline from the card token, not the divider token", () => {
    // `--border-default` on this surface is #eef0f3 (1.14:1) — a divider between
    // rows inside one surface, which is what the cards used to be. A card's edge
    // is a different job and now has its own token at 1.60:1. Reverting to the
    // divider is the exact regression Roman reported.
    const source = read("components/FactRow.tsx") + read("components/factRowStyles.ts");
    expect(source).toContain("var(--border-card)");
    expect(
      source,
      "a fact card must not draw its own edge with the row-divider token",
    ).not.toContain("var(--border-default)");
  });

  it("keeps one body size and one bold on the card", () => {
    // Study §2, binding: one Inter-class stack at regular weight, ONE body size
    // for quotes, metadata a size step DOWN rather than faded, and bold reserved
    // for the C-code. Ad-hoc per-element font styling is what this replaces, so
    // the sizes must come from the two named constants and nowhere else.
    // All three files the card is now built from — the scale, the parts and the
    // shell — so splitting a component cannot smuggle a third size back in.
    const source =
      read("components/FactRow.tsx") +
      read("components/FactRowParts.tsx") +
      read("components/factRowStyles.ts");
    const literalSizes = source.match(/fontSize: "(?!var)[^"]+"/g) ?? [];
    expect(
      literalSizes,
      `fact-card font sizes must use BODY_SIZE/META_SIZE, found ${literalSizes.join(", ")}`,
    ).toEqual([]);

    // Exactly one bold declaration beyond the two semibold labels (the C-code and
    // the "Q:" marker) would mean a third thing competing to be the landmark.
    const bolds = source.match(/fontWeight: [6-9]00/g) ?? [];
    expect(bolds.length, "only the C-code and the Q: label may be bold").toBeLessThanOrEqual(2);
  });

  it("runs the cut-spine the full height of the card", () => {
    // It used to stop at a `minHeight`, leaving a stub beside taller cards that
    // read as an alignment bug rather than a cue.
    const source = read("components/FactRow.tsx");
    expect(source).toContain("alignSelf: \"stretch\"");
    expect(source, "a fixed minHeight would cut the spine short").not.toMatch(
      /spineStyle[\s\S]{0,400}minHeight/,
    );
  });
});

// ── Task 2.13c: the two drag fixes, both of which fail silently ─────────────

describe("a drag can actually start and can actually land (task 2.13c)", () => {
  /**
   * Roman dragged a card with a real mouse and nothing happened — no error, no
   * request. Two independent causes, and BOTH fail in total silence, which is
   * why each gets a fence rather than a comment.
   */
  it("sets drag data, without which Firefox cancels the drag outright", () => {
    // Chrome starts a drag on a `draggable` element whether or not `dragstart`
    // sets data; Firefox does not — it cancels, with no event and nothing on
    // screen. That is why this worked under test and not for him.
    const source = read("components/FactRow.tsx");
    expect(
      source,
      "dragstart must set data or Firefox silently refuses the drag",
    ).toContain("dataTransfer");
    expect(source).toMatch(/setData\(/);
  });

  it("gives the space between cards to a card, not to the container", () => {
    // 2.13b separated the cards with a flex `gap` on the scroll region. A flex
    // gap belongs to the CONTAINER, which has no drop handler — so every seam
    // became 20px of dead zone, and the seam is exactly where you aim to put a
    // card BETWEEN two others. The space is now each card's bottom margin, so it
    // is part of that card's drop target.
    const row = read("components/FactRow.tsx");
    expect(row, "the card must own its separating space").toContain("marginBottom");

    const view = read("components/WorkingView.tsx");
    const region = /factsScrollRegionStyle[\s\S]{0,900}?\};/.exec(view)?.[0] ?? "";
    expect(region, "the scroll region must not re-introduce a flex gap").not.toMatch(
      /\n\s*gap:/,
    );
  });

  it("keeps the queue's counts off the queue component", () => {
    // The latch: `CardQueue` reported counts from an effect on first render,
    // before its own fetch resolved, and the resulting collapse unmounted it so
    // the real counts never arrived. The section now derives them from the page's
    // pool, so a re-introduced `onProgress` would restore the bug.
    const scan = read("components/ScanSection.tsx");
    expect(scan).toContain("progressFromCards");
    expect(
      scan,
      "the queue must not report its own counts upward again",
    ).not.toContain("onProgress");
  });
});

// ── 2.13c amendment: a card must be draggable past the visible window ───────

describe("the facts region scrolls itself during a drag", () => {
  it("drives the auto-scroll from the scroll region, not from a card", () => {
    // Roman, real mouse on .378: seam-drops landed correctly, but the list
    // scrolls in its own region and nothing moved that region mid-drag — so a
    // card could only go as far as the visible window, and the scrollbar is
    // unreachable while holding one. The handlers must sit on the REGION: a card
    // cannot scroll a container it does not own, and the cursor leaves the cards
    // entirely when it enters the edge band.
    const view = read("components/WorkingView.tsx");
    expect(view).toContain("useDragAutoScroll");
    expect(view, "the region must receive the ref that gets scrolled").toContain(
      "ref={autoScroll.regionRef}",
    );
  });

  it("stops the frame loop on every way a drag can end", () => {
    // A loop left running scrolls a region nobody is dragging over — and after an
    // unmount, a detached node forever. Drop, dragend and leaving the region are
    // three genuinely different endings and all three must stop it.
    const view = read("components/WorkingView.tsx");
    for (const ending of ["onDrop=", "onDragEnd=", "onDragLeave="]) {
      expect(view, `${ending} must end the auto-scroll`).toContain(ending);
    }
    // The hook itself must clean up on unmount, which no component-level handler
    // can do for it.
    expect(read("components/dragAutoScroll.ts")).toContain("useEffect(() => stop");
  });
});
