//! Grouping the orphaned scenario references by document (task 1.7C, defect D9).
//!
//! ## What an orphan is, and why it must never disappear
//!
//! A `scenario_fact_refs` row stores a `graph_node_id` pointing into Neo4j. There
//! is no foreign key — there cannot be, across two databases — so a ruling can
//! outlive the node it ruled on. Measured on DEV 2026-07-29 and confirmed
//! 2026-08-03: **all 26** of scenario S-2's refs are dead pointers, including six
//! human INCLUDE decisions. The documents were re-extracted on 2026-07-24/25,
//! which regenerated the Evidence nodes under new ids (a Family-A id embeds
//! `sha256(item_data)[..8]`, so the same fact re-extracted gets a different id).
//!
//! Those 26 are the July loss surfacing honestly, and they are the origin of the
//! durability law. The orphan guarantee is that a confirmed fact never silently
//! vanishes — so they are shown, plainly, rather than filtered out of a list whose
//! count would then quietly disagree with the database.
//!
//! ## Why the grouping is computed HERE and not in the browser
//!
//! Design §2.8 wants the strip grouped by document title. The refs carry only an
//! id — but the id is a Family-A composite that ENCODES the document:
//!
//! ```text
//! doc-george-phillips-response-to-discovery:evidence:5fd83f22
//! └────────────── document id ─────────────┘         └ item hash
//! ```
//!
//! Measured: all 26 of 26 parse on `:evidence:`. Parsing that in the frontend
//! would be the browser deriving provenance from an id, and it could not resolve a
//! title at all — the titles live in Postgres. So the parse, the `documents`
//! lookup and the label all happen on this side of the wire (v2 §7 item 2, and
//! Standing Rule 12).
//!
//! ## The two refs whose title is gone, and why they get a slug
//!
//! Measured grouping of the live 26: **16** under
//! `doc-george-phillips-response-to-discovery`, **8** under
//! `doc-cfs-interrogatory-response-08-08-16`, **2** under
//! `doc-court-of-appeals-ruling-01-12-2012`. The first two resolve to titles in
//! `documents`. The third does not — and the reason is spelling forensics worth
//! recording, because it looks like a bug and is not:
//!
//! > The live document is `doc-court-of-appeals-**rulling**-01-12-2012` — double
//! > `l`, the misspelling OCR produced from the cover page. The two orphan refs
//! > point at a single-`l` `ruling` id, so they predate the re-titling as well as
//! > the re-extraction. They are not the same document id with a stale hash; they
//! > are a document id that no longer exists.
//!
//! For those, §2.8 is explicit: report honestly, fall back to whatever the data
//! supports, and **do not invent titles**. They get a humanised slug derived from
//! the id itself, flagged so the strip can say the title is unrecoverable.

use std::collections::HashMap;

use crate::dto::scenario_orphans::{OrphanGroup, OrphanTitleState, ScenarioOrphansResponse};

/// The separator that splits a Family-A evidence id into document and item parts.
///
// CONST: the Family-A id grammar, not a tunable. Changing it would not
// reconfigure anything — it would mean the ids themselves had changed shape, which
// is a data migration, not a setting.
const EVIDENCE_ID_SEPARATOR: &str = ":evidence:";

/// The document id carried by an evidence node id, if the id has that shape.
///
/// ## Rust Learning: `split_once` returns the pair, not an iterator
///
/// `str::split_once(sep)` gives `Option<(&str, &str)>` — the text before and after
/// the FIRST separator, or `None` when the separator is absent. That is exactly the
/// question here ("does this id have a document prefix, and what is it?"), and it
/// answers it without allocating a `Vec` of parts or indexing into one, which is
/// where an off-by-one would hide.
///
/// Returns `None` for an id this build does not recognise. That is a real state
/// rather than a defect — a future id family may look different — and the caller
/// surfaces it as its own group instead of dropping the ref (Standing Rule 1: a
/// count that quietly excludes what it cannot parse is a lying count).
fn document_id_of(graph_node_id: &str) -> Option<&str> {
    let (document_id, _item) = graph_node_id.split_once(EVIDENCE_ID_SEPARATOR)?;
    if document_id.is_empty() {
        return None;
    }
    Some(document_id)
}

