/**
 * What the models form SENDS about a model's permission to quote the record.
 *
 * `llm_models.grounded` is the column `chat_model_check` reads, so this flag
 * decides which models the Discuss chat will accept. The rule that matters is
 * invisible from the screen: the backend's UPDATE is a COALESCE, so a field this
 * form omits means "leave it alone". For the temperature mode next door that is
 * the design; for a checkbox it would mean un-ticking saves successfully and
 * changes nothing. Rule 30 leaves a decision made inside a React tree with
 * nothing able to assert it — so it is asserted here.
 */
import { describe, expect, it } from "vitest";

import { formToCreateInput, formToUpdateInput } from "../AdminModels";

const base = {
  id: "claude-opus-5-5",
  display_name: "Claude Opus 5.5",
  provider: "anthropic",
  api_endpoint: "",
  max_context_tokens: "",
  max_output_tokens: "",
  cost_per_input_token: "0.000004",
  cost_per_output_token: "0.00002",
  notes: "",
  temperature_mode: "",
  default_temperature: "",
  grounded: false,
};

describe("granting and withdrawing permission to quote the record", () => {
  it("sends an explicit false when the box is clear, never nothing", () => {
    const input = formToUpdateInput({ ...base, grounded: false });

    // The whole point. `undefined` here would COALESCE to the stored value and
    // the un-tick would be a silent no-op — the box springing back after a save
    // that reported success.
    expect(input.grounded).toBe(false);
    expect("grounded" in input).toBe(true);
  });

  it("sends true when the box is ticked", () => {
    expect(formToUpdateInput({ ...base, grounded: true }).grounded).toBe(true);
  });

  it("is the opposite of the temperature rule in the same function", () => {
    // Same form, same save: the mode is OMITTED while unrecorded, and grounded
    // is present either way. Pinned together so a later tidy-up that makes them
    // consistent has to read why they differ.
    const input = formToUpdateInput({ ...base, grounded: false, temperature_mode: "" });
    expect("temperature_mode" in input).toBe(false);
    expect("grounded" in input).toBe(true);
  });
});

describe("creating a model already trusted to quote the record", () => {
  it("carries the ticked box into the create payload", () => {
    expect(formToCreateInput({ ...base, grounded: true }).grounded).toBe(true);
  });

  it("carries an unticked box as false, not as nothing", () => {
    // The backend COALESCEs an absent `grounded` to the column's DEFAULT false,
    // so absent and false agree here — but only by accident of the default.
    // Sending the state the operator actually chose keeps them agreeing on
    // purpose, and keeps create and update saying the same thing.
    const input = formToCreateInput({ ...base, grounded: false });
    expect(input.grounded).toBe(false);
    expect("grounded" in input).toBe(true);
  });
});
