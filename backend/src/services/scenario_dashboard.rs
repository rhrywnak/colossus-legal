// =============================================================================
// backend/src/services/scenario_dashboard.rs
// =============================================================================
//
// War Room dashboard assembler — composes the Trial Prep dashboard payload from
// the REAL scenarios in Postgres plus their live graph-derived counts.
//
// This is DELIBERATELY NOT the existing `ScenarioPageAssembler` (services/
// scenario_page.rs). That one composes a wielder/anchor *facts* page; this one
// composes the *dashboard* payload (metrics band · alerts strip · scenario
// cards). Different payload, different consumer — kept distinctly named so the
// two are never confused.
//
// Two data sources:
//   - Postgres (pipeline DB `colossus_legal_v2`, via `pipeline_pool`): the
//     authored `scenarios` rows — the list of cards and their identity/status.
//   - Neo4j (via `ScenarioRepository`): for each scenario's anchor allegation(s),
//     the live REBUTS count that drives the card's `instance_count`.
//
// With zero scenarios in the table the dashboard is honestly empty (no cards,
// zeroed metrics, no alerts). Cards appear as scenarios are authored.
//
// Testability split: the per-record shaping (status-string → enum, record →
// card, metrics) is pure and unit-tested without a DB/graph. Only `assemble`
// (and its `count_record_rebuts` helper) touch I/O; those are DEV-verified, the
// same convention the `ScenarioRepository` query methods follow.
// =============================================================================

use sqlx::PgPool;
use uuid::Uuid;

use crate::dto::scenario::{AnchoredAllegationEvidenceResponse, AnchoredEvidenceFact};
use crate::dto::trial_prep::{
    ExchangeTurn, ScenarioDetail, ScenarioStatus, ScenarioSummary, TrialPrepDashboard,
    TrialPrepMetrics,
};
use crate::neo4j::schema;
use crate::repositories::pipeline_repository::{
    get_scenario, list_scenarios_for_case, PipelineRepoError, ScenarioRecord,
};
use crate::repositories::scenario_repository::{
    EvidencePolarity, ScenarioRepository, ScenarioRepositoryError,
};

// ─────────────────────────────────────────────────────────────────────────────
// Error
// ─────────────────────────────────────────────────────────────────────────────

/// Error surface for dashboard assembly.
///
/// ## Rust Learning: one variant per distinct failure class
///
/// Each variant names a different thing that can go wrong, so a handler logging
/// `{}` gets the WHAT and the WHERE (Standing Rule 1):
/// - `Store` — the Postgres list failed; carries the `case_slug` (the WHERE).
/// - `Fetch` — fetching ONE scenario by id from Postgres failed; carries the
///   `scenario_id` (the WHERE).
/// - `Repository` — a graph traversal failed; carries the scenario id AND the
///   offending anchor allegation id (attached via `.map_err`), mirroring how
///   `ScenarioRepositoryError` itself names the offending column.
/// - `UnknownStatus` — a scenario row's status string is outside the enum
///   vocabulary. The DB CHECK-constrains it, so this should be unreachable, but
///   it is surfaced (not silently defaulted) so a schema/enum drift fails loudly.
#[derive(Debug, thiserror::Error)]
pub enum ScenarioDashboardError {
    /// Listing a case's scenarios from Postgres failed. Names the case (the
    /// WHERE) alongside the wrapped store cause.
    #[error("listing scenarios for case '{case_slug}' failed: {source}")]
    Store {
        case_slug: String,
        #[source]
        source: PipelineRepoError,
    },

    /// Fetching one scenario by id from Postgres failed (the detail read).
    /// Names the scenario id (the WHERE) alongside the wrapped store cause.
    #[error("fetching scenario '{scenario_id}' failed: {source}")]
    Fetch {
        scenario_id: String,
        #[source]
        source: PipelineRepoError,
    },

    /// A graph traversal for one of a scenario's anchor allegations failed.
    /// Names BOTH the scenario and the offending anchor so an operator can
    /// locate the failing card when a case has many.
    ///
    /// `source` is boxed so this variant stays small: `ScenarioRepositoryError`
    /// embeds a `neo4rs::DeError`, which is large enough that carrying it inline
    /// would bloat every `Result<_, ScenarioDashboardError>` return value
    /// (`clippy::result_large_err`). `Box` keeps the common Ok-path cheap.
    #[error("scenario '{scenario_id}' anchor '{allegation_id}' repository read failed: {source}")]
    Repository {
        scenario_id: String,
        allegation_id: String,
        #[source]
        source: Box<ScenarioRepositoryError>,
    },

