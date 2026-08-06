/**
 * Tests for `rehearsalEdits` — which act reaches which guarded route.
 *
 * This is the kind of wiring that is invisible when wrong. "What this is" aimed
 * at the accusation route would save the theme into the accusation column and
 * look perfectly fine until somebody read the rehearsal page — and by then the
 * plain-words accusation a human wrote would be gone.
 *
 * Every service call is mocked, so what these assert is the ROUTING and the
 * arguments, which is precisely what a copy-paste between two near-identical
 * lines would get wrong.
 */
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../services/scenarioAccusation", () => ({
  setAccusationText: vi.fn().mockResolvedValue(undefined),
}));
vi.mock("../../services/scenarioAugmentation", () => ({
  addHumanFact: vi.fn().mockResolvedValue(undefined),
  editTalkingPoint: vi.fn().mockResolvedValue(undefined),
  editWatchItem: vi.fn().mockResolvedValue(undefined),
  setTalkingPoints: vi.fn().mockResolvedValue(undefined),
}));
vi.mock("../../services/scenarioCrud", () => ({
  updateScenario: vi.fn().mockResolvedValue(undefined),
}));

import { rehearsalEdits } from "../rehearsalEdits";
import { setAccusationText } from "../../services/scenarioAccusation";
import {
  addHumanFact,
  editTalkingPoint,
  editWatchItem,
  setTalkingPoints,
} from "../../services/scenarioAugmentation";
import { updateScenario } from "../../services/scenarioCrud";
import type { RehearsalScenario } from "../../services/rehearsal";

const SCENARIO_ID = "3f1b0a9e-2c4d-4e5f-8a7b-6c5d4e3f2a1b";

/** Only the fields this factory reads; the rest of the payload is irrelevant. */
const scenario = {
  scenario_id: SCENARIO_ID,
  points: [{ position: 1, text: "I sent a certified letter." }],
} as unknown as RehearsalScenario;

/**
 * The page's runner, reduced to its contract: call the write, resolve with the
 * failure, and resolve with `null` when there was none.
 */
const run = async (write: () => Promise<void>) => {
  await write();
  return null;
};

describe("rehearsalEdits", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("saves 'What this is' through the scenario route, theme only", async () => {
    // A partial body. Sending `motivation` would put strategy on a surface §10
    // keeps it off; sending `name` would rename the scenario from a page that
    // never showed the human its name field.
    rehearsalEdits("awad", scenario, run, false).onSaveWhat("The fight over the estate.");
    await Promise.resolve();

    expect(updateScenario).toHaveBeenCalledWith("awad", SCENARIO_ID, {
      theme_statement: "The fight over the estate.",
    });
    expect(setAccusationText).not.toHaveBeenCalled();
  });

  it("saves the accusation through the accusation route, and passes null to withdraw", async () => {
    // `null` is "Withdraw it", and it has to reach the route as null: an empty
    // string is refused by the backend on purpose, because withdrawing and
    // mistyping are different intentions.
    rehearsalEdits("awad", scenario, run, false).onSaveAccusation(null);
    await Promise.resolve();

    expect(setAccusationText).toHaveBeenCalledWith("awad", SCENARIO_ID, null);
    expect(updateScenario).not.toHaveBeenCalled();
  });

  it("edits ONE talking point by its printed position", async () => {
    // Not the whole-list write: that one deletes and re-inserts every row, which
    // re-stamps each one's author and its written-on date.
    await rehearsalEdits("awad", scenario, run, false).onEditPoint(2, "Corrected words");

    expect(editTalkingPoint).toHaveBeenCalledWith("awad", SCENARIO_ID, 2, "Corrected words");
    expect(setTalkingPoints).not.toHaveBeenCalled();
  });

  it("adds a talking point as the existing list plus one, in order", async () => {
    // The ordering is server-owned and the LIST write is what owns it. Sending
    // only the new point would delete the others.
    await rehearsalEdits("awad", scenario, run, false).onAddPoint("A second point");

    expect(setTalkingPoints).toHaveBeenCalledWith("awad", SCENARIO_ID, [
      "I sent a certified letter.",
      "A second point",
    ]);
  });

  it("edits ONE watch item by its row id", async () => {
    await rehearsalEdits("awad", scenario, run, false).onEditWatchItem("fact-1", "Reworded");

    expect(editWatchItem).toHaveBeenCalledWith("awad", SCENARIO_ID, "fact-1", "Reworded");
  });

  it("adds a watch item with the kind that separates it from a human fact", async () => {
    // One table holds both, distinguished by `kind`. Forgetting it would file a
    // thing-to-expect at trial as a fact about the case.
    await rehearsalEdits("awad", scenario, run, false).onAddWatchItem("He will say she is difficult");

    expect(addHumanFact).toHaveBeenCalledWith("awad", SCENARIO_ID, {
      text: "He will say she is difficult",
      kind: "watch_list",
    });
  });

  it("hands a list write's failure back so the row can keep its draft", async () => {
    // `AuthoredLineEditor` catches this and keeps the human's words on screen.
    // Swallowing it here would close the editor on a save that never happened.
    // The real runner reports the failure in the page banner and RESOLVES with
    // it; `rethrowing` is what turns it back into a rejection for the row.
    const failing = async () => new Error("HTTP 500");

    await expect(
      rehearsalEdits("awad", scenario, failing, false).onEditPoint(1, "x"),
    ).rejects.toThrow("HTTP 500");
  });

  it("does not let a sentence write's failure become an unhandled rejection", async () => {
    // The sentence editors have no failure surface of their own — the page's
    // banner reports it, and `run` has already put it there. What must not
    // happen is an unhandled promise rejection, and the runner resolving with
    // the failure rather than throwing it is what makes that impossible.
    const failing = async () => new Error("HTTP 500");

    expect(() =>
      rehearsalEdits("awad", scenario, failing, false).onSaveAccusation("text"),
    ).not.toThrow();
    await Promise.resolve();
  });
});
