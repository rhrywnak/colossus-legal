//! The one read Stage 0 needs: a scenario's linked allegations, their verbatim
//! words, and the parties they name.
//!
//! **READ-ONLY.** One `MATCH … RETURN`; a shape test asserts it can never be
//! anything else.
//!
//! ## ⚑ How an allegation names a party — established, not assumed
//!
//! The task required this be settled from the graph before the filter was
//! designed, because "the only route is parsing names out of prose" would have
//! been a stop. Measured on DEV 2026-09-01, across all 120 `Allegation` nodes:
//!
//! ```text
//! outgoing  ABOUT      -> Organization   124
//! outgoing  ABOUT      -> Person         180
//! outgoing  BEARS_ON   -> Element        207
//! outgoing  CONTAINED_IN -> Document     120
//! ```
//!
//! **Allegations carry real `ABOUT` edges to `Person` and `Organization`** —
//! the same edge type Evidence uses for the same purpose. No prose parsing is
//! required and none is done here. (There is also an `applies_to` property on
//! all 120, which is NOT used: a property naming a party in text would be the
//! very thing the stop condition was guarding against, and the edges make it
//! unnecessary.)
//!
//! ## Which text is "verbatim"
//!
//! `a.verbatim_quote` — the complaint's own words. An `Allegation` also carries
//! `summary`, the extractor's condensation, which is what G0's gate fixture
//! froze. L2a.1 says **verbatim**, so this reads `verbatim_quote` and falls back
//! to `summary` only when the verbatim text is missing, so a badly-extracted
//! allegation still contributes something rather than nothing.

use neo4rs::{query, Graph};

use crate::models::document_status::ENTITY_ALLEGATION;
use crate::neo4j::schema;
use crate::services::gather_query::AllegationForQuery;

// STRUCTURAL: Cypher query text is wire vocabulary for the Neo4j protocol, not a
// deployment-variable setting — there is no environment in which this project
// wants to ask a different question here. Held at module scope so the shape
// tests assert against the text that runs.
//
// `ORDER BY toInteger(a.paragraph_number), a.id` is what makes the composed
// query DETERMINISTIC: the composer joins the allegations in the order it
// receives them, and the text it produces is embedded, so an unstable order
// would mean a different vector and a different pool on every run. The `a.id`
// tiebreak covers paragraph numbers that do not parse as integers.
//
// `collect(DISTINCT p.id)` is sorted in Rust rather than in Cypher: Neo4j makes
// no ordering promise about a collect, and the composer needs a stable set.
fn allegations_cypher() -> String {
    format!(
        "MATCH (a) WHERE labels(a)[0] = $allegation_label AND a.id IN $ids \
         OPTIONAL MATCH (a)-[:{about}]->(p) \
         WITH a, collect(DISTINCT p.id) AS party_ids \
         RETURN a.id AS id, \
                a.paragraph_number AS paragraph, \
                coalesce(a.verbatim_quote, a.summary, '') AS text, \
                party_ids AS parties \
         ORDER BY toInteger(a.paragraph_number), a.id",
        about = schema::ABOUT,
    )
}

// STRUCTURAL: how many ids an error message may name before it stops being
// readable in a log line. A legibility bound on the message itself, not a
// deployment setting — no environment wants a different number here, and the
// full list is never lost because the ids come from the caller's own anchors.
const ERROR_ID_SAMPLE: usize = 5;

