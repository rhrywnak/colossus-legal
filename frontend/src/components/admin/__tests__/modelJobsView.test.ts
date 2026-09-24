// The jobs panel's pure helpers: how a dropdown entry reads (board 2) and where
// each link under the panel goes.

import { describe, expect, it } from "vitest";

import { linkTarget, optionText, pendingChanges } from "../modelJobsView";
import type { AiJob } from "../../../services/aiJobs";
import { ADMIN_GROUPS } from "../../../pages/adminGroups";

const opt = (label: string, current: boolean, marker: string | null = null) => ({
  value: label,
  label,
  current,
  marker,
});

describe("a dropdown entry, as board 2 writes it", () => {
  it("marks the model in force with a tick", () => {
    expect(optionText(opt("Claude Opus 5.5", true))).toBe("Claude Opus 5.5 ✓");
    expect(optionText(opt("Claude Opus 5", false))).toBe("Claude Opus 5");
  });

  it("puts the file's marker after the tick when it is in use", () => {
    expect(optionText(opt("question_chat_prompt_v1.md", true, "in use"))).toBe(
      "question_chat_prompt_v1.md ✓ in use",
    );
  });

  it("puts the marker after a dot when it is not", () => {
    expect(optionText(opt("question_chat_prompt_v2.md", false, "new, never used"))).toBe(
      "question_chat_prompt_v2.md · new, never used",
    );
  });
});

describe("the links under the panel", () => {
  it("send each target to the admin address and tab that holds it", () => {
    expect(linkTarget("models")).toEqual({ path: "/admin/prompts", panel: "models" });
    expect(linkTarget("files")).toEqual({ path: "/admin/prompts", panel: "prompts" });
    expect(linkTarget("data")).toEqual({ path: "/admin/data", panel: "lastRun" });
  });

  it("name only tabs their group really lists", () => {
    for (const target of ["models", "files", "data"]) {
      const to = linkTarget(target);
      const group = Object.values(ADMIN_GROUPS).find((g) =>
        g.panels.some((p) => p.id === to?.panel),
      );
      expect(group, target).toBeDefined();
    }
  });

  it("draw an unknown target as text, not a link to nowhere", () => {
    expect(linkTarget("somewhere_new")).toBeNull();
  });
});

describe("the admin groups after the move (option B)", () => {
  it("Overview shows the jobs panel and nothing else", () => {
    expect(ADMIN_GROUPS.overview.panels.map((p) => p.id)).toEqual(["jobs"]);
  });

  it("Data opens on the last document run and keeps Indexing", () => {
    expect(ADMIN_GROUPS.data.panels.map((p) => p.id)).toEqual(["lastRun", "indexing"]);
  });
});

/** Answer analysis as PROD's store draws it: two dropdowns. */
function answerAnalysis(): AiJob {
  const choose = (key: string, current: string) => ({
    label: "x",
    kind: "choose" as const,
    key,
    current,
    current_label: current,
    options: [],
    foot: null,
  });
  return {
    job: "answer_analysis",
    title: "Answer analysis",
    description: "",
    first: choose("practice_read_model", "claude-opus-5"),
    instructions: choose("practice_read_prompt_file", "practice_read_prompt_v5.md"),
    meta: "",
    warnings: [],
    was: null,
    switch_back: null,
  };
}

describe("what Save would send", () => {
  it("is nothing until a pick differs from the value in force", () => {
    expect(pendingChanges(answerAnalysis(), {})).toEqual([]);
    expect(pendingChanges(answerAnalysis(), { first: "claude-opus-5" })).toEqual([]);
  });

  it("names each changed control by the setting it writes", () => {
    expect(
      pendingChanges(answerAnalysis(), {
        first: "claude-opus-5-5",
        instructions: "practice_read_prompt_v4.md",
      }),
    ).toEqual([
      { key: "practice_read_model", value: "claude-opus-5-5" },
      { key: "practice_read_prompt_file", value: "practice_read_prompt_v4.md" },
    ]);
  });

  it("never sends a control that is not a dropdown", () => {
    const job = answerAnalysis();
    job.instructions = { label: "Instructions", kind: "fixed", text: "Built in" };
    expect(pendingChanges(job, { instructions: "anything.md" })).toEqual([]);
  });
});
