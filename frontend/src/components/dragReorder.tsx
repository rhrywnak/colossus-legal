// =============================================================================
// dragReorder.tsx — the two pieces of "drag a row to re-order it" that outlived
// the mechanism
// =============================================================================
//
// ## What this file was, and what happened to it (2026-09-10)
//
// It held the MECHANICS of native HTML5 drag-and-drop — `draggable`, the
// `dragstart`/`dragover`/`drop` handlers, and the two browser quirks that make
// them work — shared by the practice deck and, the header claimed, the scenario
// facts table. The claim was stale: `FactRow` has always carried its own inline
// copy and never imported this.
//
// The practice deck moved to dnd-kit tonight, because native HTML5 drag has no
// press-and-hold, does not slide the rows apart, and does not work reliably on a
// touch screen at all — and Chuck edits Marie's decks on an iPad. So
// `reorderProps` and `useDropTarget`, which existed only to wrap those events,
// are gone. The Firefox `setData` note and the `preventDefault` note went with
// them; they are recorded in the git history of this file and in
// `CC_REPORT_DECK_DRAG_AND_ADD_v1`, and `FactRow` still carries its own.
//
// Two things survive, because both are about what a drag MEANS rather than how a
// browser reports one:
//
//   · `dropPosition` — where a dropped row lands, named as a NEIGHBOUR. The
//     server resolves it against what is stored; the browser never computes an
//     ordinal. dnd-kit reports the same two ids the HTML5 handlers did, so this
//     function did not change when the mechanism under it did, and its six tests
//     did not either.
//   · `DragHandle` — the ⠿ grip. Now the ONLY handle, rather than decoration on
//     a row that was draggable everywhere.

import React, { type CSSProperties, type HTMLAttributes } from "react";

// The grip's target box, EXPORTED so a test can assert it — these two numbers are
// the fix, and a constant nothing asserts drifts back. (The same reason
// `deckSortable.ts` exports `SENSOR_CONSTRAINTS`.)
//
// 44 is Apple's HIG minimum touch target (44pt), and the
// same figure WCAG 2.5.5 and Material name; NN/g's drag guidance asks separately
// for a grab handle that is VISIBLY grabbable, which is what the glyph size and
// the hover tint below are for.
//
// STRUCTURAL: a human-factors constant, not deployment configuration. It
// describes the size of a fingertip, which is the same on Chuck's iPad and on a
// machine nobody has built yet. See `deckSortable.ts` SENSOR_CONSTRAINTS for the
// same judgment applied to the timing side of the same gesture — and the same
// caveat: what would make it vary is an ACCESSIBILITY preference, which is
// per-person and belongs with the others whenever those exist.
export const GRIP_TARGET_PX = 44;

// The glyph inside it. 13 px — what the practice row asked for until 2026-09-10 —
// is a mark you aim at; 20 px is one you can see you are aiming at.
export const GRIP_GLYPH_PX = 20;

/**
 * The ⠿ grip — the one thing you take hold of to re-order a row.
 *
 * A `<span>` and not a `<button>`: it is not a control that does something when
 * pressed, it is the part of the row you take hold of. (dnd-kit's `attributes`
 * put `role="button"` and a `tabIndex` on it anyway, which is what a screen
 * reader and the keyboard sensor need; the element stays a span so nothing
 * submits a form or steals a click.)
 *
 * ## It is a TARGET, not a mark (2026-09-10)
 *
 * It rendered at `fontSize: 13` inside a `minWidth: 24` box — about 13×16 px of
 * actual hittable area — against a 44pt minimum in Apple's HIG and the same
 * figure in WCAG 2.5.5. On an iPad it was not reliably hittable at all, and with
 * a mouse it wanted aiming. It is now a 44×44 box around a 20 px glyph, with a
 * tint that answers a pointer so a person can see what they have got hold of —
 * NN/g's "visible grab handle".
 *
 * Nothing at rest, deliberately: eleven rows each carrying a permanent grey patch
 * would read as a column of buttons rather than as a texture you can grab.
 *
 * ## `handleProps` — what changed on 2026-09-10
 *
 * The row used to carry `draggable` and this was decoration; a drag could begin
 * anywhere on the row, including on the question text. Under dnd-kit the grip is
 * the ONLY handle, so it is the grip that carries the sensor's `attributes` and
 * `listeners` — pass them here and this becomes the thing you take hold of, in
 * fact and not only in appearance.
 *
 * That is the Primer/HIG pattern and it buys two things at once: the row body
 * stays fully clickable (its links, its buttons, its field stack), and on a
 * touch screen a finger dragged across the question text scrolls the list
 * instead of picking the row up.
 *
 * `undefined` — the default, and what a caller with no drag passes — leaves this
 * exactly the inert span it has always been.
 *
 * ## TS note: `HTMLAttributes<HTMLSpanElement>` as the prop type
 *
 * dnd-kit's `attributes` and `listeners` are both plain objects of DOM props
 * (`role`, `tabIndex`, `aria-*`, `onPointerDown`, `onKeyDown`), so the honest
 * type is the element's own attribute bag rather than an import from the
 * library. This file then knows nothing about dnd-kit, which is why it can stay
 * shared and why the facts table could adopt the same grip without adopting the
 * dependency.
 */
