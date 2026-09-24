// The "Chat case file" box's sentences, word for word as ruled.

import { describe, expect, it } from "vitest";

import {
  cents,
  dollars,
  lastQuestionLine,
  loadedLine,
  resultLine,
} from "../chatCaseFileText";
import type { ChatCaseFileActivity, KeepLoadedResult } from "../../services/chatCaseFile";

const loaded: ChatCaseFileActivity = {
  last_question_at: "1:42 pm",
  last_question_by: "Marie",
  loaded: true,
  loaded_until: "2:42 pm",
  reload_cost_dollars: null,
};

const notLoaded: ChatCaseFileActivity = {
  ...loaded,
  loaded: false,
  loaded_until: null,
  reload_cost_dollars: 2.950656,
};

const tap = (over: Partial<KeepLoadedResult>): KeepLoadedResult => ({
  outcome: "read",
  loaded_until: "3:10 pm",
  cost_dollars: 0.0737824,
  activity: loaded,
  ...over,
});

describe("the activity lines", () => {
  it("says when and by whom", () => {
    expect(lastQuestionLine(loaded)).toBe("Last chat question: 1:42 pm, by Marie");
  });

  it("says so when nobody has chatted", () => {
    expect(lastQuestionLine({ ...loaded, last_question_at: null, last_question_by: null })).toBe(
      "Last chat question: none yet.",
    );
  });

  it("shows until when it stays loaded", () => {
    expect(loadedLine(loaded)).toBe("Loaded until: 2:42 pm");
  });

  it("shows the reload cost once it has run out", () => {
    expect(loadedLine(notLoaded)).toBe(
      "Not loaded. The next question will reload it (about $2.95).",
    );
  });

  it("drops the amount when the cost is not known", () => {
    expect(loadedLine({ ...notLoaded, reload_cost_dollars: null })).toBe(
      "Not loaded. The next question will reload it.",
    );
  });
});

describe("the line after a tap", () => {
  it("a read costs cents", () => {
    expect(resultLine(tap({}))).toBe("Done. Kept loaded until 3:10 pm. Cost 7¢.");
  });

  it("a reload costs dollars", () => {
    expect(resultLine(tap({ outcome: "wrote", cost_dollars: 2.950672 }))).toBe(
      "It had run out, so it was reloaded. Kept until 3:10 pm. Cost $2.95.",
    );
  });

  it("any other model says the cost is not known, with no numbers", () => {
    expect(resultLine(tap({ cost_dollars: null }))).toBe(
      "Done. Kept loaded until 3:10 pm. Cost not known for this model.",
    );
    expect(resultLine(tap({ outcome: "wrote", cost_dollars: null }))).toBe(
      "It had run out, so it was reloaded. Kept until 3:10 pm. Cost not known for this model.",
    );
  });

  it("never shows an internal word", () => {
    const all = [
      resultLine(tap({})),
      resultLine(tap({ outcome: "wrote" })),
      loadedLine(notLoaded),
      lastQuestionLine(loaded),
    ].join(" ").toLowerCase();
    for (const word of ["cache", "prefix", "token"]) expect(all).not.toContain(word);
  });
});

describe("money", () => {
  it("formats", () => {
    expect(dollars(2.950656)).toBe("$2.95");
    expect(cents(0.0737824)).toBe("7¢");
    expect(cents(0.001)).toBe("1¢");
    expect(cents(2.95)).toBe("$2.95");
  });
});
