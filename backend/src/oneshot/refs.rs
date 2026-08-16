//! Every Postgres column that stores a graph node id — measured, not assumed.
//!
//! ## Why this is a constant with a test and not eight hand-written statements
//!
//! A one-shot tool that moves or merges a graph node id has to move EVERY row
//! that points at it. A column added to the schema and forgotten by a tool leaves
//! rows pointing at an id that no longer exists — and the count proof cannot see
//! what it does not know to count, so the run reports success. The registry is a
//! constant so that one list drives every count, every update and every proof,
//! and a test pins its membership so a change is deliberate.
//!
//! ## Measured on DEV 2026-08-15 (read-only)
//!
//! The inventory below came from `information_schema.columns` filtered to
//! plausible id-bearing names, then each candidate's contents were sampled to see
//! which id FAMILY it actually holds. Names lie; contents do not.
//!
//! The last column is what a re-key run actually moves: rows on the 483 nodes it
//! re-keys, which excludes the rows sitting on the 42 twins it refuses.
//!
//! | Column | Rows | Evidence ids | Party ids | Re-keyed |
//! |---|---|---|---|---|
//! | `scenario_candidate_ordinals.graph_node_id` | 444 | 444 | 0 | 402 |
//! | `scan_run_verdicts.graph_node_id` | 226 | 226 | 0 | 202 |
//! | `scenario_ruling_anchors.graph_node_id` | 167 | 167 | 0 | 141 |
//! | `evidence_allegation_link_events.graph_node_id` | 37 | 37 | 0 | 31 |
//! | `scenario_fact_refs.graph_node_id` | 35 | 35 | 0 | 27 |
//! | `scenario_human_facts.anchor_graph_node_id` | 18 | 18 | 0 | 16 |
//! | `evidence_allegation_links.graph_node_id` | 11 | 11 | 0 | 9 |
//! | `scenario_human_facts.answers_graph_node_id` | 9 | 9 | 0 | 7 |
//! | `evidence_summary_overrides.graph_node_id` | 0 | 0 | 0 | 0 |
//! | `response_item_fact_refs.graph_node_id` | 0 | 0 | 0 | 0 |
//! | `extraction_items.neo4j_node_id` | 849 | **525** | **133** | **483** |
//! | **TOTAL** | | **1,472** | | **1,318** |
//!
//! Two things in that table were not in the re-key's eight, and both matter:
//!
//! 1. `evidence_summary_overrides` and `response_item_fact_refs` are real curated
//!    surfaces that are merely EMPTY today. Empty is a fact about this Tuesday,
//!    not a property of the schema, so they are in the registry — a tool that
//!    counted them at zero and moved on is correct today and wrong the first time
//!    Roman writes a summary override.
//! 2. `extraction_items.neo4j_node_id` holds 525 Evidence ids and 133 party ids
//!    and is READ — `lookup_neo4j_node_ids` uses it to resolve cross-document
//!    references at ingest, and pass-2 prefers it over re-resolving. It is
//!    pipeline provenance rather than curated state, which is presumably why it
//!    was not in the re-key's original list, but a stale id there is still a
//!    stale id.
//!
//! All eleven are the registry, and **every tool walks all eleven** — the
//! re-key included, ruled 2026-08-16. It originally knew only the eight that
//! were POPULATED when Phase A measured, which left 483 rows in
//! `extraction_items` pointing at ids it had just changed. There is now one
//! list, not two, and [`REKEY_UPDATES_EVERYTHING`] plus its test say so in a
//! form that fails the build if it ever stops being true.
//!
//! ## Re-running the sweep
//!
//! The inventory is a MEASUREMENT, so it has a date and a query. To confirm it
//! still holds, or after adding a table:
//!
//! ```sql
//! SELECT table_name, column_name FROM information_schema.columns
//!  WHERE table_schema = 'public'
//!    AND (column_name LIKE '%graph_node%' OR column_name LIKE '%node_id%'
//!         OR column_name LIKE '%entity_id%' OR column_name LIKE '%person%'
//!         OR column_name LIKE '%party%')
//!  ORDER BY 1, 2;
//! ```
//!
//! Names lie, so each candidate's CONTENTS were then sampled for `%:evidence:%`
//! and
//! `person-%` / `org-%` prefixes. `authored_entities`, `authored_relationships`,
//! `scenario_human_facts.person_refs` and `scan_run_merges.selected_node_ids`
//! all matched the name filter and hold NEITHER family; they are recorded in
//! [`SWEPT_AND_EXCLUDED`] so a future sweep does not have to rediscover that.
//!
//! ## Party ids
//!
//! Measured, not assumed, exactly as the P7 addendum required: **no curated
//! Postgres table references a party id today.** `authored_entities.entity_id`
//! (40 rows) and `authored_relationships` (243 rows) hold neither Evidence nor
//! party ids; `scenario_human_facts.person_refs` has one row holding an empty
//! array; `scan_run_merges.selected_node_ids` has no rows at all. The one live
//! party reference in Postgres is `extraction_items.neo4j_node_id`. So a party
//! merge is *almost* graph-only — and "almost" is the whole reason the tool walks
//! this list and proves the zeros rather than skipping Postgres.

