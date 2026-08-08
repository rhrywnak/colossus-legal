// =============================================================================
// rulingAcknowledgment.test.ts — every ruling says what it did (2026-08-08)
// =============================================================================
//
// The defect these guard: on beta.385 a defer landed in the database — anchor,
// reference row, provenance, all measured afterwards — and the screen said
// nothing, so the feature was reported dead. A correct write plus a correct
// filter, with nothing said in between, is indistinguishable from a dead button.

import { describe, expect, it } from "vitest";

import { rulingAcknowledgment } from "../rulingAcknowledgment";
import type { LinkPanelWording } from "../../services/evidenceLinks";
import type { ScenarioCard } from "../../services/scenarioCards";

/** Only the five templates this module reads — the rest of the panel's wording
 *  is irrelevant here and a full fixture would obscure which strings matter. */
const WORDS = {
  card_ruling_saved_template: "Saved — {code} is now {state}.",
  card_ruling_left_filter_template:
    "{code} has left the {filter} list — that is where ruled candidates go.",
  card_defer_recorded_template: "Deferred. The reason recorded is: {reason}",
  card_ruling_failed_template:
    "{code} could not be saved: {detail} The queue has been reloaded, so what you see now is what is stored.",
  card_locked_condition_label: "Include and Exclude are closed on this card:",
} as unknown as LinkPanelWording;

const card = (over: Partial<ScenarioCard> = {}) =>
  ({ graph_node_id: "ev-1", code: "C-14", ...over }) as ScenarioCard;

function ack(over: Record<string, unknown> = {}) {
  return rulingAcknowledgment({
    outcome: { graphNodeId: "ev-1", action: "include", failure: null },
    card: card(),
    state: "included",
    stateLabel: "Included",
    leftTheList: false,
    filterLabel: "Proposed",
    wording: WORDS,
    ...over,
  } as Parameters<typeof rulingAcknowledgment>[0]);
}

describe("a ruling that lands", () => {
  it("names the card and the state it is now in", () => {
    // The minimum an acknowledgment owes: on a list where thirty cards look
    // alike, one that cannot name its card is the same as none.
    expect(ack()?.text).toBe("Saved — C-14 is now Included.");
    expect(ack()?.failed).toBe(false);
  });

  it("SAYS SO when the ruling takes the card out of the list", () => {
    // THE VANISH — the measured beta.385 defect. Ruling a proposed card makes it
    // human-touched, precedence stops proposing it, and it correctly leaves the
    // Proposed list. Silence here is what made a working defer read as dead.
    const receipt = ack({ leftTheList: true });

    expect(receipt?.text).toContain("Saved — C-14 is now Included.");
    expect(receipt?.text).toContain("C-14 has left the Proposed list");
  });

  it("stays quiet about the list when the card stayed put", () => {
    // A sentence explaining that nothing moved is noise on a surface whose job is
    // to be read at speed.
    expect(ack({ leftTheList: false })?.text).not.toContain("has left");
  });
});

describe("a locked card's one-press defer", () => {
  it("shows the human the sentence they just signed", () => {
    // A locked card carries the system's own reason, so Defer commits in ONE
    // press with no prompt — prompting would ask the human to retype a sentence
    // the server wrote. They should still be able to READ it: this is the exact
    // card and the exact path the architect walked on .385 and saw nothing from.
    const reason =
      "A scan scored this item, but it is not linked to any accusation yet, so there " +
      "is nothing for it to support or dispute.";
    const receipt = ack({
      outcome: { graphNodeId: "ev-1", action: "defer", reason, failure: null },
      state: "deferred",
      stateLabel: "Deferred",
    });

    expect(receipt?.text).toContain(reason);
    expect(receipt?.failed).toBe(false);
  });

  it("still says where the card went", () => {
    const receipt = ack({
      outcome: { graphNodeId: "ev-1", action: "defer", reason: "page missing", failure: null },
      leftTheList: true,
      stateLabel: "Deferred",
    });
    expect(receipt?.text).toContain("page missing");
    expect(receipt?.text).toContain("has left the Proposed list");
  });
});

describe("a ruling that is refused", () => {
  it("surfaces the failure, naming the card and the cause", () => {
    // The rider's other half. A refusal that names neither sends the reader to
    // the logs, which is the silent-failure shape Standing Rule 1 exists to end.
    const receipt = ack({
      outcome: { graphNodeId: "ev-1", action: "include", failure: "409 Conflict" },
    });

    expect(receipt?.failed).toBe(true);
    expect(receipt?.text).toContain("C-14");
    expect(receipt?.text).toContain("409 Conflict");
    // And it tells the human the screen has been reconciled — without that they
    // have no reason to trust what they are looking at afterwards.
    expect(receipt?.text).toContain("what is stored");
  });

  it("reports a failure even when the pool no longer holds the card", () => {
    // A reload landing mid-ruling. The failure still has to be said; it simply
    // cannot name a code, and a control word stands in for one.
    const receipt = ack({
      card: undefined,
      state: null,
      stateLabel: null,
      outcome: { graphNodeId: "ev-gone", action: "drop", failure: "network error" },
    });

    expect(receipt?.failed).toBe(true);
    expect(receipt?.text).toContain("network error");
    expect(receipt?.text).not.toContain("undefined");
  });
});

describe("the configuration law", () => {
  it("says nothing at all before the wording has loaded", () => {
    // R4, the absent-not-fake law: there is no compiled-in sentence to fall back
    // to, and a receipt that invented its own words would be the one line on this
    // screen the settings store cannot reach.
    expect(ack({ wording: null })).toBeNull();
  });

  it("renders whatever the stored template says, inventing nothing", () => {
    const reworded = {
      ...WORDS,
      card_ruling_saved_template: "{state}: {code}",
    } as LinkPanelWording;

    expect(ack({ wording: reworded })?.text).toBe("Included: C-14");
  });

  it("leaves an unknown placeholder visible rather than blanking it", () => {
    // A visible `{oops}` is a fault somebody reports; a silently blanked one is a
    // sentence that quietly means something else. The backend's own `render`
    // makes the same choice for the same reason.
    const broken = {
      ...WORDS,
      card_ruling_saved_template: "Saved — {code} is now {oops}.",
    } as LinkPanelWording;

    expect(ack({ wording: broken })?.text).toContain("{oops}");
  });
});
