// =============================================================================
// subsetWindow.ts — every decision the floating timeline window makes
// =============================================================================
//
// TIMELINE_SUBSET_DESIGN_v1 §5C and §5D, and the FIRST-OPEN rule of §11 — which
// supersedes §10, whose 460-px free-width test and open-minimized branch this
// module no longer carries. The sibling of `timelineFilters.ts` and
// `subsetPicker.ts`, written for the same reason and in the same shape: this
// project has no component-testing tier, so anything decided inside a component
// is decided where no test can reach it. Where the window opens, where it is
// allowed to be, what is remembered about it and which subset it shows are all
// decided here; what one of its ROWS says is decided in `subsetRows.ts`.
//
// (The header this replaces said the window was not built yet. It was built in
// T3 and mounted on the five scenario surfaces; the note outlived its fact by
// one task, which is what module headers do when nobody deletes them.)
//
// ## Rust Learning parallel, for the reader coming from the backend
//
// `decodeWindowState` is this file's `serde` boundary. It takes an untyped blob
// from outside the program and returns either a value of a known shape or the
// documented default — never a half-populated object. That is the same contract
// `#[derive(Deserialize)]` plus a `#[serde(default)]` gives a Rust struct, done
// by hand because `localStorage` hands back `string | null` and TypeScript's
// types are gone at runtime.

/** Where the window sits and what it is showing. All four are remembered. */
export type WindowState = {
  /** Page-container-relative, in CSS pixels. */
  x: number;
  y: number;
  width: number;
  height: number;
  /** Collapsed to its title bar, pinned bottom-right (§5C). */
  minimized: boolean;
  /** Which attached subset the selector is showing, by id. */
  subsetId: string | null;
};

// STRUCTURAL: the window's drawn geometry, transcribed rule for rule from
// TIMELINE_SUBSET_MOCKUP_v2_2026-08-31.html Screen 2 — `.fw{width:460px;
// height:590px; top:20px; right:22px; min-width:340px; min-height:160px}`.
//
// STRUCTURAL and not configuration, and the distinction is worth stating
// because Rule 2 is otherwise emphatic. These are not values that vary by
// environment, by case, or by deployment — they are ONE APPROVED DRAWING, ruled
// on by Roman on 2026-08-31, and a deployment that could change them could
// change the design without anybody reviewing it. There is no frontend
// configuration surface for a window's furniture and there should not be one:
// the thing that changes these is a new mockup and a new ruling, which is a
// code change by definition. Named here so the places that need them cannot
// disagree with each other.
//
// (The marker was `// CONST:` through T3, which reads as "a number the codebase
// picked" — exactly the class Rule 2 says belongs in configuration. These are
// the other thing, and the marker now says so.)
export const DEFAULT_WIDTH = 460;
export const DEFAULT_HEIGHT = 590;
export const MIN_WIDTH = 340;
export const MIN_HEIGHT = 160;

// STRUCTURAL: the mockup's `right:22px` and `top:20px`, same reasoning as above.
// The top is measured from the bottom of the app's header strip, NOT from the
// top of the viewport — the window is portalled to `body` and lives in viewport
// coordinates, so 20px from the viewport top would put it behind the header.
export const RIGHT_MARGIN = 22;
export const TOP_BELOW_HEADER = 20;

// STRUCTURAL: how often the dock asks a popped-out POPUP whether it has been
// closed. Not configuration, and the reasoning is the same shape as the
// geometry above: this is a latency a person FEELS, not a number that varies by
// environment. A second is short enough that the in-page window is back before
// the reader has finished looking for it, and long enough that the check —
// which is a single boolean read, no request — costs nothing.
//
// It exists at all because a popup is a separate document that navigates after
// `window.open` returns, so a listener attached to the handle does not survive.
// See the effect in `ScenarioTimelineDock` that uses it.
export const POPUP_CLOSED_POLL_MS = 1000;

/** The storage key for one scenario's window. */
export function windowStorageKey(scenarioId: string): string {
  return `colossus.subsetWindow.${scenarioId}`;
}

