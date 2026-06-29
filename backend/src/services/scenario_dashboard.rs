// =============================================================================
// backend/src/services/scenario_dashboard.rs
// =============================================================================
//
// War Room dashboard assembler — the thin-vertical-slice composer for the Trial
// Prep dashboard payload.
//
// This is DELIBERATELY NOT the existing `ScenarioPageAssembler` (services/
// scenario_page.rs). That one composes a wielder/anchor *facts* page
// (rebuttal_facts / contradictions / related_allegations); this one composes the
// *dashboard* payload (metrics band · alerts strip · scenario cards). Different
// payload, different consumer — kept as a separate, distinctly named assembler so
// the two are never confused.
//
// Slice scope (see CC_WARROOM_WIRING_STEP2_BUILD): exactly ONE number is live.
// `marie-obstructive`'s `instance_count` is the live REBUTS count for the ¶54
// anchor allegation; every other field is reproduced placeholder baseline. The
// graph read is the ONLY live input; everything else is honest scaffolding,
// isolated to the two clearly-marked blocks below so it is trivially found and
// removed when the real scenario-definition storage lands (Phase 1).
// =============================================================================

use crate::dto::scenario::AnchoredAllegationEvidenceResponse;
use crate::dto::trial_prep::{
    ScenarioStatus, ScenarioSummary, TrialPrepAlert, TrialPrepDashboard, TrialPrepMetrics,
};
use crate::neo4j::schema;
use crate::repositories::scenario_repository::{
    EvidencePolarity, ScenarioRepository, ScenarioRepositoryError,
};

// ─────────────────────────────────────────────────────────────────────────────
// SCAFFOLDING — temporary scenario→anchor mapping (REMOVE when scenario-
// definition storage lands). This stands in for the unbuilt Postgres scenario
// `definition` rows + slug→anchor-allegation resolution. Do NOT scatter this or
// pretend it is derived: the slice hardcodes the ONE known-good mapping.
// ─────────────────────────────────────────────────────────────────────────────

/// The ¶54 anchor allegation for the `marie-obstructive` scenario card.
///
/// `// Why:` this id is the known-good verification anchor (6 CORROBORATES + 4
/// REBUTS live in the graph). When the real scenario-definition storage exists,
/// the card's anchor will be resolved from its scenario row, and this constant —
/// together with the whole SCAFFOLDING block — is deleted.
// CONST: temporary scaffolding anchor standing in for the unbuilt scenario-
// definition Postgres row (colossus_legal_v2). NOT env-configurable on purpose —
// it is a graph node identity that must match the seeded graph exactly (a
// mismatch silently yields 0 facts), so it is fixed at data-model time, not an
// operator knob. Deleted with the SCAFFOLDING block when Phase-1 storage lands.
const MARIE_OBSTRUCTIVE_ANCHOR_ALLEGATION_ID: &str =
    "doc-awad-v-catholic-family-complaint-11-1-13:allegation:cd24fccb";

// ─────────────────────────────────────────────────────────────────────────────
// Error
// ─────────────────────────────────────────────────────────────────────────────

