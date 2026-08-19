// =============================================================================
// dragReorder.tsx — one implementation of "drag a row to re-order it"
// =============================================================================
//
// ## Why this file exists (nav cleanup Part 2, Roman's drag item)
//
// The task asked for "the scenario-facts drag-handle component (⠿)" to be reused
// on the practice deck. There is no such component. The facts drag is woven
// through `FactRow` — a card that also renders tier pickers, chips, a spine, a
// weight control and an `evidenceCardView`, all typed to `WorkingRow` and
// `AllegationOptions`. None of that means anything to a practice question.
//
// What IS reusable is the MECHANICS, and they are not obvious enough to retype:
//
//   · `dragstart` must call `setData`, or **Firefox silently cancels the drag** —
//     no event, no error, nothing on screen. That cost Roman a bug report once
//     already (it worked in Chrome, so it worked under test).
//   · `dragover` must call `preventDefault`, or the browser refuses the drop
//     outright. It reads like styling and is not.
//   · The dragged id lives in React state, not in the drag payload — the payload
//     is a formality the browser demands.
//
// So the mechanics move here and both surfaces call them. That is the smallest
// honest adaptation: one place where the Firefox fix lives, one place where the
// drop semantics are written down.

import React, { useState, type CSSProperties } from "react";

/**
 * The ⠿ grip, exactly as the facts table draws it.
 *
 * A `<span>` and not a `<button>`: it is not a control that does something when
 * pressed, it is the part of the row you take hold of. The row carries
 * `draggable`, not this — a handle that were itself draggable would let you drag
 * the grip out of its own row.
 */
export const DragHandle: React.FC<{ hint: string; style?: CSSProperties }> = ({
  hint,
  style,
}) => (
  <span
    aria-label={hint}
    title={hint}
    style={{ cursor: "grab", color: "var(--text-secondary)", ...style }}
  >
    ⠿
  </span>
);

/**
 * The four handlers a re-orderable row needs, plus whether it is being hovered.
 *
 * `onPickUp` records which row is moving (the caller holds that state, because
 * only the caller knows the list). `onDropHere` is told nothing — the caller
 * already knows both ends: the one it picked up and the one it is calling this
 * on.
 *
 * ## React Learning: why this returns props instead of being a component
 *
 * A wrapper component would have to own the row's element, its styling and its
 * children — and both callers already own theirs, with different shapes. A
 * function returning a props object composes into an existing element instead:
 * `<div {...reorderProps({…})} style={…}>`. The pattern is what the React docs
 * call a "prop getter", and it is the lightest way to share behaviour without
 * also dictating markup.
 */
export function reorderProps(options: {
  /** False turns every handler off — the row is inert, not merely unstyled. */
  enabled: boolean;
  onPickUp: () => void;
  onDropHere: () => void;
  /** Told when this row becomes (or stops being) the hovered drop target. */
  onHover: (over: boolean) => void;
}): React.HTMLAttributes<HTMLElement> & { draggable: boolean } {
  const { enabled, onPickUp, onDropHere, onHover } = options;
  return {
    draggable: enabled,
    onDragStart: (event: React.DragEvent) => {
      if (!enabled) return;
      // Firefox CANCELS a drag whose `dragstart` sets no data — the drag simply
      // never begins. Chrome does not require it, which is exactly why this is
      // the kind of bug that ships. The value is unused; that data EXISTS is the
      // whole point.
      event.dataTransfer?.setData("text/plain", "row");
      if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
      onPickUp();
    },
    onDragOver: (event: React.DragEvent) => {
      if (!enabled) return;
      // Without this the browser refuses the drop. Not a styling concern.
      event.preventDefault();
      onHover(true);
    },
    onDragLeave: () => onHover(false),
    onDrop: (event: React.DragEvent) => {
      if (!enabled) return;
      event.preventDefault();
      onHover(false);
      onDropHere();
    },
  };
}

/**
 * Track which row is hovered, so a row can show where a drop would land.
 *
 * A hook rather than a `useState` in each caller, because "am I the hovered
 * target" is per-ROW state and forgetting to clear it on `dragleave` leaves a
 * row highlighted after the pointer has gone — a stale highlight that reads as
 * a selection.
 */
export function useDropTarget(): [boolean, (over: boolean) => void] {
  const [over, setOver] = useState(false);
  return [over, setOver];
}

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
