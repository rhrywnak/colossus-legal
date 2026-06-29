// =============================================================================
// backend/src/dto/trial_prep.rs
// =============================================================================
//
// Wire DTOs for the Trial Prep ("War Room") dashboard payload.
//
// These structs MUST serialize to exactly the shape the frontend contract type
// `TrialPrepDashboard` declares in `frontend/src/pages/trialPrepData.ts`. The
// dashboard page renders this payload directly (metrics band · alerts strip ·
// scenario card grid), so a single field-name or casing mismatch silently breaks
// a card. The serialization test in this module is the cheapest guard against
// that — it encodes the field-by-field contract cross-check as an assertion.
//
// Field-name casing: the TS interface already uses snake_case keys
// (`instance_count`, `drafted_or_review`, …), so these Rust fields are spelled
// snake_case and serialize verbatim — no `rename_all` needed on the data structs.
// Only the status *enum* needs `rename_all = "snake_case"` to map Rust's
// CamelCase variants onto the lowercase wire tokens.
// =============================================================================

use serde::{Deserialize, Serialize};

/// Scenario lifecycle — drives the status dot and labels on each card.
///
/// ## Rust Learning: `#[serde(rename_all = "snake_case")]` on a unit enum
///
/// Serde serializes a fieldless enum variant as the variant *name* by default,
/// i.e. `NeedsResponse` → `"NeedsResponse"`. The frontend union is
/// `"drafted" | "review" | "ready" | "needs_response"` (lowercase, snake_case),
/// so `rename_all = "snake_case"` rewrites each variant to the wire token the TS
/// type expects: `NeedsResponse` → `"needs_response"`. Without this attribute the
/// card's status would never match and the dot would render wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioStatus {
    Drafted,
    Review,
    Ready,
    NeedsResponse,
}

/// One dashboard scenario card — mirrors `ScenarioSummary` in `trialPrepData.ts`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioSummary {
    pub id: String,
    pub attack: String,
    pub status: ScenarioStatus,
    pub instance_count: u32,
    pub response_count: u32,
    pub speakers: Vec<String>,

    /// `None` = pattern analysis pending; `Some(0)` = analysed, no baseless
    /// repeat.
    ///
    /// ## Rust Learning: `Option<u32>` serialized AS `null`, deliberately not skipped
    ///
    /// The frontend contract comment is explicit: optional display fields are
    /// "`T | null` (present-as-null, not omitted)". Serde serializes
    /// `Option::None` to JSON `null` by default — so we must NOT add
    /// `skip_serializing_if = "Option::is_none"` here. Skipping would omit the
    /// key entirely, collapsing the "analysis pending" (`null`) state into the
    /// "no data sent" state — exactly the kind of indistinguishable failure
    /// Standing Rule 1 forbids. `null` and `0` must stay distinguishable.
    pub baseless_repeat_count: Option<u32>,
}

/// A single living-binder notice ("N new instances …").
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrialPrepAlert {
    pub message: String,
}

/// The metrics band — mirrors the inline `metrics` object in `trialPrepData.ts`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrialPrepMetrics {
    pub scenarios: u32,
    pub ready: u32,
    pub drafted_or_review: u32,
    pub instances: u32,
    /// The Count IV signal — accusations repeated after a proven rebuttal.
    pub baseless_repeat_patterns: u32,
    pub no_response_yet: u32,
}