/// A readable label for a document whose title is not in `documents`.
///
/// `doc-court-of-appeals-ruling-01-12-2012` → `court-of-appeals-ruling-01-12-2012`.
///
/// Deliberately NOT title-cased or prettified into something that could be
/// mistaken for a real title: it keeps the id's own hyphens so a reader can see
/// this is a derived handle, not a document name somebody typed. §2.8: no invented
/// titles.
fn slug_label(document_id: &str) -> String {
    document_id
        .strip_prefix("doc-")
        .unwrap_or(document_id)
        .to_string()
}

/// Group orphaned refs by document, resolving titles where they still exist.
///
/// * `orphan_node_ids` — the refs whose graph node no longer resolves.
/// * `titles` — `document_id` → `title`, as read from `documents`. A document id
///   absent from this map is one that no longer exists.
///
/// Groups come back ordered by descending count, then by label, so the biggest
/// loss reads first and the order is stable across requests (a `HashMap`'s own
/// iteration order is not).
///
/// Pure — no I/O — so the grouping, the fallback and the ordering are all unit
/// tested without a database or a graph.
pub fn group_orphans(
    orphan_node_ids: &[String],
    titles: &HashMap<String, String>,
) -> ScenarioOrphansResponse {
    // Fold the refs into one entry per document, keeping the ids so the caller can
    // still name individual refs if it ever needs to.
    let mut by_document: HashMap<String, Vec<String>> = HashMap::new();
    let mut unparseable: Vec<String> = Vec::new();

    for node_id in orphan_node_ids {
        match document_id_of(node_id) {
            Some(document_id) => by_document
                .entry(document_id.to_string())
                .or_default()
                .push(node_id.clone()),
            None => unparseable.push(node_id.clone()),
        }
    }

    let mut groups: Vec<OrphanGroup> = by_document
        .into_iter()
        .map(|(document_id, node_ids)| {
            let (label, title_state) = match titles.get(&document_id) {
                Some(title) => (title.clone(), OrphanTitleState::Resolved),
                // The document id itself is gone from `documents` — see the module
                // doc's spelling forensics for the two real instances of this.
                None => (slug_label(&document_id), OrphanTitleState::Unrecoverable),
            };
            OrphanGroup {
                document_id,
                label,
                title_state,
                count: node_ids.len(),
            }
        })
        .collect();

    if !unparseable.is_empty() {
        // Kept as its own group rather than merged or dropped: "the id does not
        // parse" is a different fault from "the document is gone", and collapsing
        // them would hide a change in the id grammar.
        groups.push(OrphanGroup {
            document_id: String::new(),
            label: "references whose document cannot be identified".to_string(),
            title_state: OrphanTitleState::Unparseable,
            count: unparseable.len(),
        });
    }

    // Biggest loss first, then alphabetical — deterministic, unlike HashMap order.
    groups.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.label.cmp(&b.label)));

    ScenarioOrphansResponse {
        total: orphan_node_ids.len(),
        groups,
    }
}

/// The distinct document ids referenced by a set of orphan node ids.
///
/// The caller uses this to ask Postgres for exactly the titles it needs, rather
/// than reading the whole `documents` table.
pub fn document_ids_of(orphan_node_ids: &[String]) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for node_id in orphan_node_ids {
        if let Some(document_id) = document_id_of(node_id) {
            if !seen.iter().any(|existing| existing == document_id) {
                seen.push(document_id.to_string());
            }
        }
    }
    seen
}

#[cfg(test)]
#[path = "scenario_orphans_tests.rs"]
mod tests;