/// Error surface for dashboard assembly.
///
/// ## Rust Learning: a context-carrying variant instead of a bare `#[from]`
///
/// A plain `#[from] ScenarioRepositoryError` would let `?` convert
/// automatically, but it would drop the WHERE — which anchor allegation was
/// being read when the failure occurred. So the `Repository` variant carries the
/// `allegation_id` alongside the `#[source]` repository error, and `assemble`
/// attaches it with `.map_err`. This mirrors how `ScenarioRepositoryError` itself
/// names the offending column, and keeps the failure observable with full
/// context via `{}` (Standing Rule 1). It stays additive: when Phase-1 Postgres
/// sections arrive they add their own variant without reshaping this one.
#[derive(Debug, thiserror::Error)]
pub enum ScenarioDashboardError {
    /// The graph read underneath the dashboard failed. Names the anchor
    /// allegation being queried (the WHERE in the traversal) and carries the
    /// repository error verbatim so the offending column / Neo4j cause is
    /// preserved.
    #[error("scenario repository failed for allegation '{allegation_id}': {source}")]
    Repository {
        allegation_id: String,
        #[source]
        source: ScenarioRepositoryError,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// Assembler
// ─────────────────────────────────────────────────────────────────────────────

/// Composes the War Room dashboard payload from one live graph count plus the
/// slice baseline.
///
/// Holds a `ScenarioRepository` (a cheap `Clone` over the Neo4j connection
/// pool). Build it from `state.graph.clone()` at the handler call site, the same
/// way other handlers construct repositories (precedent: `claims.rs`).
#[derive(Clone)]
pub struct ScenarioDashboardAssembler {
    repo: ScenarioRepository,
}

impl ScenarioDashboardAssembler {
    /// Construct an assembler over a shared Neo4j connection.
    pub fn new(repo: ScenarioRepository) -> Self {
        Self { repo }
    }

    /// Read the ¶54 anchor's evidence and shape the dashboard.
    ///
    /// ## Rust Learning: thin async method delegating to a pure shaper
    ///
    /// The only `.await` here is the graph read. All shaping — counting REBUTS
    /// and overlaying the baseline — lives in the pure `assemble_dashboard`
    /// below, which takes data and returns data with no I/O. That split is what
    /// lets the counting logic be unit-tested against a fixture with no live
    /// Neo4j (build instruction test #8). A repository failure is wrapped with
    /// the anchor id into `ScenarioDashboardError::Repository` — never swallowed.
    pub async fn assemble(&self) -> Result<TrialPrepDashboard, ScenarioDashboardError> {
        // Fetch BOTH polarities so the partition is explicit at the read, even
        // though only the REBUTS side drives the slice (build instruction §2).
        let evidence = self
            .repo
            .anchored_allegation_evidence(
                MARIE_OBSTRUCTIVE_ANCHOR_ALLEGATION_ID,
                EvidencePolarity::Both,
            )
            .await
            .map_err(|source| ScenarioDashboardError::Repository {
                allegation_id: MARIE_OBSTRUCTIVE_ANCHOR_ALLEGATION_ID.to_string(),
                source,
            })?;

        Ok(assemble_dashboard(&evidence))
    }
}

/// Count the facts whose edge is a REBUTS (the live signal for the slice).
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

/// Pure shaper: produce the full dashboard from the anchor's evidence.
///
/// `// Why:` separated from `assemble` (which does the I/O) so it is a pure
/// `data -> data` function — same input always yields the same payload, no live
/// graph required to test it. Only `marie-obstructive.instance_count` is derived
/// (from the REBUTS count); every other field is the reproduced baseline.
///
/// Domain note: `instance_count` ← REBUTS count is SLICE SCAFFOLDING SEMANTICS,
/// not the field's final meaning. `instance_count` will eventually be the number
/// of *accusation instances* of the attack; here it carries the REBUTS count
/// purely to prove the value is graph-derived (the card flips 6 → 4 in DEV).
fn assemble_dashboard(evidence: &AnchoredAllegationEvidenceResponse) -> TrialPrepDashboard {
    let marie_obstructive_instance_count = count_rebuts(evidence);

    TrialPrepDashboard {
        metrics: baseline_metrics(),
        alerts: baseline_alerts(),
        scenarios: baseline_scenarios(marie_obstructive_instance_count),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SLICE BASELINE — the placeholder payload reproduced in Rust, in ONE place
// (REMOVE when real assembly lands). These values mirror the frontend `DASHBOARD`
// constant in `frontend/src/pages/trialPrepPlaceholder.ts`. Only the
// `marie-obstructive` card's `instance_count` is overridden with the live count;
// every other value here is honest placeholder scaffolding.
// ─────────────────────────────────────────────────────────────────────────────

/// The metrics band baseline (mirrors `DASHBOARD.metrics`).
fn baseline_metrics() -> TrialPrepMetrics {
    TrialPrepMetrics {
        scenarios: 5,
        ready: 1,
        drafted_or_review: 3,
        instances: 16,
        baseless_repeat_patterns: 1,
        no_response_yet: 1,
    }
}

/// The alerts-strip baseline (mirrors `DASHBOARD.alerts`).
fn baseline_alerts() -> Vec<TrialPrepAlert> {
    vec![
        TrialPrepAlert {
            message: "6 new instances of “Marie is obstructive” since last review".to_string(),
        },
        TrialPrepAlert {
            message: "Pattern analysis pending for “Selective sanctions”".to_string(),
        },
    ]
}

/// The five scenario cards (mirrors `DASHBOARD.scenarios`).
///
/// `marie_obstructive_instance_count` is the ONE live value, threaded in from the
/// graph; all other fields — including `marie-obstructive`'s own `response_count`,
/// `baseless_repeat_count`, `speakers`, and `status` — keep their baseline values.
fn baseline_scenarios(marie_obstructive_instance_count: u32) -> Vec<ScenarioSummary> {
    vec![
        card(
            "too-many-attorneys",
            "Marie hired too many attorneys",
            ScenarioStatus::Review,
            4,
            2,
            &["George Phillips", "CFS"],
            Some(0),
        ),
        card(
            "fifty-thousand",
            "The $50,000 was a gift",
            ScenarioStatus::Ready,
            3,
            2,
            &["George Phillips"],
            Some(0),
        ),
        card(
            "marie-obstructive",
            "Marie is obstructive and uncooperative",
            ScenarioStatus::Review,
            marie_obstructive_instance_count, // ← the ONE live, graph-derived value
            1,
            &["CFS", "George Phillips"],
            Some(3),
        ),
        card(
            "selective-sanctions",
            "Sanctions were never selectively pursued",
            ScenarioStatus::Drafted,
            2,
            1,
            &["CFS"],
            None, // pattern analysis pending → null on the wire
        ),
        card(
            "bias-who-gained",
            "Bias — who gained from the decisions?",
            ScenarioStatus::NeedsResponse,
            1,
            0,
            &["George Phillips"],
            Some(0),
        ),
    ]
}

/// Build one `ScenarioSummary`, mapping the borrowed speaker labels to owned
/// `String`s. A small constructor so each baseline card stays a single readable
/// call (keeps `baseline_scenarios` under the function-length limit).
fn card(
    id: &str,
    attack: &str,
    status: ScenarioStatus,
    instance_count: u32,
    response_count: u32,
    speakers: &[&str],
    baseless_repeat_count: Option<u32>,
) -> ScenarioSummary {
    ScenarioSummary {
        id: id.to_string(),
        attack: attack.to_string(),
        status,
        instance_count,
        response_count,
        speakers: speakers.iter().map(|s| s.to_string()).collect(),
        baseless_repeat_count,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests — the pure shaper (the async `assemble` needs a live graph and is
// covered by DEV verification, like the 0.3 query methods)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::scenario::AnchoredEvidenceFact;

    /// Build a fact carrying only the polarity that matters for counting; the
    /// descriptive columns are irrelevant to `count_rebuts` so they stay `None`.
    fn fact(polarity: &str) -> AnchoredEvidenceFact {
        AnchoredEvidenceFact {
            evidence_id: format!("ev-{polarity}"),
            polarity: polarity.to_string(),
            allegation_id: MARIE_OBSTRUCTIVE_ANCHOR_ALLEGATION_ID.to_string(),
            paragraph_number: None,
            verbatim_quote: None,
            page_number: None,
            document: None,
            stated_by: None,
        }
    }

    /// A response mixing the two polarities — the ¶54 shape (4 REBUTS, 6
    /// CORROBORATES). Built from the `schema::` constants so it cannot drift from
    /// what `count_rebuts` filters on.
    fn mixed_evidence() -> AnchoredAllegationEvidenceResponse {
        let mut facts: Vec<AnchoredEvidenceFact> = Vec::new();
        for _ in 0..4 {
            facts.push(fact(schema::REBUTS));
        }
        for _ in 0..6 {
            facts.push(fact(schema::CORROBORATES));
        }
        AnchoredAllegationEvidenceResponse {
            allegation_id: MARIE_OBSTRUCTIVE_ANCHOR_ALLEGATION_ID.to_string(),
            facts,
        }
    }

    /// Find a card by id (panics on the test-data invariant that it exists).
    fn find<'a>(d: &'a TrialPrepDashboard, id: &str) -> &'a ScenarioSummary {
        d.scenarios
            .iter()
            .find(|s| s.id == id)
            .expect("baseline must contain the card")
    }

    /// The slice's core assertion: `marie-obstructive.instance_count` equals the
    /// REBUTS count (4), NOT the CORROBORATES count (6) and NOT the placeholder
    /// (6). This is the behavioral proof the value is graph-derived.
    #[test]
    fn assemble_counts_rebuts_for_marie_obstructive() {
        let dashboard = assemble_dashboard(&mixed_evidence());
        assert_eq!(find(&dashboard, "marie-obstructive").instance_count, 4);
    }

    /// Zero facts (e.g. an unloaded graph) is a valid `0`, not an error and not
    /// the placeholder — an unloaded graph must be observably different from the
    /// loaded one (Standing Rule 1).
    #[test]
    fn assemble_zero_facts_yields_zero() {
        let empty = AnchoredAllegationEvidenceResponse {
            allegation_id: MARIE_OBSTRUCTIVE_ANCHOR_ALLEGATION_ID.to_string(),
            facts: Vec::new(),
        };
        let dashboard = assemble_dashboard(&empty);
        assert_eq!(find(&dashboard, "marie-obstructive").instance_count, 0);
    }

    /// Only `instance_count` on the one card is live; every other field on every
    /// other card — and the metrics/alerts — stays at the reproduced baseline.
    #[test]
    fn assemble_leaves_everything_else_at_baseline() {
        let dashboard = assemble_dashboard(&mixed_evidence());

        // Metrics + alerts untouched.
        assert_eq!(dashboard.metrics.instances, 16);
        assert_eq!(dashboard.metrics.scenarios, 5);
        assert_eq!(dashboard.alerts.len(), 2);

        // Other cards verbatim.
        let fifty = find(&dashboard, "fifty-thousand");
        assert_eq!(fifty.instance_count, 3);
        assert_eq!(fifty.status, ScenarioStatus::Ready);

        // Pending analysis stays null-bearing (None), not collapsed to 0.
        assert_eq!(
            find(&dashboard, "selective-sanctions").baseless_repeat_count,
            None
        );

        // The live card's NON-live fields keep their baseline values.
        let marie = find(&dashboard, "marie-obstructive");
        assert_eq!(marie.response_count, 1);
        assert_eq!(marie.baseless_repeat_count, Some(3));
        assert_eq!(marie.status, ScenarioStatus::Review);
    }
}
