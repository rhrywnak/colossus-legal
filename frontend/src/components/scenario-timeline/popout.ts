// =============================================================================
// popout.ts — every decision Pop out makes, with no DOM of its own
// =============================================================================
//
// TIMELINE_SUBSET_MOCKUP_v2_2026-08-31.html Screen 5, approved as drawn, and
// design §11 item 5. The sibling of `subsetWindow.ts` and `subsetRows.ts` for
// the same reason both exist: this project has no component-testing tier, so a
// decision made inside `SubsetPopout.tsx` is a decision no test can reach.
//
// Four decisions live here — can this browser do it, what would be cloned into
// the new document, how big is it, and what does the fallback ask for — and all
// four take their world as an ARGUMENT rather than reaching for the global one.
// That is what lets a test hand them a fake.
//
// ## ⚑ WHY A SECOND WINDOW AT ALL, AND WHY THE API AND NOT A POPUP FIRST
//
// Marie reads a story in dates while answering a cross-examination question.
// Design §5D is why the in-page window never steals focus; Screen 5 is the next
// step of the same argument — on two monitors the story belongs on the other
// one, and no in-page panel can go there.
//
// The Document Picture-in-Picture API (Chrome/Edge 116+; MDN "Document
// Picture-in-Picture API") gives a real OS window that stays ON TOP while she
// works in the app. `window.open` gives a window that does not, and goes behind
// the browser the moment she clicks back into her answer — which is the exact
// moment she wants to see the dates. So the API is the build and the popup is
// the fallback, not the other way round.

import type { WindowState } from "./subsetWindow";

/**
 * The slice of `documentPictureInPicture` this feature uses.
 *
 * ## Rust Learning: this is a hand-written `extern` block, in TypeScript
 *
 * `lib.dom.d.ts` does not declare this API yet — it is too new — so the
 * compiler has no idea `window.documentPictureInPicture` exists. Declaring the
 * shape we intend to call is the same move as a Rust `extern "C" { fn … }`
 * block: a promise to the type checker about something outside the program,
 * which the type checker takes on trust and cannot verify. That is exactly why
 * [`supportsDocumentPictureInPicture`] checks at RUNTIME instead of relying on
 * this declaration — a promise to `tsc` is not evidence about a browser.
 */
export type DocumentPictureInPicture = {
  requestWindow: (options: { width: number; height: number }) => Promise<Window>;
};

/** A window that may or may not carry the API. The argument every check takes. */
export type MaybePipWindow = {
  documentPictureInPicture?: DocumentPictureInPicture;
  open?: (url: string, target: string, features: string) => Window | null;
};

/**
 * Can this browser open a document picture-in-picture window?
 *
 * Feature-DETECTED and never inferred from a user-agent string: Safari and
 * Firefox lack the API today and may ship it tomorrow, and a build that decided
 * from the UA would keep sending Chrome-shaped browsers down the fallback path
 * forever. Takes the window as an argument so a test can hand it both answers.
 */
export function supportsDocumentPictureInPicture(win: MaybePipWindow | undefined): boolean {
  return typeof win?.documentPictureInPicture?.requestWindow === "function";
}

/** One stylesheet, as it would be re-created inside the new document. */
export type StyleSheetClone =
  | { kind: "inline"; css: string }
  | { kind: "link"; href: string; media: string };

/**
 * A minimal `CSSStyleSheet`, so a test can build one without a browser.
 *
 * Reading `cssRules` on a sheet loaded cross-origin THROWS a `SecurityError` —
 * that is the whole reason the link branch below exists — so the fake has to be
 * able to throw too.
 */
export type ClonableSheet = {
  cssRules?: ArrayLike<{ cssText: string }>;
  href?: string | null;
  media?: { mediaText: string } | string;
};

/**
 * What would be cloned into the picture-in-picture document, in order.
 *
 * ## ⚑ A NEW DOCUMENT INHERITS NOTHING
 *
 * The picture-in-picture window is a separate `Document`. It does not inherit
 * the page's stylesheets, its custom properties, or its fonts — an unstyled
 * portal into it renders the subset as black Times New Roman on white, which
 * looks like a broken window rather than a themed one. So every sheet is
 * re-created inside it: the documented pattern from MDN and
 * developer.chrome.com, and the reason this returns a LIST rather than doing it
 * is that the doing is four lines of `appendChild` and the DECIDING is what
 * needs a test.
 *
 * A sheet whose rules cannot be read — cross-origin, which for this app means
 * the Google Fonts sheet and nothing else — comes back as a `link` to be
 * re-fetched by the new document instead. Silently dropping it is what would
 * make the popped-out window render in a different typeface from the page it
 * came from, with nothing in the console to say why.
 */
