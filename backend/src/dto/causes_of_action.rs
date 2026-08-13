//! Response DTOs for `GET /api/cases/:slug/causes-of-action` — the Home page
//! Causes of Action tables (`HOME_PAGE_REDESIGN_v2.md` §7).
//!
//! Serialize-only response types, plus `Authority`/`DoctrinalRequirement` which
//! are ALSO `Deserialize` because they are decoded from the JSON-encoded
//! strings the canonical Element loader stores on `LegalCount`
//! (`controlling_authorities_json`, `doctrinal_requirements_json`).
//!
//! Like the case-header endpoint, nullable fields are emitted as JSON `null`
//! (present, not omitted) — the frontend renders against fields that are always
//! there, and "absent" must stay distinguishable from "empty".

use serde::{Deserialize, Serialize};

use crate::dto::matrix_wording::MatrixWordingDto;

/// Top-level payload: the requested case slug (echoed) and its Counts.
#[derive(Debug, Clone, Serialize)]
pub struct CausesOfActionResponse {
    /// Echoed from the request path. The Neo4j graph is single-case and not
    /// slug-namespaced, so the slug is not used to filter — it is returned for
    /// the caller's correlation.
    pub case_slug: String,
    pub counts: Vec<CountDetail>,
    /// The Proof Matrix's own words (task 396, P1).
    ///
    /// Riding this payload rather than a second request: the matrix page GATES on
    /// this read — it cannot draw a row without it — and both surfaces that speak
    /// these words (the row's headline and its drill-down) live on that page.
    pub matrix_wording: MatrixWordingDto,
}

/// One Count with its canonical metadata and Elements.
#[derive(Debug, Clone, Serialize)]
pub struct CountDetail {
    pub count_number: i64,
    /// Display name, sourced from the `LegalCount.title` property.
    pub count_name: Option<String>,
    pub burden_of_proof: Option<String>,
    pub m_civ_ji_reference: Option<String>,
    /// Short form for the Count header: the `.citation` of the first entry in
    /// `controlling_authorities` (the loader writes them in designed order).
    /// `null` when the Count has no authorities.
    pub controlling_authority_primary: Option<String>,
    /// Decoded from `controlling_authorities_json`. Always an array (`[]` when
    /// the property is absent) — never a raw JSON string.
    pub controlling_authorities: Vec<Authority>,
    /// Decoded from `doctrinal_requirements_json`. `null` when the property is
    /// absent (most Counts); an array for Counts that carry them (e.g. IV).
    pub doctrinal_requirements: Option<Vec<DoctrinalRequirement>>,
    /// `false` when the flag is absent — an unflagged Count is "review not
    /// required", a meaningful default, not a swallowed error.
    pub chuck_review_required: bool,
    pub chuck_review_note: Option<String>,
    pub special_note: Option<String>,
    /// Sorted by `order_in_count` ascending; `[]` when the Count has no
    /// Elements attached yet.
    pub elements: Vec<ElementDetail>,
}

/// A controlling authority (case / statute / jury instruction / court rule).
///
/// Decoded from `controlling_authorities_json` and re-emitted. `court`/`year`
/// are absent in the stored JSON for statutes; they deserialize to `None` and
/// are emitted as `null` (present) so the field set is stable for the frontend.
// serde: allows unknown fields because this decodes loader-produced JSON only,
// never untrusted external input. Tolerating an unknown key keeps the read
// endpoint forward-compatible if the canonical Element loader later adds an
// optional authority field the Home page doesn't consume.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Authority {
    pub citation: String,
    pub authority_type: String,
    pub court: Option<String>,
    pub year: Option<u32>,
    pub role: String,
}

/// A doctrinal pleading requirement (Count IV — abuse of process). Decoded from
/// `doctrinal_requirements_json`.
// serde: allows unknown fields because of the same forward-compat reason as
// `Authority` above — loader-produced JSON only, never untrusted input.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DoctrinalRequirement {
    pub requirement: String,
    pub description: String,
    pub satisfied_in_case: bool,
    pub satisfaction_evidence: String,
}

