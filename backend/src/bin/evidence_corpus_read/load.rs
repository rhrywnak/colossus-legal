//! Reading the two stores. Every statement here is a `MATCH` or a `SELECT`.
//!
//! The queries are `pub const` so `output::write_queries` can print the exact
//! text that produced each number — the instruction requires every count to sit
//! beside the query that produced it, and a query retyped into a document is a
//! query that can drift from the one that ran.

use anyhow::{Context, Result};
use colossus_legal_backend::neo4j::schema;
use neo4rs::{query, Graph};
use sqlx::{PgPool, Row};
use std::collections::{HashMap, HashSet};

use crate::model::Card;

/// STOP 0, part one: the property keys actually present, with non-null counts.
pub const Q_STOP0_KEYS: &str =
    "MATCH (e:Evidence) UNWIND keys(e) AS k RETURN k AS property_key, count(*) AS n \
     ORDER BY n DESC, property_key";

/// STOP 0, part two: outgoing relationship types by far-end label.
pub const Q_STOP0_RELS: &str =
    "MATCH (e:Evidence)-[r]->(x) RETURN type(r) AS rel_type, labels(x)[0] AS other_label, \
     count(*) AS n ORDER BY n DESC";

/// STOP 0, part three: the total that must be 1,209.
pub const Q_STOP0_TOTAL: &str = "MATCH (e:Evidence) RETURN count(e) AS n";

/// The one read that builds every card.
///
/// ## Why one query and not twelve
///
/// Each bucket could have had its own `MATCH`, and the report would then be
/// twelve counts taken at twelve different instants against a database somebody
/// else may be writing. One read produces one consistent snapshot, and every
/// bucket is computed from it in memory — so the numbers add up to 1,209 by
/// construction rather than by luck.
///
/// The two `OPTIONAL MATCH`es are separated by label, not by relationship type,
/// because STOP 0 showed `ABOUT` reaching Person, Organization AND Allegation.
const Q_CARDS_TEMPLATE: &str = "\
MATCH (e:Evidence)
OPTIONAL MATCH (e)-[:{ABOUT}]->(party)
  WHERE party:Person OR party:Organization
WITH e, collect(DISTINCT party) AS parties
OPTIONAL MATCH (e)-[r]->(a:Allegation)
  WHERE type(r) IN [{ALLEGATION_RELS}]
WITH e, parties, count(DISTINCT a) AS allegation_count
OPTIONAL MATCH (e)-[:{CONTAINED_IN}]->(d:Document)
WITH e, parties, allegation_count, count(DISTINCT d) AS document_node_count
RETURN e.id AS id,
       e.source_document AS source_document,
       e.page_number AS page_number,
       e.verbatim_quote AS quote,
       e.title AS title,
       e.question AS question,
       e.statement_type AS statement_type,
       e.grounding_status AS grounding_status,
       size(parties) AS party_count,
       size([p IN parties WHERE p.name IS NULL OR trim(p.name) = '']) AS unnamed_party_count,
       allegation_count,
       document_node_count
ORDER BY id";

/// B4 by document, for the report's breakdown.
pub const Q_GROUNDING_BY_DOC: &str =
    "MATCH (e:Evidence) WHERE e.grounding_status IN ['unverified', 'derived'] \
     RETURN e.source_document AS document, e.grounding_status AS status, count(*) AS n \
     ORDER BY n DESC";

/// The documents table — B9's page bound and B10's existence test.
pub const Q_DOCUMENTS: &str = "SELECT id, title, page_count FROM documents";

/// B11's provenance join. One row per graph node id that an extraction item
/// claims to have written.
pub const Q_PROVENANCE: &str = "\
SELECT i.neo4j_node_id AS node_id, r.template_name AS template_name, r.model_name AS model_name
FROM extraction_items i
JOIN extraction_runs r ON r.id = i.run_id
WHERE i.neo4j_node_id IS NOT NULL";

/// B12's existence test. Asked before the table is read, so a missing table is a
/// reported fact rather than a caught error.
pub const Q_MIRROR_EXISTS: &str =
    "SELECT count(*) AS n FROM information_schema.tables WHERE table_name = 'evidence_search'";

