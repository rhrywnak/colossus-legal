// =============================================================================
// printAnswerPlan.test.ts — Chuck's reading copy
// =============================================================================
//
// §10's list, for this half of L2: Print answers carries the CURRENT answer
// only, and the printed sheet has no footer and no codes.
//
// The failure this file exists to catch is the quiet one: a sheet that looks
// complete and is not. Chuck reads the questions sheet and the answers sheet
// side by side, so a row silently missing from one has him hunting a question
// that is not lost.

import { describe, expect, it } from "vitest";

import { answerLine, planAnswers } from "../printAnswerPlan";
import type { PracticeQuestion, PracticeWording } from "../../../services/practice";
import type { PracticeAnswer } from "../../../services/practiceAnswers";

const wording: PracticeWording = {
  print_answer_missing: "Not answered yet.",
  print_sheet_cross_title: "The defense asks",
  print_sheet_direct_title: "Chuck asks",
  print_sheet_redirect_title: "Chuck, after the defense",
  print_sheet_subtitle_template: "{code} · “{title}”",
  print_sheet_redirect_subtitle: "the redirects",
  print_howto_cross: "cross how-to",
  print_howto_direct: "direct how-to",
  print_howto_redirect: "redirect how-to",
  print_howto_redirect_drafts: "drafts",
  print_deck_as_of_template: "deck as of {date} · {n} of {m} questions",
  print_missing_prefix: "This deck has no",
  print_missing_cross: "defense questions",
  print_missing_direct: "direct questions",
  print_missing_redirect: "redirects",
  print_missing_joiner: " or ",
  print_hidden_template: "{n} hidden",
  print_after_template: "After the defense asks: {question}",
  print_after_missing: "gone",
} as unknown as PracticeWording;

const question = (id: string, kind: string): PracticeQuestion =>
  ({
    id,
    kind,
    side: kind === "cross" ? "george" : "chuck",
    text: `question ${id}`,
    braid: false,
    tactic: null,
    receipt: null,
    deck_key: id,
    follows_key: null,
    hidden: false,
    draft_by: null,
    answered_on: null,
    flag_note: null,
    braid_rows: null,
    watch_for: null,
    pair_said: null,
    pair_admitted: null,
    stronger: null,
    stronger_lean: null,
  }) as unknown as PracticeQuestion;

const answer = (questionId: string, text: string): PracticeAnswer => ({
  question_id: questionId,
  text,
  answered_on: "Answered on 22 Aug",
});

describe("the answers document", () => {
  it("keeps the questions sheet's order, because Chuck reads them side by side", () => {
    const questions = [question("g1", "cross"), question("g2", "cross")];
    const plan = planAnswers(questions, "S-5", "an accusation", wording, []);

    const printed = plan.plan.sheets.flatMap((s) => s.rows.map((r) => r.question.id));
    expect(printed).toEqual(["g1", "g2"]);
  });

  it("prints a question Marie has NOT answered, with the stored absence", () => {
    // Omitting it would make the two sheets disagree about how many questions
    // the deck holds — and he would go looking for one that is not missing.
    const questions = [question("g1", "cross"), question("g2", "cross")];
    const plan = planAnswers(questions, "S-5", "a", wording, [answer("g1", "her words")]);

    const rows = plan.plan.sheets.flatMap((s) => s.rows);
    expect(rows).toHaveLength(2);

    const missing = answerLine({ question: questions[1], answer: null }, wording);
    expect(missing.text).toBe("Not answered yet.");
    expect(missing.when).toBeNull();
  });

  it("carries her words and the day she gave them", () => {
    const line = answerLine(
      { question: question("g1", "cross"), answer: answer("g1", "her words") },
      wording,
    );
    expect(line.text).toBe("her words");
    expect(line.when).toBe("Answered on 22 Aug");
  });

  it("matches an answer to its OWN question and no other", () => {
    // A `[0]` where a lookup belongs would put question 1's answer under every
    // question, and every assertion above would still pass.
    const plan = planAnswers(
      [question("g1", "cross"), question("g2", "cross")],
      "S-5",
      "a",
      wording,
      [answer("g2", "the second answer")],
    );

    expect(plan.answers.get("g1")).toBeUndefined();
    expect(plan.answers.get("g2")?.text).toBe("the second answer");
  });

  it("never carries an earlier version — the map holds one answer per question", () => {
    // The endpoint returns the current answer only; this pins that the client
    // could not render two even if it were handed them.
    const plan = planAnswers([question("g1", "cross")], "S-5", "a", wording, [
      answer("g1", "the older words"),
      answer("g1", "the current words"),
    ]);

    expect(plan.answers.size).toBe(1);
    expect(plan.answers.get("g1")?.text).toBe("the current words");
  });
});
