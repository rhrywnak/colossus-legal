/**
 * Visual-language invariants.
 *
 * ## The law this file guards CHANGED in task 1.7D
 *
 * It used to assert "the page canvas is pure white" — v2 §2c, enforced after task
 * 1.7A's defect D1. Roman's 2026-08-03 visual-language ruling (spec
 * SCENARIO_PAGE_MOCKUP_v3_2026-08-03.html, authorization ruling R2) replaces that
 * with a TINTED canvas and BORDERLESS cards on layered shadows.
 *
 * The assertions are inverted rather than deleted, because the failure mode is now
 * the mirror image and just as easy to walk into: a future reader who knows only
 * §2c will "fix" the canvas back to #ffffff, and every borderless white card on the
 * page will vanish into it. So this file now guards the PAIRING — tinted canvas AND
 * shadow-carried cards are one system, and neither half is safe to change alone.
 *
 * ## What this test can and cannot prove
 *
 * It reads the source files and asserts what they DECLARE. It does not render
 * anything, and it cannot tell you what a screen looks like — for 1.7D the visual
 * check is a screenshot comparison against the signed mockup, which is a human
 * judgement. What this catches is the mistake that actually happened (D1): the app
 * shell contradicting the ratified language on every screen, through five UI tasks
 * whose definition of done claimed the check had passed, because nothing in the
 * suite looked at the shell.
 */

import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

const SRC = join(__dirname, "..", "..");

const read = (...parts: string[]) => readFileSync(join(SRC, ...parts), "utf8");

describe("the v3 visual language — tinted canvas, borderless cards", () => {
  it("declares a canvas token, and it is the v3 TINT", () => {
    // Mockup `--canvas: #f4f5f7`. Inverted from the 1.7A assertion; see the file
    // header for the ruling chain.
    const tokens = read("styles", "tokens.css");
    expect(tokens).toMatch(/--bg-canvas:\s*#f4f5f7;/i);
  });

  it("keeps the canvas and the card treatment as ONE system", () => {
    // The pairing guard, and the reason this file still exists after the reversal.
    // A tinted canvas is only correct because cards are borderless and carried by
    // shadows; white was only correct because they had hairline borders. If someone
    // sets the canvas back to white, the borderless cards disappear into it.
    //
    // So: if the v3 shadow stacks are declared, the canvas must be tinted.
    const tokens = read("styles", "tokens.css");
    const hasShadowStacks =
      tokens.includes("--shadow-card:") && tokens.includes("--shadow-raised:");
    expect(
      hasShadowStacks,
      "the v3 shadow stacks must exist — they are what carries a borderless card",
    ).toBe(true);
    expect(
      tokens,
      "cards are carried by shadows, so the canvas MUST stay tinted — a borderless " +
        "white card on a white canvas is invisible (ruling R2)",
    ).toMatch(/--bg-canvas:\s*#f4f5f7;/i);
  });

  it("scopes the v3 palette instead of restyling every screen (ruling R1)", () => {
    // Measured in Phase A: transcribing the mockup's `:root` globally would have
    // moved tokens consumed by 93–104 files and restyled ~50 unreviewed screens.
    // App-wide adoption is task 3.14. If this selector disappears, the palette has
    // either been deleted or promoted to global — and the second needs a ruling.
    const tokens = read("styles", "tokens.css");
    expect(tokens).toContain('[data-surface="v3"]');
  });

  it("paints the app shell with the canvas token, not the tint token", () => {
    // Still meaningful after the value change: the two tokens now carry the SAME
    // value, but they mean different things. `--bg-canvas` is "the page", and a
    // screen that reads it follows any future canvas change; one that hardcodes
    // `--bg-page` (a chip/stripe tint, 126 uses) silently stops following.
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
      `these screens paint a full-height canvas with the TINT token; the page ` +
        `canvas is --bg-canvas. The two happen to share a value today, which is ` +
        `exactly why this matters: a screen reading --bg-page will not follow the ` +
        `next canvas change:\n${offenders.join("\n")}`,
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