/// The card query, with the allegation relationship list interpolated from
/// [`crate::buckets::ALLEGATION_RELS`].
///
/// ## Why this is built rather than written out
///
/// The four types are documented in one place and used in another. Interpolating
/// them means the two CANNOT disagree — adding a fifth allegation-bearing type to
/// the constant changes the query in the same edit. A hand-written literal would
/// let the constant say five and the query ask for four, and B6 would
/// under-count forever with nothing to show for it.
pub fn q_cards() -> String {
    let list = crate::buckets::ALLEGATION_RELS
        .iter()
        .map(|r| format!("'{r}'"))
        .collect::<Vec<String>>()
        .join(", ");
    // Rule 12: the two relationship names are interpolated from `neo4j::schema`
    // rather than typed here, so a rename in the schema module reaches this
    // query with no edit. `.replace` rather than `format!` because the template
    // also carries `{ALLEGATION_RELS}`, which `format!` would try to resolve as
    // an argument name.
    Q_CARDS_TEMPLATE
        .replace("{ALLEGATION_RELS}", &list)
        .replace("{ABOUT}", schema::ABOUT)
        .replace("{CONTAINED_IN}", schema::CONTAINED_IN)
}

/// Load every Evidence card from the graph.
///
/// ## Rust Learning: `neo4rs` rows are dynamically typed
///
/// `row.get::<T>("name")` returns `Result` because the driver cannot know the
/// property's type until it arrives. `.ok()` on an OPTIONAL property is correct —
/// absent and wrong-typed both mean "no usable value here" — but `.ok()` on a
/// REQUIRED one would be the silent failure standing Rule 1 forbids, so `id` and
/// the other twelve-of-twelve properties use `?` and name the card in the error.
pub async fn load_cards(graph: &Graph) -> Result<Vec<Card>> {
    let cypher = q_cards();
    let mut stream = graph
        .execute(query(&cypher))
        .await
        .context("running the card query against Neo4j")?;
    let mut cards = Vec::new();
    while let Some(row) = stream
        .next()
        .await
        .context("reading the next card row from Neo4j")?
    {
        let id: String = row.get("id").context("a card row carried no `id`")?;
        cards.push(Card {
            source_document: row.get("source_document").unwrap_or_default(),
            // best-effort: an OPTIONAL graph property — absent and wrong-typed
            // both mean "no usable value", which B9 then reports as unresolvable.
            page_number: row.get::<i64>("page_number").ok(),
            quote: row.get("quote").unwrap_or_default(),
            title: row.get("title").unwrap_or_default(),
            // best-effort: 842 of 1,209 cards carry no question at all.
            question: row.get::<String>("question").ok(),
            // best-effort: absent on cards the extractor did not classify.
            statement_type: row.get::<String>("statement_type").ok(),
            grounding_status: row.get("grounding_status").unwrap_or_default(),
            party_count: row.get("party_count").unwrap_or(0),
            unnamed_party_count: row.get("unnamed_party_count").unwrap_or(0),
            allegation_count: row.get("allegation_count").unwrap_or(0),
            document_node_count: row.get("document_node_count").unwrap_or(0),
            doc_page_count: None,
            doc_row_exists: false,
            template_name: None,
            model_name: None,
            id,
        });
    }
    Ok(cards)
}

/// Run a `key, count` shaped STOP 0 query and return the pairs.
pub async fn count_pairs(graph: &Graph, cypher: &str, keys: &[&str]) -> Result<Vec<(String, i64)>> {
    let mut stream = graph
        .execute(query(cypher))
        .await
        .with_context(|| format!("running STOP 0 query: {cypher}"))?;
    let mut out = Vec::new();
    while let Some(row) = stream.next().await.context("reading a STOP 0 row")? {
        let label = keys
            .iter()
            // best-effort: a STOP 0 row names one or two key columns; the ones
            // it does not carry are simply not part of this row's label.
            .filter_map(|k| row.get::<String>(k).ok())
            .collect::<Vec<String>>()
            .join(" -> ");
        let n: i64 = row.get("n").context("a STOP 0 row carried no count")?;
        out.push((label, n));
    }
    Ok(out)
}

/// The single-number STOP 0 total.
pub async fn total_evidence(graph: &Graph) -> Result<i64> {
    let mut stream = graph
        .execute(query(Q_STOP0_TOTAL))
        .await
        .context("running the Evidence total query")?;
    let row = stream
        .next()
        .await
        .context("reading the Evidence total")?
        .context("the Evidence total query returned no row")?;
    row.get("n")
        .context("the Evidence total row carried no `n`")
}

/// Load the documents table as `id -> page_count`.
///
/// The page count is the only column either bucket needs — B9 bounds the page
/// against it, B10 asks only whether the key is present at all — so the map
/// carries that and nothing else rather than a struct of fields nobody reads.
pub async fn load_documents(pool: &PgPool) -> Result<HashMap<String, Option<i64>>> {
    let rows = sqlx::query(Q_DOCUMENTS)
        .fetch_all(pool)
        .await
        .context("selecting from documents")?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let id: String = r.get("id");
            // best-effort: a NULL page_count is a real state — B9 treats an
            // unknown bound as "cannot condemn the page" rather than an error.
            (id, r.try_get::<i32, _>("page_count").ok().map(i64::from))
        })
        .collect())
}

