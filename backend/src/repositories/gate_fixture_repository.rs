//! The two graph reads the gate fixture needs and no existing repository makes.
//!
//! **READ-ONLY.** Both queries are bare `MATCH … RETURN`; neither writes, and
//! neither can be made to — there is no `CREATE`, `MERGE`, `SET` or `DELETE` in
//! this file.
//!
//! ## Why these two are not folded into an existing repository
//!
//! The pool itself already has a reader —
//! [`crate::bias::repository::BiasRepository::all_evidence_about_subject`] — and
//! G0 uses it, precisely so the fixture's candidate list is the same list
//! `scenario_gather` builds rather than a look-alike. But that projection is the
//! Bias Explorer's, and it does not return `e.significance`, which the reranker
//! wants because the design's Stage 1 embeds *title + quote + significance*.
//! Widening the bias projection to serve a one-shot tool would change the wire
//! shape of three live endpoints for a fixture nobody serves.
//!
//! The allegation read is the same story from the other side: the catalogue
//! reader (`allegation_options_repository`) is `pub(crate)`, scoped to a subject,
//! and folds in a per-scenario pool count; and `AllegationRepository`'s list
//! deliberately FILTERS out incorporation boilerplate by paragraph. Resolving an
//! anchor id through a filtered list is exactly how an anchor would go missing
//! without anybody noticing. This asks the narrow question — "these ids, their
//! paragraph and their text" — and nothing else.
//!
//! ## Rust Learning: `$ids` as one array parameter
//!
//! Both queries take the whole id list as a single Cypher parameter and match
//! with `IN $ids`. That is one round trip for 292 nodes rather than 292, and
//! — the part that matters more — no id is ever concatenated into query text, so
//! a document id containing a quote character cannot change what the query says.

use std::collections::HashMap;

use neo4rs::{query, Graph};

use crate::models::document_status::{ENTITY_ALLEGATION, ENTITY_EVIDENCE};

// STRUCTURAL: Cypher query text is wire vocabulary for the Neo4j protocol, not a
// deployment-variable setting. There is no environment in which this project
// wants to ask the graph a DIFFERENT question here, and no operator has any
// reason to edit it — moving it to YAML would make the query a runtime surprise
// instead of a compile-time fact. Both are built by a function rather than held
// as a bare literal so the node label comes from `models::document_status`,
// which is what makes a schema rename reach them (Rule 16).
//
// The shape test at the bottom of this file asserts against the TEXT THAT RUNS,
// which is why they are built here rather than inline at the call site.

/// The significance read, with its node label sourced from the shared constant.
///
/// ## Why the label is interpolated and not a `$parameter`
///
/// Cypher cannot parameterize a node label inside a MATCH pattern, and
/// `MATCH (e) WHERE labels(e)[0] = $label` — which CAN — gives up the label
/// index and makes Neo4j consider every node in the graph. `bias/queries.rs`
/// resolves the same tension the same way: interpolate the constant into the
/// pattern with `format!`. Nothing user-supplied is interpolated, so there is no
/// injection surface; `$ids` is still a real parameter.
fn significance_cypher() -> String {
    format!(
        "MATCH (e:{label}) WHERE e.id IN $ids \
         RETURN e.id AS evidence_id, e.significance AS significance",
        label = ENTITY_EVIDENCE,
    )
}

// STRUCTURAL: see the note above. This one takes its label as a real parameter
// because it matches on `labels(a)[0]` rather than on a pattern label — the
// shape `allegation_options_repository::allegation_options_query` already uses,
// and the reason it can is that the allegation set is small enough that the
// label index is not load-bearing.
const ALLEGATIONS_CYPHER: &str =
    "MATCH (a) WHERE labels(a)[0] = $allegation_label AND a.id IN $ids \
     RETURN a.id AS allegation_id, a.paragraph_number AS paragraph, \
            coalesce(a.summary, a.title) AS text \
     ORDER BY toInteger(a.paragraph_number), a.id";

/// One allegation, exactly as the query composer needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllegationTextRow {
    pub allegation_id: String,
    /// The complaint paragraph — `"16"`, which renders as `A-16`. `None` for an
    /// allegation node carrying no paragraph number, which is a real (if poor)
    /// node and not a decode failure.
    pub paragraph: Option<String>,
    /// The allegation's own words: its summary, or its title when it has no
    /// summary. `None` when it has neither, which the audit then reports as a
    /// blank allegation rather than papering over.
    pub text: Option<String>,
}

/// Errors these two reads can raise, each naming the operation that raised it.
///
/// ## Rust Learning: `#[source]` and why the operation is a `&'static str`
///
/// `thiserror`'s `#[source]` keeps the underlying `neo4rs` error attached, so a
/// caller printing the chain sees both "which read" and "what the driver said".
/// The operation name is a `&'static str` because it is always a literal written
/// at the raise site — an owned `String` would allocate on every error path to
/// carry a constant.
#[derive(Debug, thiserror::Error)]
pub enum GateFixtureRepoError {
    #[error("gate fixture read '{operation}' failed: {source}")]
    Query {
        operation: &'static str,
        #[source]
        source: neo4rs::Error,
    },
    #[error("gate fixture read '{operation}' could not decode a row: {source}")]
    RowDecode {
        operation: &'static str,
        #[source]
        source: neo4rs::DeError,
    },
}

