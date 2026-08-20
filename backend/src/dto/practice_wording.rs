//! The wire mirror of the practice tool's two wording blocks.
//!
//! Same argument as every sibling mirror in this directory: the domain layer does
//! not derive serde, so a change to how a value is STORED cannot silently change
//! the API, and vice versa.
//!
//! ## Why TWO domain blocks arrive as ONE wire object
//!
//! The blocks are split under Rule 17 (`domain::wording_practice` and
//! `domain::wording_practice_report`), which is a fact about how the backend is
//! organised. The BROWSER has one page holding four screens, and asking it to
//! know which of two objects a given sentence lives in would push a backend
//! filing decision onto the client. So the mirror flattens them, and the
//! conversion takes both.
//!
//! All of them ride the deck payload, which the page fetches ONCE on mount.
//! There is no second request and no per-screen fetch: S0, S1, S2 and S3 are four
//! states of one page, and a witness moving between them mid-session must never
//! wait on a network.

use serde::{Deserialize, Serialize};

// The constructor that flattens the stored blocks into this shape lives in
// `practice_wording_map`, which is one `impl` block and nothing else. Split on
// 2026-08-19 when the twelve v1 rows carried this file past Rule 17's limit —
// see that module's header for why the seam falls where it does.

/// The practice tool's words, as the browser receives them.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PracticeWordingDto {
    pub kicker: String,
    pub intro: String,
    pub who_heading: String,
    pub who_george_title: String,
    pub who_george_detail: String,
    pub who_chuck_title: String,
    pub who_chuck_detail: String,
    pub who_mixed_title: String,
    pub who_mixed_detail: String,
    pub who_george_term: String,
    pub who_chuck_term: String,
    pub who_redirect_term: String,
    pub redirects_subheader: String,
    pub how_many_heading: String,
    pub count_all_template: String,
    pub start_label: String,
    pub always_label: String,
    pub always_line: String,
    pub last_session_template: String,
    pub no_last_session: String,
    pub progress_template: String,
    pub pill_george: String,
    pub pill_chuck: String,
    pub pill_braid: String,
    pub answer_label: String,
    pub answer_hint: String,
    pub answer_placeholder: String,
    pub answer_button: String,
    pub dont_recall_button: String,
    pub dont_recall_text: String,
    pub pause_button: String,
    pub pause_note_prefix: String,
    pub pause_note_emphasis: String,
    pub empty_deck: String,
    pub load_failed: String,
    pub answer_failed: String,
    pub tactic_braid_suffix: String,
    pub what_you_said_kicker: String,
    pub read_tag: String,
    pub read_footnote: String,
    pub read_unavailable: String,
    pub points_kicker: String,
    pub receipt_prefix: String,
    pub point_no_receipt: String,
    pub pair_kicker: String,
    pub pair_said_label: String,
    pub pair_admitted_label: String,
    pub check_kicker: String,
    pub check_only_asked: String,
    pub check_accepted_premise: String,
    pub check_explained_unasked: String,
    pub check_guessed: String,
    pub stronger_summary: String,
    pub stronger_note_prefix: String,
    pub stronger_note_emphasis: String,
    pub stronger_note_suffix: String,
    pub stronger_no_receipt: String,
    pub mark_not_recorded: String,
    pub help_not_recorded: String,
    pub next_button: String,
    pub again_button: String,
    pub sheet_kicker_template: String,
    pub sheet_heading_template: String,
    pub sheet_repeat_clause_template: String,
    pub sheet_nothing_to_repeat: String,
    pub sheet_sub_prefix: String,
    pub sheet_sub_suffix: String,
    pub sheet_col_number: String,
    pub sheet_col_from: String,
    pub sheet_col_tactic: String,
    pub sheet_col_question: String,
    pub sheet_col_answer: String,
    pub sheet_col_mark: String,
    pub sheet_col_help: String,
    pub sheet_from_george: String,
    pub sheet_from_george_braid: String,
    pub sheet_from_chuck: String,
    pub mark_fine: String,
    pub mark_repeat: String,
    pub help_opened: String,
    pub help_none: String,
    pub tactic_none: String,
    pub sheet_again_button: String,
    pub print_button: String,
    pub homelab_line: String,

    // ── mockup v3: the words that let her MOVE ───────────────────────────
    // Flattened alongside the other two blocks rather than nested, because the
    // wire contract this file pins is "one field per stored key, prefix
    // dropped" — a nested object would be the one field on this page whose
    // name is not a key, and the two tests below could no longer say so.
    pub deck_heading: String,
    pub deck_count_template: String,
    pub deck_skipped_suffix_template: String,
    pub deck_hide_link: String,
    pub deck_show_link: String,
    pub deck_instruction_template: String,
    pub skip_today_label: String,
    pub skipped_today_label: String,
    pub flag_label: String,
    pub flag_edit_label: String,
    pub flag_placeholder: String,
    pub flag_save_label: String,
    pub flag_cancel_label: String,
    pub flag_shown_template: String,
    pub nothing_left_label: String,
    pub unfinished_label: String,
    pub unfinished_detail_template: String,
    pub resume_label: String,
    pub start_over_label: String,
    pub start_over_hint: String,
    pub back_label: String,
    pub back_hint_question: String,
    pub back_hint_reveal: String,
    pub skip_question_label: String,
    pub end_session_label: String,
    pub skipped_answer_text: String,
    pub mark_skipped: String,
    pub sheet_skipped_clause_template: String,
    pub sheet_ended_early_clause: String,
    pub flag_summary_heading: String,
    pub flag_summary_hint: String,
    pub flag_summary_item_template: String,
    pub mark_hidden_before_asked: String,

    // ── v1 (the Chuck review): the words about ONE question ──────────────
    // Flattened by the same rule as the block above. The `row_` prefix on the
    // first five is part of the stored key (`practice_row_…`), not a nesting
    // this file invented — the wire name is the key with `practice_` removed
    // and nothing else done to it.
    pub row_practice_this_label: String,
    pub row_answered_today_template: String,
    pub row_skipped_today: String,
    pub row_earlier_template: String,
    pub row_attempt_suffix_template: String,
    pub redirect_tag: String,
    pub redirect_stronger_line: String,
    pub points_to_label: String,
    pub points_to_done_label: String,
    pub points_to_reveal_prefix: String,
    pub points_to_sheet_prefix: String,
    pub unfinished_today_word: String,
    pub answer_empty_hint: String,
    pub answer_already_recorded: String,

    // ── Part B: the deck editor and what it records (Chuck's words) ─────
    // Same flattening rule as every block above: one field per stored key,
    // the `practice_` prefix dropped and nothing else done to the name.
    // `note_authors` and `editor_authors` arrive as the stored comma-separated
    // STRING — the readers split them — so this object stays what its two
    // tests below say it is: every value a string, every name a key.
    pub editor_switch_label: String,
    pub editor_done_label: String,
    pub editor_edit_label: String,
    pub editor_hide_label: String,
    pub editor_drag_hint: String,
    pub editor_unhide_label: String,
    pub editor_hidden_badge: String,
    pub editor_up_label: String,
    pub editor_down_label: String,
    pub editor_save_label: String,
    pub editor_cancel_label: String,
    pub editor_saved_hint_template: String,
    pub editor_field_question: String,
    pub editor_field_tactic: String,
    pub editor_field_follows: String,
    pub editor_field_watch_for: String,
    pub editor_field_stronger: String,
    pub editor_field_side: String,
    pub editor_field_attach: String,
    pub editor_side_cross: String,
    pub editor_side_direct: String,
    pub editor_side_redirect: String,
    pub editor_attach_none: String,
    pub editor_attach_instance_template: String,
    pub editor_attach_point_template: String,
    pub editor_add_label: String,
    pub editor_add_heading: String,
    pub editor_add_button: String,
    pub editor_add_hint: String,
    pub editor_question_placeholder: String,
    pub editor_failed: String,
    pub changed_heading_template: String,
    pub changed_notes_template: String,
    pub changed_summary: String,
    pub change_added_template: String,
    pub change_reworded_template: String,
    pub change_edited_template: String,
    pub change_moved_template: String,
    pub change_hidden_template: String,
    pub change_unhidden_template: String,
    pub badge_changed: String,
    pub badge_draft: String,
    pub sheet_changes_heading: String,
    pub sheet_change_item_template: String,
    pub editor_busy_hint: String,
    pub editor_discard_confirm_template: String,

    // ── Part B: notes, and the review page ───────────────────────────────
    pub notes_heading_template: String,
    pub notes_scenario_title: String,
    pub notes_question_title: String,
    pub notes_hint: String,
    pub notes_placeholder: String,
    pub notes_attempt_placeholder: String,
    pub notes_save_label: String,
    pub notes_empty_hint: String,
    pub notes_strike_label: String,
    pub notes_struck_template: String,
    pub notes_empty: String,
    pub notes_failed: String,
    pub row_review_link: String,
    pub review_progress_template: String,
    pub review_attempts_kicker: String,
    pub review_attempt_template: String,
    pub review_detail_template: String,
    pub review_boxes_none: String,
    pub review_no_attempts: String,
    pub review_practice_again: String,
    pub review_asked_as_template: String,
    pub review_stronger_heading: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::wording_practice::PracticeWording;
    use crate::domain::wording_practice::PRACTICE_WORDING_KEYS;
    use crate::domain::wording_practice_editor::PRACTICE_EDITOR_WORDING_KEYS;
    use crate::domain::wording_practice_flow::PRACTICE_FLOW_WORDING_KEYS;
    use crate::domain::wording_practice_report::PracticeReportWording;
    use crate::domain::wording_practice_report::PRACTICE_REPORT_WORDING_KEYS;
    use crate::domain::wording_practice_review::PRACTICE_REVIEW_WORDING_KEYS;
    use crate::domain::wording_practice_row::PRACTICE_ROW_WORDING_KEYS;

    fn mirror() -> PracticeWordingDto {
        PracticeWordingDto::from_blocks(
            &PracticeWording::for_test(),
            &PracticeReportWording::for_test(),
        )
    }

    /// The mirror carries every declared key from BOTH blocks — so a field added
    /// to either and forgotten here fails at `cargo test` rather than as an
    /// `undefined` in the middle of Marie's session.
    #[test]
    fn the_mirror_carries_every_declared_key_from_both_blocks() {
        let value = serde_json::to_value(mirror()).expect("the mirror serializes");
        assert_eq!(
            value.as_object().expect("an object body").len(),
            PRACTICE_WORDING_KEYS.len()
                + PRACTICE_FLOW_WORDING_KEYS.len()
                + PRACTICE_REPORT_WORDING_KEYS.len()
                + PRACTICE_ROW_WORDING_KEYS.len()
                + PRACTICE_EDITOR_WORDING_KEYS.len()
                + PRACTICE_REVIEW_WORDING_KEYS.len(),
        );
    }

    /// Every wire name is a stored key without its `practice_` prefix, and comes
    /// from one of the two blocks.
    #[test]
    fn every_wire_key_is_a_stored_key_without_its_prefix() {
        let value = serde_json::to_value(mirror()).expect("the mirror serializes");
        for key in value.as_object().expect("an object body").keys() {
            let stored = format!("practice_{key}");
            assert!(
                PRACTICE_WORDING_KEYS.contains(&stored.as_str())
                    || PRACTICE_FLOW_WORDING_KEYS.contains(&stored.as_str())
                    || PRACTICE_REPORT_WORDING_KEYS.contains(&stored.as_str())
                    || PRACTICE_ROW_WORDING_KEYS.contains(&stored.as_str())
                    || PRACTICE_EDITOR_WORDING_KEYS.contains(&stored.as_str())
                    || PRACTICE_REVIEW_WORDING_KEYS.contains(&stored.as_str()),
                "wire field '{key}' implies stored key '{stored}', which is not declared",
            );
        }
    }

    /// No wire value is blank.
    ///
    /// The fixtures are built through the production builders, which refuse a
    /// blank row — so this asserts the CLONE did not lose one. A `String::new()`
    /// left by a mapping line would render an empty button on a witness screen,
    /// and nothing else in the stack would notice.
    #[test]
    fn no_string_arrives_blank() {
        let value = serde_json::to_value(mirror()).expect("the mirror serializes");
        for (key, v) in value.as_object().expect("an object body") {
            assert!(
                !v.as_str().unwrap_or_default().trim().is_empty(),
                "{key} arrived blank"
            );
        }
    }
}
