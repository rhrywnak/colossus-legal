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

/// Top-level payload: the requested case slug (echoed) and its Counts.
#[derive(Debug, Clone, Serialize)]
pub struct CausesOfActionResponse {
    /// Echoed from the request path. The Neo4j graph is single-case and not
    /// slug-namespaced, so the slug is not used to filter — it is returned for
    /// the caller's correlation.
    pub case_slug: String,
    pub counts: Vec<CountDetail>,
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
// serde: deliberately no `deny_unknown_fields` — this decodes loader-produced
// JSON only, never untrusted external input. Tolerating an unknown key keeps
// the read endpoint forward-compatible if the canonical Element loader later
// adds an optional authority field the Home page doesn't consume.
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
// serde: no `deny_unknown_fields` for the same forward-compat reason as
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
    /// Count of incoming `PROVES_ELEMENT` edges (Allegations proving this
    /// Element). Computed from the graph, currently 0 for all until the
    /// Allegation-to-Element mapping pass runs.
    pub allegation_count: i64,
}
