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
/// The vocabulary is the real `scenarios` table CHECK set: `draft`,
/// `needs_evidence`, `ready` (it replaced the old placeholder set
/// `drafted/review/ready/needs_response` when the dashboard began reading real
/// scenarios — Chunk 2). This enum is the single source of truth for the
/// vocabulary: the dashboard assembler parses a DB status string back THROUGH
/// this enum's `Deserialize` rather than re-spelling the tokens.
///
/// ## Rust Learning: `#[serde(rename_all = "snake_case")]` on a unit enum
///
/// Serde serializes a fieldless enum variant as the variant *name* by default,
/// i.e. `NeedsEvidence` → `"NeedsEvidence"`. The wire/DB tokens are snake_case
/// (`"needs_evidence"`), so `rename_all = "snake_case"` rewrites each variant on
/// BOTH serialize and deserialize. Without it the status dot would never match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioStatus {
    Draft,
    NeedsEvidence,
    Ready,
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

// ─────────────────────────────────────────────────────────────────────────────
// Scenario detail (the per-scenario page payload) — mirrors `ScenarioDetail` /
// `ExchangeTurn` / `MarieResponse` in `trialPrepData.ts`.
// ─────────────────────────────────────────────────────────────────────────────

/// One turn in a scenario's exchange timeline — mirrors `ExchangeTurn`.
///
/// All optional display fields are `Option<…>` serialized present-as-null
/// (no `skip_serializing_if`), matching the TS `T | null` contract. `kind` is a
/// string (the assembler emits the neutral `"evidence"` for graph facts);
/// `page_number` is an `i64` (the assembler parses the graph fact's string page
/// to a number, or `null` when un-parseable).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExchangeTurn {
    pub kind: String,
    pub grounded: bool,
    pub speaker: Option<String>,
    pub date: Option<String>,
    pub text: String,
    pub relationship_type: Option<String>,
    pub source_document: Option<String>,
    pub page_number: Option<i64>,
    pub paragraph: Option<String>,
    pub repeated_after_rebuttal: bool,
}

/// One rehearsable response — mirrors `MarieResponse`. Not wired yet (the
/// detail payload returns an empty `responses` vec this chunk), but typed so the
/// vec element is well-defined.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarieResponse {
    pub id: String,
    pub label: String,
    pub text: String,
    pub authored_by: String,
}

/// The full per-scenario detail payload — mirrors `ScenarioDetail`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioDetail {
    pub id: String,
    pub attack: String,
    pub status: ScenarioStatus,
    pub pattern_summary: Option<String>,
    pub timeline: Vec<ExchangeTurn>,
    pub responses: Vec<MarieResponse>,
    pub notes: Option<String>,
    /// The scenario's authored definition body, carried opaquely from the
    /// Postgres `scenarios.definition` column (same `serde_json::Value` shape as
    /// `ScenarioDto.definition`). `{}` for an un-authored scenario. The typed
    /// shape (`ScenarioDefinition`, 8 keys) lives at the CRUD boundary in
    /// `dto/scenario_crud.rs`; this endpoint stays JSON-opaque so the War Room
    /// payload never has to re-model it.
    pub definition: serde_json::Value,
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
            serde_json::to_value(ScenarioStatus::Draft).expect("serialize"),
            json!("draft")
        );
        assert_eq!(
            serde_json::to_value(ScenarioStatus::NeedsEvidence).expect("serialize"),
            json!("needs_evidence")
        );
        assert_eq!(
            serde_json::to_value(ScenarioStatus::Ready).expect("serialize"),
            json!("ready")
        );
    }

    /// The same tokens must DESERIALIZE back to the variants (the assembler relies
    /// on this to parse a DB status string), and an unknown token must error
    /// rather than silently pick a default.
    #[test]
    fn status_deserializes_from_contract_tokens() {
        assert_eq!(
            serde_json::from_value::<ScenarioStatus>(json!("needs_evidence")).expect("parse"),
            ScenarioStatus::NeedsEvidence
        );
        assert!(serde_json::from_value::<ScenarioStatus>(json!("bogus")).is_err());
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
                    status: ScenarioStatus::NeedsEvidence,
                    instance_count: 4,
                    response_count: 1,
                    speakers: vec!["CFS".to_string()],
                    baseless_repeat_count: Some(3),
                },
                ScenarioSummary {
                    id: "selective-sanctions".to_string(),
                    attack: "Selective sanctions".to_string(),
                    status: ScenarioStatus::Draft,
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
                        "status": "needs_evidence",
                        "instance_count": 4,
                        "response_count": 1,
                        "speakers": ["CFS"],
                        "baseless_repeat_count": 3
                    },
                    {
                        "id": "selective-sanctions",
                        "attack": "Selective sanctions",
                        "status": "draft",
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
            status: ScenarioStatus::Draft,
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

    /// The detail payload serializes to the `trialPrepData.ts` ScenarioDetail
    /// contract: snake_case keys, `pattern_summary`/`notes`/`date` present-as-null,
    /// and an evidence turn carrying `kind`/`relationship_type`/`page_number`.
    #[test]
    fn scenario_detail_serializes_to_contract_shape() {
        let detail = ScenarioDetail {
            id: "00000000-0000-0000-0000-000000000000".to_string(),
            attack: "Marie is obstructive".to_string(),
            status: ScenarioStatus::Draft,
            pattern_summary: None,
            timeline: vec![ExchangeTurn {
                kind: "evidence".to_string(),
                grounded: true,
                speaker: Some("George Phillips".to_string()),
                date: None,
                text: "the quote".to_string(),
                relationship_type: Some("rebuts".to_string()),
                source_document: Some("doc-x".to_string()),
                page_number: Some(54),
                paragraph: Some("¶54".to_string()),
                repeated_after_rebuttal: false,
            }],
            responses: Vec::new(),
            notes: None,
            // A non-empty authored definition so the assertion PROVES the body is
            // carried into the payload (not merely that an empty key exists). Only
            // the required pair (attack_text + schema_v) is needed to be a valid
            // shape; the extra keys exercise the opaque passthrough.
            definition: json!({
                "attack_text": "Marie is obstructive",
                "schema_v": 1,
                "wielders": ["George Phillips"]
            }),
        };

        let value = serde_json::to_value(&detail).expect("detail serializes");
        assert_eq!(
            value,
            json!({
                "id": "00000000-0000-0000-0000-000000000000",
                "attack": "Marie is obstructive",
                "status": "draft",
                "pattern_summary": null,
                "timeline": [{
                    "kind": "evidence",
                    "grounded": true,
                    "speaker": "George Phillips",
                    "date": null,
                    "text": "the quote",
                    "relationship_type": "rebuts",
                    "source_document": "doc-x",
                    "page_number": 54,
                    "paragraph": "¶54",
                    "repeated_after_rebuttal": false
                }],
                "responses": [],
                "notes": null,
                "definition": {
                    "attack_text": "Marie is obstructive",
                    "schema_v": 1,
                    "wielders": ["George Phillips"]
                }
            })
        );
    }
}
