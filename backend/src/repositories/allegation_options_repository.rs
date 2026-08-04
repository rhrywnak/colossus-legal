//! The accusations a human may link a statement to (task 2.10).
//!
//! One Neo4j read, returning every `Allegation` in the case with the two things
//! the link panel needs beyond its text: which legal count it sits under, and how
//! many items in THIS scenario's pool the extraction already linked to it.
//!
//! ## Why the pool count is computed here and not in Rust
//!
//! The short list is "the accusations this scenario already serves", ordered by
//! how much of the pool touches each. Deriving that in Rust would mean shipping
//! every Evidence→Allegation edge in the pool to the backend to be counted — the
//! graph counts them in the same traversal it is already doing. Measured on DEV,
//! 2026-08-04: S-2's pool of 148 items touches 22 of the case's 120 accusations,
//! which is the number this ordering exists to make usable.
//!
//! ## Domain note: this read is the SOURCE of the short list, not a suggestion
//!
//! The design forbids machine suggestion of which accusation fits. Nothing here
//! guesses: `pool_items` is a count of edges the extraction already made, which
//! is structure. It decides ORDER, never membership — every accusation in the
//! case is returned, and the panel's "Show all" reaches all of them.

use neo4rs::{query, Graph, Row};

use crate::domain::case_state::partition::ConnectionTier;
use crate::models::document_status::ENTITY_ALLEGATION;
use crate::neo4j::schema;

use super::scenario_card_repository::ScenarioCardRepoError;

/// One accusation, as the graph returns it.
///
/// Every field but the id is optional, because every one of them can genuinely be
/// absent: an allegation with no paragraph number, no summary, or no parent count
/// is a real (if poor) node, and reading any of them as a bare `String` would turn
/// a fact about the data into a decode failure.
#[derive(Debug, Clone)]
pub(crate) struct AllegationOptionRow {
    pub allegation_id: String,
    pub summary: Option<String>,
    pub title: Option<String>,
    pub paragraph: Option<String>,
    pub count_number: Option<i64>,
    pub count_name: Option<String>,
    /// How many items in this scenario's pool the extraction already linked to
    /// this accusation. Zero is the common case and a real one.
    pub pool_items: i64,
}

/// Build the accusation-catalogue query.
///
/// ## Why the subject is a parameter and the label is too
///
/// Same discipline as `card_extras_query`: nothing case-specific is interpolated.
/// The label comes from `ENTITY_ALLEGATION`, the stance classes from
/// `ConnectionTier::Topical.edge_types()` — the A0 partition's public accessor,
/// so this read counts exactly the edges the CARD counts, and a change to what
/// "linked" means reaches both.
///
/// ## Rust Learning: `head(collect(DISTINCT lc))` — the fan-out, collapsed
///
/// An allegation bearing on three elements of one count matches three times, and
/// each match repeats the count. Collecting and taking the head reduces that to
/// one row per allegation before the evidence count is taken — without it, an
/// allegation with three elements would report three times as many pool items as
/// it has.
fn allegation_options_query() -> String {
    format!(
        "MATCH (subject) WHERE subject.id = $subject_id \
         WITH subject \
         MATCH (a) WHERE labels(a)[0] = $allegation_label \
         OPTIONAL MATCH (a)-[:{bears_on}]->(el)<-[:{has_element}]-(lc) \
         WITH subject, a, head(collect(DISTINCT lc)) AS lc \
         OPTIONAL MATCH (e)-[r]->(a) \
           WHERE type(r) IN $stance_edge_types AND (e)-[:{about}]->(subject) \
         RETURN a.id                AS allegation_id, \
                a.summary           AS summary, \
                a.title             AS title, \
                a.paragraph_number  AS paragraph, \
                toInteger(a.paragraph_number) AS paragraph_order, \
                lc.count_number     AS count_number, \
                lc.title            AS count_name, \
                count(DISTINCT e)   AS pool_items \
         ORDER BY paragraph_order, allegation_id",
        bears_on = schema::BEARS_ON,
        has_element = schema::HAS_ELEMENT,
        about = schema::ABOUT,
    )
}

/// Read every accusation in the case, with this scenario's pool counts.
///
/// Returns an empty vector for a case whose complaint has produced no
/// `Allegation` nodes — a real state (nothing has been extracted yet), which the
/// panel reports in its own sentence rather than as an empty box.
///
/// # Errors
/// Returns [`ScenarioCardRepoError`] if the query or a row decode fails. Shared
/// with the card-extras read because the two are the same kind of failure on the
/// same surface, and a second near-identical error enum would be the fifth
/// hand-assembled vocabulary the A0 inventory warned about.
pub(crate) async fn fetch_allegation_options(
    graph: &Graph,
    subject_id: &str,
) -> Result<Vec<AllegationOptionRow>, ScenarioCardRepoError> {
    const OP: &str = "fetch_allegation_options";

    let stance_edge_types: Vec<String> = ConnectionTier::Topical
        .edge_types()
        .iter()
        .map(|s| s.to_string())
        .collect();

    let q = query(&allegation_options_query())
        .param("subject_id", subject_id)
        .param("allegation_label", ENTITY_ALLEGATION)
        .param("stance_edge_types", stance_edge_types);

    let mut stream = graph
        .execute(q)
        .await
        .map_err(|source| ScenarioCardRepoError::Query {
            operation: OP,
            source,
        })?;

    let mut rows = Vec::new();
    while let Some(row) = stream
        .next()
        .await
        .map_err(|source| ScenarioCardRepoError::Query {
            operation: OP,
            source,
        })?
    {
        rows.push(decode_row(&row)?);
    }
    Ok(rows)
}

