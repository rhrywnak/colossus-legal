//! Where one scenario stands — the War Room status card's numbers.
//!
//! CC_TASK_WAR_ROOM_v1. Mirrors `ScenarioProgress` in `trialPrepData.ts`.
//!
//! ## Numbers, never sentences
//!
//! Every field is a count, a date, or a name. The card's words ("4 of 99",
//! "Scan: never run", "3 new or changed for Marie") are stored templates on
//! `WarRoomWordingDto`, and the browser fills them. A pre-composed English string
//! here would be a second place the wording lives.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The card's EVIDENCE pane, its PREP & REHEARSAL pane, and the changed badge.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioProgress {
    /// Candidate facts a human ruled Included — the scenario page's fact list.
    pub facts_included: u32,
    /// Cards the latest completed scan proposes that nobody has ruled.
    pub candidates_to_rule: u32,
    /// The stuck pile and how much of it a human has linked.
    pub matrix_linked: MatrixLinked,
    /// The newest scan of any status, or `null` when none has ever run.
    ///
    /// ## Rust Learning: `Option` serialized as `null`, deliberately not skipped
    ///
    /// "Never scanned" is a fact the card shows in the warning colour. Omitting
    /// the key would make it indistinguishable from a build that does not send the
    /// field at all (Standing Rule 1), so `None` goes out as JSON `null`.
    pub last_scan: Option<LastScan>,
    pub deck: DeckSummary,
    pub answered: AnsweredSplit,
    /// Visible questions new or changed since Marie last answered on this deck
    /// (CC_GO_WAR_ROOM_v3). `0` on a deck nobody has answered.
    pub marie_changed: u32,
    /// Answers awaiting the REVIEWER on this scenario — the same number for every
    /// viewer (CC_TASK_SIMPLE_COUNTS_v1). Counted against the
    /// `practice_reviewer_username` settings row's Done reviewing mark.
    pub awaiting_review: u32,
    /// When the oldest of those answers was written; `null` when nothing waits.
    /// Serialized as `null`, not skipped, for the reason `last_scan` gives.
    pub oldest_awaiting_review: Option<DateTime<Utc>>,
}

/// `linked` of `total` — `total` is the cards the extraction left unlinked.
///
/// `total == 0` means nothing was stuck; the card renders an em dash, never
/// "0 of 0" (ruling Q4).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixLinked {
    pub linked: u32,
    pub total: u32,
}

/// The scan line's one fact: the day. The model and the relevant counts left the
/// card for the scenario page (CC_TASK_REVIEW_LOOP_v1 §5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LastScan {
    /// When the run started. The browser formats the day.
    pub when: DateTime<Utc>,
}

/// The practice deck: visible questions, and the day it last changed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeckSummary {
    pub questions: u32,
    /// `null` for a scenario with no deck at all.
    pub built_on: Option<DateTime<Utc>>,
}

/// The answered line: `total` answered `of` the visible questions, split by side.
///
/// `total` / `of` are the task's own names: "0 of 17 answered".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnsweredSplit {
    pub total: u32,
    pub of: u32,
    pub chuck_answered: u32,
    pub chuck_total: u32,
    pub defense_answered: u32,
    pub defense_total: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The wire carries the global review queue — and no per-viewer count any more
    /// (CC_TASK_SIMPLE_COUNTS_v1: one truth for every viewer).
    #[test]
    fn the_wire_carries_the_review_queue_and_no_viewer_count() {
        let value = serde_json::to_value(ScenarioProgress::default()).expect("serializes");
        let fields = value.as_object().expect("an object");
        assert!(!fields.contains_key("new_answers_for_viewer"));
        assert_eq!(fields.get("awaiting_review"), Some(&serde_json::json!(0)));
        assert_eq!(
            fields.get("oldest_awaiting_review"),
            Some(&serde_json::Value::Null)
        );
    }
}
