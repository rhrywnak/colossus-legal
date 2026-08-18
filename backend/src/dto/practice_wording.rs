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

use crate::domain::wording_practice::PracticeWording;
use crate::domain::wording_practice_report::PracticeReportWording;

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
}

impl PracticeWordingDto {
    /// Flatten the two stored blocks into the one object the page reads.
    ///
    /// ## Rust Learning: an inherent constructor instead of `From`
    ///
    /// `From` takes ONE value. The natural workaround — `From<(&A, &B)>` for a
    /// tuple — makes the call site `PracticeWordingDto::from((&a, &b))`, where the
    /// two references are positional and a transposition would compile the day a
    /// third block appears. A named constructor with named parameters says which
    /// is which, and that is the whole reason to prefer it here.
    pub fn from_blocks(drill: &PracticeWording, report: &PracticeReportWording) -> Self {
        Self {
            kicker: drill.kicker.clone(),
            intro: drill.intro.clone(),
            who_heading: drill.who_heading.clone(),
            who_george_title: drill.who_george_title.clone(),
            who_george_detail: drill.who_george_detail.clone(),
            who_chuck_title: drill.who_chuck_title.clone(),
            who_chuck_detail: drill.who_chuck_detail.clone(),
            who_mixed_title: drill.who_mixed_title.clone(),
            who_mixed_detail: drill.who_mixed_detail.clone(),
            how_many_heading: drill.how_many_heading.clone(),
            count_all_template: drill.count_all_template.clone(),
            start_label: drill.start_label.clone(),
            always_label: drill.always_label.clone(),
            always_line: drill.always_line.clone(),
            last_session_template: drill.last_session_template.clone(),
            no_last_session: drill.no_last_session.clone(),
            progress_template: drill.progress_template.clone(),
            pill_george: drill.pill_george.clone(),
            pill_chuck: drill.pill_chuck.clone(),
            pill_braid: drill.pill_braid.clone(),
            answer_label: drill.answer_label.clone(),
            answer_hint: drill.answer_hint.clone(),
            answer_placeholder: drill.answer_placeholder.clone(),
            answer_button: drill.answer_button.clone(),
            dont_recall_button: drill.dont_recall_button.clone(),
            dont_recall_text: drill.dont_recall_text.clone(),
            pause_button: drill.pause_button.clone(),
            pause_note_prefix: drill.pause_note_prefix.clone(),
            pause_note_emphasis: drill.pause_note_emphasis.clone(),
            empty_deck: drill.empty_deck.clone(),
            load_failed: drill.load_failed.clone(),
            answer_failed: drill.answer_failed.clone(),
            tactic_braid_suffix: drill.tactic_braid_suffix.clone(),
            what_you_said_kicker: report.what_you_said_kicker.clone(),
            read_tag: report.read_tag.clone(),
            read_footnote: report.read_footnote.clone(),
            read_unavailable: report.read_unavailable.clone(),
            points_kicker: report.points_kicker.clone(),
            receipt_prefix: report.receipt_prefix.clone(),
            point_no_receipt: report.point_no_receipt.clone(),
            pair_kicker: report.pair_kicker.clone(),
            pair_said_label: report.pair_said_label.clone(),
            pair_admitted_label: report.pair_admitted_label.clone(),
            check_kicker: report.check_kicker.clone(),
            check_only_asked: report.check_only_asked.clone(),
            check_accepted_premise: report.check_accepted_premise.clone(),
            check_explained_unasked: report.check_explained_unasked.clone(),
            check_guessed: report.check_guessed.clone(),
            stronger_summary: report.stronger_summary.clone(),
            stronger_note_prefix: report.stronger_note_prefix.clone(),
            stronger_note_emphasis: report.stronger_note_emphasis.clone(),
            stronger_note_suffix: report.stronger_note_suffix.clone(),
            stronger_no_receipt: report.stronger_no_receipt.clone(),
            mark_not_recorded: report.mark_not_recorded.clone(),
            help_not_recorded: report.help_not_recorded.clone(),
            next_button: report.next_button.clone(),
            again_button: report.again_button.clone(),
            sheet_kicker_template: report.sheet_kicker_template.clone(),
            sheet_heading_template: report.sheet_heading_template.clone(),
            sheet_repeat_clause_template: report.sheet_repeat_clause_template.clone(),
            sheet_nothing_to_repeat: report.sheet_nothing_to_repeat.clone(),
            sheet_sub_prefix: report.sheet_sub_prefix.clone(),
            sheet_sub_suffix: report.sheet_sub_suffix.clone(),
            sheet_col_number: report.sheet_col_number.clone(),
            sheet_col_from: report.sheet_col_from.clone(),
            sheet_col_tactic: report.sheet_col_tactic.clone(),
            sheet_col_question: report.sheet_col_question.clone(),
            sheet_col_answer: report.sheet_col_answer.clone(),
            sheet_col_mark: report.sheet_col_mark.clone(),
            sheet_col_help: report.sheet_col_help.clone(),
            sheet_from_george: report.sheet_from_george.clone(),
            sheet_from_george_braid: report.sheet_from_george_braid.clone(),
            sheet_from_chuck: report.sheet_from_chuck.clone(),
            mark_fine: report.mark_fine.clone(),
            mark_repeat: report.mark_repeat.clone(),
            help_opened: report.help_opened.clone(),
            help_none: report.help_none.clone(),
            tactic_none: report.tactic_none.clone(),
            sheet_again_button: report.sheet_again_button.clone(),
            print_button: report.print_button.clone(),
            homelab_line: report.homelab_line.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::wording_practice::PRACTICE_WORDING_KEYS;
    use crate::domain::wording_practice_report::PRACTICE_REPORT_WORDING_KEYS;

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
            PRACTICE_WORDING_KEYS.len() + PRACTICE_REPORT_WORDING_KEYS.len(),
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
                    || PRACTICE_REPORT_WORDING_KEYS.contains(&stored.as_str()),
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