use std::collections::HashMap;

use sqlx::{Postgres, Transaction};

/// One column that stores a graph node id.
///
/// ## Rust Learning: why `&'static str` and not `String`
///
/// These values are compiled in and never built at run time, so they need no
/// allocation and no ownership. `&'static str` says exactly that: a string slice
/// that lives for the whole program. It also makes the SQL construction below
/// safe to reason about — a table name that cannot come from input cannot carry
/// an injection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferencingColumn {
    pub table: &'static str,
    pub column: &'static str,
}

impl ReferencingColumn {
    /// `table.column` — the form every proof line and every log field uses.
    pub fn reference(&self) -> String {
        format!("{}.{}", self.table, self.column)
    }
}

/// Shorthand so the registries below read as data rather than as constructor
/// calls.
const fn col(table: &'static str, column: &'static str) -> ReferencingColumn {
    ReferencingColumn { table, column }
}

/// Every column that can hold an `Evidence` graph node id.
///
/// Ordered most-populated first, so a proof's first lines are its biggest
/// numbers and an operator skimming a report sees the load-bearing counts
/// immediately.
pub const EVIDENCE_REFERENCES: &[ReferencingColumn] = &[
    col("scenario_candidate_ordinals", "graph_node_id"),
    col("scan_run_verdicts", "graph_node_id"),
    col("scenario_ruling_anchors", "graph_node_id"),
    col("evidence_allegation_link_events", "graph_node_id"),
    col("scenario_fact_refs", "graph_node_id"),
    col("scenario_human_facts", "anchor_graph_node_id"),
    col("evidence_allegation_links", "graph_node_id"),
    col("scenario_human_facts", "answers_graph_node_id"),
    col("evidence_summary_overrides", "graph_node_id"),
    col("response_item_fact_refs", "graph_node_id"),
    col("extraction_items", "neo4j_node_id"),
];

/// The subset of [`EVIDENCE_REFERENCES`] that carries a HUMAN RULING.
///
/// Domain note: this is the distinction the twin-merge turns on. A row in
/// `scenario_fact_refs` is Roman deciding a statement carries a scenario; a row
/// in `extraction_items` is the pipeline recording where it put a node. Both must
/// be repointed, but only the first makes a twin un-mergeable without him —
/// which is why the two lists are separate and neither is derived from the other
/// by a naming rule that a future table could break.
pub const EVIDENCE_CURATED_REFERENCES: &[ReferencingColumn] = &[
    col("scenario_candidate_ordinals", "graph_node_id"),
    col("scan_run_verdicts", "graph_node_id"),
    col("scenario_ruling_anchors", "graph_node_id"),
    col("evidence_allegation_link_events", "graph_node_id"),
    col("scenario_fact_refs", "graph_node_id"),
    col("scenario_human_facts", "anchor_graph_node_id"),
    col("evidence_allegation_links", "graph_node_id"),
    col("scenario_human_facts", "answers_graph_node_id"),
    col("evidence_summary_overrides", "graph_node_id"),
    col("response_item_fact_refs", "graph_node_id"),
];

/// Every column that can hold a `Person` or `Organization` graph node id.
///
/// One entry, measured. See the module header for what was checked and found
/// empty — the short list is a finding, not an omission.
pub const PARTY_REFERENCES: &[ReferencingColumn] = &[col("extraction_items", "neo4j_node_id")];

/// Every tool walks [`EVIDENCE_REFERENCES`] in full, `rekey_evidence` included.
///
/// This replaced a `REKEY_OMITS` list that recorded three columns the re-key did
/// not update. Ruled 2026-08-16: it updates all of them, so the list of
/// exceptions is gone rather than shortened, and the flag below is what a test
/// asserts instead. Keeping a `const` here rather than nothing at all is
/// deliberate — it gives the test something to name, and it gives anyone
/// tempted to add an exception a place where the refusal is written down.
pub const REKEY_UPDATES_EVERYTHING: bool = true;

