// =============================================================================
// backend/src/repositories/scenario_repository.rs
// =============================================================================
//
// Read-only Neo4j traversals that compose a "scenario" view for trial prep:
//   1. rebuttal_facts                  — evidence that REBUTS a wielder's claims
//   2. contradictions_against_wielder  — CONTRADICTS edges touching a wielder
//   3. related_allegations             — allegations the anchor evidence targets
//
// Each method mirrors a Cypher shape already proven against the live graph
// (see RebuttalsRepository / ContradictionRepository), but parameterizes the
// case identity that those older queries hardcoded: the wielder / anchor is
// always a bound `$param`, never a literal. Zero case-identity strings live in
// this file.
//
// Follows the established repository pattern: a `#[derive(Clone)]` struct
// holding a `neo4rs::Graph`, constructed `new(graph)`, exposing async methods.
// =============================================================================

use neo4rs::{query, DeError, Graph, Row};

use crate::dto::scenario::{
    AnchoredAllegationEvidenceResponse, AnchoredEvidenceFact, ScenarioContradiction,
    ScenarioContradictionEvidence, ScenarioContradictionsResponse, ScenarioRebuttalFact,
    ScenarioRebuttalFactsResponse, ScenarioRelatedAllegation, ScenarioRelatedAllegationsResponse,
};
use crate::neo4j::schema;

// ─────────────────────────────────────────────────────────────────────────────
// Error type
// ─────────────────────────────────────────────────────────────────────────────

/// Errors returned by the scenario read methods.
///
/// ## Rust Learning: `#[derive(thiserror::Error)]` for observable errors
///
/// `thiserror` generates the `Display` and `std::error::Error` impls from the
/// `#[error("…")]` annotations, so each variant has a human-readable message a
/// handler can log with `{}` (not just `{:?}`) — satisfying Standing Rule 1
/// (every failure is observable, with context). `#[from]` on the `Neo4j`
/// variant generates the `From<neo4rs::Error>` impl so `?` on
/// `graph.execute(...).await?` / `result.next().await?` converts automatically.
/// Decode errors are NOT produced by `?` — the helpers below build
/// `Decode` / `MissingRequired` explicitly so they can attach the column name —
/// so there is deliberately no `#[from]` for `DeError` (which would reintroduce
/// an untagged decode path).
///
/// Variant set is intentionally narrow: no bare `Value(DeError)` catch-all
/// (every decode is column-tagged, so it would be dead code) and no
/// `GraphAccess(colossus_graph::GraphAccessError)` variant (no method calls a
/// `colossus_graph::*` helper, so it too would be dead).
#[derive(Debug, thiserror::Error)]
pub enum ScenarioRepositoryError {
    /// A Neo4j transport / query-execution error (connection, Cypher syntax,
    /// stream error). Surfaced via `?` on `execute` / `next`.
    #[error("Neo4j query failed: {0}")]
    Neo4j(#[from] neo4rs::Error),

    /// A column was present but held the wrong Bolt type for the target Rust
    /// type. This is the failure the house `.ok()` convention silently eats;
    /// here it is named, with the offending column, so a malformed fact on the
    /// scenario page fails visibly instead of presenting as empty.
    #[error("type mismatch decoding column '{column}': {source}")]
    Decode {
        column: String,
        #[source]
        source: DeError,
    },

    /// A required column (e.g. an evidence id) was null or absent. A fact with
    /// no identity is meaningless, so this is an error rather than an empty
    /// string.
    #[error("required column '{column}' was null or absent")]
    MissingRequired { column: String },
}

// ─────────────────────────────────────────────────────────────────────────────
// Polarity selector (task 0.3c)
// ─────────────────────────────────────────────────────────────────────────────

/// Which evidence→allegation edges `anchored_allegation_evidence` returns.
///
/// ## Rust Learning: a closed enum instead of a free `&str` rel-name parameter
///
/// The traversal filters on the *relationship type* (`REBUTS` vs
/// `CORROBORATES`). That type is NOT user input — accepting an arbitrary
/// `&str` rel-name would both admit names the graph never uses and invite a
/// label injected into the query text. A three-variant enum closes the set:
/// the only rel types that can reach the Cypher are the two `schema::`
/// constants below, mapped from the variant. The single varying input — the
/// allegation id — stays a bound `$allegation_id` parameter, never interpolated.
///
/// Domain note: against an Allegation, `REBUTS` means the evidence *counters*
/// the alleged fact and `CORROBORATES` means it *confirms* it (see
/// `schema::REBUTS` / `schema::CORROBORATES`). `Both` returns the full picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidencePolarity {
    /// Only `REBUTS` edges — evidence that counters the allegation.
    Rebutting,
    /// Only `CORROBORATES` edges — evidence that confirms the allegation.
    Corroborating,
    /// Both `REBUTS` and `CORROBORATES` edges.
    Both,
}

