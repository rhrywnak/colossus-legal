// =============================================================================
// dragAutoScroll — moving a fact past the edge of the window (task 2.13c)
// =============================================================================
//
// Roman, real mouse, deployed .378: a drop onto a card seam works and lands
// correctly, but the facts list scrolls inside its own region and NOTHING moves
// that region during a drag. So a fact can only be moved as far as the visible
// window, and the scrollbar cannot be reached mid-drag because the pointer is
// holding a card. On a list of forty-six that means most destinations are
// unreachable — the feature works and is still unusable.
//
// The fix is the one every list with a scroll container needs: while a drag is
// live, a cursor inside an edge band scrolls the container toward that edge,
// faster the closer it gets.
//
// ## Why the arithmetic is a pure function
//
// The rest of this is a `requestAnimationFrame` loop and a DOM node — untestable
// without a browser, and the part most likely to be wrong is not the loop but the
// SPEED: the sign, the scaling, and what happens when the cursor leaves the
// region entirely. That is separated into `autoScrollSpeed`, which is arithmetic
// over four numbers and is tested exhaustively.

import { useCallback, useEffect, useRef } from "react";

// CONST: how deep the edge band reaches into the region, in pixels.
//
// Presentational, and therefore NOT a settings row (the standing R5 line): it
// changes how a gesture FEELS, never what the list contains or what any of it
// means. 64px is about two card-heights' worth of margin at the reading size —
// wide enough to enter without aiming, narrow enough that the middle of the
// region is still a dead zone where a careful drop does not drift.
export const EDGE_BAND_PX = 64;

// CONST: the fastest the region scrolls, in pixels per animation frame.
//
// At ~60fps this is roughly 1080 px/second at the very edge, which crosses a
// 60vh region in well under a second. Faster overshoots the target on a long
// list; slower makes a drag to the far end feel stuck, which is the complaint
// this exists to answer.
export const MAX_SCROLL_PX_PER_FRAME = 18;

/**
 * How far to scroll this frame, from where the cursor is.
 *
 * Negative scrolls UP, positive scrolls DOWN, `0` means the cursor is in the
 * quiet middle and nothing should move.
 *
 * @param cursorY viewport Y of the pointer (a drag event's `clientY`).
 * @param top     viewport Y of the scroll region's top edge.
 * @param bottom  viewport Y of its bottom edge.
 *
 * ## The three decisions this encodes
 *
 * 1. **Speed scales with proximity.** At the very edge it is `maxSpeed`; at the
 *    inner boundary of the band it is 0. A constant speed makes the region feel
 *    like it lurches, and makes fine positioning near a target impossible.
 * 2. **Past the edge is full speed, not no speed.** A cursor dragged ABOVE the
 *    region is still asking to go up — clamping the distance at zero rather than
 *    returning 0 is what stops the scroll dying exactly when the human pushes
 *    hardest.
 * 3. **A short region prefers the nearer edge.** If the region is shorter than
 *    two bands they overlap, and every point is "near" both. Comparing the two
 *    distances keeps the direction unambiguous instead of letting whichever
 *    branch runs first win.
 */
export function autoScrollSpeed(
  cursorY: number,
  top: number,
  bottom: number,
  band: number = EDGE_BAND_PX,
  maxSpeed: number = MAX_SCROLL_PX_PER_FRAME,
): number {
  // A zero or negative band would divide by zero below, and an inverted rect
  // (bottom above top) describes no region at all. Neither can scroll.
  if (band <= 0 || maxSpeed <= 0 || bottom <= top) return 0;

  // Clamped at zero so a cursor beyond an edge counts as being ON it — see
  // decision 2 above.
  const fromTop = Math.max(0, cursorY - top);
  const fromBottom = Math.max(0, bottom - cursorY);

  const inTopBand = fromTop < band;
  const inBottomBand = fromBottom < band;

  // Overlapping bands on a short region: the nearer edge wins, and a dead tie
  // resolves to no movement rather than an arbitrary direction.
  if (inTopBand && inBottomBand) {
    if (fromTop === fromBottom) return 0;
    return fromTop < fromBottom
      ? -scaled(fromTop, band, maxSpeed)
      : scaled(fromBottom, band, maxSpeed);
  }
  if (inTopBand) return -scaled(fromTop, band, maxSpeed);
  if (inBottomBand) return scaled(fromBottom, band, maxSpeed);
  return 0;
}

/** Full speed at the edge, tapering to zero at the band's inner boundary. */
function scaled(distanceFromEdge: number, band: number, maxSpeed: number): number {
  return maxSpeed * (1 - distanceFromEdge / band);
}

/**
 * Drive a scroll container from the cursor while a drag is in flight.
 *
 * Returns the pieces a scrollable region needs: a `ref` for the element, an
 * `onDragOver` to feed it the cursor, and a `stop` for the events that end a
 * drag.
 *
 * ## Rust Learning parallel — why the frame loop holds a REF, not state
 *
 * The cursor moves every few milliseconds. Storing it in React state would
 * re-render the whole facts list on each move, which on forty-six cards is the
 * jank the auto-scroll is supposed to remove. A ref is mutable storage that does
 * not trigger rendering — the loop reads the latest value each frame and React
 * never hears about it.
 */
export function useDragAutoScroll() {
  const regionRef = useRef<HTMLDivElement | null>(null);
  const speedRef = useRef(0);
  const frameRef = useRef<number | null>(null);

  const stop = useCallback(() => {
    speedRef.current = 0;
    if (frameRef.current !== null) {
      cancelAnimationFrame(frameRef.current);
      frameRef.current = null;
    }
  }, []);

  const step = useCallback(() => {
    const region = regionRef.current;
    // The loop stops itself when there is nothing to do, rather than spinning a
    // frame callback for the whole time a card is held over the middle.
    if (!region || speedRef.current === 0) {
      frameRef.current = null;
      return;
    }
    region.scrollTop += speedRef.current;
    frameRef.current = requestAnimationFrame(step);
  }, []);

  const onDragOver = useCallback(
    (event: React.DragEvent) => {
      const region = regionRef.current;
      if (!region) return;
      const rect = region.getBoundingClientRect();
      speedRef.current = autoScrollSpeed(event.clientY, rect.top, rect.bottom);
      if (speedRef.current !== 0 && frameRef.current === null) {
        frameRef.current = requestAnimationFrame(step);
      }
    },
    [step],
  );

  /**
   * Stop when the cursor leaves the region ENTIRELY.
   *
   * `dragleave` also fires when the cursor crosses between two children — from
   * one card to the next — which happens constantly during a drag. Stopping on
   * those would kill the scroll the moment it started. The `contains` check is
   * what separates leaving the region from moving around inside it.
   */
  const onDragLeave = useCallback(
    (event: React.DragEvent) => {
      if (!event.currentTarget.contains(event.relatedTarget as Node | null)) {
        stop();
      }
    },
    [stop],
  );

  // A drag can end in ways this region never sees — the card dropped on another
  // element, the browser cancelling it, or the component unmounting mid-gesture.
  // Any of those would leave a frame loop scrolling a detached node forever.
  useEffect(() => stop, [stop]);

  return { regionRef, onDragOver, onDragLeave, stop };
}
