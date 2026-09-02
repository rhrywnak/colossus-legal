// =============================================================================
// accentHairline.test.ts — --accent-primary is INK or FILL, never a hairline
// =============================================================================
//
// Roman's standing rule, given 2026-08-31 and applied to two buttons by P3:
// `--accent-primary` may be the text colour of a control or the fill behind
// one. It may not be a 1-px border. Two "soft" outlined buttons drew exactly
// that — an accent hairline on an `--accent-bg-soft` ground — and both now take
// `--border-default` instead. The ink and the ground are unchanged, and they
// are what still distinguish a primary button from a secondary one.
//
// ## ⚑ WHY THIS IS TWO NAMED FILES AND NOT A SCANNER
//
// A repo-wide scan for `border…--accent-primary` finds around forty call sites
// today, and most are not furniture: filled buttons whose border matches their
// own fill, a drop-zone's dashed outline while a file is being dragged over it,
// hover and selection markers, 3-px accent bars down the left of a row. Those
// are state or fill, and P3 says in as many words that Roman decides them, not
// this task. A scanner written now would either fail on all of them or need a
// forty-line allowlist that nobody would keep true.
//
// So this fences the two that were ruled, by name. When Roman rules on the
// rest, this file is where the scanner belongs — and the survey it would start
// from is in the POLISH report under FINDINGS.
//
// ## What this CANNOT prove
//
// That the buttons still read as primary with a grey hairline. That is a look,
// and Roman's eye on DEV is what knows it.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

/** Source with its comments removed — both files now explain the rule in prose. */
function code(source: string): string {
  return source
    .replace(/\{\/\*[\s\S]*?\*\/\}/g, "")
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/^\s*\/\/.*$/gm, "");
}

const SRC = join(__dirname, "..", "..");
const DASHBOARD = code(readFileSync(join(SRC, "pages", "TrialPrepDashboardPage.tsx"), "utf8"));
const CREATE_FORM = code(readFileSync(join(SRC, "components", "ScenarioCreateForm.tsx"), "utf8"));

/**
 * The style block a named constant declares, up to its closing brace.
 *
 * Read per-STYLE rather than per-file because both files legitimately name
 * `--accent-primary` elsewhere — as ink on the very buttons in question. A
 * whole-file assertion would fail on the thing the rule permits.
 */
function styleBlock(source: string, name: string): string {
  const at = source.indexOf(`const ${name}`);
  if (at === -1) throw new Error(`${name} is gone — the rule it carries has no home`);
  const end = source.indexOf("};", at);
  return source.slice(at, end);
}

describe("the New scenario button (P3)", () => {
  const style = styleBlock(DASHBOARD, "newScenarioButtonStyle");

  it("draws its border in the furniture colour", () => {
    expect(style).toContain('border: "1px solid var(--border-default)"');
  });

  it("draws no accent hairline", () => {
    expect(style).not.toContain('border: "1px solid var(--accent-primary)"');
  });

  it("keeps the accent as INK and the soft ground as FILL — the rule permits both", () => {
    // The assertion that stops this test from passing on a button that lost its
    // identity altogether. Grey ink on a white ground would satisfy the line
    // above and be a different control.
    expect(style).toContain('color: "var(--accent-primary)"');
    expect(style).toContain('backgroundColor: "var(--accent-bg-soft)"');
  });
});

describe("the create form's primary button (P3)", () => {
  const style = styleBlock(CREATE_FORM, "primaryButtonStyle");

  it("draws its border in the furniture colour", () => {
    expect(style).toContain('border: "1px solid var(--border-default)"');
  });

  it("draws no accent hairline", () => {
    expect(style).not.toContain('border: "1px solid var(--accent-primary)"');
  });

  it("keeps the accent as INK and the soft ground as FILL", () => {
    expect(style).toContain('color: "var(--accent-primary)"');
    expect(style).toContain('backgroundColor: "var(--accent-bg-soft)"');
  });
});