/**
 * Where a window opens when nothing is remembered about it.
 *
 * ## ⚑ ALWAYS FULL SIZE, AT THE RIGHT EDGE — the §10 rule is WITHDRAWN
 *
 * This is defect D8 and its fix. §10 said the window opened in the right margin
 * when at least 460px of free width existed beside the content column and
 * otherwise opened MINIMIZED to a title bar bottom-right. On a MacBook that
 * second branch is the one that fires, so the feature's first contact with its
 * reader was a grey bar in the corner — Roman opened it on 08-31 and got a bar.
 * The reasoning was sound (better a bar out of the way than a window over the
 * question she is answering) and the outcome was not: a reader who clicks
 * "View Timeline" has asked to see a timeline.
 *
 * Design §11 item 2 withdraws it in as many words: first open is ALWAYS full
 * size, 460 × 590, at the right edge, 20px below the header strip. The window
 * is draggable and resizable and the position is remembered, so a reader who
 * wants it elsewhere moves it once and it stays there. There is no free-width
 * test left, and no branch that opens minimized.
 *
 * `headerBottom` is the app header strip's bottom edge in viewport pixels —
 * measured by the caller, because the strip's height is a rendered fact and not
 * a constant this module could honestly hold.
 *
 * The size is still CLAMPED to the viewport: 590px of window on a 500px-tall
 * browser would put the footer — and its two links — off the bottom of the
 * screen, and unlike the minimized branch that is not a placement the reader
 * can drag their way out of.
 */
export function openStateFor(
  viewportWidth: number,
  viewportHeight: number,
  headerBottom: number,
  subsetId: string | null,
): WindowState {
  const y = Math.max(0, headerBottom + TOP_BELOW_HEADER);
  const width = Math.max(MIN_WIDTH, Math.min(DEFAULT_WIDTH, viewportWidth));
  const height = Math.max(MIN_HEIGHT, Math.min(DEFAULT_HEIGHT, viewportHeight - y));
  return {
    x: Math.max(0, viewportWidth - width - RIGHT_MARGIN),
    y,
    width,
    height,
    minimized: false,
    subsetId,
  };
}

/**
 * Pull a remembered window back inside the viewport.
 *
 * ## ⚑ Why a remembered position is clamped and not trusted
 *
 * The position was stored on a wider screen, or an external monitor that is no
 * longer attached, or before the browser was resized. Restoring it verbatim
 * strands the window off-screen — and because the title bar is the only handle,
 * a window whose title bar is past the right edge cannot be dragged back. The
 * reader's only remedy would be clearing site data, which nothing on the page
 * tells them to do.
 *
 * At least `MIN_WIDTH`/`MIN_HEIGHT` of it stays reachable, and the size is
 * clamped to the viewport too, so a remembered 900px window on a phone-width
 * screen comes back usable rather than clipped.
 */
export function clampToViewport(
  state: WindowState,
  viewportWidth: number,
  viewportHeight: number,
): WindowState {
  const width = Math.max(MIN_WIDTH, Math.min(state.width, Math.max(MIN_WIDTH, viewportWidth)));
  const height = Math.max(MIN_HEIGHT, Math.min(state.height, Math.max(MIN_HEIGHT, viewportHeight)));
  // The window may hang off the right/bottom, but never so far that less than
  // the minimum remains on screen — that minimum includes the title bar, which
  // is the only thing that can drag it back.
  const x = Math.max(0, Math.min(state.x, Math.max(0, viewportWidth - MIN_WIDTH)));
  const y = Math.max(0, Math.min(state.y, Math.max(0, viewportHeight - MIN_HEIGHT)));
  return { ...state, x, y, width, height };
}

/**
 * Where the minimized bar sits: pinned bottom-right (design §5C).
 *
 * ## ⚑ Why this is COMPUTED and not stored
 *
 * Minimizing must not overwrite the position the reader chose for the open
 * window. §5C says the bar is pinned bottom-right and §5D says the reader gets
 * to put the window where they want it — so `x`/`y` keep meaning "where the
 * OPEN window goes", the bar is placed here regardless, and restoring returns
 * the window to the place it was rather than to wherever its bar happened to
 * sit. Storing the bar's corner would silently discard a deliberate placement
 * every time somebody collapsed the window to read the page underneath.
 */
export function minimizedPosition(
  viewportWidth: number,
  viewportHeight: number,
): { x: number; y: number } {
  return {
    x: Math.max(0, viewportWidth - DEFAULT_WIDTH - RIGHT_MARGIN),
    y: Math.max(0, viewportHeight - MIN_HEIGHT - 16),
  };
}

