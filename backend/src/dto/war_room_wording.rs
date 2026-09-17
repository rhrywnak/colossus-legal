//! The wire mirror of `domain::wording_war_room::WarRoomWording`.
//!
//! Task 396 P3b. Same argument as every sibling mirror in this directory: the
//! domain layer does not derive serde, so a change to how a value is STORED
//! cannot silently change the API, and vice versa.
//!
//! These ride the dashboard payload for the reason `ScenarioCreateWordingDto`
//! does — the surface lives on that page and nowhere else, and the page already
//! fetches exactly once on mount.

use serde::{Deserialize, Serialize};

use crate::domain::wording_war_room::WarRoomWording;

/// The Trial Prep dashboard's words, as the browser receives them.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WarRoomWordingDto {
    pub subtitle: String,
    pub metric_scenarios_label: String,
    pub metric_ready_label: String,
    pub metric_draft_label: String,
    // The status card's words (CC_TASK_WAR_ROOM_v1). Each is the stored key
    // without its `war_room_` prefix, like the four above.
    pub card_evidence_heading: String,
    pub card_prep_heading: String,
    pub card_facts_included_label: String,
    pub card_candidates_label: String,
    pub card_matrix_linked_label: String,
    pub card_matrix_linked_template: String,
    pub card_matrix_linked_none: String,
    pub card_scan_template: String,
    pub card_scan_never: String,
    pub card_deck_none: String,
    pub card_answered_count_template: String,
    pub card_answered_word: String,
    pub card_prep_meta_template: String,
    pub card_changed_template: String,
    pub card_review_template: String,
    pub card_review_one: String,
    pub card_changed_one: String,
    pub card_not_started: String,
    pub card_up_to_date: String,
    pub card_practice_action: String,
    pub card_timeline_action: String,
    pub card_delete_action: String,
    // The summary card (CC_TASK_SIMPLE_COUNTS_v1), flattened from the nested
    // domain block. Wire name = stored key without `war_room_`.
    pub summary_answered_label: String,
    pub summary_answered_rest_template: String,
    pub summary_unanswered_label: String,
    pub summary_review_label: String,
    pub summary_candidates_label: String,
    pub owner_marie: String,
    pub owner_roman: String,
    pub summary_unanswered_context_template: String,
    pub summary_unanswered_context_one: String,
    pub summary_unanswered_context_none_untouched: String,
    pub summary_unanswered_context_none_untouched_one: String,
    pub summary_review_context_template: String,
    pub summary_candidates_pile_template: String,
    pub summary_list_joiner: String,
    pub summary_tie_joiner: String,
    pub summary_code_joiner: String,
    pub summary_unanswered_zero: String,
    pub summary_review_zero: String,
    pub summary_candidates_zero: String,
    /// The reviewer's name as screens print it — NOT a wording row: the
    /// `practice_reviewer_display_name` settings row (GO ruling 5). It rides here
    /// because every sentence that prints it is on this page, filled from this
    /// object, and one snapshot is what keeps the chip and the pill in step.
    pub reviewer_display_name: String,
}