/// `e.significance` for the given Evidence ids, as an `id → significance` index.
///
/// Ids with no significance property are simply absent from the map — the caller
/// reads a miss as `None` and writes `null` into the fixture, which is a
/// different fact from an empty string and stays that way all the way to G1.
///
/// # Errors
/// Returns [`GateFixtureRepoError`] if the query or a row decode fails.
pub async fn significance_by_ids(
    graph: &Graph,
    ids: &[String],
) -> Result<HashMap<String, String>, GateFixtureRepoError> {
    const OP: &str = "significance_by_ids";

    // An empty list short-circuits: sending `[]` would be a pointless round trip
    // and its result is knowable without one.
    if ids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut stream = graph
        .execute(query(&significance_cypher()).param("ids", ids.to_vec()))
        .await
        .map_err(|source| GateFixtureRepoError::Query {
            operation: OP,
            source,
        })?;

    let mut index = HashMap::new();
    while let Some(row) = stream
        .next()
        .await
        .map_err(|source| GateFixtureRepoError::Query {
            operation: OP,
            source,
        })?
    {
        let id: String =
            row.get("evidence_id")
                .map_err(|source| GateFixtureRepoError::RowDecode {
                    operation: OP,
                    source,
                })?;
        let significance: Option<String> =
            row.get("significance")
                .map_err(|source| GateFixtureRepoError::RowDecode {
                    operation: OP,
                    source,
                })?;
        if let Some(text) = significance {
            index.insert(id, text);
        }
    }
    Ok(index)
}

/// The paragraph and text of each named allegation — unfiltered.
///
/// Domain note: the label comes from [`ENTITY_ALLEGATION`] rather than a literal
/// `:Allegation`, so a schema rename reaches this read with the rest of the
/// graph layer (Rule 16). Ids that match nothing simply do not come back, and
/// the caller reports the gap — an anchor pointing at a node that no longer
/// exists is exactly the stale-pointer defect this fixture must not hide.
///
/// # Errors
/// Returns [`GateFixtureRepoError`] if the query or a row decode fails.
pub async fn allegations_by_ids(
    graph: &Graph,
    ids: &[String],
) -> Result<Vec<AllegationTextRow>, GateFixtureRepoError> {
    const OP: &str = "allegations_by_ids";

    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut stream = graph
        .execute(
            query(ALLEGATIONS_CYPHER)
                .param("allegation_label", ENTITY_ALLEGATION)
                .param("ids", ids.to_vec()),
        )
        .await
        .map_err(|source| GateFixtureRepoError::Query {
            operation: OP,
            source,
        })?;

    let mut rows = Vec::new();
    while let Some(row) = stream
        .next()
        .await
        .map_err(|source| GateFixtureRepoError::Query {
            operation: OP,
            source,
        })?
    {
        let decode = |source: neo4rs::DeError| GateFixtureRepoError::RowDecode {
            operation: OP,
            source,
        };
        rows.push(AllegationTextRow {
            allegation_id: row.get("allegation_id").map_err(decode)?,
            paragraph: row.get("paragraph").map_err(decode)?,
            text: row.get("text").map_err(decode)?,
        });
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Neither query can write. A shape test, not a live one: it is the only
    /// check that survives someone "just adding a SET" during a later edit, and
    /// the whole task this file serves is defined as read-only.
    #[test]
    fn neither_query_can_write() {
        for cypher in [significance_cypher().as_str(), ALLEGATIONS_CYPHER] {
            for forbidden in ["CREATE", "MERGE", " SET ", "DELETE", "REMOVE", "DETACH"] {
                assert!(
                    !cypher.to_uppercase().contains(forbidden),
                    "a gate-fixture read must not contain {forbidden}"
                );
            }
        }
    }

    /// The Evidence label reaches the query from the shared constant.
    ///
    /// The point of Rule 16 is that a schema rename lands in one place. This
    /// asserts the query actually carries whatever that constant says — a test
    /// that would have caught the hardcoded `:Evidence` this file shipped with.
    #[test]
    fn the_evidence_label_comes_from_the_shared_constant() {
        assert!(
            significance_cypher().contains(&format!("(e:{ENTITY_EVIDENCE})")),
            "the significance read must match on the shared Evidence label, not a literal"
        );
    }

    /// Both error variants name the operation that raised them.
    ///
    /// ## Rust Learning: why this test exists at all
    ///
    /// `thiserror` generates `Display` from the `#[error("…")]` format string,
    /// which means the message is only as correct as that string — and nothing
    /// else in the build checks it. An operator reading a failed run's log has
    /// exactly these two sentences to work from, so "which read failed" being
    /// interpolated into them is load-bearing, and a refactor that dropped
    /// `{operation}` would otherwise be silent.
    #[test]
    fn both_error_variants_name_the_operation_that_raised_them() {
        let query = GateFixtureRepoError::Query {
            operation: "significance_by_ids",
            // Any constructible variant will do: the assertion is about OUR
            // wrapper's message, not about what neo4rs had to say.
            source: neo4rs::Error::ConnectionError,
        };
        let rendered = query.to_string();
        assert!(
            rendered.contains("significance_by_ids"),
            "the Query variant must name the read that failed, got: {rendered}"
        );
        assert!(rendered.contains("failed"), "got: {rendered}");

        let decode = GateFixtureRepoError::RowDecode {
            operation: "allegations_by_ids",
            source: neo4rs::DeError::InvalidLength {
                received: 0,
                expected: "one row".to_string(),
            },
        };
        let rendered = decode.to_string();
        assert!(
            rendered.contains("allegations_by_ids"),
            "the RowDecode variant must name the read that failed, got: {rendered}"
        );
        assert!(rendered.contains("could not decode"), "got: {rendered}");
    }
}
