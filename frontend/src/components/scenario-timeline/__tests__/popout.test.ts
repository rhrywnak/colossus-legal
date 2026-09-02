// =============================================================================
// popout.test.ts — the four decisions Pop out makes
// =============================================================================
//
// T4.3 names them: the "supports PiP?" predicate with an injectable window, the
// stylesheet-clone helper tested against a FAKE document, and the size
// passthrough. The fallback's feature string and the theme mirror ride with
// them, because both are decisions and neither has a DOM.
//
// ## ⚑ WHAT IS NOT TESTED HERE, AND WHY IT IS SAID OUT LOUD
//
// T4.3 also asks for a COMPONENT test — "⧉ hidden when no subset loaded;
// fallback path chosen when the API is absent". This project has no
// component-testing tier: no jsdom, no Testing Library, and CLAUDE.md rule 30
// records the house pattern as pure-helper tests plus service tests. Rather
// than claim a component test that cannot exist, both behaviours are asserted
// at the level that CAN hold them — the predicate that chooses the path is
// below, and the "⧉ hidden until the subset is loaded" condition is a plain
// `subset !== null` guard in the dock, quoted in the T4 report. Reported as a
// DEVIATION, not passed over.

import { describe, expect, it } from "vitest";

import {
  containerForOutcome,
  firstPopoutRung,
  MIRRORED_ROOT_ATTRIBUTES,
  popoutSize,
  popupFeatures,
  rootAttributesToMirror,
  stylesheetsToClone,
  supportsDocumentPictureInPicture,
  unclonableCount,
  type ClonableSheet,
  type MaybePipWindow,
} from "../popout";
import type { WindowState } from "../subsetWindow";

const STATE: WindowState = {
  x: 958,
  y: 84,
  width: 460,
  height: 590,
  minimized: false,
  subsetId: "sub-1",
};

// -----------------------------------------------------------------------------
// Which path a browser takes
// -----------------------------------------------------------------------------

describe("supportsDocumentPictureInPicture", () => {
  it("is true for a window carrying a callable requestWindow", () => {
    const chrome: MaybePipWindow = {
      documentPictureInPicture: { requestWindow: async () => window },
    };
    expect(supportsDocumentPictureInPicture(chrome)).toBe(true);
  });

  it("is false where the API is absent — Safari and Firefox take the popup", () => {
    expect(supportsDocumentPictureInPicture({})).toBe(false);
  });

  it("is false for a window that is undefined at all", () => {
    expect(supportsDocumentPictureInPicture(undefined)).toBe(false);
  });

  it("is false when the object exists but requestWindow is not a function", () => {
    // A polyfill, an extension, or a half-shipped flag. `"documentPictureInPicture"
    // in window` would have said true here and the call would have thrown at the
    // one moment the reader was watching.
    const half = { documentPictureInPicture: {} } as unknown as MaybePipWindow;
    expect(supportsDocumentPictureInPicture(half)).toBe(false);
  });
});

// -----------------------------------------------------------------------------
// The fallback chain — design §12.1's table, as four assertions (T7.4)
// -----------------------------------------------------------------------------
//
// `View Timeline` opens the floating window, and the interesting half of that
// sentence is what happens when it cannot. The chain has three rungs and the
// reader must never reach the bottom of it and find nothing.

describe("firstPopoutRung — which rung the click takes first", () => {
  it("asks for a real OS window where the API is there", () => {
    const chrome: MaybePipWindow = {
      documentPictureInPicture: { requestWindow: async () => window },
    };
    expect(firstPopoutRung(chrome)).toBe("pip");
  });

  it("asks for a popup where it is not — Safari and Firefox, today", () => {
    expect(firstPopoutRung({})).toBe("popup");
  });

  it("never answers `inpage` — the page is where a REFUSAL lands, not a click", () => {
    // The distinction the resolver exists to keep: this function reads the
    // browser, and the browser can say "I have no such API". It cannot say
    // "put it in the page" — only a refused request can, which is
    // `containerForOutcome`'s decision and is made later.
    expect(firstPopoutRung(undefined)).not.toBe("inpage");
  });
});

describe("containerForOutcome — where the story actually ends up", () => {
  it("keeps the OS window when the request was granted", () => {
    expect(containerForOutcome({ attempted: "pip", granted: true })).toBe("pip");
  });

  it("keeps the popup when it opened", () => {
    expect(containerForOutcome({ attempted: "popup", granted: true })).toBe("popup");
  });

  it("falls into the page when the picture-in-picture request is REJECTED", () => {
    // `NotAllowedError` and its relatives. The reader clicked a button; they
    // get their story in the page rather than an unchanged screen.
    expect(containerForOutcome({ attempted: "pip", granted: false })).toBe("inpage");
  });

  it("falls into the page when a popup blocker refuses the popup", () => {
    // `window.open` returning null. Indistinguishable to the reader from the
    // rejection above, and identical in consequence — which is why one
    // function answers both.
    expect(containerForOutcome({ attempted: "popup", granted: false })).toBe("inpage");
  });
});

// -----------------------------------------------------------------------------
// What gets rebuilt inside the new document
// -----------------------------------------------------------------------------

/** A same-origin sheet whose rules can be read. */
function readable(...rules: string[]): ClonableSheet {
  return { cssRules: rules.map((cssText) => ({ cssText })), href: null, media: "" };
}