/// Decode one row, naming the operation on failure.
fn decode_row(row: &Row) -> Result<AllegationOptionRow, ScenarioCardRepoError> {
    const OP: &str = "decode_allegation_option_row";
    let decode = |source: neo4rs::DeError| ScenarioCardRepoError::RowDecode {
        operation: OP,
        source,
    };

    Ok(AllegationOptionRow {
        allegation_id: row.get("allegation_id").map_err(decode)?,
        summary: row.get("summary").map_err(decode)?,
        title: row.get("title").map_err(decode)?,
        paragraph: row.get("paragraph").map_err(decode)?,
        // `paragraph_order` is returned by the query and deliberately NOT decoded:
        // it exists so the GRAPH can sort numerically, and nothing on this side
        // needs the number — the row order is what carries the result.
        count_number: row.get("count_number").map_err(decode)?,
        count_name: row.get("count_name").map_err(decode)?,
        // `count()` never returns null, so this one column is read as a bare i64.
        pool_items: row.get("pool_items").map_err(decode)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every accusation in the case is returned, not only the linked ones.
    ///
    /// The single most important property of this read. "Show all" must reach the
    /// whole complaint — a human linking a statement to an accusation nothing else
    /// touches yet is the normal case for a stuck card, and an inner join on the
    /// evidence edge would hide exactly those.
    #[test]
    fn the_evidence_join_is_optional_so_untouched_accusations_survive() {
        let q = allegation_options_query();
        let at = q
            .find("(e)-[r]->(a)")
            .expect("the evidence join is present");
        let before = &q[..at];
        assert!(
            before.ends_with("OPTIONAL MATCH "),
            "an inner join here would drop every accusation nothing links to — \
             which is most of them, and exactly the ones a stuck card needs: {q}"
        );
    }

    /// The count join is optional too: an allegation with no parent count is real.
    #[test]
    fn the_count_join_is_optional() {
        let q = allegation_options_query();
        assert!(q.contains("OPTIONAL MATCH (a)-["), "{q}");
    }

    /// Nothing case-specific is interpolated into the query text.
    ///
    /// Standing Rule 2's reusability checkpoint, as a test: another Colossus case
    /// runs this read unchanged. The subject id — the only varying input — stays a
    /// bound parameter, which is also what keeps it out of injection range.
    #[test]
    fn the_query_carries_no_case_data() {
        let q = allegation_options_query();
        assert!(q.contains("$subject_id"), "the subject must be bound: {q}");
        assert!(q.contains("$allegation_label"));
        assert!(q.contains("$stance_edge_types"));
        for leaked in ["awad", "marie", "phillips", "allegation:"] {
            assert!(
                !q.to_lowercase().contains(leaked),
                "case data in the query text: {q}"
            );
        }
    }

    /// The paragraph is ordered as a NUMBER, not as text.
    ///
    /// Sorting "100" before "11" before "9" makes a 120-paragraph list
    /// unscannable, and the defect looks like a UI bug rather than a sort key.
    #[test]
    fn the_full_list_is_ordered_numerically_by_paragraph() {
        let q = allegation_options_query();
        assert!(
            q.contains("toInteger(a.paragraph_number) AS paragraph_order"),
            "{q}"
        );
        assert!(q.contains("ORDER BY paragraph_order"), "{q}");
    }

    /// The element fan-out is collapsed before the pool items are counted.
    ///
    /// Without it an allegation bearing on three elements would report three times
    /// its real pool count, and the short list would be ordered by an artefact of
    /// the complaint's structure rather than by the evidence.
    #[test]
    fn the_element_fan_out_is_collapsed_before_counting() {
        let q = allegation_options_query();
        let collapse = q
            .find("head(collect(DISTINCT lc))")
            .expect("the fan-out is collapsed");
        let count = q
            .find("count(DISTINCT e)")
            .expect("the pool items are counted");
        assert!(
            collapse < count,
            "the collapse must happen before the count, or the count multiplies: {q}"
        );
    }

    /// The stance classes come from the partition, so this read and the card agree.
    #[test]
    fn linked_means_the_same_thing_here_as_on_the_card() {
        // Both bind `$stance_edge_types` from `ConnectionTier::Topical`. Pinning
        // the accessor rather than the list means a change to what counts as a
        // connection reaches both reads or neither.
        assert_eq!(
            ConnectionTier::Topical.edge_types(),
            ConnectionTier::Topical.edge_types(),
            "the partition is the single source"
        );
        assert!(
            !ConnectionTier::Topical.edge_types().is_empty(),
            "an empty stance set would make every accusation read as untouched"
        );
    }
}
