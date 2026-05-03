//! Bias Explorer — row aggregation.
//!
//! `execute_filtered_query` in the repository runs a Cypher that returns
//! one row per (Evidence, ABOUT-subject) tuple. The types in this module
//! collapse those flat rows into one `BiasInstance` per Evidence with a
//! deduplicated `about` list, in a stable display order.
//!
//! Lives in its own file to keep `repository.rs` focused on Cypher and
//! orchestration. The aggregation logic is pure (no I/O); only the public
//! `parse_pattern_tags` helper is exposed across modules.

use std::collections::{BTreeMap, HashMap, HashSet};

use neo4rs::Row;

use super::dto::{ActorOption, BiasInstance, DocumentRef};

/// One row from `execute_filtered_query`'s Cypher, in extracted-but-
/// unaggregated form.
///
/// The Cypher returns one row per (Evidence, ABOUT-subject) pair. We
/// collect each row into this flat struct, then `AggregationState::absorb`
/// merges rows that share an evidence_id into a single `BiasInstance`.
pub(crate) struct BiasRow {
    evidence_id: String,
    title: String,
    verbatim_quote: Option<String>,
    page_number: Option<i64>,
    pattern_tags_raw: Option<String>,
    actor_id: String,
    actor_name: String,
    actor_type: String,
    subject: Option<ActorOption>,
    document: Option<DocumentRef>,
}

impl BiasRow {
    /// Map a single neo4rs Row to a `BiasRow`, returning None when the row
    /// has no usable evidence_id (the only field whose absence makes the
    /// row meaningless).
    pub(crate) fn from_row(row: &Row) -> Option<Self> {
        let evidence_id: String = row.get("evidence_id").unwrap_or_default();
        if evidence_id.is_empty() {
            tracing::warn!("bias.run_query: skipped row with empty evidence_id");
            return None;
        }
        let document_id: Option<String> = row.get("document_id").ok();
        let document = document_id.map(|id| DocumentRef {
            id,
            title: row.get::<String>("document_title").unwrap_or_default(),
            document_type: row.get::<String>("document_type").ok(),
        });

        let subject = match (
            row.get::<String>("subject_id").ok(),
            row.get::<String>("subject_name").ok(),
            row.get::<String>("subject_type").ok(),
        ) {
            (Some(id), Some(name), Some(actor_type)) => Some(ActorOption {
                id,
                name,
                actor_type,
                tagged_statement_count: 0,
            }),
            _ => None,
        };

        Some(Self {
            evidence_id,
            title: row.get("title").unwrap_or_default(),
            verbatim_quote: row.get("verbatim_quote").ok(),
            page_number: row.get("page_number").ok(),
            pattern_tags_raw: row.get("pattern_tags_raw").ok(),
            actor_id: row.get("actor_id").unwrap_or_default(),
            actor_name: row.get("actor_name").unwrap_or_default(),
            actor_type: row.get("actor_type").unwrap_or_default(),
            subject,
            document,
        })
    }
}

/// Aggregation state for `execute_filtered_query`.
///
/// Keeps a sort-ordered map of evidence_id → BiasInstance plus a
/// per-evidence dedupe set so that an Evidence with multiple ABOUT edges
/// produces exactly one entry with each subject listed once.
pub(crate) struct AggregationState {
    /// Sorted by `(actor_name, document_title, page_number, evidence_id)`
    /// to mirror the Cypher's ORDER BY.
    by_evidence: BTreeMap<SortKey, BiasInstance>,
    /// Subject ids already merged into each evidence_id's `about` list.
    seen_about: HashMap<String, HashSet<String>>,
}

type SortKey = (String, String, i64, String);

impl AggregationState {
    pub(crate) fn new() -> Self {
        Self {
            by_evidence: BTreeMap::new(),
            seen_about: HashMap::new(),
        }
    }

    pub(crate) fn absorb(&mut self, row: BiasRow) {
        let sort_key: SortKey = (
            row.actor_name.clone(),
            row.document
                .as_ref()
                .map(|d| d.title.clone())
                .unwrap_or_default(),
            row.page_number.unwrap_or(0),
            row.evidence_id.clone(),
        );

        let evidence_id = row.evidence_id.clone();
        let entry = self
            .by_evidence
            .entry(sort_key)
            .or_insert_with(|| BiasInstance {
                evidence_id: row.evidence_id,
                title: row.title,
                verbatim_quote: row.verbatim_quote,
                page_number: row.page_number,
                pattern_tags: parse_pattern_tags(row.pattern_tags_raw.as_deref().unwrap_or("")),
                stated_by: Some(ActorOption {
                    id: row.actor_id,
                    name: row.actor_name,
                    actor_type: row.actor_type,
                    // Per-card surface doesn't show counts; this stays at 0.
                    tagged_statement_count: 0,
                }),
                about: Vec::new(),
                document: row.document,
            });

        if let Some(subject) = row.subject {
            let dedupe = self.seen_about.entry(evidence_id).or_default();
            if dedupe.insert(subject.id.clone()) {
                entry.about.push(subject);
            }
        }
    }

    pub(crate) fn finish(self) -> (i64, Vec<BiasInstance>) {
        let instances: Vec<BiasInstance> = self.by_evidence.into_values().collect();
        let total = instances.len() as i64;
        (total, instances)
    }
}

// ─── Pure helpers (no I/O — easy to unit-test) ──────────────────────────────

/// Parse a comma-separated pattern_tags string into a clean `Vec<String>`.
///
/// - Splits on `,`
/// - Trims each token of leading/trailing whitespace
/// - Drops empty tokens (common when extraction emits `"a, , b"` or trailing
///   commas)
/// - Preserves ordering so the UI can display tags in the order the
///   extraction template authored them
///
/// This is the same transformation `available_filters` does in Cypher, but
/// applied per-Evidence-node when building `BiasInstance.pattern_tags`.
pub(crate) fn parse_pattern_tags(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect()
}