export function stylesheetsToClone(sheets: ArrayLike<ClonableSheet>): StyleSheetClone[] {
  const out: StyleSheetClone[] = [];
  for (let i = 0; i < sheets.length; i += 1) {
    const sheet = sheets[i];
    const media = typeof sheet.media === "string" ? sheet.media : (sheet.media?.mediaText ?? "");
    let css: string | null = null;
    try {
      const rules = sheet.cssRules;
      if (rules !== undefined && rules !== null) {
        const texts: string[] = [];
        for (let r = 0; r < rules.length; r += 1) texts.push(rules[r].cssText);
        css = texts.join("\n");
      }
    } catch {
      // A SecurityError on a cross-origin sheet. Not swallowed — it is the
      // documented signal to fall through to the link branch, and a sheet with
      // neither readable rules nor an href is reported below.
      css = null;
    }
    if (css !== null) {
      out.push({ kind: "inline", css });
      continue;
    }
    const href = sheet.href ?? "";
    if (href !== "") out.push({ kind: "link", href, media });
    // A sheet with no readable rules AND no href cannot be reproduced by any
    // means. It is dropped from the list, and the caller logs the count — see
    // `SubsetPopout`'s `console.warn`. Nothing here fails silently.
  }
  return out;
}

/**
 * How many sheets could not be reproduced at all — for the caller's log.
 *
 * The mirror of [`stylesheetsToClone`]: that returns what CAN be cloned, this
 * says how much was lost, so "the popped-out window looks wrong" has a number
 * behind it in the console instead of a shrug.
 */
export function unclonableCount(sheets: ArrayLike<ClonableSheet>): number {
  return sheets.length - stylesheetsToClone(sheets).length;
}

/**
 * The size the popped-out window asks for: the in-page window's own.
 *
 * A passthrough, and it is a decision even so — the alternative was a fixed
 * size from the mockup, which would have thrown away a reader's deliberate
 * resize the moment they popped the window out. Clamped to positive integers
 * because `requestWindow` rejects a zero or fractional dimension, and a
 * remembered state can hold either.
 */
export function popoutSize(state: WindowState): { width: number; height: number } {
  return {
    width: Math.max(1, Math.round(state.width)),
    height: Math.max(1, Math.round(state.height)),
  };
}

/**
 * `window.open`'s feature string for the fallback popup.
 *
 * `popup` is what asks for a window with no tab strip and no address bar —
 * without it a "popup" is a new tab, which is not a second window at all and
 * would put the story behind the app instead of beside it.
 */
export function popupFeatures(size: { width: number; height: number }): string {
  return `popup,width=${size.width},height=${size.height}`;
}

/**
 * The attributes to copy onto the new document's root element.
 *
 * ## ⚑ THE APP HAS NO DARK THEME, AND THIS IS STILL THE RIGHT SHAPE
 *
 * T4.3 asks for the picture-in-picture document's theme to match the app's.
 * Today that is a copy of nothing: `tokens.css` says "Light theme only; dark
 * mode is explicitly out of scope for v2", and the build carries no
 * `data-theme`, no theme class and no `prefers-color-scheme` rule. See the T4
 * report.
 *
 * So rather than hard-code "light" — which would be a lie the day a theme lands
 * — this MIRRORS whatever the page's own root element carries. It copies
 * nothing today and copies the toggle's own attribute the day there is one,
 * with no second place to remember to update. The `class` and `style`
 * attributes ride along for the same reason: those are the other two places a
 * theme is conventionally stamped.
 */
export const MIRRORED_ROOT_ATTRIBUTES = ["data-theme", "class", "style"] as const;

export function rootAttributesToMirror(
  root: { getAttribute: (name: string) => string | null } | null,
): Array<{ name: string; value: string }> {
  if (root === null) return [];
  const out: Array<{ name: string; value: string }> = [];
  for (const name of MIRRORED_ROOT_ATTRIBUTES) {
    const value = root.getAttribute(name);
    if (value !== null && value !== "") out.push({ name, value });
  }
  return out;
}
