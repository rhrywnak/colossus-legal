// =============================================================================
// scenarioPageStructure.test.ts — what the scenario page is NOT (task 1.7B)
// =============================================================================
//
// Defect D2: a split-pane PDF viewer sat beside the ruling queue and rendered
// every page from the cited one to the end of the document, stacked. Roman's
// ruling retired it — document viewing is popup-only: a pinpoint opens the
// DEDICATED viewer at the cited page, in a new tab, where a legal page is
// actually readable.
//
// A rule like that decays the moment someone adds "just a small preview". This
// test is the fence. It reads the source and asserts what the files DECLARE —
// it does not render anything, so it cannot prove no PDF appears on screen; it
// proves no component on this page's tree imports one, which is the mistake that
// would actually be made.

import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

const SRC = join(__dirname, "..", "..");
const read = (...parts: string[]) => readFileSync(join(SRC, ...parts), "utf8");

/** Every file the scenario page pulls in, directly or one level down. */
const SCENARIO_TREE = [
  "pages/ScenarioDetailPage.tsx",
  "components/ScenarioCurationPanel.tsx",
  "components/CardQueue.tsx",
  "components/WorkingView.tsx",
  "components/AugmentationPanel.tsx",
  "components/ThemeScanPanel.tsx",
  "components/RunHistoryList.tsx",
  "components/ScenarioIdentityModal.tsx",
  "components/Modal.tsx",
];

describe("no PDF renders on the scenario page (D2)", () => {
  it("no component in the scenario tree imports a PDF viewer", () => {
    const offenders = SCENARIO_TREE.filter((file) => read(file).includes("PdfViewer"));
    expect(
      offenders,
      `these files import a PDF viewer; the scenario page is popup-only — a ` +
        `pinpoint opens the dedicated viewer in a new tab:\n${offenders.join("\n")}`,
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

    expect(importers.sort()).toEqual([
      "DocumentWorkspace.tsx",
      "DocumentWorkspaceTabs.tsx",
    ]);
  });

  it("every pinpoint opens the viewer in a NEW TAB, not in place", () => {
    // In-place navigation would lose the queue's position and the human's place
    // in a 148-card pass — the reason the pane existed in the first place.
    for (const file of ["components/CardQueue.tsx", "components/WorkingView.tsx"]) {
      expect(read(file), `${file} must open its pinpoint in a new tab`).toContain(
        'target="_blank"',
      );
    }
  });
});

describe("the page has one editor for identity (1.7B)", () => {
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
    // The augmentation panel edited `theme_statement` / `motivation` until 1.7B.
    // Two editors for one field is how one of them ends up forgotten.
    const panel = read("components", "AugmentationPanel.tsx");
    expect(panel).not.toContain("theme_statement:");
    expect(panel).not.toContain("motivation,");

    const modal = read("components", "ScenarioIdentityModal.tsx");
    expect(modal).toContain("patchFrom");
  });
});
