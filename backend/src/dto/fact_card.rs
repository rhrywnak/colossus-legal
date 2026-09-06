//! The five sentences a witness's card carries, as the browser receives them
//! (FACT_CARD_v2 §2).
//!
//! ## Everything here is display-ready
//!
//! The same law `dto::scenario_card` states: every string is finished trial
//! language, composed server-side from the stored wording. The browser renders
//! and concatenates nothing — no "Point " + n, no verb chosen from a token, no
//! count tag assembled from a number and a title.
//!
//! ## Why the RAW values travel beside the composed ones
//!
//! `backs_position` and `supports_refs` are the values an EDITOR needs: clicking
//! the Backs row opens a picker that has to know which point is currently
//! selected, and Including a fact prefills its stance and allegation from
//! `supports[0]` (§2). A client that had to parse "Point 2 — …" back into a 2
//! would be reading meaning out of a display string, which is exactly what these
//! payloads exist to forbid.
//!
//! ## Domain note: nothing here is hidden for being absent
//!
//! Every field is `Option`, and `None` renders as the stored em dash rather than
//! removing the row. §2 is explicit — a card missing its Answer is work somebody
//! still owes, and hiding the row hides the work rather than the gap. (The mockup
//! of record hides such cards from Marie; the instruction governs.)

use serde::{Deserialize, Serialize};

use crate::domain::fact_card::CardStance;

/// One accusation a card names, with the label a human reads.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CardSupportRef {
    /// The accusation's graph id — what an Include writes into the links table.
    pub allegation_id: String,
    /// Which way this card cuts, as a token to branch on.
    pub stance: CardStance,
    /// The accusation's handle — `"A-21"`. Composed by
    /// [`crate::domain::scenario_code::allegation_code`], the same function the
    /// link panel uses, so the two surfaces name one accusation identically.
    pub code: String,
}

/// Which fields are still the machine's draft (§2).
///
/// ## Why five booleans and not a list of tokens
///
/// The client asks "does THIS row get a draft mark", five times, once per row. A
/// list would make each of those a membership test against strings — the browser
/// branching on a vocabulary it has to keep in step with the backend's. Five
/// named booleans are the same answer with no vocabulary in the client at all.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct FactCardDrafts {
    pub title: bool,
    pub backs: bool,
    pub supports: bool,
    pub watch_out: bool,
    pub answer: bool,
}

impl FactCardDrafts {
    /// Whether ANY field on this card is still a draft.
    ///
    /// Not rendered anywhere today. It exists because "is this deck prepared" is
    /// the question a readiness check will ask, and deriving it in a second place
    /// from five booleans is how the two answers start to differ.
    pub fn any(self) -> bool {
        self.title || self.backs || self.supports || self.watch_out || self.answer
    }
}