    /// A scenario row carried a status outside the `ScenarioStatus` vocabulary.
    #[error("scenario '{scenario_id}' has unrecognized status '{status}'")]
    UnknownStatus { scenario_id: String, status: String },
}

// ─────────────────────────────────────────────────────────────────────────────
// Assembler
// ─────────────────────────────────────────────────────────────────────────────

/// Composes the dashboard from the real scenarios + their live graph counts.
///
/// Holds both data-source handles, each a cheap `Clone`: `ScenarioRepository`
/// (Arc over the Neo4j pool) and `PgPool` (Arc over the Postgres pool). Build it
/// at the handler call site from `state.graph.clone()` and
/// `state.pipeline_pool.clone()`.
#[derive(Clone)]
pub struct ScenarioDashboardAssembler {
    repo: ScenarioRepository,
    pipeline_pool: PgPool,
}

impl ScenarioDashboardAssembler {
    /// Construct an assembler over the Neo4j repository and the pipeline pool.
    pub fn new(repo: ScenarioRepository, pipeline_pool: PgPool) -> Self {
        Self {
            repo,
            pipeline_pool,
        }
    }

    /// List the case's scenarios and shape them into the dashboard payload.
    ///
    /// The only I/O here: the Postgres list, then one graph read per anchor
    /// allegation per scenario. Everything after that is the pure shaping in
    /// `record_to_card` / `compute_metrics`, so the mapping is unit-testable
    /// without a live DB/graph.
    #[tracing::instrument(skip(self), fields(case_slug = %case_slug))]
    pub async fn assemble(
        &self,
        case_slug: &str,
    ) -> Result<TrialPrepDashboard, ScenarioDashboardError> {
        let records = list_scenarios_for_case(&self.pipeline_pool, case_slug)
            .await
            .map_err(|source| ScenarioDashboardError::Store {
                case_slug: case_slug.to_string(),
                source,
            })?;

        let mut cards = Vec::with_capacity(records.len());
        for record in &records {
            let instance_count = self.count_record_rebuts(record).await?;
            cards.push(record_to_card(record, instance_count)?);
        }

        Ok(TrialPrepDashboard {
            metrics: compute_metrics(&cards),
            // Alerts are derived signals not yet sourced (Chunk 2). Honest empty —
            // NOT the old hardcoded placeholder strings.
            alerts: Vec::new(),
            scenarios: cards,
        })
    }

    /// Assemble ONE scenario's detail: the Postgres record plus its anchor
    /// allegations' graph evidence shaped into a timeline.
    ///
    /// `Ok(None)` when no such scenario row exists — the handler maps that to a
    /// 404 (a legitimately-absent / deleted id, distinct from a read error).
    /// Responses / pattern / notes are empty/None for this chunk (not wired) —
    /// an honest partial, the same principle as the dashboard.
    #[tracing::instrument(skip(self), fields(scenario_id = %scenario_id, step = "assemble_detail"))]
    pub async fn assemble_detail(
        &self,
        scenario_id: Uuid,
    ) -> Result<Option<ScenarioDetail>, ScenarioDashboardError> {
        let record = get_scenario(&self.pipeline_pool, scenario_id)
            .await
            .map_err(|source| ScenarioDashboardError::Fetch {
                scenario_id: scenario_id.to_string(),
                source,
            })?;
        let Some(record) = record else {
            return Ok(None);
        };

        // Collect every anchor's evidence facts (0 anchors → empty timeline).
        let mut facts = Vec::new();
        for anchor in record.anchor_allegation_ids.as_deref().unwrap_or(&[]) {
            let evidence = self
                .repo
                .anchored_allegation_evidence(anchor, EvidencePolarity::Both)
                .await
                .map_err(|source| ScenarioDashboardError::Repository {
                    scenario_id: record.scenario_id.to_string(),
                    allegation_id: anchor.clone(),
                    source: Box::new(source),
                })?;
            facts.extend(evidence.facts);
        }

        Ok(Some(build_detail(&record, &facts)?))
    }

