// =============================================================================
// factActionBody.test.ts — the frontend half of the Include wire contract
// =============================================================================
//
// The Include button returned HTTP 400 for months and both test suites were
// green the whole time. That is the part worth sitting with: the backend's tests
// built a request in Rust and asserted the service accepted it; this suite
// asserted the body builder produced what this suite expected. Neither side ever
// saw the other's bytes, so the one thing that was wrong — that they disagreed —
// was the one thing nothing could fail on.
//
// `contracts/fact_action_include.json` is the shared artifact that closes it.
// This file asserts the builder emits exactly those bytes;
// `backend/src/dto/scenario_facts_wire_tests.rs` asserts those same bytes parse
// as `FactActionRequest` and survive `include_link`. Break the agreement from
// either side and a test fails in the OTHER language.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { factActionBody } from "../scenarioGather";

/**
 * The contract file, resolved from THIS file rather than from the working
 * directory — vitest's cwd is `frontend/`, but a test that depended on that
 * would break the first time somebody ran it from the repo root.
 */
const CONTRACT = readFileSync(
  fileURLToPath(new URL("../../../../contracts/fact_action_include.json", import.meta.url)),
  "utf8",
).trim();

describe("the include body", () => {
  it("is the contract file, byte for byte (M)", () => {
    // MUTATION: drop `allegation_id` from the builder and this goes red — which
    // is precisely the defect, caught here instead of on PROD S-13.
    const body = factActionBody("include", {
      allegationId: "alleg-7",
      stance: "supports",
    });

    expect(body).toBe(CONTRACT);
  });

  it("names the three keys the route requires", () => {
    const parsed = JSON.parse(
      factActionBody("include", { allegationId: "alleg-7", stance: "supports" }),
    );

    expect(Object.keys(parsed).sort()).toEqual(["action", "allegation_id", "stance"]);
    expect(parsed.allegation_id).toBe("alleg-7");
    expect(parsed.stance).toBe("supports");
  });

  it("carries `rebuts` unchanged — the graph's own token, never a synonym", () => {
    const parsed = JSON.parse(
      factActionBody("include", { allegationId: "alleg-9", stance: "rebuts" }),
    );

    expect(parsed.stance).toBe("rebuts");
  });

  it("omits the pair entirely when the picker has not answered", () => {
    // A HALF pair is what the route refuses by name. Sending one would turn a
    // picker bug into an HTTP error a human has to interpret, so the builder
    // sends neither and the row's own Save stays disabled until it has both.
    expect(factActionBody("include", { allegationId: "alleg-7" })).toBe('{"action":"include"}');
    expect(factActionBody("include", { stance: "supports" })).toBe('{"action":"include"}');
    expect(factActionBody("include")).toBe('{"action":"include"}');
  });
});

describe("every other action's body is unchanged", () => {
  it("is byte-identical to what shipped before the picker existed (M)", () => {
    // The promise this task made: only INCLUDE's body moves. If the builder ever
    // started attaching the link pair to a drop, `deny_unknown_fields` on the
    // backend would 400 every exclude in the queue — the same defect, aimed at a
    // different button.
    expect(factActionBody("drop")).toBe('{"action":"drop"}');
    expect(factActionBody("undrop")).toBe('{"action":"undrop"}');
    expect(factActionBody("reopen")).toBe('{"action":"reopen"}');
    expect(factActionBody("defer", { reason: "waiting on a clean copy" })).toBe(
      '{"action":"defer","reason":"waiting on a clean copy"}',
    );
  });

  it("ignores link fields handed to a NON-include action", () => {
    // The queue sends one options object for every ruling, so the pair is
    // present on the object whenever a picker has been used. It must not reach
    // the wire for anything but an include.
    expect(factActionBody("drop", { allegationId: "alleg-7", stance: "supports" })).toBe(
      '{"action":"drop"}',
    );
  });

  it("omits `reason` entirely rather than sending null", () => {
    // A key absent is not a key null: `deny_unknown_fields` accepts the absence
    // and the service's own validation refuses a null.
    expect(factActionBody("drop")).not.toContain("reason");
  });
});
