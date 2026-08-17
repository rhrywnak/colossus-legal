//! Which Postgres columns hold a graph node id — the registries every reader of
//! the graph's Postgres side shares.
//!
//! ## Why this is in `models` and not with the tools that use it most
//!
//! These are `(table, column)` facts about the schema. The four one-shot
//! maintenance tools use them heavily, so they lived in `oneshot::refs` — but
//! when a production HTTP handler (`api/pipeline/curated_rows.rs`, the
//! re-extraction guard) needed the curated list, that made a live API surface
//! depend on the batch-maintenance layer, and a future reshuffle of `oneshot`
//! would have broken an endpoint with no warning from the type system.
//!
//! The data is neutral, so it lives somewhere neutral. `oneshot::refs`
//! re-exports it, and every existing tool import keeps working unchanged.

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
pub const fn col(table: &'static str, column: &'static str) -> ReferencingColumn {
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
