// =============================================================================
// viewerWindow.test.ts — the pinpoint opens a WINDOW (defect D5, ruling R8)
// =============================================================================

import { describe, expect, it, vi } from "vitest";

import {
  openViewerWindow,
  VIEWER_WINDOW_FEATURES_FOR_TEST,
  VIEWER_WINDOW_NAME_FOR_TEST,
} from "../viewerWindow";

/** A stub `Window` carrying only the `focus` this module touches. */
function fakeWindow(focus: () => void = () => {}): Window {
  return { focus } as unknown as Window;
}

/**
 * The opener's ARGUMENT tuple, as vitest 1.x's `vi.fn` takes it.
 *
 * `vi.fn<TArgs extends any[], TReturn>` is parameterised on the argument tuple,
 * not on the function type — so this is `[url, target, features]` rather than a
 * signature. Declared explicitly because a bare `vi.fn(() => …)` infers a
 * ZERO-length tuple, and every `calls[0][2]` below would then be a type error
 * instead of the assertion it looks like.
 */
type OpenerArgs = [url: string, target: string, features: string];

describe("opening the viewer", () => {
  it("asks for a SIZED window, not a tab", () => {
    // The whole defect: `window.open(href)` with no features opens a TAB in every
    // engine, and a tab hides the page the human is reading the quote against.
    const opener = vi.fn<OpenerArgs, Window | null>(() => fakeWindow());
    openViewerWindow("/documents/doc-7?page=25&tab=document", opener);

    expect(opener).toHaveBeenCalledTimes(1);
    const [, , features] = opener.mock.calls[0];
    expect(features).toContain("width=1100");
    expect(features).toContain("height=1000");
  });

  it("passes the server-composed href through untouched", () => {
    // The href is composed backend-side (`viewer_href`). A browser rewriting it is
    // a browser guessing at a route.
    const href = "/documents/doc-7?page=25&tab=document";
    const opener = vi.fn<OpenerArgs, Window | null>(() => fakeWindow());
    openViewerWindow(href, opener);
    expect(opener.mock.calls[0][0]).toBe(href);
  });

  it("targets ONE named window so repeat clicks reuse it", () => {
    // A human ruling 148 candidates clicks ~148 pinpoints. 148 windows is not a
    // workspace, so every click targets the same name and re-navigates it.
    const opener = vi.fn<OpenerArgs, Window | null>(() => fakeWindow());
    openViewerWindow("/documents/a", opener);
    openViewerWindow("/documents/b", opener);

    expect(opener.mock.calls[0][1]).toBe(VIEWER_WINDOW_NAME_FOR_TEST);
    expect(opener.mock.calls[1][1]).toBe(VIEWER_WINDOW_NAME_FOR_TEST);
  });

  it("re-raises the window on a repeat click", () => {
    // Without the raise, the second pinpoint re-navigates a window still buried
    // behind the app — which reads exactly like nothing happened.
    const focus = vi.fn();
    const opener = vi.fn<OpenerArgs, Window | null>(() => fakeWindow(focus));
    openViewerWindow("/documents/a", opener);
    expect(focus).toHaveBeenCalledTimes(1);
  });

  it("does NOT ask for noopener, which would break the reuse (ruling R8)", () => {
    // Per the WHATWG spec, `noopener` puts the new context in a separate
    // browsing-context group — the named target then cannot be found and every
    // click opens a fresh window. The two features are mutually exclusive and
    // reuse is the one that matters. Safe here because the viewer is our own
    // same-origin route.
    expect(VIEWER_WINDOW_FEATURES_FOR_TEST).not.toContain("noopener");
  });
});

describe("when the browser refuses", () => {
  it("reports a blocked popup rather than failing silently", () => {
    // Standing Rule 1. `window.open` returns null when a blocker refuses, and a
    // swallowed null is a pinpoint that does nothing when clicked.
    const result = openViewerWindow("/documents/a", () => null);

    expect(result.opened).toBe(false);
    if (!result.opened) {
      expect(result.message).toContain("popup");
      // The message must say what to DO, not merely what happened.
      expect(result.message).toContain("Allow popups");
    }
  });

  it("reports success with no message when the window opened", () => {
    const result = openViewerWindow("/documents/a", () => fakeWindow());
    expect(result).toEqual({ opened: true });
  });

  it("still reports success when only the RAISE fails", () => {
    // `focus()` is advisory — some engines and window managers refuse it. A refusal
    // to raise is not a failure to open, and treating it as one would show the human
    // an error about a window that is sitting there working.
    const throwingFocus = () => {
      throw new Error("focus refused by the window manager");
    };
    const result = openViewerWindow("/documents/a", () => fakeWindow(throwingFocus));
    expect(result).toEqual({ opened: true });
  });
});