/// Columns the 2026-08-15 sweep surfaced by NAME and excluded by CONTENT.
///
/// Recorded so a later sweep does not have to re-derive that these hold neither
/// an Evidence id nor a party id. If any of them starts carrying one, it belongs
/// in a registry above and this entry comes out.
///
/// - `authored_entities.entity_id` — 40 rows, hand-authored Tier-1 ids
/// - `authored_relationships.from_entity_id` / `.to_entity_id` — 243 rows each
/// - `scenario_human_facts.person_refs` — 1 row, an empty array
/// - `scan_run_merges.selected_node_ids` — no rows
pub const SWEPT_AND_EXCLUDED: &[ReferencingColumn] = &[
    col("authored_entities", "entity_id"),
    col("authored_relationships", "from_entity_id"),
    col("authored_relationships", "to_entity_id"),
    col("scenario_human_facts", "person_refs"),
    col("scan_run_merges", "selected_node_ids"),
];

/// Count the rows each column holds for these ids, inside the caller's
/// transaction.
///
/// ## Rust Learning: why the SQL is built with `format!` and the ids are bound
///
/// A table or column name cannot be a bind parameter in Postgres — the planner
/// needs it before it sees the values — so the identifiers are interpolated. That
/// is only safe because they come from the compiled-in registry above and can
/// never come from input. The VALUES, which always can, are bound: `= ANY($1)`
/// takes the whole id list as one parameter.
pub async fn count_rows(
    tx: &mut Transaction<'_, Postgres>,
    columns: &[ReferencingColumn],
    ids: &[String],
) -> Result<HashMap<String, u64>, sqlx::Error> {
    let mut counts = HashMap::new();
    for c in columns {
        let sql = format!(
            "SELECT count(*) FROM {} WHERE {} = ANY($1)",
            c.table, c.column
        );
        let count: i64 = sqlx::query_scalar(&sql)
            .bind(ids)
            .fetch_one(&mut **tx)
            .await?;
        // A negative count is impossible from `count(*)`; the cast is the
        // narrowing sqlx's i64 return forces, not a silent clamp.
        counts.insert(c.reference(), count.max(0) as u64);
    }
    Ok(counts)
}

/// Repoint every row from an old id to a new one, returning what each column
/// actually changed.
///
/// One statement per (column, move) rather than a bulk `CASE` update, because the
/// per-column total is the number the proof is checked against and a bulk update
/// would only report a grand total. This is a handful of documents' worth of
/// rows, not a hot path.
pub async fn repoint(
    tx: &mut Transaction<'_, Postgres>,
    columns: &[ReferencingColumn],
    moves: &[(String, String)],
) -> Result<HashMap<String, u64>, sqlx::Error> {
    let mut updated = HashMap::new();
    for c in columns {
        let sql = format!(
            "UPDATE {} SET {} = $1 WHERE {} = $2",
            c.table, c.column, c.column
        );
        let mut rows = 0u64;
        for (old, new) in moves {
            let result = sqlx::query(&sql)
                .bind(new)
                .bind(old)
                .execute(&mut **tx)
                .await?;
            rows += result.rows_affected();
        }
        updated.insert(c.reference(), rows);
    }
    Ok(updated)
}

/// What one referencing column did for one unit of work.
///
/// ## Why this lives here and not in each tool's report module
///
/// All four tools count the same thing the same way — rows that pointed at an old
/// id before the write, against rows the write actually changed — and all four
/// abort on the same comparison. Three identical copies of this struct and its
/// `is_sound` was exactly the duplication that drifts: one of them gets a `>=`
/// and nobody notices until a proof lies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableProof {
    /// `table.column`, because two of the eleven are on one table.
    pub reference: String,
    /// Rows that pointed at an old id BEFORE the write.
    pub expected: u64,
    /// Rows the UPDATE actually changed.
    pub updated: u64,
}

impl TableProof {
    /// Whether the write did exactly what the pre-count said it would.
    ///
    /// Domain note: equality, not "at least". A row count HIGHER than expected
    /// means the UPDATE matched something the plan did not know about, which is
    /// as much of a failure as missing rows — and the reason every abort is on
    /// `!=` rather than `<`.
    pub fn is_sound(&self) -> bool {
        self.expected == self.updated
    }
}

/// Pair the before-counts with the after-counts, one proof per column.
///
/// Walks the REGISTRY rather than either map, so a column that produced no row
/// in either is still proved — as a `0 / 0`, which is a claim, where an absent
/// line would be a silence.
pub fn table_proofs(
    columns: &[ReferencingColumn],
    expected: &HashMap<String, u64>,
    updated: &HashMap<String, u64>,
) -> Vec<TableProof> {
    columns
        .iter()
        .map(|c| {
            let reference = c.reference();
            TableProof {
                expected: expected.get(&reference).copied().unwrap_or(0),
                updated: updated.get(&reference).copied().unwrap_or(0),
                reference,
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "refs_tests.rs"]
mod tests;