    /// Sum the live REBUTS count across all of a scenario's anchor allegations.
    ///
    /// A scenario may have 0, 1, or several anchors (`Option<Vec<String>>`). No
    /// anchors → 0 (a not-yet-anchored scenario is valid; it shows 0, not an
    /// error). Each anchor's graph read that fails is wrapped with that anchor's
    /// id so the failure names WHERE it occurred.
    async fn count_record_rebuts(
        &self,
        record: &ScenarioRecord,
    ) -> Result<u32, ScenarioDashboardError> {
        let anchors = record.anchor_allegation_ids.as_deref().unwrap_or(&[]);

        let mut total = 0u32;
        for anchor in anchors {
            let evidence = self
                .repo
                .anchored_allegation_evidence(anchor, EvidencePolarity::Both)
                .await
                .map_err(|source| ScenarioDashboardError::Repository {
                    scenario_id: record.scenario_id.to_string(),
                    allegation_id: anchor.clone(),
                    source: Box::new(source),
                })?;
            total += count_rebuts(&evidence);
        }
        Ok(total)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Pure shaping (no I/O — unit-tested)
// ─────────────────────────────────────────────────────────────────────────────

// CONST: the timeline `kind` for a graph-evidence turn. A deliberately NEUTRAL,
// honest label — NOT one of the litigation-narrative kinds
// (accusation/rebuttal/…). The REBUTS/CORROBORATES polarity is carried in the
// turn's `relationship_type`, so the UI shows the polarity as a pill and never
// fabricates an accusation/rebuttal meaning the graph does not assert. Matches
// the `"evidence"` member of the frontend `ExchangeTurnKind` union.
const EVIDENCE_TURN_KIND: &str = "evidence";

/// Pure: shape a scenario record + its collected graph facts into the detail
/// payload. `responses`/`pattern_summary`/`notes` are empty/None for this chunk
/// (their sources are not wired yet) — honest, not placeholder.
fn build_detail(
    record: &ScenarioRecord,
    facts: &[AnchoredEvidenceFact],
) -> Result<ScenarioDetail, ScenarioDashboardError> {
    Ok(ScenarioDetail {
        id: record.scenario_id.to_string(),
        attack: record.name.clone(),
        status: parse_status(&record.status, record.scenario_id)?,
        pattern_summary: None,
        timeline: facts.iter().map(fact_to_turn).collect(),
        responses: Vec::new(),
        notes: None,
    })
}

/// Pure: map one anchored graph fact onto a timeline turn.
///
/// Domain note: `kind` is the neutral `EVIDENCE_TURN_KIND`; the fact's
/// REBUTS/CORROBORATES polarity rides in `relationship_type` (lowercased), so the
/// screen labels the turn "evidence" with a "rebuts"/"corroborates" pill — never
/// a fabricated accusation/rebuttal narrative. Every graph fact is `grounded`
/// (it carries a citation); `date` is `null` (the facts have no date here, and
/// null sorts last); `repeated_after_rebuttal` is `false` (pattern analysis is
/// not wired).
fn fact_to_turn(fact: &AnchoredEvidenceFact) -> ExchangeTurn {
    ExchangeTurn {
        kind: EVIDENCE_TURN_KIND.to_string(),
        grounded: true,
        speaker: fact.stated_by.clone(),
        date: None,
        text: fact.verbatim_quote.clone().unwrap_or_default(),
        relationship_type: Some(fact.polarity.to_lowercase()),
        source_document: fact.document.clone(),
        // The fact carries `page_number` as a String; the turn's contract wants
        // `number | null`. Parse it; an un-parseable value (e.g. "iv", "12-13")
        // degrades to `None` rather than guessing.
        page_number: fact
            .page_number
            .as_deref()
            .and_then(|p| p.parse::<i64>().ok()),
        paragraph: fact.paragraph_number.clone(),
        repeated_after_rebuttal: false,
    }
}

/// Count the facts whose edge is a REBUTS.
///
/// Compares against `schema::REBUTS` rather than a re-spelled `"REBUTS"` literal
/// (Rule 16 — no magic strings; a rename in schema.rs flows here automatically).
fn count_rebuts(evidence: &AnchoredAllegationEvidenceResponse) -> u32 {
    evidence
        .facts
        .iter()
        .filter(|f| f.polarity == schema::REBUTS)
        .count() as u32
}

/// Parse a DB status string into the `ScenarioStatus` enum.
///
/// ## Rust Learning: reuse the enum's `Deserialize` as the single vocabulary source
///
/// Rather than re-spell `"draft"`/`"needs_evidence"`/`"ready"` here (which would
/// duplicate the vocabulary and risk drift — Rule 16), we deserialize the string
/// THROUGH `ScenarioStatus`'s own serde mapping. The enum is the one source of
/// truth; an out-of-vocabulary value (which the DB CHECK should prevent) becomes
/// a named `UnknownStatus` error rather than a silent default (Standing Rule 1).
fn parse_status(status: &str, scenario_id: Uuid) -> Result<ScenarioStatus, ScenarioDashboardError> {
    serde_json::from_value::<ScenarioStatus>(serde_json::Value::String(status.to_string())).map_err(
        |_| ScenarioDashboardError::UnknownStatus {
            scenario_id: scenario_id.to_string(),
            status: status.to_string(),
        },
    )
}

/// Map one scenario record + its computed REBUTS count into a dashboard card.
///
/// Several fields are honestly empty for this chunk because their sources are not
/// wired yet (documented inline) — they are NOT placeholders to be invented.
fn record_to_card(
    record: &ScenarioRecord,
    instance_count: u32,
) -> Result<ScenarioSummary, ScenarioDashboardError> {
    Ok(ScenarioSummary {
        // The frontend uses the id for the detail-page link.
        id: record.scenario_id.to_string(),
        attack: record.name.clone(),
        status: parse_status(&record.status, record.scenario_id)?,
        instance_count,
        // Responses are not wired until a later chunk — honest 0, not a placeholder.
        response_count: 0,
        // Speaker derivation is not sourced yet — honest empty.
        speakers: Vec::new(),
        // Pattern analysis is not wired — `None` = "not yet analysed" (pending),
        // the correct state (distinct from `Some(0)` = "analysed, none found").
        baseless_repeat_count: None,
    })
}

/// Compute the metrics band from the real card list (nothing hardcoded).
///
/// `// Why:` derived from the cards' own fields so the band stays consistent with
/// the list and stays forward-correct as the now-zero fields gain real sources.
/// `drafted_or_review` maps to the count of `Draft` cards — the closest real
/// equivalent now that the old `review` status is gone. `baseless_repeat_patterns`
/// is the count of cards with a positive baseless-repeat count (0 today, since
/// that signal is unwired). `no_response_yet` is the count of cards with no
/// responses (every card today, since responses are unwired).
fn compute_metrics(cards: &[ScenarioSummary]) -> TrialPrepMetrics {
    TrialPrepMetrics {
        scenarios: cards.len() as u32,
        ready: count_status(cards, ScenarioStatus::Ready),
        drafted_or_review: count_status(cards, ScenarioStatus::Draft),
        instances: cards.iter().map(|c| c.instance_count).sum(),
        baseless_repeat_patterns: cards
            .iter()
            .filter(|c| c.baseless_repeat_count.is_some_and(|n| n > 0))
            .count() as u32,
        no_response_yet: cards.iter().filter(|c| c.response_count == 0).count() as u32,
    }
}

/// Count cards in a given status.
fn count_status(cards: &[ScenarioSummary], status: ScenarioStatus) -> u32 {
    cards.iter().filter(|c| c.status == status).count() as u32
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests — pure shaping (the DB/graph-touching `assemble` is DEV-verified)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::scenario::AnchoredEvidenceFact;

    /// A scenario record with a given status / anchors (other fields fixed).
    fn record(status: &str, anchors: Option<Vec<String>>) -> ScenarioRecord {
        let ts = chrono::DateTime::from_timestamp(0, 0).expect("epoch is valid");
        ScenarioRecord {
            scenario_id: Uuid::nil(),
            name: "Marie is obstructive".to_string(),
            direction: "defense".to_string(),
            status: status.to_string(),
            case_slug: "awad_v_catholic_family_service".to_string(),
            feeds_count_id: None,
            anchor_allegation_ids: anchors,
            definition: serde_json::json!({}),
            created_at: ts,
            updated_at: ts,
        }
    }

    /// A dashboard card with a given status + counts (other fields the chunk-2
    /// honest defaults).
    fn card_with(status: ScenarioStatus, instance_count: u32) -> ScenarioSummary {
        ScenarioSummary {
            id: "card".to_string(),
            attack: "attack".to_string(),
            status,
            instance_count,
            response_count: 0,
            speakers: Vec::new(),
            baseless_repeat_count: None,
        }
    }

    fn evidence_fact(polarity: &str) -> AnchoredEvidenceFact {
        AnchoredEvidenceFact {
            evidence_id: format!("ev-{polarity}"),
            polarity: polarity.to_string(),
            allegation_id: "doc-x:allegation:abc".to_string(),
            paragraph_number: None,
            verbatim_quote: None,
            page_number: None,
            document: None,
            stated_by: None,
        }
    }

    #[test]
    fn count_rebuts_counts_only_rebuts() {
        let resp = AnchoredAllegationEvidenceResponse {
            allegation_id: "doc-x:allegation:abc".to_string(),
            facts: vec![
                evidence_fact(schema::REBUTS),
                evidence_fact(schema::REBUTS),
                evidence_fact(schema::CORROBORATES),
            ],
        };
        assert_eq!(count_rebuts(&resp), 2);
    }

    #[test]
    fn parse_status_maps_each_valid_token() {
        assert_eq!(
            parse_status("draft", Uuid::nil()).expect("ok"),
            ScenarioStatus::Draft
        );
        assert_eq!(
            parse_status("needs_evidence", Uuid::nil()).expect("ok"),
            ScenarioStatus::NeedsEvidence
        );
        assert_eq!(
            parse_status("ready", Uuid::nil()).expect("ok"),
            ScenarioStatus::Ready
        );
    }

    #[test]
    fn parse_status_rejects_unknown() {
        let err = parse_status("archived", Uuid::nil()).expect_err("unknown status must error");
        match &err {
            ScenarioDashboardError::UnknownStatus { status, .. } => {
                assert_eq!(status, "archived");
            }
            other => panic!("expected UnknownStatus, got {other:?}"),
        }
        // The Display message must name both the offending status and the row id
        // (Standing Rule 1 — observable WHERE context).
        let msg = err.to_string();
        assert!(
            msg.contains("archived"),
            "message must name the status: {msg}"
        );
        assert!(
            msg.contains(&Uuid::nil().to_string()),
            "message must name the scenario id: {msg}"
        );
    }

    #[test]
    fn store_error_display_wraps_the_cause() {
        // The Store variant must surface the underlying Postgres cause through
        // its Display, prefixed so an operator sees which step failed.
        let err = ScenarioDashboardError::Store {
            case_slug: "awad_v_catholic_family_service".to_string(),
            source: PipelineRepoError::Database("boom".to_string()),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("awad_v_catholic_family_service"),
            "message must name the case (the WHERE): {msg}"
        );
        assert!(msg.contains("boom"), "cause must be preserved: {msg}");
    }

    #[test]
    fn record_to_card_carries_count_and_honest_defaults() {
        let card = record_to_card(&record("ready", None), 7).expect("maps");
        assert_eq!(card.instance_count, 7);
        assert_eq!(card.status, ScenarioStatus::Ready);
        assert_eq!(card.attack, "Marie is obstructive");
        assert_eq!(card.id, "00000000-0000-0000-0000-000000000000");
        // Unwired fields are honestly empty/zero/None.
        assert_eq!(card.response_count, 0);
        assert!(card.speakers.is_empty());
        assert_eq!(card.baseless_repeat_count, None);
    }

    #[test]
    fn record_to_card_propagates_unknown_status() {
        assert!(record_to_card(&record("bogus", None), 0).is_err());
    }

    #[test]
    fn compute_metrics_over_mixed_cards() {
        let cards = vec![
            card_with(ScenarioStatus::Ready, 4),
            card_with(ScenarioStatus::Draft, 2),
            card_with(ScenarioStatus::NeedsEvidence, 0),
        ];
        let m = compute_metrics(&cards);
        assert_eq!(m.scenarios, 3);
        assert_eq!(m.ready, 1);
        assert_eq!(m.drafted_or_review, 1); // one Draft
        assert_eq!(m.instances, 6); // 4 + 2 + 0
        assert_eq!(m.baseless_repeat_patterns, 0); // unwired
        assert_eq!(m.no_response_yet, 3); // none have responses yet
    }

    #[test]
    fn compute_metrics_empty_list_is_all_zero() {
        let m = compute_metrics(&[]);
        assert_eq!(m.scenarios, 0);
        assert_eq!(m.ready, 0);
        assert_eq!(m.drafted_or_review, 0);
        assert_eq!(m.instances, 0);
        assert_eq!(m.baseless_repeat_patterns, 0);
        assert_eq!(m.no_response_yet, 0);
    }

    #[test]
    fn compute_metrics_counts_only_positive_baseless_repeat() {
        let mut positive = card_with(ScenarioStatus::Ready, 1);
        positive.baseless_repeat_count = Some(2);
        let mut analysed_none_found = card_with(ScenarioStatus::Draft, 0);
        analysed_none_found.baseless_repeat_count = Some(0);
        let pending = card_with(ScenarioStatus::Draft, 0); // baseless_repeat_count: None

        let m = compute_metrics(&[positive, analysed_none_found, pending]);
        // Only `Some(n > 0)` is a pattern: `Some(0)` ("analysed, none found") and
        // `None` ("pending") must NOT increment the signal.
        assert_eq!(m.baseless_repeat_patterns, 1);
    }

    /// A fully-populated anchored fact (every descriptive column present unless
    /// `page` is None), for exercising the fact → turn mapping.
    fn full_fact(polarity: &str, page: Option<&str>) -> AnchoredEvidenceFact {
        AnchoredEvidenceFact {
            evidence_id: "ev-1".to_string(),
            polarity: polarity.to_string(),
            allegation_id: "doc-x:allegation:abc".to_string(),
            paragraph_number: Some("¶54".to_string()),
            verbatim_quote: Some("the quote".to_string()),
            page_number: page.map(|s| s.to_string()),
            document: Some("doc-x".to_string()),
            stated_by: Some("George Phillips".to_string()),
        }
    }

    #[test]
    fn fact_to_turn_maps_a_rebuts_fact() {
        let turn = fact_to_turn(&full_fact(schema::REBUTS, Some("54")));
        // kind is the neutral "evidence"; polarity rides in relationship_type.
        assert_eq!(turn.kind, "evidence");
        assert!(turn.grounded);
        assert_eq!(turn.relationship_type.as_deref(), Some("rebuts"));
        assert_eq!(turn.speaker.as_deref(), Some("George Phillips"));
        assert_eq!(turn.text, "the quote");
        assert_eq!(turn.page_number, Some(54));
        assert_eq!(turn.paragraph.as_deref(), Some("¶54"));
        assert_eq!(turn.source_document.as_deref(), Some("doc-x"));
        assert_eq!(turn.date, None);
        assert!(!turn.repeated_after_rebuttal);
    }

    #[test]
    fn fact_to_turn_lowercases_corroborates_polarity() {
        let turn = fact_to_turn(&full_fact(schema::CORROBORATES, Some("3")));
        assert_eq!(turn.relationship_type.as_deref(), Some("corroborates"));
    }

    #[test]
    fn fact_to_turn_unparseable_page_is_none() {
        // A non-numeric page string degrades to None rather than guessing.
        let turn = fact_to_turn(&full_fact(schema::REBUTS, Some("iv")));
        assert_eq!(turn.page_number, None);
    }

    #[test]
    fn fact_to_turn_missing_quote_is_empty_string() {
        let mut f = full_fact(schema::REBUTS, None);
        f.verbatim_quote = None;
        let turn = fact_to_turn(&f);
        assert_eq!(turn.text, "");
        assert_eq!(turn.page_number, None);
    }

    #[test]
    fn build_detail_shapes_record_and_facts() {
        let facts = vec![
            full_fact(schema::REBUTS, Some("1")),
            full_fact(schema::CORROBORATES, Some("2")),
        ];
        let detail = build_detail(&record("ready", None), &facts).expect("builds");
        assert_eq!(detail.attack, "Marie is obstructive");
        assert_eq!(detail.status, ScenarioStatus::Ready);
        assert_eq!(detail.id, "00000000-0000-0000-0000-000000000000");
        assert_eq!(detail.timeline.len(), 2); // one turn per fact
                                              // Unwired sections are honestly empty/None.
        assert!(detail.responses.is_empty());
        assert_eq!(detail.pattern_summary, None);
        assert_eq!(detail.notes, None);
    }

    #[test]
    fn build_detail_propagates_unknown_status() {
        assert!(build_detail(&record("bogus", None), &[]).is_err());
    }
}
