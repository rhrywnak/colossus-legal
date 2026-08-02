/**
 * Visual-language invariants — v2 §2c, "pure white page backgrounds".
 *
 * ## What this test can and cannot prove
 *
 * It reads the source files and asserts what they DECLARE. It does not render
 * anything, and it cannot tell you what a screen looks like — the §2c visual
 * check stays a human one. What it does catch is the mistake that actually
 * happened (task 1.7A, defect D1): the app shell painted every screen with the
 * grey tint token, and did so through five Phase-1 UI tasks whose definition of
 * done claimed the §2c check. Nothing in the suite could see it, because nothing
 * in the suite looked at the shell.
 *
 * A screenshot ritual was considered and rejected: it would need a browser, a
 * baseline image and a human to approve every diff, and it would still not say
 * WHICH declaration was wrong.
 */

import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

const SRC = join(__dirname, "..", "..");

const read = (...parts: string[]) => readFileSync(join(SRC, ...parts), "utf8");

describe("§2c — the page canvas is pure white", () => {
  it("declares a canvas token, and it is white", () => {
    const tokens = read("styles", "tokens.css");
    expect(tokens).toMatch(/--bg-canvas:\s*#ffffff;/i);
  });

  it("paints the app shell with the canvas token, not the tint", () => {
    const shell = read("App.tsx");
    expect(shell).toContain("var(--bg-canvas)");
    expect(shell).not.toContain("var(--bg-page)");
  });

  /**
   * The regression guard proper. Two screens paint their own full-height
   * background on top of the shell (EvidenceExplorer and Graph), so a fix
   * applied only to the shell would have left them grey and contradicting it.
   * Any future screen that does the same must reach for the canvas token.
   *
   * Keyed on `minHeight: "100vh"` — the thing that makes a background a page
   * canvas rather than a chip, a stripe or a hover state. The tint token has 126
   * legitimate uses and this must not flag any of them.
   */
  it("has no screen painting a full-height background with the tint token", () => {
    const offenders: string[] = [];

    for (const file of tsxFilesUnder("pages")) {
      const source = read("pages", file);
      source.split("\n").forEach((line, index) => {
        const fullHeight = line.includes('minHeight: "100vh"');
        const tinted = line.includes("var(--bg-page)") || line.includes("COLORS.bgPage");
        if (fullHeight && tinted) {
          offenders.push(`pages/${file}:${index + 1}`);
        }
      });
    }

    expect(
      offenders,
      `these screens paint a full-height canvas with the tint token; §2c wants ` +
        `--bg-canvas:\n${offenders.join("\n")}`,
    ).toEqual([]);
  });

  /**
   * Anti-vacuity: the sweep above proves nothing if it is reading an empty
   * directory or the wrong one.
   */
  it("actually reads the pages it claims to sweep", () => {
    const files = tsxFilesUnder("pages");
    expect(files.length).toBeGreaterThan(20);
    expect(files).toContain("SettingsPage.tsx");
    expect(files).toContain("TrialPrepDashboardPage.tsx");
  });
});

/** Every `.tsx` directly under `src/<dir>`. */
function tsxFilesUnder(dir: string): string[] {
  return readdirSync(join(SRC, dir))
    .filter((name) => name.endsWith(".tsx"))
    .sort();
}