impl WarRoomWordingDto {
    /// The page's words and the reviewer's display name, from one settings snapshot.
    ///
    /// ## Rust Learning: a named constructor instead of `From`
    ///
    /// `From<&T>` converts ONE value. This needs two — the wording block and the
    /// reviewer's display name, which lives on a different settings block — and
    /// `From<(&A, &B)>` would hide at the call site which argument is which. A
    /// named `new` with two parameters says it plainly. Both are borrowed: the
    /// settings snapshot lives behind an `Arc` and must outlive this conversion.
    pub fn new(w: &WarRoomWording, reviewer_display_name: &str) -> Self {
        let m = &w.summary;
        Self {
            subtitle: w.subtitle.clone(),
            metric_scenarios_label: w.metric_scenarios_label.clone(),
            metric_ready_label: w.metric_ready_label.clone(),
            metric_draft_label: w.metric_draft_label.clone(),
            card_evidence_heading: w.card_evidence_heading.clone(),
            card_prep_heading: w.card_prep_heading.clone(),
            card_facts_included_label: w.card_facts_included_label.clone(),
            card_candidates_label: w.card_candidates_label.clone(),
            card_matrix_linked_label: w.card_matrix_linked_label.clone(),
            card_matrix_linked_template: w.card_matrix_linked_template.clone(),
            card_matrix_linked_none: w.card_matrix_linked_none.clone(),
            card_scan_template: w.card_scan_template.clone(),
            card_scan_never: w.card_scan_never.clone(),
            card_deck_none: w.card_deck_none.clone(),
            card_answered_count_template: w.card_answered_count_template.clone(),
            card_answered_word: w.card_answered_word.clone(),
            card_prep_meta_template: w.card_prep_meta_template.clone(),
            card_changed_template: w.card_changed_template.clone(),
            card_review_template: w.card_review_template.clone(),
            card_review_one: w.card_review_one.clone(),
            card_changed_one: w.card_changed_one.clone(),
            card_not_started: w.card_not_started.clone(),
            card_up_to_date: w.card_up_to_date.clone(),
            card_practice_action: w.card_practice_action.clone(),
            card_timeline_action: w.card_timeline_action.clone(),
            card_delete_action: w.card_delete_action.clone(),
            summary_answered_label: m.answered_label.clone(),
            summary_answered_rest_template: m.answered_rest_template.clone(),
            summary_unanswered_label: m.unanswered_label.clone(),
            summary_review_label: m.review_label.clone(),
            summary_candidates_label: m.candidates_label.clone(),
            owner_marie: m.owner_marie.clone(),
            owner_roman: m.owner_roman.clone(),
            summary_unanswered_context_template: m.unanswered_context_template.clone(),
            summary_unanswered_context_one: m.unanswered_context_one.clone(),
            summary_unanswered_context_none_untouched: m.unanswered_context_none_untouched.clone(),
            summary_unanswered_context_none_untouched_one: m
                .unanswered_context_none_untouched_one
                .clone(),
            summary_review_context_template: m.review_context_template.clone(),
            summary_candidates_pile_template: m.candidates_pile_template.clone(),
            summary_list_joiner: m.list_joiner.clone(),
            summary_tie_joiner: m.tie_joiner.clone(),
            summary_code_joiner: m.code_joiner.clone(),
            summary_unanswered_zero: m.unanswered_zero.clone(),
            summary_review_zero: m.review_zero.clone(),
            summary_candidates_zero: m.candidates_zero.clone(),
            reviewer_display_name: reviewer_display_name.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::practice_params::KEY_PRACTICE_REVIEWER_DISPLAY_NAME;
    use crate::domain::wording_war_room::WAR_ROOM_WORDING_KEYS;
    use crate::domain::wording_war_room_summary::WAR_ROOM_SUMMARY_WORDING_KEYS;

    fn mirror() -> WarRoomWordingDto {
        WarRoomWordingDto::new(&WarRoomWording::for_test(), "Chuck")
    }

    /// The mirror carries every declared key — both sides derived, so a field
    /// added to the domain block and forgotten here fails at `cargo test` rather
    /// than as an `undefined` under a metric tile.
    #[test]
    fn the_mirror_carries_every_declared_key() {
        let dto = mirror();
        let value = serde_json::to_value(&dto).expect("the mirror serializes");
        // Both wording blocks, plus the one settings row the page prints.
        assert_eq!(
            value.as_object().expect("an object body").len(),
            WAR_ROOM_WORDING_KEYS.len() + WAR_ROOM_SUMMARY_WORDING_KEYS.len() + 1,
        );
    }

    /// Every wire name is the stored key without its `war_room_` prefix.
    #[test]
    fn every_wire_key_is_the_stored_key_without_its_prefix() {
        let dto = mirror();
        let value = serde_json::to_value(&dto).expect("the mirror serializes");
        for key in value.as_object().expect("an object body").keys() {
            // The one field that is not wording names its settings row instead.
            let stored = if key == "reviewer_display_name" {
                KEY_PRACTICE_REVIEWER_DISPLAY_NAME.to_string()
            } else {
                format!("war_room_{key}")
            };
            assert!(
                WAR_ROOM_WORDING_KEYS.contains(&stored.as_str())
                    || WAR_ROOM_SUMMARY_WORDING_KEYS.contains(&stored.as_str())
                    || stored == KEY_PRACTICE_REVIEWER_DISPLAY_NAME,
                "wire field '{key}' implies stored key '{stored}', which is not declared",
            );
        }
        assert_eq!(value["reviewer_display_name"], "Chuck");
    }
}
