/**
 * The Proof Matrix's served words, as a test fixture.
 *
 * ## Why this is shared rather than written per test file
 *
 * `MatrixWording` carries thirty strings since PROOF_MATRIX_v2, and two suites
 * need one. Two hand-written copies would drift, and the drift would be
 * invisible: a test asserting a label that no longer matches the seeded value
 * still passes, because both halves of the comparison come from the same
 * literal.
 *
 * The values here are the ones the migration seeds — the same list the backend
 * pins in `domain::wording_matrix_tests::TEST_SEED`, which is itself pinned to
 * the migration file on disk. That chain is what makes an assertion about a
 * label in this file an assertion about what a reader actually sees.
 */
import type { MatrixWording } from "../services/causesOfAction";

export const MATRIX_WORDING_FIXTURE: MatrixWording = {
  strong_column_label: "Strong support",
  raw_approved_template: "· {count} approved",
  strong_hint:
    "Sworn admissions by the other side, and the court's own findings. The number beside it is every approved item, however qualified.",
  tier_strong_chip: "Their own words",
  tier_hedged_chip: "Qualified",
  tier_other_chip: "Our sworn word",
  duplicate_template: "×{count}",
  ranked_list_note: "Strongest first",
  more_template: "{count} more",
  show_hidden_template: "Show hidden ({count})",
  hide_hidden_label: "Hide those again",
  keep_label: "Keep",
  remove_label: "Remove",
  undo_label: "Undo",
  machine_label: "machine",
  kept_template: "kept · {actor}",
  conflict_label: "conflicts",
  confidence_high_label: "high",
  confidence_medium_label: "medium",
  confidence_low_label: "low",
  confidence_unrated_label: "unrated",
  rfa_template: "RFA {number} — {request} — {answer}",
  rfa_unnumbered_template: "{request} — {answer}",
  legend_line:
    "Red bar = the complaint's words · green = supports · red on a quote = disputes",
  export_button_label: "Export this count (Word)",
  export_title_template: "Count {number} — {name}",
  export_supporting_heading: "Supporting",
  export_disputing_heading: "Disputing",
  export_confirmed_mark: "\u2713",
  export_empty_line: "No evidence in the corpus.",
  export_footer_template:
    "Generated {date}. Machine-ranked; items marked ✓ confirmed by Roman.",
  ruling_failed_template:
    "That decision did not save: {detail} Reload the page — what you see now may not be what is stored.",
  no_supporting_line: "Nothing in the record supports this yet.",
  no_disputing_line: "Nothing in the record disputes this.",
};
