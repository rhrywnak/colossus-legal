// =============================================================================
// backend/src/services/scenario_dashboard.rs
// =============================================================================
//
// War Room dashboard assembler — composes the Trial Prep dashboard payload from
// the REAL scenarios in Postgres. The per-scenario DETAIL timeline additionally
// reads the graph; the dashboard list does not (2026-08-07 — see below).
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
//     This is now the DASHBOARD's only source.
//   - Neo4j (via `ScenarioRepository`): the anchor allegations' evidence, for
//     ONE scenario's detail timeline (`assemble_detail`).
//
// Until 2026-08-07 the dashboard also read the graph, once per anchor allegation
// per scenario, to compute a REBUTS total for each card's `instance_count`. That
// metric was removed (Roman's ruling — see `TrialPrepMetrics`), and the reads
// went with it: listing scenarios no longer touches Neo4j at all.
//
// With zero scenarios in the table the dashboard is honestly empty (no cards,
// zeroed metrics, no alerts). Cards appear as scenarios are authored.
//
// Testability split: the per-record shaping (status-string → enum, record →
// card, metrics) is pure and unit-tested without a DB/graph. Only the two
// `assemble*` methods touch I/O; those are DEV-verified, the same convention the
// `ScenarioRepository` query methods follow.
// =============================================================================

use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::scenario_code::scenario_code;
use crate::dto::scenario::AnchoredEvidenceFact;
use crate::dto::scenario_authoring_wording::ScenarioCreateWordingDto;
use crate::dto::trial_prep::{
    ExchangeTurn, ScenarioDetail, ScenarioStatus, ScenarioSummary, TrialPrepDashboard,
    TrialPrepMetrics,
};
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

/// Assembles the dashboard LIST from Postgres scenarios, and one scenario's
/// DETAIL timeline from Postgres plus the graph.
///
/// The distinction is load-bearing since 2026-08-07: `assemble` performs no
/// graph reads at all, so a reader here for latency, caching or connection-pool
/// reasons should not conclude from the held `ScenarioRepository` that listing
/// scenarios touches Neo4j. It does not — the repository is here for
/// `assemble_detail`.
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
    ///
    /// ## Why the wording arrives as a PARAMETER and not from a held handle
    ///
    /// The assembler owns two data-source handles and no `AppState` — that is
    /// what keeps it constructible in a test from a repo and a pool. Giving it a
    /// `SettingsHandle` to read the create form's words from would drag the whole
    /// configuration store into every construction site. The caller already holds
    /// a snapshot (`state.settings.current()`), so it passes the block it needs;
    /// the assembler stays a shaper of data it was handed.
    #[tracing::instrument(skip(self, create_wording), fields(case_slug = %case_slug))]
    pub async fn assemble(
        &self,
        case_slug: &str,
        create_wording: ScenarioCreateWordingDto,
    ) -> Result<TrialPrepDashboard, ScenarioDashboardError> {
        let records = list_scenarios_for_case(&self.pipeline_pool, case_slug)
            .await
            .map_err(|source| ScenarioDashboardError::Store {
                case_slug: case_slug.to_string(),
                source,
            })?;

        // Pure shaping, no I/O. Until 2026-08-07 this loop ran one graph read per
        // anchor allegation per scenario to compute a REBUTS count for the card —
        // the dashboard's only per-scenario graph traffic. Dropping the metric
        // dropped the reads with it.
        let cards = records
            .iter()
            .map(record_to_card)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(TrialPrepDashboard {
            metrics: compute_metrics(&cards),
            // Alerts are derived signals not yet sourced (Chunk 2). Honest empty —
            // NOT the old hardcoded placeholder strings.
            alerts: Vec::new(),
            scenarios: cards,
            create_wording,
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
        code: scenario_code(record.code_ordinal),
        attack: record.name.clone(),
        // Verbatim: an unrecognised direction is the screen's problem to SHOW,
        // not this function's to normalise away.
        direction: record.direction.clone(),
        status: parse_status(&record.status, record.scenario_id)?,
        pattern_summary: None,
        timeline: facts.iter().map(fact_to_turn).collect(),
        responses: Vec::new(),
        notes: None,
        // Flatten the row's `Option<Vec<String>>` to `[]` when absent — the wire
        // never sees null for a list it only iterates (mirrors `ScenarioDto`). The
        // define form pre-fills its allegation picker from this.
        anchor_allegation_ids: record.anchor_allegation_ids.clone().unwrap_or_default(),
        // Carry the authored definition through opaquely — `record` is already in
        // hand (no extra fetch), and the column is an opaque `serde_json::Value`,
        // so no transform applies (contrast `attack`/`status`, which reshape).
        definition: record.definition.clone(),
    })
}