/// Errors this read can raise, naming the operation, WHICH allegations it was
/// reading, and what an operator should check.
///
/// ## Domain note: the ids are the only handle back to the scenario
///
/// This read is given anchor ids, not a scenario id — so if the error does not
/// carry the ids, a failure in the log cannot be traced back to the scenario
/// whose gather failed. That is the difference between an operator who can act
/// and one who has to go query the graph by hand.
#[derive(Debug, thiserror::Error)]
pub enum GatherQueryReadError {
    #[error(
        "gather query read '{operation}' failed for {id_count} allegation(s) [{sample}]: \
         {source} — check Neo4j is reachable and that the Allegation label still exists"
    )]
    Query {
        operation: &'static str,
        id_count: usize,
        sample: String,
        #[source]
        source: neo4rs::Error,
    },
    #[error(
        "gather query read '{operation}' could not decode field '{field}': {source} — the \
         Allegation node shape may have changed since this query was written"
    )]
    RowDecode {
        operation: &'static str,
        field: &'static str,
        #[source]
        source: neo4rs::DeError,
    },
}

/// Up to [`ERROR_ID_SAMPLE`] ids, comma-joined, with a count of the rest.
///
/// Kept out of the error's `Display` so the truncation is testable on its own
/// and the message stays one line however many anchors a scenario carries.
fn id_sample(ids: &[String]) -> String {
    let shown: Vec<&str> = ids
        .iter()
        .take(ERROR_ID_SAMPLE)
        .map(String::as_str)
        .collect();
    match ids.len().checked_sub(ERROR_ID_SAMPLE) {
        Some(rest) if rest > 0 => format!("{}, +{rest} more", shown.join(", ")),
        _ => shown.join(", "),
    }
}

/// The scenario's linked allegations, ready for the composer.
///
/// ## ⚑ A missing allegation is REPORTED, not silently dropped
///
/// Ids that match no node do not come back. That is a stale-pointer defect —
/// an anchor pointing at an allegation the graph no longer has — and it has a
/// visible consequence: the composed query is shorter, so the pool is thinner,
/// so evidence goes unseen. If this returned quietly, that chain would be
/// invisible from one end to the other.
///
/// So the gap is logged here, naming the ids that vanished, and the returned
/// `Vec` is short by exactly that many. A caller that wants to surface it on
/// the page can compare the lengths; a caller that does not still leaves a
/// trace in the log an operator can act on.
///
/// # Errors
/// Returns [`GatherQueryReadError`] if the query or a row decode fails.
pub async fn allegations_for_query(
    graph: &Graph,
    allegation_ids: &[String],
) -> Result<Vec<AllegationForQuery>, GatherQueryReadError> {
    const OP: &str = "allegations_for_query";

    // An empty anchor list short-circuits: the answer is knowable without a
    // round trip, and it is the `theme_only` case rather than an error.
    if allegation_ids.is_empty() {
        return Ok(Vec::new());
    }

    let fail = |source: neo4rs::Error| GatherQueryReadError::Query {
        operation: OP,
        id_count: allegation_ids.len(),
        sample: id_sample(allegation_ids),
        source,
    };

    let mut stream = graph
        .execute(
            query(&allegations_cypher())
                .param("allegation_label", ENTITY_ALLEGATION)
                .param("ids", allegation_ids.to_vec()),
        )
        .await
        .map_err(fail)?;

    let mut rows = Vec::new();
    while let Some(row) = stream.next().await.map_err(fail)? {
        rows.push(allegation_from_row(&row, OP)?);
    }

    let missing = missing_ids(allegation_ids, &rows);
    if !missing.is_empty() {
        tracing::warn!(
            operation = OP,
            requested = allegation_ids.len(),
            found = rows.len(),
            missing = %missing.join(", "),
            "scenario anchors point at allegations the graph does not have; the composed \
             query is short by that many and the gather will be correspondingly thin"
        );
    }
    Ok(rows)
}

