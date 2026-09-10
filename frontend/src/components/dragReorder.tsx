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

/**
 * The ⠿ grip, exactly as the facts table draws it.
 *
 * A `<span>` and not a `<button>`: it is not a control that does something when
 * pressed, it is the part of the row you take hold of.
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
}> = ({ hint, style, handleProps }) => (
  <span
    aria-label={hint}
    title={hint}
    {...handleProps}
    style={{
      cursor: handleProps ? "grab" : "default",
      color: "var(--text-secondary)",
      // A grip you can put a finger on. 24 px is the minimum touch target every
      // one of the cited guidelines names, and the ⠿ glyph alone is about eight.
      ...(handleProps ? { display: "inline-block", minWidth: 24, touchAction: "none" } : {}),
      ...style,
    }}
  >
    ⠿
  </span>
);

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