/// One card's authored block.
///
/// Absent from a `ScenarioCard` entirely when nobody — machine or human — has
/// written anything for that statement in that scenario. Absent, not empty: a
/// card with five em dashes and a card nobody has drafted are different states,
/// and the second one is what the loader has not reached yet.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FactCardBlock {
    /// What Marie says in three seconds. `None` renders as the stored em dash.
    pub title: Option<String>,
    /// The Backs row, composed — "Point 2 — My sisters' claim rested on hearsay."
    ///
    /// `None` when the card backs no point, AND when it names a position no
    /// talking point occupies: a stale pointer renders as the em dash rather than
    /// as "Point 7 — " with nothing after it. `backs_position` still carries the
    /// number, so the editor can show what was stored and a human can fix it.
    pub backs: Option<String>,
    /// The stored position, for the editor. See the module note on raw values.
    pub backs_position: Option<i32>,
    /// The Supports rows, composed — "Supports A-21 — CFS could have returned…".
    ///
    /// Empty is a real state and renders as the em dash. At most two (§1).
    pub supports: Vec<String>,
    /// The same accusations as ids and stances, for the editor and for the
    /// Include prefill (§2).
    pub supports_refs: Vec<CardSupportRef>,
    /// How the other side uses this fact.
    pub watch_out: Option<String>,
    /// Her reply — the one sentence written to be said out loud.
    pub answer: Option<String>,
    /// Which fields still carry the machine's draft mark.
    pub drafts: FactCardDrafts,
    /// The count tag(s) for the title bar — "Count 1 — Breach of Fiduciary Duty".
    ///
    /// Derived from `supports` → allegation → count (§2), deduplicated and in the
    /// order the accusations are named. Empty when no named accusation reaches a
    /// count: 25 of the case's 120 allegations are wired to no element yet, which
    /// is a real state the title bar shows by carrying no tag.
    pub count_tags: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The block's wire shape, pinned. The frontend reads exactly these
    /// snake_case keys.
    #[test]
    fn the_block_serializes_with_the_expected_keys() {
        let block = FactCardBlock {
            title: Some("The court ordered the $50,000 back".to_string()),
            backs: Some("Point 1 — The court itself says so.".to_string()),
            backs_position: Some(1),
            supports: vec!["Supports A-21 — CFS could have returned it.".to_string()],
            supports_refs: vec![CardSupportRef {
                allegation_id: "doc-x:allegation:45984d77".to_string(),
                stance: CardStance::Supports,
                code: "A-21".to_string(),
            }],
            watch_out: None,
            answer: Some("The money was Dad's.".to_string()),
            drafts: FactCardDrafts {
                title: true,
                ..FactCardDrafts::default()
            },
            count_tags: vec!["Count 1 — Breach of Fiduciary Duty".to_string()],
        };
        let value = serde_json::to_value(&block).expect("serializes cleanly");
        let mut keys: Vec<&str> = value
            .as_object()
            .expect("an object body")
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec![
                "answer",
                "backs",
                "backs_position",
                "count_tags",
                "drafts",
                "supports",
                "supports_refs",
                "title",
                "watch_out",
            ]
        );
        // An empty field is `null`, PRESENT — the row renders an em dash rather
        // than disappearing (§2: nothing is hidden for lacking a field).
        assert_eq!(value["watch_out"], json!(null));
        assert_eq!(value["drafts"]["title"], json!(true));
        assert_eq!(value["drafts"]["answer"], json!(false));
        assert_eq!(value["supports_refs"][0]["stance"], json!("supports"));
    }

    /// A wholly undrafted block still serializes every key, with empty lists
    /// rather than absent ones.
    ///
    /// The client reads `.length` on both lists to decide between the rows and
    /// the em dash; a missing key would throw mid-render, and this app has no
    /// React error boundary — that surfaces as a BLANK PAGE.
    #[test]
    fn an_empty_block_still_carries_both_lists() {
        let block = FactCardBlock {
            title: None,
            backs: None,
            backs_position: None,
            supports: Vec::new(),
            supports_refs: Vec::new(),
            watch_out: None,
            answer: None,
            drafts: FactCardDrafts::default(),
            count_tags: Vec::new(),
        };
        let value = serde_json::to_value(&block).expect("serializes cleanly");
        assert_eq!(value["supports"], json!([]));
        assert_eq!(value["supports_refs"], json!([]));
        assert_eq!(value["count_tags"], json!([]));
    }

    /// `any()` is true when any single field is a draft, and false when none is.
    #[test]
    fn any_draft_is_true_for_each_field_alone() {
        assert!(!FactCardDrafts::default().any());
        for drafts in [
            FactCardDrafts {
                title: true,
                ..Default::default()
            },
            FactCardDrafts {
                backs: true,
                ..Default::default()
            },
            FactCardDrafts {
                supports: true,
                ..Default::default()
            },
            FactCardDrafts {
                watch_out: true,
                ..Default::default()
            },
            FactCardDrafts {
                answer: true,
                ..Default::default()
            },
        ] {
            assert!(drafts.any(), "{drafts:?} carries a draft");
        }
    }
}
