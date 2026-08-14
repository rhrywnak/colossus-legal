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
