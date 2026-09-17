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
    pub card_viewer_new_template: String,
    pub card_viewer_new_one: String,
    pub card_changed_one: String,
    pub card_not_started: String,
    pub card_up_to_date: String,
    pub card_practice_action: String,
    pub card_timeline_action: String,
    pub card_delete_action: String,
    // The queue strip (CC_TASK_REVIEW_LOOP_v1 §5).
    pub strip_answered_label: String,
    pub strip_answered_template: String,
    pub strip_waiting_label: String,
    pub strip_new_for_you_label: String,
    pub strip_candidates_label: String,
}

/// ## Rust Learning: `From<&T>` rather than `From<T>`
///
/// The settings snapshot lives behind an `Arc` and must outlive this conversion;
/// cloning four `String`s is a far smaller copy than cloning `Settings`.
impl From<&WarRoomWording> for WarRoomWordingDto {
    fn from(w: &WarRoomWording) -> Self {
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
            card_viewer_new_template: w.card_viewer_new_template.clone(),
            card_viewer_new_one: w.card_viewer_new_one.clone(),
            card_changed_one: w.card_changed_one.clone(),
            card_not_started: w.card_not_started.clone(),
            card_up_to_date: w.card_up_to_date.clone(),
            card_practice_action: w.card_practice_action.clone(),
            card_timeline_action: w.card_timeline_action.clone(),
            card_delete_action: w.card_delete_action.clone(),
            strip_answered_label: w.strip_answered_label.clone(),
            strip_answered_template: w.strip_answered_template.clone(),
            strip_waiting_label: w.strip_waiting_label.clone(),
            strip_new_for_you_label: w.strip_new_for_you_label.clone(),
            strip_candidates_label: w.strip_candidates_label.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::wording_war_room::WAR_ROOM_WORDING_KEYS;

    /// The mirror carries every declared key — both sides derived, so a field
    /// added to the domain block and forgotten here fails at `cargo test` rather
    /// than as an `undefined` under a metric tile.
    #[test]
    fn the_mirror_carries_every_declared_key() {
        let dto = WarRoomWordingDto::from(&WarRoomWording::for_test());
        let value = serde_json::to_value(&dto).expect("the mirror serializes");
        assert_eq!(
            value.as_object().expect("an object body").len(),
            WAR_ROOM_WORDING_KEYS.len(),
        );
    }

    /// Every wire name is the stored key without its `war_room_` prefix.
    #[test]
    fn every_wire_key_is_the_stored_key_without_its_prefix() {
        let dto = WarRoomWordingDto::from(&WarRoomWording::for_test());
        let value = serde_json::to_value(&dto).expect("the mirror serializes");
        for key in value.as_object().expect("an object body").keys() {
            let stored = format!("war_room_{key}");
            assert!(
                WAR_ROOM_WORDING_KEYS.contains(&stored.as_str()),
                "wire field '{key}' implies stored key '{stored}', which is not declared",
            );
        }
    }
}
