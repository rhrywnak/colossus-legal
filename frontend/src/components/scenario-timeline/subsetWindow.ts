// =============================================================================
// subsetWindow.ts — every decision the floating timeline window makes
// =============================================================================
//
// TIMELINE_SUBSET_DESIGN_v1 §5C and §5D, and the open-position rule of §10.
// The sibling of `timelineFilters.ts` and `subsetPicker.ts`, written for the
// same reason and in the same shape: this project has no component-testing
// tier, so anything decided inside a component is decided where no test can
// reach it. Where the window opens, where it is allowed to be, what is
// remembered about it and which subset it shows are all decided here.
//
// ## ⚑ THE WINDOW ITSELF IS NOT BUILT YET, AND THIS IS NOT DEAD CODE
//
// Task 3 could not mount it: there is no header component the five scenario
// views share and no read they have in common, so the wording block has nowhere
// to be delivered and the button has nowhere to live. The T3 report carries
// what each of the five actually calls and asks for a ruling. These functions
// are the part of task 3 that does NOT depend on that answer — where a window
// opens and what is remembered about it are the same wherever it is mounted —
// so they are built and tested now rather than guessed at later.
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

// CONST: the window's drawn geometry, from the approved mockup and design §5C —
// 420px wide, 60% of the viewport tall, minimum 320×140. Not settings: there is
// no frontend config surface for a window's default size and these are not
// per-deployment values, they are one approved drawing transcribed. Named here
// so the four places that need them cannot disagree.
export const DEFAULT_WIDTH = 420;
export const DEFAULT_HEIGHT_RATIO = 0.6;
export const MIN_WIDTH = 320;
export const MIN_HEIGHT = 140;

// CONST: the open-position rule of design §10, standing unless overruled — the
// window opens in the right margin when at least this much free width exists
// beside the content column, and otherwise opens minimized. 460 is the drawn
// 420 plus the margin that keeps it off the text.
export const MARGIN_OPEN_MIN_WIDTH = 460;

/** The storage key for one scenario's window. */
export function windowStorageKey(scenarioId: string): string {
  return `colossus.subsetWindow.${scenarioId}`;
}

/**
 * Where a window opens when nothing is remembered about it.
 *
 * ## ⚑ The rule, and why it opens MINIMIZED rather than smaller
 *
 * Design §10: the right margin when there is room beside the content column,
 * otherwise minimized to the bottom-right for the reader to place. It does not
 * shrink to fit, because §5D is explicit that Marie is reading this beside a
 * cross-examination question and the window must never cover the question she
 * is answering. A narrow window over the text is worse than a title bar out of
 * the way — she can open it where she wants it, but she cannot un-read the line
 * it hid.
 */
export function openStateFor(
  viewportWidth: number,
  viewportHeight: number,
  contentRight: number,
  subsetId: string | null,
): WindowState {
  const freeWidth = viewportWidth - contentRight;
  const height = Math.max(MIN_HEIGHT, Math.round(viewportHeight * DEFAULT_HEIGHT_RATIO));
  if (freeWidth >= MARGIN_OPEN_MIN_WIDTH) {
    return {
      x: Math.max(0, viewportWidth - DEFAULT_WIDTH - 26),
      y: 96,
      width: DEFAULT_WIDTH,
      height,
      minimized: false,
      subsetId,
    };
  }
  return {
    x: Math.max(0, viewportWidth - DEFAULT_WIDTH - 26),
    y: Math.max(0, viewportHeight - MIN_HEIGHT),
    width: DEFAULT_WIDTH,
    height,
    minimized: true,
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
    x: Math.max(0, viewportWidth - DEFAULT_WIDTH - 26),
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
