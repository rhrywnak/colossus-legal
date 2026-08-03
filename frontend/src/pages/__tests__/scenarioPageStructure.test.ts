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
  "components/ScenarioKebab.tsx",
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
    // (§2.7 names both). The card's rendering moved to `CandidateCard` when
    // `CardQueue` was split for Rule 17, so that is where its pinpoint now lives.
    for (const file of ["components/CandidateCard.tsx", "components/WorkingView.tsx"]) {
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

describe("destructive actions live only in the kebab (task 1.7C, defect D7)", () => {
  it("the header tiers carry no Delete of their own", () => {
    // D7: `Delete` sat as a bare button in the middle of the 1.7B header row, one
    // mis-click from "Mark ready to rehearse".
    const header = read("components", "ScenarioHeaderTiers.tsx");
    expect(header).not.toContain("Delete</button>");
    expect(header, "the header must delegate to the kebab").toContain("ScenarioKebab");
  });

  it("no Archive action is rendered, because none exists (ruling R1)", () => {
    // Measured in Phase A: no archive endpoint, and `scenarios_status_check` allows
    // only draft | needs_evidence | ready, so there is no state to archive INTO.
    // A control that cannot act is a promise the page cannot keep — and a DISABLED
    // one would say the feature is here and broken rather than unbuilt.
    const kebab = read("components", "ScenarioKebab.tsx");
    expect(kebab).not.toContain("Archive scenario");
    expect(kebab).not.toContain('role="menuitem"\n            disabled');
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

  it("the ruling buttons dispatch the reducer's OWN key event", () => {
    // Item 2's whole point: one state machine, two input devices. A parallel click
    // path would be a second machine to keep in step, and buttons that disagree
    // with the keys are worse than no buttons.
    const queue = read("components", "CardQueue.tsx");
    expect(queue).toContain('dispatch({ type: "key", key, typing: false })');
  });

  it("cardTriage.ts is untouched by the button work", () => {
    // The 31 §7 tests test the reducer. If the reducer grew a click event, they
    // would still pass while the contract they protect had quietly widened.
    const reducer = read("components", "cardTriage.ts");
    expect(reducer).not.toContain("onRule");
    expect(reducer).not.toContain('type: "click"');
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
