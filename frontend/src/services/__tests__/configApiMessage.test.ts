// How an admin config refusal reads: the backend's own sentence, not its JSON.

import { describe, expect, it } from "vitest";

import { messageFrom } from "../configApi";

describe("the message of an error body", () => {
  it("is the backend's `message` when the body is its JSON error shape", () => {
    const body = JSON.stringify({
      error: "conflict",
      message: "Claude Opus 5.5 is used by Discuss chat on an answer.",
      details: { reason: "model_in_use" },
    });
    expect(messageFrom(body)).toBe("Claude Opus 5.5 is used by Discuss chat on an answer.");
  });

  it("is the raw text when the body is not JSON", () => {
    expect(messageFrom("Bad Gateway")).toBe("Bad Gateway");
  });

  it("is null for an empty body, so the caller names the operation instead", () => {
    expect(messageFrom("")).toBeNull();
  });
});
