/**
 * What the models form actually SENDS about a model's temperature (ruling R5).
 *
 * The column this control writes is the one that was never set on
 * `claude-opus-5`, and the omission cost 104 judge calls on 2026-08-09. Two
 * decisions in `formToUpdateInput` are load-bearing, and neither is visible from
 * the screen — which is why they are pinned here rather than trusted.
 */
import { describe, expect, it } from "vitest";

import { formToUpdateInput } from "../AdminModels";

/** A form with everything else filled, so only the temperature fields vary. */
const base = {
  id: "claude-opus-5",
  display_name: "Claude Opus 5",
  provider: "anthropic",
  api_endpoint: "",
  max_context_tokens: "1000000",
  max_output_tokens: "64000",
  cost_per_input_token: "",
  cost_per_output_token: "",
  notes: "",
  temperature_mode: "",
  default_temperature: "",
};

describe("a model's temperature capability is recorded, never guessed", () => {
  it("sends nothing about temperature while the mode is unrecorded", () => {
    const input = formToUpdateInput(base);

    // Not `null` — ABSENT. The backend's UPDATE is a COALESCE, so a null would
    // mean "leave it alone" too; omitting says so in the request instead of
    // relying on the server to read a null that way. The consequence is the
    // intended asymmetry: this form can record a capability and cannot
    // un-record one.
    expect("temperature_mode" in input).toBe(false);
    expect("default_temperature" in input).toBe(false);
  });

  it("records the capability the operator picked", () => {
    const input = formToUpdateInput({ ...base, temperature_mode: "omit" });

    expect(input.temperature_mode).toBe("omit");
    // The token, not the label: the screen shows "Send no temperature — this
    // model rejects it", and the column stores `omit`. A form that sent its own
    // label would be a 400 the operator cannot act on.
    expect(input.temperature_mode).not.toContain(" ");
  });

  it("sends a temperature value only with the mode that carries one", () => {
    // A number left over from a previous choice must not be written to a row
    // whose calls will never carry it — a stored value that does nothing is the
    // kind of thing an operator later reads as evidence that it does something.
    const omitting = formToUpdateInput({
      ...base,
      temperature_mode: "omit",
      default_temperature: "0.2",
    });
    expect("default_temperature" in omitting).toBe(false);

    const sending = formToUpdateInput({
      ...base,
      temperature_mode: "zero-ok",
      default_temperature: "0.2",
    });
    expect(sending.default_temperature).toBe(0.2);

    // ANTI-VACUITY: blank stays absent even under the mode that accepts a value,
    // so "not sent" above is about the MODE and not merely about emptiness.
    const blank = formToUpdateInput({ ...base, temperature_mode: "zero-ok" });
    expect("default_temperature" in blank).toBe(false);
  });
});