export const DragHandle: React.FC<{
  hint: string;
  style?: CSSProperties;
  handleProps?: HTMLAttributes<HTMLSpanElement>;
}> = ({ hint, style, handleProps }) => {
  const [hovered, setHovered] = React.useState(false);
  const [pressed, setPressed] = React.useState(false);
  const grabbable = handleProps !== undefined;

  return (
    <span
      aria-label={hint}
      title={hint}
      {...handleProps}
      // ⚑ Every handler below is COMPOSED, never replaced. `handleProps` carries
      // dnd-kit's `onPointerDown` (and `onKeyDown` from the keyboard sensor), and
      // an `onPointerDown` of our own written after the spread would silently
      // shadow it — the hover tint would work and the drag would stop. dnd-kit's
      // runs first in every one of these.
      onMouseEnter={(event) => {
        handleProps?.onMouseEnter?.(event);
        setHovered(true);
      }}
      onMouseLeave={(event) => {
        handleProps?.onMouseLeave?.(event);
        setHovered(false);
        setPressed(false);
      }}
      onPointerDown={(event) => {
        handleProps?.onPointerDown?.(event);
        setPressed(true);
      }}
      onPointerUp={(event) => {
        handleProps?.onPointerUp?.(event);
        setPressed(false);
      }}
      // A drag that ends outside the grip — which is every successful drag —
      // fires neither `pointerup` here nor `mouseleave` reliably, so without this
      // the grip would stay visibly pressed after the row was dropped.
      onPointerCancel={(event) => {
        handleProps?.onPointerCancel?.(event);
        setPressed(false);
      }}
      onLostPointerCapture={(event) => {
        handleProps?.onLostPointerCapture?.(event);
        setPressed(false);
      }}
      style={{
        // The TARGET, sized the same whether or not it is currently grabbable.
        // `canDrag` goes false for the moment a write is in flight, and a box
        // that resized on that would make the row jump on every save.
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        minWidth: GRIP_TARGET_PX,
        minHeight: GRIP_TARGET_PX,
        fontSize: GRIP_GLYPH_PX,
        lineHeight: 1,
        borderRadius: 8,
        // Nothing at rest: eleven rows each with a grey patch would read as a
        // column of buttons. The tint is the answer to a pointer arriving.
        background: pressed
          ? "var(--border-default)"
          : hovered
            ? "var(--bg-page)"
            : "transparent",
        color: hovered || pressed ? "var(--text-primary)" : "var(--text-secondary)",
        transition: "background 120ms ease, color 120ms ease",
        // Dragging a glyph otherwise selects it, and the selection survives the
        // drop as a blue smear across the grip.
        userSelect: "none",
        ...(grabbable
          ? {
              cursor: pressed ? "grabbing" : "grab",
              // Without this the browser claims the gesture for scrolling and
              // the touch sensor never sees it. It is what makes the iPad work.
              touchAction: "none",
            }
          : { cursor: "default" }),
        ...style,
      }}
    >
      ⠿
    </span>
  );
};

/**
 * Where a dropped row lands: the ids of `items`, re-sequenced.
 *
 * Dropping X ONTO Y means "put X where Y is", i.e. immediately above Y — so a
 * drop onto the row directly below X asks for the arrangement that already
 * exists. That is deliberate and shared with the facts table; the practice
 * repository's `resequenced` pins the same rule server-side.
 *
 * Returns `null` when the drop names no position (same row, or a target not in
 * the list). The caller does nothing rather than sending a request the server
 * would have to refuse.
 *
 * ## TS note: the `getId` callback
 *
 * Generic over the item so both callers keep their own row type — the facts
 * table keys on `graphNodeId`, the practice deck on `id`. Passing an accessor
 * rather than requiring an `{ id: string }` shape means neither has to build a
 * throwaway array to use this.
 */
export function dropPosition<T>(
  items: T[],
  getId: (item: T) => string,
  draggedId: string,
  targetId: string,
): { before: string | null } | null {
  if (draggedId === targetId) return null;
  const without = items.filter((item) => getId(item) !== draggedId);
  const at = without.findIndex((item) => getId(item) === targetId);
  if (at === -1) return null;
  return { before: getId(without[at]) };
}
