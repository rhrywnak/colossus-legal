// =============================================================================
// viewerWindow.ts — pinpoints open the viewer as a real WINDOW (defect D5)
// =============================================================================
//
// Roman's ruling 2026-08-02: a pinpoint opens the dedicated viewer in a separate,
// sized WINDOW — not a plain new tab. The rationale is the whole point of the
// surface: reading a quote against its own page means having both visible at once,
// and a tab hides the page behind the queue.
//
// This replaces the 1.7B `<a target="_blank">`. The href itself is unchanged and
// still composed server-side (`card.pinpoint.viewer_href`); only the opening
// changes.
//
// ## Why the window is NAMED, and why `noopener` is absent
//
// A human ruling 148 candidates clicks ~148 pinpoints. Opening 148 windows is not
// a workspace, so every pinpoint targets the SAME named window, which re-navigates
// and is re-raised.
//
// `noopener` was in the Phase A proposal and Roman's ruling R8 corrected it: per
// the WHATWG spec, `noopener` puts the new top-level browsing context in a
// SEPARATE browsing-context group, which makes it unreachable by name — so every
// click would open a fresh window and the reuse would silently stop working. The
// two features are mutually exclusive, and reuse is the one that matters here.
//
// Dropping `noopener` is safe in this specific case and only this case: the viewer
// is our own first-party, same-origin route (`/documents/:id`). The
// reverse-tabnabbing threat `noopener` exists to blunt is a threat from a page you
// do not control, and `window.opener` reaching between two of our own pages is not
// a capability an attacker gains anything from. A pinpoint pointing off-origin
// would be a different question — and it cannot, because the backend composes the
// href.
//
// ## Why a popup blocker does not bite
//
// Every engine allows `window.open` during a user-initiated gesture. This is
// called synchronously inside the click handler, with no `await` before it, so the
// call is still inside that gesture. If a blocker refuses anyway, `open` returns
// `null` — and that must be SURFACED, never swallowed (Standing Rule 1). The
// caller renders the message this module returns and keeps the plain link as a
// fallback the human can use.

/**
 * The window features string.
 *
 * 1100×1000 is measured, not guessed: the viewer is the app's own
 * `/documents/:id` route, which renders inside the app shell whose `<main>` is
 * `max-width: 1080px` (`App.tsx`). Below ~1100 the page gutters collapse and the
 * document pane starts squeezing — the exact unreadability that retired the
 * split-pane viewer in 1.7B.
 *
 * `scrollbars` and `resizable` are explicit because some engines default a
 * featured popup to neither, and a legal page you cannot scroll is a page you
 * cannot read.
 */
// CONST: window geometry for our own viewer route, derived from the app shell's
// own max-width. Not deployment-varying — it is a property of this UI's layout,
// so it cannot live in env/config (there is nothing an operator would set it to).
const VIEWER_FEATURES = "width=1100,height=1000,scrollbars=yes,resizable=yes";

/**
 * The window name every pinpoint targets, so they share one window.
 *
 * ## Why one name and not one per document
 *
 * A name per document would still accumulate a window per document read, and the
 * human's mental model is "the viewer" — one place they look, which changes to
 * follow what they clicked.
 */
// CONST: an internal window handle, not a tunable.
const VIEWER_WINDOW_NAME = "colossus-viewer";

/** What happened when a pinpoint was clicked. */
export type ViewerOpenResult =
  | { opened: true }
  /** The window did not open. `message` is shown to the human, verbatim. */
  | { opened: false; message: string };

/**
 * The message shown when the browser refused to open the viewer.
 *
 * Says what happened, why it probably happened, and what the human can do
 * instead — the three things a refusal has to carry to be actionable. It names
 * the popup blocker because that is the only realistic cause of a `null` return
 * on a click-initiated open, and a human who reads "popup" knows where to look in
 * their own browser.
 */
// CONST: user-facing copy. Not configuration.
const BLOCKED_MESSAGE =
  "The document viewer did not open — your browser most likely blocked the " +
  "popup. Allow popups for this site, or open the pinpoint link in a new tab " +
  "instead.";

/**
 * Open the dedicated viewer at a cited page, in the shared viewer window.
 *
 * Call this synchronously from a click handler — see the module header on why
 * that matters for popup blockers.
 *
 * `opener` exists so this is testable without a browser: the tests pass a stub and
 * assert the name and features that were requested. It defaults to the real
 * `window.open`, so callers pass nothing.
 *
 * @param href The server-composed `viewer_href` from the card's pinpoint.
 * @returns whether the window opened, with the message to show if it did not.
 */
export function openViewerWindow(
  href: string,
  opener: (
    url: string,
    target: string,
    features: string,
  ) => Window | null = defaultOpener,
): ViewerOpenResult {
  const opened = opener(href, VIEWER_WINDOW_NAME, VIEWER_FEATURES);

  if (!opened) {
    // Standing Rule 1: a refused open is reported, never shrugged off. The caller
    // renders `message`; the plain link stays available as the way through.
    return { opened: false, message: BLOCKED_MESSAGE };
  }

  // Re-raise on a repeat click. Without this the second pinpoint re-navigates a
  // window that is still buried behind the app, which reads exactly like nothing
  // happened. Wrapped because `focus()` is advisory — some engines and window
  // managers refuse it, and a refusal to RAISE is not a failure to OPEN.
  try {
    opened.focus();
  } catch {
    // best-effort: the window IS open and showing the right page; only the raise
    // failed. A banner here would be disproportionate — the human can see the
    // window in their own task switcher — and it stays observable in the console.
    console.warn("could not raise the document viewer window; it is open");
  }

  return { opened: true };
}

/** The real `window.open`, isolated so the exported function stays pure to test. */
function defaultOpener(url: string, target: string, features: string): Window | null {
  return window.open(url, target, features);
}

/** Exposed for the structural test that pins the geometry and the reuse name. */
export const VIEWER_WINDOW_FEATURES_FOR_TEST = VIEWER_FEATURES;
export const VIEWER_WINDOW_NAME_FOR_TEST = VIEWER_WINDOW_NAME;
