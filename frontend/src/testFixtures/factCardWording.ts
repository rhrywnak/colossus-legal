/**
 * The fact card's served words, as a test fixture (FACT_CARD_v2).
 *
 * ## Why this is shared rather than written per test file
 *
 * `FactCardWording` carries twenty-three strings and three suites need one. Two
 * hand-written copies would drift, and the drift would be invisible: a test
 * asserting a label that no longer matches the seeded value still passes, because
 * both halves of the comparison come from the same literal.
 *
 * The values here are the ones the migration seeds — the same list the backend
 * pins in `domain::wording_fact_card_tests::TEST_SEED`, which is itself pinned to
 * the migration file on disk. That chain is what makes an assertion about a label
 * in this file an assertion about what a witness actually reads.
 */
import type { FactCardWording } from "../services/evidenceLinks";

export const FACT_CARD_WORDING_FIXTURE: FactCardWording = {
  proof_label: "Proof",
  backs_label: "Backs",
  supports_label: "Supports",
  watch_out_label: "Watch out",
  answer_label: "Answer",
  empty_value: "—",
  draft_mark: "draft",
  context_label: "Show context",
  context_hide_label: "Hide context",
  backs_template: "Point {position} — {text}",
  supports_template: "{verb} {code} — {text}",
  stance_supports_verb: "Supports",
  stance_rebuts_verb: "Disputes",
  rfa_template: "RFA {number} — {request} — {answer}",
  rfa_unnumbered_template: "{request} — {answer}",
  deck_template: "{shown} of {total} shown · {collapsed} more collapsed",
  edit_label: "Edit",
  save_label: "Save",
  cancel_label: "Cancel",
  save_failed_template:
    "That edit did not save: {detail} Reopen the field — what you see now may not be what is stored.",
  no_answer_notice: "No answer written yet.",
  accusation_heading: "The Accusation",
  no_cards_notice: "No cards have been written for this scenario yet.",
};
