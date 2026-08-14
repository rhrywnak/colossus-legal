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
// (`drafted_or_review`, `baseless_repeat_count`, …), so these Rust fields are spelled
// snake_case and serialize verbatim — no `rename_all` needed on the data structs.
// Only the status *enum* needs `rename_all = "snake_case"` to map Rust's
// CamelCase variants onto the lowercase wire tokens.
// =============================================================================

use serde::{Deserialize, Serialize};

use crate::dto::scenario_authoring_wording::ScenarioCreateWordingDto;
use crate::dto::war_room_wording::WarRoomWordingDto;

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
    /// The scenario's human handle — `"S-3"` (§2a), formatted by the backend.
    /// Rendered beside the attack text on every card so the code a human says out
    /// loud is visible wherever the scenario is.
    pub code: String,
    pub attack: String,
    pub status: ScenarioStatus,

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
///
/// ## Two figures were removed on 2026-07-27
///
/// `baseless_repeat_patterns` counted cards whose `baseless_repeat_count` was
/// `> 0`; that field is hardcoded `None` because pattern analysis is unwired, so
/// the figure was **structurally 0** — indistinguishable on screen from a real
/// zero. `no_response_yet` counted cards with `response_count == 0`; that field
/// was hardcoded `0`, so the figure **always equalled the scenario count**.
///
/// What was wrong there was not the stubs — which say what they are — but
/// deriving band figures from them and presenting the result as a measurement.
/// Both return when their sources are real.
///
/// ## And a third on 2026-08-07: `instances` (Roman's ruling)
///
/// This one was NOT a stub — it was a live per-scenario graph read, summing
/// REBUTS relationships across each scenario's anchor allegations. It is gone
/// for a different reason: it earned nothing. Nobody could act on the number,
/// and its name collided head-on with task 2.11's "accusation instances", which
/// counts something entirely unrelated. Two meanings for one word on one product
/// is worse than no number.
///
/// The per-card `instance_count`, `speakers` and `response_count` went with it.
/// `instance_count` was the graph read; the other two were the rest of the card
/// line that displayed it, hardcoded stubs with no remaining reader once the
/// line was gone.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrialPrepMetrics {
    pub scenarios: u32,
    pub ready: u32,
    pub drafted_or_review: u32,
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
    /// The words the create-scenario form speaks (2026-08-07).
    ///
    /// ## Why they ride the dashboard payload rather than their own endpoint
    ///
    /// The form lives on this page and nowhere else, and this page already
    /// fetches exactly once on mount. A dedicated route would be a second
    /// request for five strings, on a surface that has one; riding along means
    /// the form's words and the scenario cards beside it come from one snapshot
    /// of the settings store and cannot disagree.
    pub create_wording: ScenarioCreateWordingDto,
    /// The words the PAGE itself speaks — its subtitle and its three metric tile
    /// labels (task 396, P3b).
    ///
    /// Ruled by R2 on 2026-08-10 and never migrated; measured still-literal on
    /// 2026-08-13. They ride here for the same reason `create_wording` does.
    pub war_room_wording: WarRoomWordingDto,
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
    /// The scenario's human handle — `"S-3"` (§2a), formatted by the backend and
    /// rendered in the detail header beside the name.
    pub code: String,
    pub attack: String,
    /// `offense` | `defense` — the scenario's posture (task 1.7B).
    ///
    /// Carried so the lean header can show its direction chip without a second
    /// request. DISPLAY ONLY: direction is set once at create and the update
    /// route refuses it (`ScenarioUpdateRequest` has no such field), because
    /// flipping a scenario's posture would make it a different scenario. The
    /// token is passed through verbatim rather than mapped to a label — an
    /// unrecognised one must reach the screen as itself, not as a default
    /// posture the page invented.
    pub direction: String,
    pub status: ScenarioStatus,
    pub pattern_summary: Option<String>,
    pub timeline: Vec<ExchangeTurn>,
    pub responses: Vec<MarieResponse>,
    pub notes: Option<String>,
    /// Complaint-paragraph anchors this scenario touches (the `scenarios`
    /// `anchor_allegation_ids` column). Flattened from `Option<Vec<String>>` to a
    /// plain `Vec<String>` (`None` → `[]`) so the client never distinguishes null
    /// from empty — same convention as `ScenarioDto`. The define form (D1)
    /// pre-fills its allegation picker from this and writes edits back through the
    /// update route, so the value must round-trip on the detail payload.
    pub anchor_allegation_ids: Vec<String>,
    /// The scenario's authored definition body, carried opaquely from the
    /// Postgres `scenarios.definition` column (same `serde_json::Value` shape as
    /// `ScenarioDto.definition`). `{}` for an un-authored scenario. The typed
    /// shape (`ScenarioDefinition`) lives at the CRUD boundary in
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
        let dashboard = sample_dashboard();

        let value = serde_json::to_value(&dashboard).expect("dashboard serializes");

        assert_eq!(
            value,
            json!({
                "metrics": {
                    "scenarios": 5,
                    "ready": 1,
                    "drafted_or_review": 3,
                },
                "alerts": [{ "message": "an alert" }],
                "scenarios": [
                    {
                        "code": "S-1",
                        "id": "marie-obstructive",
                        "attack": "Marie is obstructive",
                        "status": "needs_evidence",
                        "baseless_repeat_count": 3
                    },
                    {
                        "code": "S-2",
                        "id": "selective-sanctions",
                        "attack": "Selective sanctions",
                        "status": "draft",
                        "baseless_repeat_count": null
                    }
                ],
                "create_wording": {
                    "target_label": "Who this scenario is about",
                    "target_helper": "Evidence is gathered about this person.",
                    "target_unset_option": "Choose a person…",
                    "accusation_label": "The accusation, in plain language",
                    "accusation_helper": "What the other side is saying.",
                    "target_required": "Choose who this scenario is about.",
                    "accusation_required": "Write the accusation in plain language."
                },
                "war_room_wording": {
                    "subtitle": "The attacks and what we answer them with — built by you, gathered by the system, rehearsed by Marie.",
                    "metric_scenarios_label": "Scenarios",
                    "metric_ready_label": "Ready",
                    "metric_draft_label": "Draft"
                }
            })
        );
    }

    /// The page's own words reach the browser.
    ///
    /// R2 ruled these rows on 2026-08-10 and the batch shipped without them, so
    /// the subtitle stayed a literal for three days with every test green. This is
    /// the assertion that would have caught it: the payload must carry the words,
    /// and the subtitle must not be the sentence they were meant to replace.
    #[test]
    fn the_dashboard_carries_the_pages_own_words() {
        let value = serde_json::to_value(sample_dashboard()).expect("serializes");
        let subtitle = value["war_room_wording"]["subtitle"]
            .as_str()
            .expect("the subtitle is served");
        assert!(!subtitle.is_empty());
        assert!(
            !subtitle.to_lowercase().contains("system-generated"),
            "the served subtitle still credits the machine: {subtitle}",
        );
    }

    /// The dashboard payload both serialization tests read.
    ///
    /// One fixture rather than two: the contract test below asserts the exact
    /// JSON, and the removal test asserts which keys are ABSENT. Two fixtures
    /// would let the second one keep passing against a shape the first no longer
    /// describes.
    fn sample_dashboard() -> TrialPrepDashboard {
        TrialPrepDashboard {
            metrics: TrialPrepMetrics {
                scenarios: 5,
                ready: 1,
                drafted_or_review: 3,
            },
            alerts: vec![TrialPrepAlert {
                message: "an alert".to_string(),
            }],
            scenarios: vec![
                ScenarioSummary {
                    code: "S-1".to_string(),
                    id: "marie-obstructive".to_string(),
                    attack: "Marie is obstructive".to_string(),
                    status: ScenarioStatus::NeedsEvidence,
                    baseless_repeat_count: Some(3),
                },
                ScenarioSummary {
                    code: "S-2".to_string(),
                    id: "selective-sanctions".to_string(),
                    attack: "Selective sanctions".to_string(),
                    status: ScenarioStatus::Draft,
                    // Analysis pending → must serialize as null, not be omitted.
                    baseless_repeat_count: None,
                },
            ],
            create_wording: ScenarioCreateWordingDto {
                target_label: "Who this scenario is about".to_string(),
                target_helper: "Evidence is gathered about this person.".to_string(),
                target_unset_option: "Choose a person…".to_string(),
                accusation_label: "The accusation, in plain language".to_string(),
                accusation_helper: "What the other side is saying.".to_string(),
                target_required: "Choose who this scenario is about.".to_string(),
                accusation_required: "Write the accusation in plain language.".to_string(),
            },
            war_room_wording: WarRoomWordingDto::from(
                &crate::domain::wording_war_room::WarRoomWording::for_test(),
            ),
        }
    }

    /// The dashboard payload carries no `instances` figure and no per-card count
    /// (2026-08-07).
    ///
    /// Behavioural, not shape-pinning: the removal's whole point is that the
    /// dashboard stops SERVING a number nobody can act on — and stops doing the
    /// per-scenario graph read that produced it. A field re-appearing here means
    /// the read is back on the page load, whether or not anything renders it.
    #[test]
    fn the_dashboard_serves_no_instances_figure() {
        let dashboard = sample_dashboard();
        let value = serde_json::to_value(&dashboard).expect("dashboard serializes");

        assert!(
            value["metrics"]
                .as_object()
                .expect("metrics")
                .keys()
                .all(|k| k != "instances"),
            "the metrics band must not carry an instances figure: {}",
            value["metrics"]
        );

        for card in value["scenarios"].as_array().expect("cards") {
            let keys = card.as_object().expect("card body");
            for retired in ["instance_count", "response_count", "speakers"] {
                assert!(
                    !keys.contains_key(retired),
                    "a scenario card must not carry '{retired}' — it went with the \
                     card line that displayed it"
                );
            }
        }
    }

    /// Guard the present-as-null rule in isolation: the `selective-sanctions`
    /// card's pending state (`None`) MUST appear as a `baseless_repeat_count`
    /// key whose value is `null`. If a future edit adds `skip_serializing_if`,
    /// the key would vanish and this fails — catching the Rule-1 collapse of
    /// "pending" into "absent".
    #[test]
    fn pending_baseless_count_is_present_as_null() {
        let card = ScenarioSummary {
            code: "S-1".to_string(),
            id: "selective-sanctions".to_string(),
            attack: "Selective sanctions".to_string(),
            status: ScenarioStatus::Draft,
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
            code: "S-1".to_string(),
            id: "00000000-0000-0000-0000-000000000000".to_string(),
            attack: "Marie is obstructive".to_string(),
            direction: "defense".to_string(),
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
            anchor_allegation_ids: vec!["54".to_string(), "55".to_string()],
            // A non-empty authored definition so the assertion PROVES the body is
            // carried into the payload (not merely that an empty key exists). Only
            // the required pair (attack_text + schema_v) is needed to be a valid
            // shape; the extra keys exercise the opaque passthrough.
            definition: json!({
                "attack_text": "Marie is obstructive",
                "schema_v": 2,
                "wielders": [{ "party_id": "person-george-phillips", "actor_role": "originated" }]
            }),
        };

        let value = serde_json::to_value(&detail).expect("detail serializes");
        assert_eq!(
            value,
            json!({
                "code": "S-1",
                "id": "00000000-0000-0000-0000-000000000000",
                "attack": "Marie is obstructive",
                // Task 1.7B: the lean header's direction chip reads this.
                "direction": "defense",
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
                "anchor_allegation_ids": ["54", "55"],
                "definition": {
                    "attack_text": "Marie is obstructive",
                    "schema_v": 2,
                    "wielders": [{ "party_id": "person-george-phillips", "actor_role": "originated" }]
                }
            })
        );
    }
}
