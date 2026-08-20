/**
 * Where a dropped row lands.
 *
 * `dropPosition` is the browser half of the drag: it names the NEIGHBOUR a
 * dropped row goes above, never an ordinal — the number is the server's,
 * derived from what is stored. The server's own rule is pinned separately in
 * `practice_reorder_tests.rs`, and the two must agree; these tests are the
 * browser side of that agreement.
 */
import { describe, expect, it } from "vitest";

import { dropPosition } from "../dragReorder";

/** Rows keyed the way the practice deck keys them. */
const rows = [{ id: "a" }, { id: "b" }, { id: "c" }, { id: "d" }];
const id = (row: { id: string }) => row.id;

describe("dropPosition", () => {
  it("names the row the dragged one lands above", () => {
    // Drag d onto a: d goes above a, so `before` is a.
    expect(dropPosition(rows, id, "d", "a")).toEqual({ before: "a" });
  });

  it("names the target even when dragging DOWN the list", () => {
    // Drag a onto d. The dragged row is lifted out first, so the target's
    // identity is unaffected by its own index shifting.
    expect(dropPosition(rows, id, "a", "d")).toEqual({ before: "d" });
  });

  it("returns the target's own id for a drop onto the next row down", () => {
    // "Drop onto Y" means "land above Y", so this asks for the arrangement that
    // already exists. It is a legal gesture, not a refusal — the SERVER decides
    // it changes nothing. A `null` here would make the browser silently swallow
    // a drag the server would have accepted.
    expect(dropPosition(rows, id, "b", "c")).toEqual({ before: "c" });
  });

  it("refuses a drop onto itself", () => {
    // The gesture a person makes constantly by accident: starting a drag and
    // changing their mind. Nothing is sent.
    expect(dropPosition(rows, id, "b", "b")).toBeNull();
  });

  it("refuses a target the list does not hold", () => {
    // A drop across sides, or onto a row that was removed while the drag was in
    // flight. The caller does nothing rather than sending a request the server
    // would have to refuse.
    expect(dropPosition(rows, id, "a", "zzz")).toBeNull();
  });

  it("works on a list keyed by something other than `id`", () => {
    // The generic accessor is the whole reason both surfaces can share this:
    // the facts table keys on `graphNodeId`, the practice deck on `id`.
    const facts = [{ graphNodeId: "g1" }, { graphNodeId: "g2" }];
    expect(dropPosition(facts, (f) => f.graphNodeId, "g2", "g1")).toEqual({
      before: "g1",
    });
  });
});