/// The full dashboard payload — mirrors `TrialPrepDashboard` in
/// `trialPrepData.ts`. Every field is always present (empty arrays, never
/// omitted keys), matching the contract's "present even when empty" rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrialPrepDashboard {
    pub metrics: TrialPrepMetrics,
    pub alerts: Vec<TrialPrepAlert>,
    pub scenarios: Vec<ScenarioSummary>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The status enum must serialize to the EXACT lowercase tokens the TS union
    /// declares. A regression that dropped `rename_all` (or renamed a variant)
    /// would surface here, not as a silently mis-rendered status dot in DEV.
    #[test]
    fn status_serializes_to_contract_tokens() {
        assert_eq!(
            serde_json::to_value(ScenarioStatus::Drafted).expect("serialize"),
            json!("drafted")
        );
        assert_eq!(
            serde_json::to_value(ScenarioStatus::Review).expect("serialize"),
            json!("review")
        );
        assert_eq!(
            serde_json::to_value(ScenarioStatus::Ready).expect("serialize"),
            json!("ready")
        );
        assert_eq!(
            serde_json::to_value(ScenarioStatus::NeedsResponse).expect("serialize"),
            json!("needs_response")
        );
    }

    /// The whole payload must serialize field-for-field to the `trialPrepData.ts`
    /// contract — this is the field-name cross-check encoded as a test (the most
    /// likely break per the build instruction). Critically, it asserts a
    /// `None` `baseless_repeat_count` becomes JSON `null` (present-as-null), NOT
    /// an omitted key.
    #[test]
    fn dashboard_serializes_to_contract_shape() {
        let dashboard = TrialPrepDashboard {
            metrics: TrialPrepMetrics {
                scenarios: 5,
                ready: 1,
                drafted_or_review: 3,
                instances: 16,
                baseless_repeat_patterns: 1,
                no_response_yet: 1,
            },
            alerts: vec![TrialPrepAlert {
                message: "an alert".to_string(),
            }],
            scenarios: vec![
                ScenarioSummary {
                    id: "marie-obstructive".to_string(),
                    attack: "Marie is obstructive".to_string(),
                    status: ScenarioStatus::Review,
                    instance_count: 4,
                    response_count: 1,
                    speakers: vec!["CFS".to_string()],
                    baseless_repeat_count: Some(3),
                },
                ScenarioSummary {
                    id: "selective-sanctions".to_string(),
                    attack: "Selective sanctions".to_string(),
                    status: ScenarioStatus::Drafted,
                    instance_count: 2,
                    response_count: 1,
                    speakers: vec!["CFS".to_string()],
                    // Analysis pending → must serialize as null, not be omitted.
                    baseless_repeat_count: None,
                },
            ],
        };

        let value = serde_json::to_value(&dashboard).expect("dashboard serializes");

        assert_eq!(
            value,
            json!({
                "metrics": {
                    "scenarios": 5,
                    "ready": 1,
                    "drafted_or_review": 3,
                    "instances": 16,
                    "baseless_repeat_patterns": 1,
                    "no_response_yet": 1
                },
                "alerts": [{ "message": "an alert" }],
                "scenarios": [
                    {
                        "id": "marie-obstructive",
                        "attack": "Marie is obstructive",
                        "status": "review",
                        "instance_count": 4,
                        "response_count": 1,
                        "speakers": ["CFS"],
                        "baseless_repeat_count": 3
                    },
                    {
                        "id": "selective-sanctions",
                        "attack": "Selective sanctions",
                        "status": "drafted",
                        "instance_count": 2,
                        "response_count": 1,
                        "speakers": ["CFS"],
                        "baseless_repeat_count": null
                    }
                ]
            })
        );
    }

    /// Guard the present-as-null rule in isolation: the `selective-sanctions`
    /// card's pending state (`None`) MUST appear as a `baseless_repeat_count`
    /// key whose value is `null`. If a future edit adds `skip_serializing_if`,
    /// the key would vanish and this fails — catching the Rule-1 collapse of
    /// "pending" into "absent".
    #[test]
    fn pending_baseless_count_is_present_as_null() {
        let card = ScenarioSummary {
            id: "selective-sanctions".to_string(),
            attack: "Selective sanctions".to_string(),
            status: ScenarioStatus::Drafted,
            instance_count: 2,
            response_count: 1,
            speakers: vec!["CFS".to_string()],
            baseless_repeat_count: None,
        };

        let value = serde_json::to_value(&card).expect("card serializes");
        let obj = value.as_object().expect("object body");
        assert!(
            obj.contains_key("baseless_repeat_count"),
            "the key must be present even when None"
        );
        assert!(
            obj["baseless_repeat_count"].is_null(),
            "None must serialize as JSON null, not be omitted"
        );
    }
}
