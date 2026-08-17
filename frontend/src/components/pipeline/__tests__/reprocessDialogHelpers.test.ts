import { describe, it, expect } from "vitest";
import { evaluateGuard, CuratedState } from "../reprocessDialogHelpers";

const DOC = "doc-court-of-appeals-rulling-01-12-2012";

const guard = (curated: CuratedState, typed = "", running = false) =>
  evaluateGuard({ curated, typed, documentId: DOC, running });

describe("evaluateGuard", () => {
  it("asks for nothing when the document carries no rulings", () => {
    const g = guard({ kind: "loaded", total: 0 });
    expect(g.needsTypedId).toBe(false);
    expect(g.canRun).toBe(true);
  });

  it("demands the typed id when the document carries rulings", () => {
    // The live Court of Appeals ruling: 225 curated rows.
    const g = guard({ kind: "loaded", total: 225 });
    expect(g.needsTypedId).toBe(true);
    expect(g.canRun).toBe(false);
  });

  it("enables only on an exact id match", () => {
    expect(guard({ kind: "loaded", total: 225 }, DOC).canRun).toBe(true);
    expect(guard({ kind: "loaded", total: 225 }, `  ${DOC}  `).canRun).toBe(true);
    expect(guard({ kind: "loaded", total: 225 }, DOC.toUpperCase()).canRun).toBe(false);
    expect(guard({ kind: "loaded", total: 225 }, DOC.slice(0, -1)).canRun).toBe(false);
    expect(guard({ kind: "loaded", total: 225 }, "yes").canRun).toBe(false);
  });

  it("treats a FAILED count as at-risk, not as zero", () => {
    // The case that matters: a broken count must not read as "nothing at stake".
    const g = guard({ kind: "failed" });
    expect(g.needsTypedId).toBe(true);
    expect(g.canRun).toBe(false);
    expect(guard({ kind: "failed" }, DOC).canRun).toBe(true);
  });

  it("blocks while the count is still loading, whatever was typed", () => {
    expect(guard({ kind: "loading" }).canRun).toBe(false);
    expect(guard({ kind: "loading" }, DOC).canRun).toBe(false);
  });

  it("blocks while a request is already in flight", () => {
    expect(guard({ kind: "loaded", total: 0 }, "", true).canRun).toBe(false);
    expect(guard({ kind: "loaded", total: 225 }, DOC, true).canRun).toBe(false);
  });
});