/**
 * Read one window state out of an untrusted string.
 *
 * `null` for absent, unparseable, or wrong-shaped input — the caller falls back
 * to [`openStateFor`]. Every field is checked, because a blob written by an
 * older build (or edited by hand in devtools) that carried `width: "420"` as a
 * string would otherwise reach `react-rnd` as a string and lay the window out
 * at zero width, which looks like the window failing to open at all.
 */
export function decodeWindowState(raw: string | null): WindowState | null {
  if (raw === null || raw.trim() === "") return null;
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return null;
  }
  if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) return null;
  const o = parsed as Record<string, unknown>;
  const num = (v: unknown): number | null =>
    typeof v === "number" && Number.isFinite(v) ? v : null;
  const x = num(o.x);
  const y = num(o.y);
  const width = num(o.width);
  const height = num(o.height);
  if (x === null || y === null || width === null || height === null) return null;
  if (typeof o.minimized !== "boolean") return null;
  const subsetId =
    o.subsetId === null || o.subsetId === undefined
      ? null
      : typeof o.subsetId === "string"
        ? o.subsetId
        : null;
  return { x, y, width, height, minimized: o.minimized, subsetId };
}

/** Write one window state to a string. The mirror of [`decodeWindowState`]. */
export function encodeWindowState(state: WindowState): string {
  return JSON.stringify(state);
}

/**
 * Which subset the selector shows, and in what order.
 *
 * Attachment order — `position` on `scenario_subsets`, which is what the
 * scenario's author chose — with the remembered one first if it is still
 * attached. "Still attached" is the whole point of the check: a subset detached
 * on another screen must not come back as the selection just because this
 * browser remembers picking it.
 */
export function selectorOrder<T extends { id: string; position?: number }>(
  attached: T[],
  rememberedId: string | null,
): T[] {
  const ordered = [...attached].sort((a, b) => (a.position ?? 0) - (b.position ?? 0));
  if (rememberedId === null) return ordered;
  const at = ordered.findIndex((s) => s.id === rememberedId);
  if (at === -1) return ordered;
  return [ordered[at], ...ordered.slice(0, at), ...ordered.slice(at + 1)];
}

/** The subset the window opens on: the remembered one, else the first attached. */
export function initialSubsetId<T extends { id: string; position?: number }>(
  attached: T[],
  rememberedId: string | null,
): string | null {
  const ordered = selectorOrder(attached, rememberedId);
  return ordered.length === 0 ? null : ordered[0].id;
}

/**
 * Which subset the title bar NAMES.
 *
 * ## ⚑ PREVIEW BROKE THE ONE-LINER THIS REPLACES, AND CLICKING FOUND IT
 *
 * The dock read `attached.find(…) ?? attached[0]`, which is right for every
 * ordinary open — the window can only show a subset the scenario carries — and
 * wrong for the one case Preview exists for: reading a subset BEFORE attaching
 * it. `find` missed, the fallback named `attached[0]`, and clicking Preview on
 * "The fee engine" opened a window titled "The $50,000 · 15 events" over the
 * right events. The bar lied about which story was on screen.
 *
 * So a PREVIEWED subset is named from the detail fetched for it, and the
 * fallback survives for the ordinary path where it has always been correct.
 * `detail` is `null` until that fetch lands, which is why the preview arm also
 * checks the id: a stale detail from the previous subset must not name this one.
 */
export function namedSubset<T extends { id: string; name: string; event_count: number }>(
  previewSubsetId: string | null,
  detail: { id: string; name: string; event_count: number } | null,
  attached: T[],
  selectedId: string | null,
): { id: string; name: string; event_count: number } | undefined {
  if (previewSubsetId !== null) {
    return detail !== null && detail.id === previewSubsetId ? detail : undefined;
  }
  return attached.find((s) => s.id === selectedId) ?? attached[0];
}

/**
 * Where a PREVIEW window opens: the ordinary first-open place, on that subset.
 *
 * Preview deliberately does NOT consult the remembered position. The reader is
 * deciding whether to carry a story, not returning to one they arranged — and a
 * preview that opened wherever they last dragged the real window would look
 * like the real window, which is the one thing it must not be mistaken for.
 */
export function previewWindowState(
  previewSubsetId: string,
  viewportWidth: number,
  viewportHeight: number,
  headerBottom: number,
): WindowState {
  return openStateFor(viewportWidth, viewportHeight, headerBottom, previewSubsetId);
}
