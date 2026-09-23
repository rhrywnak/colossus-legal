// envBanner.test.tsx — which machine, what words, and what the shell draws.
//
// The rule is the part a witness's afternoon depends on, so it is a table. The
// markup tests are static renders (rule 30: no DOM tier in this project).

import { renderToStaticMarkup } from "react-dom/server";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

import EnvBanner from "../EnvBanner";
import PrintWarningFrame from "../PrintWarningFrame";
import { FALLBACK_TEXT, bannerText, isTestSystem } from "../testSystem";
import { PRINT_CSS } from "../practice/printStyles";

/** The migration that seeds the bar's words. */
const MIGRATION = join(
  __dirname,
  "../../../../backend/pipeline_migrations/20260922142803_env_banner_wording.sql",
);

/** The value the migration INSERTs for one key. */
function seeded(key: string): string {
  const sql = readFileSync(MIGRATION, "utf8");
  const at = sql.indexOf(`('${key}', '`);
  if (at < 0) throw new Error(`${key} is not seeded by the migration`);
  const from = at + `('${key}', '`.length;
  // Values carry doubled quotes for an apostrophe, exactly as Postgres reads them.
  let out = "";
  for (let i = from; i < sql.length; i++) {
    if (sql[i] !== "'") {
      out += sql[i];
      continue;
    }
    if (sql[i + 1] === "'") {
      out += "'";
      i++;
      continue;
    }
    return out;
  }
  throw new Error(`${key}'s value is unterminated`);
}

describe("which machine this is", () => {
  it("shows the warning on anything that is not exactly the production token", () => {
    for (const value of [
      undefined,
      null,
      "",
      "   ",
      "dev",
      "unknown",
      "production",
      "PROD",
      "prod-2",
      "colossus-legal-dev.cogmai.com",
      "v2.2.1",
    ]) {
      expect(isTestSystem(value as string | undefined)).toBe(true);
    }
  });

  it("shows nothing on the real system, and tolerates a stray newline", () => {
    expect(isTestSystem("prod")).toBe(false);
    expect(isTestSystem("prod\n")).toBe(false);
    expect(isTestSystem(" prod ")).toBe(false);
  });
});

describe("the warning's words", () => {
  // ⚑ The whole point of the fallback: it must BE the stored sentence.
  it("ships a fallback that is word-for-word the migration's env_banner_text", () => {
    expect(FALLBACK_TEXT).toBe(seeded("env_banner_text"));
  });

  it("prefers the stored words and falls back when they are absent or blank", () => {
    expect(bannerText("Stored words.")).toBe("Stored words.");
    expect(bannerText(null)).toBe(FALLBACK_TEXT);
    expect(bannerText("   ")).toBe(FALLBACK_TEXT);
  });
});

/** Render the bar as a browser on `environment` would. */
function drawBar(environment: string | undefined): string {
  const original = globalThis.window;
  // @ts-expect-error — a minimal stand-in for the runtime config
  globalThis.window = { __COLOSSUS_CONFIG__: environment === undefined ? {} : { environment } };
  try {
    return renderToStaticMarkup(<EnvBanner />);
  } finally {
    globalThis.window = original;
  }
}

describe("what the shell draws", () => {
  it("draws the bar, with the warning and no close control, on the test machine", () => {
    const html = drawBar("dev");
    expect(html).toContain(FALLBACK_TEXT);
    expect(html).toContain("data-env-banner");
    // Not dismissible: no button of any kind lives in this bar.
    expect(html).not.toContain("<button");
  });

  it("draws NOTHING on the real system — not an empty bar, not a reserved strip", () => {
    expect(drawBar("prod")).toBe("");
  });

  it("draws the bar when the machine cannot say which it is (board 5)", () => {
    expect(drawBar(undefined)).toContain(FALLBACK_TEXT);
  });
});

/** Render a printable page as a browser on `environment` would. */
function drawPrint(environment: string | undefined): string {
  const original = globalThis.window;
  // @ts-expect-error — a minimal stand-in for the runtime config
  globalThis.window = { __COLOSSUS_CONFIG__: environment === undefined ? {} : { environment } };
  try {
    return renderToStaticMarkup(
      <PrintWarningFrame line="TEST SYSTEM — NOT FOR TRIAL">
        <div data-print-sheet>a sheet</div>
      </PrintWarningFrame>,
    );
  } finally {
    globalThis.window = original;
  }
}

describe("the printed warning (board 6)", () => {
  it("wraps the sheets in the repeating-header frame on the test machine", () => {
    const html = drawPrint("dev");
    expect(html).toContain("data-print-frame");
    expect(html).toContain("data-print-frame-head");
    expect(html).toContain("data-print-frame-body");
    expect(html).toContain("TEST SYSTEM — NOT FOR TRIAL");
    // The order that makes the repeat work: the head precedes the body.
    expect(html.indexOf("data-print-frame-head")).toBeLessThan(html.indexOf("data-print-frame-body"));
  });

  it("adds NO wrapper at all on the real system", () => {
    expect(drawPrint("prod")).toBe('<div data-print-sheet="true">a sheet</div>');
  });
});

describe("the print CSS the frame depends on", () => {
  it("hides the warning on screen and repeats it as a table header in print", () => {
    // The pair `printChrome.test.ts` pins for app chrome, for this element.
    expect(PRINT_CSS).toMatch(/\[data-print-warning\] \{ display: none; \}/);
    const printBlock = PRINT_CSS.slice(PRINT_CSS.indexOf("@media print"));
    expect(printBlock).toMatch(/\[data-print-frame\] \{ width: 100%/);
    expect(printBlock).toMatch(/\[data-print-frame-head\] \{ display: table-header-group/);
    expect(printBlock).toMatch(/\[data-print-frame-body\] \{ display: table-row-group/);
    expect(printBlock).toMatch(/\[data-print-warning\] \{\s*\n\s*display: block/);
  });
});