impl EvidencePolarity {
    /// The `schema::` relationship-type constants this polarity selects.
    ///
    /// ## Rust Learning: `&'static [&'static str]` via const promotion
    ///
    /// Each arm builds an array literal whose elements are all compile-time
    /// constants (`schema::REBUTS` / `schema::CORROBORATES` are `const &str`).
    /// Because the array is itself a constant expression, the compiler promotes
    /// the borrow to `'static` — so we can return a borrowed slice from a fn
    /// that owns no backing storage. The list is sourced ONLY from the `schema::`
    /// constants, never re-spelled literals, so a rename in `schema.rs` flows
    /// here automatically.
    fn rel_types(self) -> &'static [&'static str] {
        match self {
            EvidencePolarity::Rebutting => &[schema::REBUTS],
            EvidencePolarity::Corroborating => &[schema::CORROBORATES],
            EvidencePolarity::Both => &[schema::REBUTS, schema::CORROBORATES],
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Repository
// ─────────────────────────────────────────────────────────────────────────────

/// Read-only library of scenario-page traversals over the knowledge graph.
///
/// ## Rust Learning: `#[derive(Clone)]` + `neo4rs::Graph`
///
/// `Graph` is internally reference-counted (an `Arc` over the connection pool),
/// so cloning it is cheap and shares one pool. That is why handlers can build a
/// fresh `ScenarioRepository::new(state.graph.clone())` per request without
/// opening new connections — the standard pattern across this repositories
/// module.
#[derive(Clone)]
pub struct ScenarioRepository {
    graph: Graph,
}

impl ScenarioRepository {
    /// Construct a repository over a shared Neo4j connection.
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    /// Method 1 — facts that rebut a wielder's claims.
    ///
    /// Mirrors the proven `RebuttalsRepository::get_rebuttals` Cypher
    /// (`Evidence-[:REBUTS]->Evidence`, anchored through `STATED_BY`), but the
    /// wielder — the person whose claims are being rebutted — is the bound
    /// `$wielder_id` parameter (matched on the node's `id` property) rather
    /// than the `{name: 'George Phillips'}` literal the original baked in.
    pub async fn rebuttal_facts(
        &self,
        wielder_id: &str,
    ) -> Result<ScenarioRebuttalFactsResponse, ScenarioRepositoryError> {
        let cypher = rebuttal_facts_cypher();
        let mut result = self
            .graph
            .execute(query(&cypher).param("wielder_id", wielder_id))
            .await?;

        let mut facts = Vec::new();
        while let Some(row) = result.next().await? {
            facts.push(map_rebuttal_fact(&row)?);
        }

        tracing::debug!(wielder_id, count = facts.len(), "scenario rebuttal_facts");
        Ok(ScenarioRebuttalFactsResponse {
            wielder_id: wielder_id.to_string(),
            facts,
        })
    }

    /// Method 2 — contradiction edges that impeach a wielder.
    ///
    /// Mirrors the proven `ContradictionRepository::list_contradictions` Cypher
    /// (`Evidence-[:CONTRADICTS]->Evidence` with the `CONTAINED_IN` document
    /// joins) and adds the filter that method lacks: at least one side must be
    /// anchored to the wielder, identified by the bound `$wielder_id` matched
    /// against either a `Document.id` (via `CONTAINED_IN`) or a `Person.id`
    /// (via `STATED_BY`) — honoring "wielder may be a person or a document".
    pub async fn contradictions_against_wielder(
        &self,
        wielder_id: &str,
    ) -> Result<ScenarioContradictionsResponse, ScenarioRepositoryError> {
        let cypher = contradictions_against_wielder_cypher();
        let mut result = self
            .graph
            .execute(query(&cypher).param("wielder_id", wielder_id))
            .await?;

        let mut contradictions = Vec::new();
        while let Some(row) = result.next().await? {
            contradictions.push(map_contradiction(&row)?);
        }

        let total = contradictions.len();
        tracing::debug!(wielder_id, total, "scenario contradictions_against_wielder");
        Ok(ScenarioContradictionsResponse {
            anchor_id: wielder_id.to_string(),
            contradictions,
            total,
        })
    }

    /// Method 3 — allegations the anchor evidence directly targets.
    ///
    /// Returns the `Allegation` nodes that the anchoring evidence points at via
    /// the proven opposing/corroborating axis
    /// (`Evidence-[:CORROBORATES|REBUTS|CONTRADICTS]->Allegation`). The anchor
    /// evidence is the bound `$anchor_id` (matched on `id`). This is NOT the
    /// Count-sibling traversal (deferred to 0.3c with the BEARS_ON-vs-
    /// count_number reliability question) — only the allegations this scenario's
    /// own evidence edges reach.
    ///
    /// Domain note: an Allegation's text is its `summary` property (not
    /// `allegation_text`); `id` is the stable identifier.
    pub async fn related_allegations(
        &self,
        anchor_id: &str,
    ) -> Result<ScenarioRelatedAllegationsResponse, ScenarioRepositoryError> {
        let cypher = related_allegations_cypher();
        let mut result = self
            .graph
            .execute(query(&cypher).param("anchor_id", anchor_id))
            .await?;

        let mut allegations = Vec::new();
        while let Some(row) = result.next().await? {
            allegations.push(map_related_allegation(&row)?);
        }

        tracing::debug!(
            anchor_id,
            count = allegations.len(),
            "scenario related_allegations"
        );
        Ok(ScenarioRelatedAllegationsResponse {
            anchor_id: anchor_id.to_string(),
            allegations,
        })
    }

    /// Method 4 — evidence anchored on an allegation (task 0.3c).
    ///
    /// The INVERSE of `related_allegations`: where that method anchors on an
    /// Evidence id and returns the Allegations it targets, this one anchors on
    /// an **Allegation** id (the bound `$allegation_id`, matched on `id`) and
    /// returns the **Evidence** pointing at it, carrying the per-edge polarity
    /// plus the document and speaker joins.
    ///
    /// `polarity` selects which edges count: `Rebutting` → `REBUTS` only,
    /// `Corroborating` → `CORROBORATES` only, `Both` → either. The rel-type
    /// filter is built from `schema::` constants via the closed
    /// [`EvidencePolarity`] enum — no rel name is ever a free string parameter.
    ///
    /// Domain note: the populated axis is `(:Evidence)-[r]->(:Allegation)` and
    /// these edges carry NO properties (no `r.topic`), so the polarity lives in
    /// `type(r)` alone. Two distinct evidence nodes may carry identical
    /// `verbatim_quote` text — that is real data, not a duplicate; both rows are
    /// returned and any display-level dedup is a later UI concern.
    pub async fn anchored_allegation_evidence(
        &self,
        allegation_id: &str,
        polarity: EvidencePolarity,
    ) -> Result<AnchoredAllegationEvidenceResponse, ScenarioRepositoryError> {
        let cypher = anchored_allegation_evidence_cypher(polarity);
        let mut result = self
            .graph
            .execute(query(&cypher).param("allegation_id", allegation_id))
            .await?;

        let mut facts = Vec::new();
        while let Some(row) = result.next().await? {
            facts.push(map_anchored_evidence_fact(&row)?);
        }

        tracing::debug!(
            allegation_id,
            ?polarity,
            count = facts.len(),
            "scenario anchored_allegation_evidence"
        );
        Ok(AnchoredAllegationEvidenceResponse {
            allegation_id: allegation_id.to_string(),
            facts,
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Cypher builders
// ─────────────────────────────────────────────────────────────────────────────
//
// The fixed Cypher is built here, one function per query, kept beside the
// methods. Relationship-type names come from `schema::` constants rather than
// re-spelled literals (the legitimate fixed kind of value). Case identity is
// never a literal — it arrives as the `$wielder_id` / `$anchor_id` bind.

/// ## Rust Learning: brace-doubling in `format!`
///
/// `format!` treats `{` / `}` as placeholder delimiters, so a Cypher node
/// property map like `{id: $wielder_id}` must be written `{{id: $wielder_id}}`
/// to emit literal braces. The `{stated_by}` etc. are real named placeholders
/// filled from the `schema::` constants. (`graph_repository.rs` documents the
/// alternative — splicing a pre-built `&str` — used when a query mixes many
/// literal braces.)
fn rebuttal_facts_cypher() -> String {
    format!(
        "MATCH (rebE:Evidence)-[r:{rebuts}]->(targetE:Evidence)\
         -[:{stated_by}]->(w:Person {{id: $wielder_id}})
         MATCH (rebE)-[:{contained_in}]->(rebDoc:Document)
         OPTIONAL MATCH (rebE)-[:{stated_by}]->(rebSpeaker)
         RETURN rebE.id AS evidence_id,
                r.topic AS topic,
                rebE.verbatim_quote AS verbatim_quote,
                rebE.page_number AS page_number,
                rebDoc.title AS document,
                CASE WHEN rebSpeaker:Person OR rebSpeaker:Organization
                     THEN rebSpeaker.name ELSE null END AS stated_by
         ORDER BY rebE.id",
        rebuts = schema::REBUTS,
        stated_by = schema::STATED_BY,
        contained_in = schema::CONTAINED_IN,
    )
}

fn contradictions_against_wielder_cypher() -> String {
    format!(
        "MATCH (a:Evidence)-[r:{contradicts}]->(b:Evidence)
         WHERE (a)-[:{contained_in}]->(:Document {{id: $wielder_id}})
            OR (b)-[:{contained_in}]->(:Document {{id: $wielder_id}})
            OR (a)-[:{stated_by}]->(:Person {{id: $wielder_id}})
            OR (b)-[:{stated_by}]->(:Person {{id: $wielder_id}})
         OPTIONAL MATCH (a)-[:{contained_in}]->(da:Document)
         OPTIONAL MATCH (b)-[:{contained_in}]->(db:Document)
         RETURN a.id AS evidence_a_id,
                a.title AS evidence_a_title,
                a.answer AS evidence_a_answer,
                da.title AS evidence_a_document,
                b.id AS evidence_b_id,
                b.title AS evidence_b_title,
                b.answer AS evidence_b_answer,
                db.title AS evidence_b_document,
                r.description AS description,
                r.topic AS topic,
                r.impeachment_value AS impeachment_value,
                r.earlier_claim AS earlier_claim,
                r.later_admission AS later_admission
         ORDER BY a.id",
        contradicts = schema::CONTRADICTS,
        contained_in = schema::CONTAINED_IN,
        stated_by = schema::STATED_BY,
    )
}

fn related_allegations_cypher() -> String {
    format!(
        "MATCH (anchorE:Evidence {{id: $anchor_id}})\
         -[:{corroborates}|{rebuts}|{contradicts}]->(al:Allegation)
         RETURN al.id AS id,
                al.summary AS summary,
                al.title AS title,
                al.paragraph_number AS paragraph_number
         ORDER BY al.id",
        corroborates = schema::CORROBORATES,
        rebuts = schema::REBUTS,
        contradicts = schema::CONTRADICTS,
    )
}

/// Build the allegation-anchored evidence query for a given polarity.
///
/// The `type(r) IN [...]` list is assembled from the polarity's `schema::`
/// constants. `type(r)` evaluates to a string, so each rel name is emitted as a
/// quoted Cypher string literal. These names are fixed (the closed
/// `EvidencePolarity` enum → `schema::` constants), NOT user input — so building
/// the in-list this way is safe; the only varying input, the allegation id,
/// stays the bound `$allegation_id` parameter.
///
/// The graph pattern below is the shape verified live against DEV on 2026-06-27
/// (the 0.3c shape-verification probes) — directions, labels, and the two
/// `OPTIONAL MATCH` hops are not to be altered, only the polarity list injected.
fn anchored_allegation_evidence_cypher(polarity: EvidencePolarity) -> String {
    let rel_list = polarity
        .rel_types()
        .iter()
        .map(|t| format!("'{t}'"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "MATCH (e:Evidence)-[r]->(a:Allegation {{id: $allegation_id}})
         WHERE type(r) IN [{rel_list}]
         OPTIONAL MATCH (e)-[:{contained_in}]->(doc:Document)
         OPTIONAL MATCH (e)-[:{stated_by}]->(spk)
         RETURN e.id AS evidence_id,
                type(r) AS polarity,
                a.id AS allegation_id,
                a.paragraph_number AS paragraph_number,
                e.verbatim_quote AS verbatim_quote,
                e.page_number AS page_number,
                doc.title AS document,
                CASE WHEN spk:Person OR spk:Organization
                     THEN spk.name ELSE null END AS stated_by
         ORDER BY e.id",
        rel_list = rel_list,
        contained_in = schema::CONTAINED_IN,
        stated_by = schema::STATED_BY,
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Row → DTO mappers
// ─────────────────────────────────────────────────────────────────────────────

/// Map one REBUTS row into a typed fact.
///
/// Domain note: `page_number` is treated as a string property here (the
/// rebuttals feature's existing contract); a non-string value would surface as
/// a `Decode` error rather than silently vanish.
fn map_rebuttal_fact(row: &Row) -> Result<ScenarioRebuttalFact, ScenarioRepositoryError> {
    Ok(ScenarioRebuttalFact {
        evidence_id: decode_required_str(row, "evidence_id")?,
        topic: decode_opt_str(row, "topic")?,
        verbatim_quote: decode_opt_str(row, "verbatim_quote")?,
        page_number: decode_opt_str(row, "page_number")?,
        document: decode_opt_str(row, "document")?,
        stated_by: decode_opt_str(row, "stated_by")?,
    })
}

/// Map one CONTRADICTS row (both evidence sides + edge properties).
fn map_contradiction(row: &Row) -> Result<ScenarioContradiction, ScenarioRepositoryError> {
    Ok(ScenarioContradiction {
        evidence_a: ScenarioContradictionEvidence {
            id: decode_required_str(row, "evidence_a_id")?,
            title: decode_opt_str(row, "evidence_a_title")?,
            answer: decode_opt_str(row, "evidence_a_answer")?,
            document_title: decode_opt_str(row, "evidence_a_document")?,
        },
        evidence_b: ScenarioContradictionEvidence {
            id: decode_required_str(row, "evidence_b_id")?,
            title: decode_opt_str(row, "evidence_b_title")?,
            answer: decode_opt_str(row, "evidence_b_answer")?,
            document_title: decode_opt_str(row, "evidence_b_document")?,
        },
        description: decode_opt_str(row, "description")?,
        topic: decode_opt_str(row, "topic")?,
        impeachment_value: decode_opt_str(row, "impeachment_value")?,
        earlier_claim: decode_opt_str(row, "earlier_claim")?,
        later_admission: decode_opt_str(row, "later_admission")?,
    })
}

/// Map one Allegation row.
fn map_related_allegation(row: &Row) -> Result<ScenarioRelatedAllegation, ScenarioRepositoryError> {
    Ok(ScenarioRelatedAllegation {
        id: decode_required_str(row, "id")?,
        summary: decode_opt_str(row, "summary")?,
        title: decode_opt_str(row, "title")?,
        paragraph_number: decode_opt_str(row, "paragraph_number")?,
    })
}

/// Map one allegation-anchored evidence row.
///
/// Domain note: `polarity` (the edge's `type(r)`) is required — an edge always
/// has a type — alongside `evidence_id` and `allegation_id`. The descriptive
/// columns (`paragraph_number`, `verbatim_quote`, `page_number`, `document`,
/// `stated_by`) are optional: a node or join may legitimately omit them, and
/// per the tightened decode discipline an absent/null value degrades to `None`
/// while a wrong-type value surfaces as a named `Decode` error.
fn map_anchored_evidence_fact(row: &Row) -> Result<AnchoredEvidenceFact, ScenarioRepositoryError> {
    Ok(AnchoredEvidenceFact {
        evidence_id: decode_required_str(row, "evidence_id")?,
        polarity: decode_required_str(row, "polarity")?,
        allegation_id: decode_required_str(row, "allegation_id")?,
        paragraph_number: decode_opt_str(row, "paragraph_number")?,
        verbatim_quote: decode_opt_str(row, "verbatim_quote")?,
        page_number: decode_opt_str(row, "page_number")?,
        document: decode_opt_str(row, "document")?,
        stated_by: decode_opt_str(row, "stated_by")?,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Tightened decode helpers (operate on neo4rs::Row)
// ─────────────────────────────────────────────────────────────────────────────
//
// Why: this deliberately tightens decode against the house `.ok()` /
// `.unwrap_or_default()` convention, which collapses BOTH "column null/absent"
// AND "present but wrong type" into None/empty. The scenario surface is what
// Chuck and Marie read at trial prep, so a malformed fact must fail VISIBLY
// (Standing Rule 1, no silent failure). This is scoped to new code only — the
// older repositories are intentionally left on the `.ok()` convention (Roman
// parked that remediation; see DECISION_LOG 2026-06-25).
//
// The classifier/`require` decision logic is split out as pure functions (no
// `Row` involved) so the error paths are unit-testable without a live graph —
// matching how the rest of this module's repositories are exercised (no
// repository unit-constructs a `neo4rs::Row`; the async queries are
// integration-tested against a graph fixture).

/// Decide what a `row.get::<Option<String>>(col)` outcome means under the
/// tightened discipline.
///
/// ## Rust Learning: telling null from type-mismatch via `Option<T>`
///
/// `neo4rs` deserializes a Bolt `Null` into `Option<T>` as `Ok(None)`, a
/// present value of the right type as `Ok(Some(v))`, and a present value of the
/// WRONG type as `Err(DeError::InvalidType { .. })`. An entirely absent column
/// (e.g. a `RETURN` alias typo) is `Err(DeError::NoSuchProperty)`. So decoding
/// into `Option<String>` and then classifying the result is exactly the
/// three-way distinction we want.
fn classify_opt_str(
    column: &str,
    raw: Result<Option<String>, DeError>,
) -> Result<Option<String>, ScenarioRepositoryError> {
    match raw {
        // Present + correct (`Some`) or present + null (`None`) — both fine.
        Ok(value) => Ok(value),
        // Column not in the row at all: degrade to `None` like a null, per the
        // approved discipline (a legitimately-absent column is not an error).
        Err(DeError::NoSuchProperty) => Ok(None),
        // Present but the wrong Bolt type for a String — the case the `.ok()`
        // convention would silently swallow. Surface it, named, with column.
        Err(source) => Err(ScenarioRepositoryError::Decode {
            column: column.to_string(),
            source,
        }),
    }
}

/// Promote a decoded optional into a required value, or a named error.
fn require(column: &str, decoded: Option<String>) -> Result<String, ScenarioRepositoryError> {
    decoded.ok_or_else(|| ScenarioRepositoryError::MissingRequired {
        column: column.to_string(),
    })
}

/// Decode an optional string column (null/absent → `None`; wrong type → error).
fn decode_opt_str(row: &Row, column: &str) -> Result<Option<String>, ScenarioRepositoryError> {
    classify_opt_str(column, row.get::<Option<String>>(column))
}

/// Decode a required string column (null/absent → `MissingRequired` error).
fn decode_required_str(row: &Row, column: &str) -> Result<String, ScenarioRepositoryError> {
    require(column, decode_opt_str(row, column)?)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests — pure decode-decision logic (error paths included)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_passes_present_value_through() {
        let out = classify_opt_str("topic", Ok(Some("conversion".to_string())));
        assert_eq!(out.expect("ok"), Some("conversion".to_string()));
    }

    #[test]
    fn classify_null_value_is_none() {
        let out = classify_opt_str("topic", Ok(None));
        assert_eq!(out.expect("ok"), None);
    }

    #[test]
    fn classify_absent_column_is_none() {
        // An absent column (alias typo / OPTIONAL MATCH miss) degrades to None.
        let out = classify_opt_str("topic", Err(DeError::NoSuchProperty));
        assert_eq!(out.expect("ok"), None);
    }

    #[test]
    fn classify_type_mismatch_is_named_decode_error() {
        // A present-but-wrong-type value must NOT silently become None — it is a
        // named Decode error carrying the column. (DeError::Other stands in for
        // an InvalidType here; both take the same non-NoSuchProperty branch.)
        let out = classify_opt_str("page_number", Err(DeError::Other("expected string".into())));
        match out {
            Err(ScenarioRepositoryError::Decode { column, .. }) => {
                assert_eq!(column, "page_number")
            }
            other => panic!("expected Decode error, got {other:?}"),
        }
    }

    #[test]
    fn require_returns_present_value() {
        let out = require("evidence_id", Some("evidence-001".to_string()));
        assert_eq!(out.expect("ok"), "evidence-001");
    }

    #[test]
    fn require_missing_is_named_error() {
        match require("evidence_id", None) {
            Err(ScenarioRepositoryError::MissingRequired { column }) => {
                assert_eq!(column, "evidence_id")
            }
            other => panic!("expected MissingRequired error, got {other:?}"),
        }
    }

    #[test]
    fn errors_display_human_readable_messages() {
        // Standing Rule 1: each variant must format with `{}` (not just `{:?}`)
        // so a handler can log a context-bearing message.
        let decode = ScenarioRepositoryError::Decode {
            column: "page_number".to_string(),
            source: DeError::Other("expected string".into()),
        };
        assert_eq!(
            decode.to_string(),
            "type mismatch decoding column 'page_number': expected string"
        );

        let missing = ScenarioRepositoryError::MissingRequired {
            column: "evidence_id".to_string(),
        };
        assert_eq!(
            missing.to_string(),
            "required column 'evidence_id' was null or absent"
        );
    }

    #[test]
    fn cypher_builders_bind_params_and_carry_no_case_identity() {
        // Guard: the parameterized identity is a bind, and no party name or
        // id-prefix leaked into the fixed Cypher.
        for cypher in [
            rebuttal_facts_cypher(),
            contradictions_against_wielder_cypher(),
            related_allegations_cypher(),
        ] {
            assert!(!cypher.to_lowercase().contains("phillips"));
            assert!(!cypher.to_lowercase().contains("george"));
            assert!(!cypher.contains("evidence-"));
        }
        assert!(rebuttal_facts_cypher().contains("$wielder_id"));
        assert!(contradictions_against_wielder_cypher().contains("$wielder_id"));
        assert!(related_allegations_cypher().contains("$anchor_id"));
        // Relationship names come from schema:: constants, not re-spelled.
        assert!(related_allegations_cypher().contains(schema::CORROBORATES));
    }

    // ── Task 0.3c — polarity → rel-type mapping (pure) ──────────────────────

    #[test]
    fn polarity_rebutting_selects_only_rebuts() {
        assert_eq!(EvidencePolarity::Rebutting.rel_types(), &[schema::REBUTS]);
    }

    #[test]
    fn polarity_corroborating_selects_only_corroborates() {
        assert_eq!(
            EvidencePolarity::Corroborating.rel_types(),
            &[schema::CORROBORATES]
        );
    }

    #[test]
    fn polarity_both_selects_rebuts_and_corroborates() {
        assert_eq!(
            EvidencePolarity::Both.rel_types(),
            &[schema::REBUTS, schema::CORROBORATES]
        );
    }

    #[test]
    fn anchored_cypher_binds_param_and_carries_no_case_identity() {
        // The allegation id is a bind, never interpolated; no party name leaked.
        for polarity in [
            EvidencePolarity::Rebutting,
            EvidencePolarity::Corroborating,
            EvidencePolarity::Both,
        ] {
            let cypher = anchored_allegation_evidence_cypher(polarity);
            assert!(cypher.contains("$allegation_id"));
            assert!(!cypher.to_lowercase().contains("phillips"));
            assert!(!cypher.contains("doc-awad"));
            // The verified shape must NOT depend on an edge property.
            assert!(!cypher.contains("r.topic"));
        }
    }

    #[test]
    fn anchored_cypher_injects_polarity_rel_types_as_quoted_literals() {
        // Each polarity emits exactly its schema:: rel names, quoted, in the
        // `type(r) IN [...]` list — and only those.
        let rebut = anchored_allegation_evidence_cypher(EvidencePolarity::Rebutting);
        assert!(rebut.contains(&format!("type(r) IN ['{}']", schema::REBUTS)));
        assert!(!rebut.contains(schema::CORROBORATES));

        let corrob = anchored_allegation_evidence_cypher(EvidencePolarity::Corroborating);
        assert!(corrob.contains(&format!("type(r) IN ['{}']", schema::CORROBORATES)));

        let both = anchored_allegation_evidence_cypher(EvidencePolarity::Both);
        assert!(both.contains(&format!(
            "type(r) IN ['{}', '{}']",
            schema::REBUTS,
            schema::CORROBORATES
        )));
    }
}