/// One canonical Element of a Count.
#[derive(Debug, Clone, Serialize)]
pub struct ElementDetail {
    pub element_id: String,
    pub order_in_count: Option<i64>,
    pub element_name: String,
    pub what_plaintiff_must_prove: Option<String>,
    pub controlling_authority: Option<String>,
    /// Theory variant for Count II elements (`silent_fraud` /
    /// `common_law_fraud`); `null` for other Counts.
    pub theory_variant: Option<String>,
    /// Count of incoming `BEARS_ON` edges (Allegations bearing on this
    /// Element) — the proof **denominator** `T`.
    pub allegation_count: i64,
    /// Number of DISTINCT Evidence items corroborating any Allegation that
    /// bears on this Element — the SUPPORTING magnitude. Walks
    /// `(Evidence)-[:CORROBORATES]->(Allegation)-[:BEARS_ON]->(Element)`.
    ///
    /// Domain note: this is the PRE-COLLAPSE magnitude, and since .396 it is no
    /// longer what the matrix column renders. The two numbers on screen are
    /// `strong_evidence_count` and `approved_evidence_count`, both computed AFTER
    /// near-duplicate collapse. This field stays because it is a different, still
    /// true reading — how many distinct Evidence NODES corroborate — and it is
    /// what a "show me the duplicates" affordance would need. It must never be
    /// rendered beside the other two without saying which of them it is, or the
    /// page would show three numbers for one Element and invite the reader to
    /// treat a disagreement as a bug.
    pub supporting_evidence_count: i64,
    /// Corroborating items that count toward the HEADLINE: the ones whose
    /// `(statement_type, evidence_strength)` pair maps to the strong tier, after
    /// near-identical statements have been collapsed.
    ///
    /// Domain note: strong means what the opposing side cannot dispute — their own
    /// sworn admissions, and the court's own findings and orders (Roman's ruling,
    /// 2026-08-13). Computed in `services::matrix_strength`, which also produces
    /// the drill-down, so the row and the list it opens cannot disagree.
    pub strong_evidence_count: i64,
    /// Every corroborating item after collapse, whatever its tier — the depth
    /// line beside the headline ("· 15 approved").
    ///
    /// `strong_evidence_count <= approved_evidence_count` always: the strong
    /// figure is a subset of this one, which is what makes the pair readable as
    /// "this many of these".
    pub approved_evidence_count: i64,
    /// Number of this Element's Allegations that have >=1 incoming
    /// `CORROBORATES` — the coverage **numerator** `C` (`C <= allegation_count`).
    pub covered_allegation_count: i64,
    /// Number of DISTINCT Evidence items DISPUTING any Allegation that bears on
    /// this Element — the "Disputes" column magnitude. Walks
    /// `(Evidence)-[:REBUTS]->(Allegation)-[:BEARS_ON]->(Element)`, the mirror of
    /// the `supporting_evidence_count` traversal.
    ///
    /// Domain note: "Disputes", deliberately not "Contradicts" — CONTRADICTS is
    /// reserved for the future evidence-vs-evidence impeachment layer, and reusing
    /// the word here would make two different relationships read as one. Nor
    /// "Opposing": this counts what the record actually disputes, not a party's
    /// posture.
    ///
    /// This does NOT enter `proof_status`. Support and dispute are independent
    /// readings of the same Element — a well-corroborated Element can also be
    /// heavily disputed, and that Element is the one worth arguing about, so the
    /// two are shown side by side rather than netted into one verdict.
    pub disputing_evidence_count: i64,

    /// Coverage label derived in the builder from `T = allegation_count` and
    /// `C = covered_allegation_count`: one of `"no_allegations"`, `"gap"`,
    /// `"partial"`, `"supported"`.
    ///
    /// Domain note: this is **presence-of-evidence**, NOT a legal-sufficiency
    /// claim — there is deliberately no `"proven"` state. A lowercase string the
    /// frontend can `switch` on. See `causes_of_action_builder::derive_proof_status`.
    pub proof_status: String,
}
