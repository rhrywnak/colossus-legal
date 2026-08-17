//! Deliberate collapse of duplicate entities within one ingest run.
//!
//! This is the module `evidence_key`'s header has pointed at since the stable-id
//! arm shipped ("see `ingest_dedupe` for the door-side cure"). It did not exist
//! until now; the reference was a promise, and this is it.
//!
//! ## What was happening instead
//!
//! Two extraction items that resolve to the same Evidence id are the same
//! statement extracted twice — measured live on 2026-08-17: **21 pairs, 42 items**
//! (Phillips discovery 13, CFS interrogatory 7, the 12-15-2009 hearing 1). Before
//! this module, both were written: `MERGE (n:Evidence {id: $id})` matched the
//! node the first item had just created, the ON MATCH arm overwrote `title`,
//! `verbatim_quote` and `grounding_status`, and the per-property loop overwrote
//! everything else. **The second item won every field, silently, and nothing
//! counted it.** The step's `entities_written` counted two, the graph held one,
//! and no log line said so.
//!
//! ## The ruling this implements (2026-08-14, re-affirmed 2026-08-17)
//!
//! Exact twins are extraction duplicates, so: **one node, occurrence count +1,
//! and the FIRST item wins.** First is defined as lowest `extraction_items.id` —
//! the loops iterate in `ORDER BY id`, so "first seen" and "lowest id" are the
//! same row, and the outcome does not depend on fetch order.
//!
//! ## Why first-wins rather than last-wins
//!
//! Neither is more "correct" about the mooded fields — the two items differ only
//! in prose the key deliberately ignores. What matters is that the choice is
//! DETERMINISTIC and stated: re-ingesting the same document twice must produce
//! the same node, and an operator asking "whose text is on this node?" must have
//! an answer that does not depend on which row Postgres handed back first.
//! Last-wins was the old accidental behaviour precisely because nobody chose it.
//!
//! ## What this does NOT do
//!
//! It does not merge across runs or across documents — `merge_evidence_twins` is
//! the tool for the 21 pairs already in the graph, and 7 of those carry curated
//! rows on BOTH twins, which is a human ruling and not a collapse. This module
//! only stops a single run from writing the same node twice.

use std::collections::HashMap;

use neo4rs::query;

use crate::error::AppError;

/// What the writer should do with one item.
///
/// ## Rust Learning: an enum return instead of a `bool`
///
/// `bool` would force the caller's reader to remember which way round `true`
/// meant, and it could not carry the first writer's item id — which is the one
/// piece of information the INFO log needs to be actionable ("collapsed onto
/// which row?"). Two variants, two behaviours, and the payload rides with the
/// variant that needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Disposition {
    /// First sighting of this id in this run. Write the node.
    Write,
    /// Already written by an earlier item. Skip the write, keep the mapping.
    Collapse {
        /// `extraction_items.id` of the row that won.
        first_item_id: i32,
    },
}

/// One entity id seen during a run.
#[derive(Debug, Clone)]
struct Entry {
    /// The Neo4j label, needed to stamp the count without a label-less scan.
    entity_type: String,
    first_item_id: i32,
    occurrences: usize,
}

/// Which ids a run has written, and how many items claimed each.
///
/// One per ingest run, threaded through the entity loop. Deliberately holds no
/// database handle: the decisions are pure and unit-testable, and the single
/// method that talks to Neo4j ([`DuplicateLedger::flush`]) is called once, after
/// the loop.
#[derive(Debug, Default)]
pub struct DuplicateLedger {
    seen: HashMap<String, Entry>,
}

impl DuplicateLedger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one item against its computed node id and say what to do with it.
    ///
    /// The caller passes the id `stable_entity_id` produced, so this module never
    /// computes a key itself — one derivation, no chance of the ledger and the
    /// writer disagreeing about what "the same node" means.
    pub fn observe(&mut self, neo4j_id: &str, entity_type: &str, item_id: i32) -> Disposition {
        match self.seen.get_mut(neo4j_id) {
            Some(entry) => {
                entry.occurrences += 1;
                Disposition::Collapse {
                    first_item_id: entry.first_item_id,
                }
            }
            None => {
                self.seen.insert(
                    neo4j_id.to_string(),
                    Entry {
                        entity_type: entity_type.to_string(),
                        first_item_id: item_id,
                        occurrences: 1,
                    },
                );
                Disposition::Write
            }
        }
    }

    /// How many item writes were skipped — items minus nodes.
    ///
    /// This is the number the step summary reports. It is NOT the number of
    /// duplicated nodes: a hypothetical id claimed by three items collapses two
    /// writes onto one node.
    pub fn collapsed_writes(&self) -> usize {
        self.seen.values().map(|e| e.occurrences - 1).sum()
    }

    /// How many distinct nodes ended up carrying more than one item.
    pub fn duplicated_nodes(&self) -> usize {
        self.seen.values().filter(|e| e.occurrences > 1).count()
    }

    /// Stamp `duplicate_count` on every node more than one item claimed.
    ///
    /// ## Why the count is written once at the end, not incremented per collapse
    ///
    /// An increment (`SET n.duplicate_count = coalesce(n.duplicate_count, 1) + 1`)
    /// is only correct if the node was freshly created by this run. Ingest is
    /// cleanup-then-write today, so that holds — but it holds by a property of a
    /// DIFFERENT function, and if that ever changed, re-ingesting a document
    /// would ratchet the count up forever with nothing to notice it. Writing the
    /// final absolute value once is idempotent regardless: run it twice, get the
    /// same number.
    ///
    /// Nodes claimed by exactly one item are left alone rather than stamped with
    /// `1` — an absent property and a `1` mean the same thing, and not writing
    /// 500 properties to say "nothing happened" keeps the ingest transaction
    /// small.
    pub async fn flush(&self, txn: &mut neo4rs::Txn) -> Result<usize, AppError> {
        let mut stamped = 0usize;
        for (neo4j_id, entry) in &self.seen {
            if entry.occurrences <= 1 {
                continue;
            }
            // The label is interpolated because a Neo4j label cannot be a bind
            // parameter. It comes from `extraction_items.entity_type`, which
            // `create_entity_node` has already validated as alphanumeric — the
            // same discipline that function applies to its own MERGE.
            let cypher = format!(
                "MATCH (n:{} {{id: $id}}) SET n.duplicate_count = $count",
                entry.entity_type
            );
            txn.run(
                query(&cypher)
                    .param("id", neo4j_id.as_str())
                    .param("count", entry.occurrences as i64),
            )
            .await
            .map_err(|e| AppError::Internal {
                message: format!(
                    "Failed to stamp duplicate_count={} on {} '{}': {e}. The node exists \
                     and carries the first item's fields; only the occurrence count is \
                     missing.",
                    entry.occurrences, entry.entity_type, neo4j_id
                ),
            })?;
            stamped += 1;
        }
        Ok(stamped)
    }
}

#[cfg(test)]
#[path = "ingest_dedupe_tests.rs"]
mod tests;
