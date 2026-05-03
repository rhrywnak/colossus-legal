//! Bias Explorer — request and response DTOs.
//!
//! These types form the wire contract for `GET /api/bias/available-filters`
//! and `POST /api/bias/query`. They are deliberately additive: every new
//! filter dimension or response field gets added here without changing the
//! existing fields' semantics.
//!
//! ## Rust Learning: serde attributes worth knowing here
//!
//! - `#[serde(skip_serializing_if = "Option::is_none")]` — when the field is
//!   `None`, it is omitted from the JSON output entirely (rather than
//!   serialized as `null`). This preserves the *missing vs. empty*
//!   distinction the standing rules require, e.g. a missing
//!   `Document.document_type` round-trips as "absent" rather than as the
//!   string `"unknown"`.
//! - `#[serde(default)]` on a struct — every absent field deserializes to
//!   its `Default` value. Combined with `#[derive(Default)]` on
//!   `BiasQueryFilters`, the client can `POST {}` and it deserializes to
//!   "all actors, all patterns" rather than failing.
//! - Snake_case is implicit: every field below is already snake_case, so
//!   no `rename_all` is needed. The frontend mirrors these names verbatim.

use serde::{Deserialize, Serialize};

// ─── Available-filters response ─────────────────────────────────────────────

/// Response payload for `GET /api/bias/available-filters`.
///
/// Tells the frontend which dropdown values to render. Both lists are
/// derived from the data — there is no compile-time list of pattern tags
/// or actor names anywhere in the codebase. (Standing Rule 2.)
///
/// `actors` is sorted by `tagged_statement_count` descending, then by name
/// ascending; `pattern_tags` is sorted alphabetically. The repository
/// guarantees this ordering, so the frontend can render directly without
/// re-sorting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableFilters {
    pub actors: Vec<ActorOption>,
    pub pattern_tags: Vec<String>,
}

/// One actor that has at least one tagged Evidence statement.
///
/// `actor_type` is the Neo4j label — currently `"Person"` or `"Organization"`,
/// but the backend pulls `labels(actor)[0]` so a future `Court` or `Agency`
/// label would flow through with no code change. The frontend treats it as
/// an opaque string for display only.
///
/// `tagged_statement_count` is the count used for both sorting and the
/// "George Phillips (114)" badge in the dropdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorOption {
    pub id: String,
    pub name: String,
    pub actor_type: String,
    pub tagged_statement_count: i64,
}

// ─── Query request / response ───────────────────────────────────────────────

/// Request body for `POST /api/bias/query`.
///
/// ## Why POST and not GET
///
/// The filter object will grow over time and may include arrays
/// (`document_ids`, `document_types`). POST keeps the payload structured
/// and avoids encoding-the-world-into-a-querystring gymnastics.
///
/// ## Why an Option-of-everything shape rather than separate endpoints
///
/// The instruction's extensibility constraint: adding a new filter
/// dimension must be a new field plus a corresponding Cypher fragment, not
/// a new endpoint. `None` means "no filter on this dimension".
///
/// ## Rust Learning: `#[serde(default)]` at the struct level
///
/// Combined with `#[derive(Default)]`, an empty JSON body `{}` deserializes
/// to "all None". Every new field added below should also default sensibly,
/// or the existing clients break.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BiasQueryFilters {
    /// Filter to a single actor's STATED_BY edges. `None` = all actors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<String>,

    /// Filter to a single pattern tag (e.g., `"disparagement"`). `None` =
    /// all patterns.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_tag: Option<String>,
    // FUTURE: pub date_from: Option<NaiveDate>,
    // FUTURE: pub date_to: Option<NaiveDate>,
    // FUTURE: pub document_ids: Option<Vec<String>>,
    // FUTURE: pub document_types: Option<Vec<String>>,
}

/// Response payload for `POST /api/bias/query`.
///
/// `applied_filters` echoes the filter object back so the UI can render
/// "Showing N instances matching {actor=X, pattern=Y}" without having to
/// remember what it sent. (It also disambiguates server-side coercion —
/// e.g., trimming whitespace — should we add any.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiasQueryResult {
    pub total_count: i64,
    pub instances: Vec<BiasInstance>,
    pub applied_filters: BiasQueryFilters,
}

/// One Evidence node that matched the current filter combination.
///
/// `pattern_tags` is exposed as a `Vec<String>` even though it is stored
/// as a CSV string on the Evidence node; the parsing happens in the
/// repository so the wire shape stays consistent with the rest of the
/// codebase (the frontend never sees CSV strings).
///
/// `stated_by` is `Option` because — in principle — an Evidence node
/// could exist without a STATED_BY edge. In practice the bias query
/// requires the edge (it's the basis of the filter), so this is `Some`
/// for every row this endpoint returns. We keep it `Option` for forward
/// compatibility: the day someone wants "all evidence" without the
/// STATED_BY constraint, the DTO already supports it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiasInstance {
    pub evidence_id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbatim_quote: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_number: Option<i64>,
    pub pattern_tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stated_by: Option<ActorOption>,
    pub about: Vec<ActorOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document: Option<DocumentRef>,
}

/// Reference to the source document an Evidence node was extracted from.
///
/// `document_type` is `Option` so a document with no `document_type`
/// property in the graph is distinguishable from one whose type is
/// the empty string. (Standing Rule 1: distinct states, distinct
/// observables.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentRef {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_type: Option<String>,
}