/// One row, decoded into the composer's shape.
///
/// ## Rust Learning: a closure that names the field it was decoding
///
/// Each `row.get` is fallible for its own reason, so the `map_err` closure is
/// built per field rather than once for the row. It costs one line each and
/// buys an error that says `could not decode field 'parties'` instead of
/// `could not decode a row` — the difference between knowing which property
/// changed shape and having to guess.
fn allegation_from_row(
    row: &neo4rs::Row,
    operation: &'static str,
) -> Result<AllegationForQuery, GatherQueryReadError> {
    let decode = |field: &'static str| {
        move |source: neo4rs::DeError| GatherQueryReadError::RowDecode {
            operation,
            field,
            source,
        }
    };

    let id: String = row.get("id").map_err(decode("id"))?;
    let paragraph: Option<String> = row.get("paragraph").map_err(decode("paragraph"))?;
    let mut parties: Vec<String> = row.get("parties").map_err(decode("parties"))?;
    // Sorted here, not in Cypher: `collect` makes no ordering promise, and the
    // composer's party set must be stable across runs.
    parties.sort();

    Ok(AllegationForQuery {
        label: label_for(paragraph.as_deref(), &id),
        id,
        text: row.get("text").map_err(decode("text"))?,
        parties,
    })
}

/// The anchor ids that came back with nothing behind them.
///
/// Pure, so the stale-pointer detection is testable without a graph — which
/// matters, because the defect it detects is exactly the one nobody can
/// reproduce on demand.
fn missing_ids(requested: &[String], found: &[AllegationForQuery]) -> Vec<String> {
    requested
        .iter()
        .filter(|id| !found.iter().any(|a| &a.id == *id))
        .cloned()
        .collect()
}