/** A cross-origin sheet: reading `cssRules` throws a SecurityError. */
function crossOrigin(href: string, media = ""): ClonableSheet {
  return {
    get cssRules(): never {
      throw new Error("SecurityError: cannot access rules");
    },
    href,
    media,
  };
}

describe("stylesheetsToClone", () => {
  it("inlines a same-origin sheet's rules, in order", () => {
    const sheets = [readable(":root{--a:1}", "body{margin:0}")];
    expect(stylesheetsToClone(sheets)).toEqual([
      { kind: "inline", css: ":root{--a:1}\nbody{margin:0}" },
    ]);
  });

  it("falls back to a LINK for a cross-origin sheet rather than dropping it", () => {
    // For this app that sheet is the font. Dropping it silently is what would
    // render the popped-out window in a different typeface from the page it
    // came from, with nothing in the console to say why.
    const sheets = [crossOrigin("https://fonts.example/inter.css", "screen")];
    expect(stylesheetsToClone(sheets)).toEqual([
      { kind: "link", href: "https://fonts.example/inter.css", media: "screen" },
    ]);
  });

  it("keeps the page's order — later rules must still win in the new document", () => {
    const sheets = [readable("a{}"), crossOrigin("https://x/y.css"), readable("b{}")];
    expect(stylesheetsToClone(sheets).map((c) => c.kind)).toEqual(["inline", "link", "inline"]);
  });

  it("reads a MediaList as well as a plain string", () => {
    const sheet: ClonableSheet = {
      get cssRules(): never {
        throw new Error("SecurityError");
      },
      href: "https://x/y.css",
      media: { mediaText: "print" },
    };
    expect(stylesheetsToClone([sheet])).toEqual([
      { kind: "link", href: "https://x/y.css", media: "print" },
    ]);
  });

  it("drops a sheet that can be neither read nor re-fetched, and COUNTS it", () => {
    // No rules and no href: there is no way to reproduce it. The count is what
    // the caller logs, so "the window looks wrong" has a number behind it.
    const orphan: ClonableSheet = {
      get cssRules(): never {
        throw new Error("SecurityError");
      },
      href: null,
    };
    const sheets = [readable("a{}"), orphan];
    expect(stylesheetsToClone(sheets)).toHaveLength(1);
    expect(unclonableCount(sheets)).toBe(1);
  });

  it("an empty sheet list clones nothing and loses nothing", () => {
    expect(stylesheetsToClone([])).toEqual([]);
    expect(unclonableCount([])).toBe(0);
  });
});

// -----------------------------------------------------------------------------
// Size, and the fallback's ask
// -----------------------------------------------------------------------------

describe("popoutSize — the in-page window's own size, passed through", () => {
  it("carries the drawn 460 × 590 out unchanged", () => {
    expect(popoutSize(STATE)).toEqual({ width: 460, height: 590 });
  });

  it("carries a size the READER chose, not the drawn one", () => {
    // The alternative was a fixed size from the mockup, which would throw away
    // a deliberate resize the moment the window was popped out.
    expect(popoutSize({ ...STATE, width: 720, height: 400 })).toEqual({
      width: 720,
      height: 400,
    });
  });

  it("rounds a fractional remembered size — requestWindow rejects one", () => {
    expect(popoutSize({ ...STATE, width: 460.4, height: 589.6 })).toEqual({
      width: 460,
      height: 590,
    });
  });

  it("never asks for a zero or negative dimension", () => {
    expect(popoutSize({ ...STATE, width: 0, height: -20 })).toEqual({ width: 1, height: 1 });
  });
});

describe("popupFeatures", () => {
  it("asks for a real popup and not a tab", () => {
    // Without `popup`, `window.open` gives a TAB — which is not a second window
    // at all, and puts the story behind the app rather than beside it.
    expect(popupFeatures({ width: 460, height: 590 })).toBe("popup,width=460,height=590");
  });
});

// -----------------------------------------------------------------------------
// The theme, mirrored rather than assumed
// -----------------------------------------------------------------------------

describe("rootAttributesToMirror", () => {
  it("copies NOTHING from a root carrying no theme — today's app, exactly", () => {
    // `tokens.css`: "Light theme only; dark mode is explicitly out of scope for
    // v2". There is no attribute to copy, and hard-coding "light" would be a
    // lie the day a toggle lands.
    expect(rootAttributesToMirror({ getAttribute: () => null })).toEqual([]);
  });

  it("copies a data-theme the day the app has one", () => {
    const root = { getAttribute: (n: string) => (n === "data-theme" ? "dark" : null) };
    expect(rootAttributesToMirror(root)).toEqual([{ name: "data-theme", value: "dark" }]);
  });

  it("copies a theme stamped as a class instead", () => {
    const root = { getAttribute: (n: string) => (n === "class" ? "theme-dark" : null) };
    expect(rootAttributesToMirror(root)).toEqual([{ name: "class", value: "theme-dark" }]);
  });

  it("ignores an empty attribute, which is not a theme", () => {
    expect(rootAttributesToMirror({ getAttribute: () => "" })).toEqual([]);
  });

  it("survives a document with no root element", () => {
    expect(rootAttributesToMirror(null)).toEqual([]);
  });

  it("watches the three places a theme is conventionally stamped", () => {
    // A guard on the list itself: if a future toggle picks a fourth attribute
    // and nobody adds it here, the popped-out window silently renders light
    // while the page renders dark.
    expect([...MIRRORED_ROOT_ATTRIBUTES]).toEqual(["data-theme", "class", "style"]);
  });
});