/// Load the template/model provenance, keyed by graph node id.
pub async fn load_provenance(
    pool: &PgPool,
) -> Result<HashMap<String, (Option<String>, Option<String>)>> {
    let rows = sqlx::query(Q_PROVENANCE)
        .fetch_all(pool)
        .await
        .context("joining extraction_items to extraction_runs")?;
    let mut out = HashMap::new();
    for r in rows {
        let node_id: String = r.get("node_id");
        out.insert(
            node_id,
            (
                // best-effort: either being absent is exactly what B11 counts —
                // a card whose provenance the extraction run never recorded.
                r.try_get::<String, _>("template_name").ok(),
                // best-effort: see above.
                r.try_get::<String, _>("model_name").ok(),
            ),
        );
    }
    Ok(out)
}

/// Does the `evidence_search` mirror exist? Returns the ok-id set when it does.
pub async fn load_mirror(pool: &PgPool) -> Result<Option<HashSet<String>>> {
    let exists: i64 = sqlx::query(Q_MIRROR_EXISTS)
        .fetch_one(pool)
        .await
        .context("asking information_schema whether evidence_search exists")?
        .get("n");
    if exists == 0 {
        return Ok(None);
    }
    let rows = sqlx::query(
        "SELECT evidence_id FROM evidence_search WHERE probe_text IS NOT NULL AND btrim(probe_text) <> ''",
    )
    .fetch_all(pool)
    .await
    .context("reading evidence_search")?;
    Ok(Some(
        rows.into_iter().map(|r| r.get::<String, _>(0)).collect(),
    ))
}

/// Fold the Postgres facts into the cards.
pub fn widen(
    cards: &mut [Card],
    documents: &HashMap<String, Option<i64>>,
    provenance: &HashMap<String, (Option<String>, Option<String>)>,
) {
    for card in cards.iter_mut() {
        if let Some(page_count) = documents.get(&card.source_document) {
            card.doc_row_exists = true;
            card.doc_page_count = *page_count;
        }
        if let Some((template, model)) = provenance.get(&card.id) {
            card.template_name.clone_from(template);
            card.model_name.clone_from(model);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rule 21, in miniature: the relationship set is documented in ONE place
    /// (`buckets::ALLEGATION_RELS`) and used in another (the Cypher literal). A
    /// test that reads the query text is what stops the two from drifting — add a
    /// fifth allegation-bearing type to the constant and forget the query, and
    /// this fails rather than silently under-counting B6 forever.
    #[test]
    fn the_card_query_matches_the_documented_allegation_relationships() {
        let cypher = q_cards();
        for rel in crate::buckets::ALLEGATION_RELS {
            assert!(
                cypher.contains(&format!("'{rel}'")),
                "the card query does not mention {rel}, but ALLEGATION_RELS does"
            );
        }
        assert!(
            !cypher.contains("{ALLEGATION_RELS}"),
            "the placeholder was not filled"
        );
        // And the party side must stay label-based, never type-based: `ABOUT`
        // reaches Person, Organization AND Allegation (STOP 0), so a party count
        // taken from the type alone would silently include allegations.
        assert!(cypher.contains("party:Person OR party:Organization"));
    }

    #[test]
    fn every_query_is_a_read() {
        for (name, text) in [
            ("q_cards()", q_cards().as_str()),
            ("Q_STOP0_KEYS", Q_STOP0_KEYS),
            ("Q_STOP0_RELS", Q_STOP0_RELS),
            ("Q_STOP0_TOTAL", Q_STOP0_TOTAL),
            ("Q_GROUNDING_BY_DOC", Q_GROUNDING_BY_DOC),
            ("Q_DOCUMENTS", Q_DOCUMENTS),
            ("Q_PROVENANCE", Q_PROVENANCE),
            ("Q_MIRROR_EXISTS", Q_MIRROR_EXISTS),
        ] {
            let upper = text.to_uppercase();
            for forbidden in [
                "CREATE ",
                "MERGE ",
                "DELETE ",
                "DETACH ",
                " SET ",
                "INSERT ",
                "UPDATE ",
                "DROP ",
                "ALTER ",
                "TRUNCATE ",
            ] {
                assert!(
                    !upper.contains(forbidden),
                    "{name} contains {forbidden:?} — this tool is read-only"
                );
            }
        }
    }
}