/// `A-16` when the paragraph is known, else the raw id.
///
/// Display only — the query text is built from the allegation's words, never
/// from its handle. Falling back to the id rather than to an empty label means a
/// paragraph-less allegation is still nameable in a log or a report.
fn label_for(paragraph: Option<&str>, id: &str) -> String {
    match paragraph {
        Some(p) if !p.trim().is_empty() => crate::domain::scenario_code::allegation_code(p.trim()),
        _ => id.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The read cannot write.
    #[test]
    fn the_allegation_read_cannot_write() {
        let cypher = allegations_cypher().to_uppercase();
        for forbidden in ["CREATE", "MERGE", " SET ", "DELETE", "REMOVE", "DETACH"] {
            assert!(
                !cypher.contains(forbidden),
                "the gather query read must not contain {forbidden}"
            );
        }
    }

    /// The read is ORDERED, which is what makes the composed query stable.
    ///
    /// Without it Cypher promises nothing about row order, the composer would
    /// join the same allegations differently on different runs, and the embedded
    /// vector — and therefore the pool — would change for no visible reason.
    #[test]
    fn the_read_is_ordered_so_the_query_is_deterministic() {
        let cypher = allegations_cypher();
        assert!(
            cypher.contains("ORDER BY toInteger(a.paragraph_number), a.id"),
            "paragraph order with an id tiebreak, or the composition is unstable"
        );
    }

    /// It reads the VERBATIM words first, and the label comes from the shared
    /// code helper rather than a hand-built string.
    #[test]
    fn the_read_prefers_the_verbatim_quote() {
        let cypher = allegations_cypher();
        assert!(
            cypher.contains("coalesce(a.verbatim_quote, a.summary, '')"),
            "L2a.1 says verbatim; summary is the extractor's condensation and is only \
             the fallback"
        );
        assert_eq!(label_for(Some("16"), "doc:allegation:x"), "A-16");
    }

    /// A paragraph-less allegation is still nameable.
    #[test]
    fn an_allegation_with_no_paragraph_falls_back_to_its_id() {
        assert_eq!(label_for(None, "doc:allegation:x"), "doc:allegation:x");
        assert_eq!(
            label_for(Some("  "), "doc:allegation:x"),
            "doc:allegation:x"
        );
    }

    /// The stale-pointer gap is DETECTED — the whole reason the warn exists.
    ///
    /// An anchor whose allegation the graph no longer has must be nameable, not
    /// merely absent from the result: a shorter query is a thinner pool, and a
    /// thinner pool is evidence unseen.
    #[test]
    fn an_anchor_with_no_node_behind_it_is_named() {
        let requested = vec![
            "doc:allegation:16".to_string(),
            "doc:allegation:gone".to_string(),
            "doc:allegation:17".to_string(),
        ];
        let found = vec![
            AllegationForQuery {
                id: "doc:allegation:16".to_string(),
                label: "A-16".to_string(),
                text: "x".to_string(),
                parties: Vec::new(),
            },
            AllegationForQuery {
                id: "doc:allegation:17".to_string(),
                label: "A-17".to_string(),
                text: "y".to_string(),
                parties: Vec::new(),
            },
        ];

        assert_eq!(
            missing_ids(&requested, &found),
            vec!["doc:allegation:gone"],
            "the vanished anchor must be nameable, not just absent"
        );
        assert!(
            missing_ids(&requested, &requested_as_rows(&requested)).is_empty(),
            "a complete read reports no gap, so the warn stays quiet on the happy path"
        );
    }

    fn requested_as_rows(ids: &[String]) -> Vec<AllegationForQuery> {
        ids.iter()
            .map(|id| AllegationForQuery {
                id: id.clone(),
                label: id.clone(),
                text: "x".to_string(),
                parties: Vec::new(),
            })
            .collect()
    }

    /// A read failure names WHICH allegations it was reading and what to check.
    ///
    /// This read is handed anchor ids, never a scenario id, so without the ids
    /// in the message a failure in the log cannot be traced back to the
    /// scenario whose gather broke.
    #[test]
    fn a_read_failure_names_the_allegations_and_the_thing_to_check() {
        let rendered = GatherQueryReadError::Query {
            operation: "allegations_for_query",
            id_count: 2,
            sample: id_sample(&["a-16".to_string(), "a-17".to_string()]),
            source: neo4rs::Error::UnknownMessage("bolt closed".to_string()),
        }
        .to_string();

        assert!(rendered.contains("allegations_for_query"), "{rendered}");
        assert!(rendered.contains("2 allegation(s)"), "{rendered}");
        assert!(rendered.contains("a-16, a-17"), "{rendered}");
        assert!(
            rendered.contains("bolt closed"),
            "the cause survives: {rendered}"
        );
        assert!(
            rendered.contains("Neo4j is reachable"),
            "an operator needs to be told what to check: {rendered}"
        );
    }

    /// A long anchor list is sampled, not dumped, and the remainder is counted.
    #[test]
    fn the_id_sample_truncates_and_says_how_many_it_left_out() {
        let many: Vec<String> = (1..=8).map(|n| format!("a-{n}")).collect();
        assert_eq!(id_sample(&many), "a-1, a-2, a-3, a-4, a-5, +3 more");
        assert_eq!(id_sample(&many[..2]), "a-1, a-2");
        assert_eq!(id_sample(&[]), "");
    }

    /// A decode failure names the FIELD, so an operator knows which property
    /// changed shape rather than guessing across four of them.
    #[test]
    fn a_decode_failure_names_the_field_that_would_not_decode() {
        let rendered = GatherQueryReadError::RowDecode {
            operation: "allegations_for_query",
            field: "parties",
            source: neo4rs::DeError::PropertyMissingButRequired,
        }
        .to_string();

        assert!(rendered.contains("field 'parties'"), "{rendered}");
        assert!(
            rendered.contains("Allegation node shape may have changed"),
            "an operator needs the likely cause: {rendered}"
        );
    }

    /// Parties come from the ABOUT edge — the measured linkage — and the label
    /// and relationship type come from the shared constants (Rule 16).
    #[test]
    fn parties_come_from_the_about_edge_via_the_shared_constants() {
        let cypher = allegations_cypher();
        assert!(cypher.contains(&format!("-[:{}]->", schema::ABOUT)));
        assert!(cypher.contains("$allegation_label"));
        assert!(
            !cypher.contains("applies_to"),
            "the ABOUT edge is the linkage; a text property naming a party is exactly \
             what the stop condition guarded against"
        );
    }
}
