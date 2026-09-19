// =============================================================================
// ⚑ THE SETTINGS PAGE IMPORTS NOTHING THAT KNOWS WHAT APPLICATION IT IS IN
// =============================================================================
//
// The common-contract constraint on this page: it renders a wire shape, not a
// product. The areas, the blocks, their labels and their counts all arrive from
// the server, which derives them from the `*_KEYS` constants the wording blocks
// already declare. Nothing in the page tree names a case, a scenario, an
// allegation, a witness or a stored wording key.
//
// ## Why a disk test and not a review note
//
// This is the kind of rule that holds perfectly until the afternoon somebody
// needs one label and there is a constant three folders away that has it. The
// import is one line, it typechecks, it works, and the page silently stops being
// liftable into another Colossus application. Nothing but a scanner notices.
//
// ## The rule, stated exactly
//
//   * Every file of the page tree — `pages/SettingsPage.tsx` and everything
//     under `components/settings/` — may import:
//       - `react`
//       - any file inside `components/settings/`
//       - `services/settings`, and nothing else outside the folder
//   * `services/settings.ts` is that one door, so it is held narrow too: it may
//     import only the three shared fetch primitives.
//
// Type-only imports count. `import type { ScenarioCard }` carries exactly as
// much knowledge of the product as a value import does.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

/** `frontend/src`, resolved from THIS file rather than from the working dir. */
const SRC = fileURLToPath(new URL("../../../", import.meta.url));

const PAGE = resolve(SRC, "pages/SettingsPage.tsx");
const COMPONENTS = resolve(SRC, "components/settings");
const SERVICE = resolve(SRC, "services/settings");

/** The three shared fetch primitives the settings service is allowed. */
const SERVICE_ALLOWANCE = ["./api", "./auth", "./fetchUtils"];

/** Every source file of the page tree. Test files are not part of the page. */
function pageTreeFiles(): string[] {
  const out = [PAGE];
  const walk = (dir: string) => {
    for (const entry of readdirSync(dir)) {
      const path = resolve(dir, entry);
      if (statSync(path).isDirectory()) {
        if (entry !== "__tests__") walk(path);
        continue;
      }
      if (/\.tsx?$/.test(entry) && !entry.includes(".test.")) out.push(path);
    }
  };
  walk(COMPONENTS);
  return out;
}

/**
 * Every module specifier a file imports from.
 *
 * Catches `import … from "x"`, `import type … from "x"`, `export … from "x"`
 * and bare `import "x"` alike — all four bring a module in.
 */
function importsOf(source: string): string[] {
  const found: string[] = [];
  const pattern = /(?:\bfrom\s*|^\s*import\s*)["']([^"']+)["']/gm;
  for (const match of source.matchAll(pattern)) found.push(match[1]);
  return found;
}

/** Where a relative specifier lands, without its extension. */
const landsAt = (file: string, specifier: string) =>
  resolve(dirname(file), specifier);

describe("the settings page's imports", () => {
  it("finds the page tree it is supposed to police", () => {
    // Anti-vacuity: a scanner that walked an empty folder would report the rule
    // as kept about no files at all.
    const files = pageTreeFiles();
    expect(files.length).toBeGreaterThanOrEqual(6);
    expect(files).toContain(PAGE);
  });

  it("names nothing outside the folder but the settings service (M)", () => {
    // MUTATION: add `import type { ScenarioCard } from "../../services/scenarioCards"`
    // to any component and this names the file, the line and the offending
    // specifier.
    const offences: string[] = [];

    for (const file of pageTreeFiles()) {
      const source = readFileSync(file, "utf8");
      for (const specifier of importsOf(source)) {
        if (specifier === "react") continue;
        if (!specifier.startsWith(".")) {
          offences.push(`${file} imports the package "${specifier}"`);
          continue;
        }
        const target = landsAt(file, specifier);
        const inFolder = target === COMPONENTS || target.startsWith(`${COMPONENTS}/`);
        if (inFolder || target === SERVICE) continue;
        offences.push(`${file} reaches outside the page tree: "${specifier}"`);
      }
    }

    expect(offences, offences.join("\n")).toEqual([]);
  });

  it("holds the one door narrow: the settings service imports only the fetch primitives", () => {
    const source = readFileSync(`${SERVICE}.ts`, "utf8");
    const specifiers = importsOf(source).filter((s) => s !== "react");

    expect(specifiers.sort()).toEqual([...SERVICE_ALLOWANCE].sort());
  });

  it("recognises a forbidden import when one is put in front of it (M)", () => {
    // The scanner's own proof. Without this, every assertion above would pass
    // just as happily if `importsOf` had stopped matching anything at all.
    const planted = `import type { ScenarioCard } from "../../services/scenarioCards";\n`;
    const specifiers = importsOf(planted);

    expect(specifiers).toEqual(["../../services/scenarioCards"]);
    const target = landsAt(resolve(COMPONENTS, "SettingRow.tsx"), specifiers[0]);
    expect(target.startsWith(`${COMPONENTS}/`)).toBe(false);
    expect(target).not.toBe(SERVICE);
  });
});
