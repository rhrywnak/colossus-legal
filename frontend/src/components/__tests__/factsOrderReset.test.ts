/**
 * "Forget where every fact was placed" (Piece 5b), testable at last.
 *
 * This lived inside `ScenarioFactsSection` until change D split it out, which
 * means its partial-failure behaviour — the part with the most reasoning
 * attached and the most to go wrong — has never had a test. Rule 30: no
 * component tier here, so a rule inside a component is a rule no test reaches.
 *
 * The write itself is stubbed with `vi.mock`; what is asserted is the DECISION
 * layer around it — which cards are written, what the human is told, and whether
 * the caller re-reads.
 */
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../services/scenarioFactCuration", () => ({
  clearFactOrder: vi.fn(),
}));

import { resetFactOrder, type ResetOrderContext } from "../factsOrderReset";
import { clearFactOrder } from "../../services/scenarioFactCuration";
import type { ScenarioCard } from "../../services/scenarioCards";
import type { CardGrammarWording } from "../../services/evidenceLinks";

const cleared = vi.mocked(clearFactOrder);

/** Only the two fields the loop reads: the id it writes, and whether it is placed. */
function card(id: string, sortOrdinal: number | null): ScenarioCard {
  return {
    graph_node_id: id,
    ...(sortOrdinal === null ? {} : { sort_ordinal: sortOrdinal }),
  } as unknown as ScenarioCard;
}

/** The two templates this module fills. Values match the migration's seed shape. */
const grammar = {
  reset_order_done_template: "Order reset — {count} cleared.",
  reset_order_failed_template: "Some could not be cleared — {reason}",
} as unknown as CardGrammarWording;

function ctx(over: Partial<ResetOrderContext> = {}): ResetOrderContext {
  return {
    slug: "awad-v-cfs",
    scenarioId: "s-7",
    cards: [card("a", 10240), card("b", 20480)],
    grammar,
    setError: vi.fn(),
    setNotice: vi.fn(),
    onDone: vi.fn(),
    ...over,
  };
}

beforeEach(() => {
  cleared.mockReset();
  cleared.mockResolvedValue(undefined);
});

describe("resetFactOrder", () => {
  it("writes only the PLACED facts", () => {
    // The loop is bounded by what a human actually dragged. A fact nobody placed
    // has nothing to clear, and sending a write for it would turn a handful of
    // requests into forty-six against an audited route.
    const c = ctx({ cards: [card("a", 10240), card("unplaced", null), card("b", 20480)] });
    return resetFactOrder(c).then(() => {
      expect(cleared).toHaveBeenCalledTimes(2);
      expect(cleared).toHaveBeenCalledWith("awad-v-cfs", "s-7", "a");
      expect(cleared).toHaveBeenCalledWith("awad-v-cfs", "s-7", "b");
      expect(cleared).not.toHaveBeenCalledWith("awad-v-cfs", "s-7", "unplaced");
    });
  });

  it("acknowledges what ACTUALLY cleared and clears the error", async () => {
    const c = ctx();
    await resetFactOrder(c);
    expect(c.setNotice).toHaveBeenCalledWith("Order reset — 2 cleared.");
    expect(c.setError).toHaveBeenCalledWith(null);
    expect(c.onDone).toHaveBeenCalledTimes(1);
  });

  it("reports a PARTIAL failure and counts only the successes", async () => {
    // The reasoning this module exists to protect: a loop that said "reset"
    // after one refusal would leave the order half-cleared with the screen
    // claiming otherwise (Standing Rule 1). Both sentences are shown — the count
    // is what landed, the failure names why.
    cleared
      .mockResolvedValueOnce(undefined)
      .mockRejectedValueOnce(new Error("the server answered HTTP 409"));

    const c = ctx();
    await resetFactOrder(c);

    expect(c.setNotice).toHaveBeenCalledWith("Order reset — 1 cleared.");
    expect(c.setError).toHaveBeenCalledWith(
      "Some could not be cleared — the server answered HTTP 409",
    );
    // …and it still re-reads: one fact DID move, and leaving the screen showing
    // the old order would be a second lie on top of the first.
    expect(c.onDone).toHaveBeenCalledTimes(1);
  });

  it("names ONE reason even when every write fails", async () => {
    // Forty-six identical "connection lost" lines are less readable than one,
    // and the count already says how many failed.
    cleared.mockRejectedValue(new Error("connection lost"));

    const c = ctx();
    await resetFactOrder(c);

    expect(c.setError).toHaveBeenCalledWith("Some could not be cleared — connection lost");
    expect(c.setNotice).toHaveBeenCalledWith("Order reset — 0 cleared.");
  });

  it("survives a rejection that carries no Error", async () => {
    // `Promise.reject("nope")` is legal and a fetch layer can produce one. The
    // template must still fill rather than printing "[object Object]" or
    // throwing inside the handler that was reporting the failure.
    cleared.mockRejectedValue("nope");
    const c = ctx();
    await resetFactOrder(c);
    expect(c.setError).toHaveBeenCalledWith("Some could not be cleared — nope");
  });

  it("acknowledges a reset with nothing to clear, rather than staying silent", async () => {
    // A human who clicks Reset order on a list nobody has dragged has asked a
    // question and deserves an answer. Silence reads as a control that did not
    // work — which is exactly what "every action acknowledges itself" means.
    const c = ctx({ cards: [card("a", null)] });
    await resetFactOrder(c);
    expect(cleared).not.toHaveBeenCalled();
    expect(c.setNotice).toHaveBeenCalledWith("Order reset — 0 cleared.");
    expect(c.setError).toHaveBeenCalledWith(null);
  });

  it("REFUSES, loudly, when the stored words have not loaded", async () => {
    // Unreachable through the UI — the control is withheld until the wording
    // lands — but not a silent `return`: a second caller wired up without it
    // would otherwise get a confirm dialog, a click, and nothing at all.
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    const c = ctx({ grammar: null });

    await resetFactOrder(c);

    expect(cleared, "nothing may be written without words to report it").not.toHaveBeenCalled();
    expect(c.setError).toHaveBeenCalledWith(
      "The order could not be reset — please reload and try again.",
    );
    expect(c.setNotice).not.toHaveBeenCalled();
    expect(c.onDone, "no re-read: nothing changed").not.toHaveBeenCalled();
    expect(warn).toHaveBeenCalled();
    warn.mockRestore();
  });
});