/// Pure: map one anchored graph fact onto a timeline turn.
///
/// Domain note: `kind` is the neutral `EVIDENCE_TURN_KIND`; the fact's
/// REBUTS/CORROBORATES polarity rides in `relationship_type` (lowercased), so the
/// screen labels the turn "evidence" with a "rebuts"/"corroborates" pill — never
/// a fabricated accusation/rebuttal narrative. `date` is `null` (the facts have
/// no date here, and null sorts last); `repeated_after_rebuttal` is `false`
/// (pattern analysis is not wired).
///
/// ## `grounded` was hardcoded `true` until 2026-08-01 (task 1.2 Q3)
///
/// The old comment here asserted that every graph fact is grounded "because it
/// carries a citation". That was false: the ingest path records
/// `grounding_status` per node, and `not_found` nodes exist, as do nodes never
/// checked. A field that always says `true` is not a measurement — it is the same
/// class of displayed falsehood the 2026-07-27 honesty batch removed elsewhere,
/// and it would have contradicted the real grounding state task 1.2 now serves on
/// the candidate card.
///
/// The turn's contract is a `bool`, so the three-state truth collapses to two
/// here: `exact` and `normalized` are grounded, everything else is not. That is
/// lossy but not dishonest — an unchecked quote genuinely is not grounded yet.
/// The full state, with its own label, rides the card payload where the
/// distinction matters (§7.7).
fn fact_to_turn(fact: &AnchoredEvidenceFact) -> ExchangeTurn {
    ExchangeTurn {
        kind: EVIDENCE_TURN_KIND.to_string(),
        grounded: matches!(
            fact.grounding_status.as_deref(),
            Some("exact") | Some("normalized")
        ),
        speaker: fact.stated_by.clone(),
        date: None,
        text: fact.verbatim_quote.clone().unwrap_or_default(),
        relationship_type: Some(fact.polarity.to_lowercase()),
        source_document: fact.document.clone(),
        // best-effort: the fact carries `page_number` as a String; the turn's
        // contract wants `number | null`. An un-parseable value (e.g. "iv",
        // "12-13") degrades to `None` rather than guessing a number — `None` is
        // a first-class state in the contract ("no page locator"), rendered as a
        // turn with no page link, so nothing is silently lost.
        page_number: fact
            .page_number
            .as_deref()
            // best-effort: an un-parseable page degrades to `None` (see above).
            .and_then(|p| p.parse::<i64>().ok()),
        paragraph: fact.paragraph_number.clone(),
        repeated_after_rebuttal: false,
    }
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

/// Map one scenario record into a dashboard card.
///
/// ## Why this no longer takes a count (2026-08-07)
///
/// It used to take an `instance_count` — a live REBUTS total the caller computed
/// with one graph read per anchor allegation, per scenario, on every dashboard
/// load. Roman ruled the number earned nothing: nobody could act on it, and its
/// name collided with task 2.11's unrelated "accusation instances".
///
/// Removing it took `response_count` and `speakers` with it. Both were honest
/// stubs, and both existed only to fill out the card line that displayed the
/// count. With the line gone they had no reader, and a served field nobody reads
/// is a field the next person has to work out the status of.
fn record_to_card(record: &ScenarioRecord) -> Result<ScenarioSummary, ScenarioDashboardError> {
    Ok(ScenarioSummary {
        // The frontend uses the id for the detail-page link.
        id: record.scenario_id.to_string(),
        code: scenario_code(record.code_ordinal),
        attack: record.name.clone(),
        status: parse_status(&record.status, record.scenario_id)?,
        // Pattern analysis is not wired — `None` = "not yet analysed" (pending),
        // the correct state (distinct from `Some(0)` = "analysed, none found").
        baseless_repeat_count: None,
    })
}

/// Compute the metrics band from the real card list (nothing hardcoded).
///
/// `// Why:` derived from the cards' own fields so the band stays consistent with
/// the list. `drafted_or_review` maps to the count of `Draft` cards — the
/// closest real equivalent now that the old `review` status is gone.
///
/// Every figure here is derived from a field with a real source. The two that
/// were not — `baseless_repeat_patterns` and `no_response_yet` — were removed on
/// 2026-07-27; see the note on [`TrialPrepMetrics`]. A band figure computed from
/// a stub is not forward-correct, it is a constant wearing a measurement's
/// clothes, and it reads identically to a real result.
///
/// `instances` was removed on 2026-08-07 for the opposite reason: it was a real
/// measurement of something nobody could act on. Three figures remain, and each
/// answers a question a human actually asks of this page.
fn compute_metrics(cards: &[ScenarioSummary]) -> TrialPrepMetrics {
    TrialPrepMetrics {
        scenarios: cards.len() as u32,
        ready: count_status(cards, ScenarioStatus::Ready),
        drafted_or_review: count_status(cards, ScenarioStatus::Draft),
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
    // Only the timeline tests still name graph relationship types; production
    // code here stopped touching them when the REBUTS count was removed, so the
    // import lives with its remaining users rather than at file scope.
    use crate::dto::scenario::AnchoredEvidenceFact;
    use crate::neo4j::schema;

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
            // Non-empty so the `build_detail` carry-through assertion is
            // meaningful (an authored definition, not the un-authored `{}`).
            // `record_to_card` ignores this field, so bumping it is safe.
            definition: serde_json::json!({ "attack_text": "Marie is obstructive", "schema_v": 1 }),
            created_at: ts,
            updated_at: ts,
            // Every scenario carries a code after the 2026-08-01 backfill;
            // a fixture without one would be a state the column forbids.
            code_ordinal: 1,
            // Unframed: a scenario is created before anyone writes its theme, and
            // `None` is the honest value rather than invented prose.
            theme_statement: None,
            motivation: None,
            // Task 2.11: nobody has written the plain-words accusation for this
            // fixture, which is the honest default — the page renders its gap.
            accusation_text: None,
            accusation_text_authored_by: None,
            accusation_text_authored_at: None,
            theme_authored_by: None,
            theme_authored_at: None,
        }
    }

    /// A dashboard card with a given status (other fields the honest defaults).
    fn card_with(status: ScenarioStatus) -> ScenarioSummary {
        ScenarioSummary {
            code: "S-1".to_string(),
            id: "card".to_string(),
            attack: "attack".to_string(),
            status,
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
            // Unchecked: this minimal fixture exercises the polarity mapping, not
            // grounding, and `None` is the honest value for a node with no
            // recorded state.
            grounding_status: None,
        }
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
    fn record_to_card_carries_the_record_and_honest_defaults() {
        let card = record_to_card(&record("ready", None)).expect("maps");
        assert_eq!(card.status, ScenarioStatus::Ready);
        assert_eq!(card.attack, "Marie is obstructive");
        assert_eq!(card.id, "00000000-0000-0000-0000-000000000000");
        // Pattern analysis is unwired: `None` = not yet analysed, which is a
        // different statement from `Some(0)` = analysed, none found.
        assert_eq!(card.baseless_repeat_count, None);
    }

    #[test]
    fn record_to_card_propagates_unknown_status() {
        assert!(record_to_card(&record("bogus", None)).is_err());
    }

    #[test]
    fn compute_metrics_over_mixed_cards() {
        let cards = vec![
            card_with(ScenarioStatus::Ready),
            card_with(ScenarioStatus::Draft),
            card_with(ScenarioStatus::NeedsEvidence),
        ];
        let m = compute_metrics(&cards);
        assert_eq!(m.scenarios, 3);
        assert_eq!(m.ready, 1);
        assert_eq!(m.drafted_or_review, 1); // one Draft
    }

    #[test]
    fn compute_metrics_empty_list_is_all_zero() {
        let m = compute_metrics(&[]);
        assert_eq!(m.scenarios, 0);
        assert_eq!(m.ready, 0);
        assert_eq!(m.drafted_or_review, 0);
    }

    /// The band carries ONLY figures a human can act on.
    ///
    /// Pinning the field set is the guard against both ways this band has gone
    /// wrong: re-deriving a metric from a card stub (how `baseless_repeat_patterns`
    /// and `no_response_yet` came to read as measurements on 2026-07-27), and
    /// re-introducing `instances` — a real graph count of something nobody could
    /// act on, whose name collided with 2.11's "accusation instances"
    /// (2026-08-07). A fourth key appearing here fails this test by name.
    #[test]
    fn metrics_band_exposes_no_figure_nobody_can_act_on() {
        let m = compute_metrics(&[card_with(ScenarioStatus::Ready)]);
        let value = serde_json::to_value(m).expect("metrics serialize");
        // `serde_json::Value` keys are a BTreeMap, so compare as a sorted set —
        // the assertion is about WHICH figures the band exposes, not their order.
        let mut keys: Vec<&str> = value
            .as_object()
            .expect("object body")
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(keys, vec!["drafted_or_review", "ready", "scenarios"]);
    }

    /// The card field itself keeps its three states — `Some(n>0)` (a pattern),
    /// `Some(0)` (analysed, none found) and `None` (not yet analysed). The band
    /// no longer collapses them into a count, but the distinction on the card is
    /// the thing pattern analysis will populate, so it is pinned here.
    #[test]
    fn card_baseless_repeat_keeps_its_three_states() {
        let mut positive = card_with(ScenarioStatus::Ready);
        positive.baseless_repeat_count = Some(2);
        let mut analysed_none_found = card_with(ScenarioStatus::Draft);
        analysed_none_found.baseless_repeat_count = Some(0);
        let pending = card_with(ScenarioStatus::Draft);

        assert_eq!(positive.baseless_repeat_count, Some(2));
        assert_eq!(analysed_none_found.baseless_repeat_count, Some(0));
        assert_eq!(pending.baseless_repeat_count, None);
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
            grounding_status: Some("exact".to_string()),
        }
    }

    #[test]
    fn fact_to_turn_maps_a_rebuts_fact() {
        let turn = fact_to_turn(&full_fact(schema::REBUTS, Some("54")));
        // kind is the neutral "evidence"; polarity rides in relationship_type.
        assert_eq!(turn.kind, "evidence");
        // `exact` on the fixture → genuinely grounded. This assertion used to
        // pass against a hardcoded `true` and therefore proved nothing.
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

    /// `grounded` reports the node's real state, not a constant.
    ///
    /// Until 2026-08-01 this field was hardcoded `true`, so a fact whose quote was
    /// never found in its source still rendered as grounded. The two assertions
    /// below are the ones a constant cannot satisfy.
    #[test]
    fn grounded_reflects_the_nodes_real_state() {
        let mut fact = full_fact(schema::REBUTS, Some("54"));

        fact.grounding_status = Some("normalized".to_string());
        assert!(
            fact_to_turn(&fact).grounded,
            "a normalized match IS grounded"
        );

        fact.grounding_status = Some("not_found".to_string());
        assert!(
            !fact_to_turn(&fact).grounded,
            "a quote never found in its source must not render as grounded"
        );

        fact.grounding_status = None;
        assert!(
            !fact_to_turn(&fact).grounded,
            "an unchecked quote is not grounded YET; claiming otherwise is the \
             falsehood this fix removed"
        );
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
        let rec = record("ready", None);
        let detail = build_detail(&rec, &facts).expect("builds");
        assert_eq!(detail.attack, "Marie is obstructive");
        assert_eq!(detail.status, ScenarioStatus::Ready);
        assert_eq!(detail.id, "00000000-0000-0000-0000-000000000000");
        assert_eq!(detail.timeline.len(), 2); // one turn per fact
                                              // Unwired sections are honestly empty/None.
        assert!(detail.responses.is_empty());
        assert_eq!(detail.pattern_summary, None);
        assert_eq!(detail.notes, None);
        // A `None` anchor column flattens to `[]` on the wire (never null).
        assert!(detail.anchor_allegation_ids.is_empty());
        // Carry-through: the authored definition rides through unchanged (opaque
        // Value, no transform) — the B2a contract at the service layer.
        assert_eq!(detail.definition, rec.definition);
    }

    #[test]
    fn build_detail_flattens_present_anchor_allegation_ids() {
        // With anchors present they must ride through to the detail payload so the
        // define form (D1) can pre-fill its allegation picker.
        let rec = record("draft", Some(vec!["54".to_string(), "55".to_string()]));
        let detail = build_detail(&rec, &[]).expect("builds");
        assert_eq!(
            detail.anchor_allegation_ids,
            vec!["54".to_string(), "55".to_string()]
        );
    }

    #[test]
    fn build_detail_propagates_unknown_status() {
        assert!(build_detail(&record("bogus", None), &[]).is_err());
    }
}
