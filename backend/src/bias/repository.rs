//! Bias Explorer — Neo4j repository.
//!
//! All Cypher for the two bias endpoints lives here. This module is the
//! only place that talks to Neo4j on behalf of the Bias Explorer; the
//! handlers stay thin wrappers around it.
//!
//! ## Why these queries
//!
//! Pattern tags are stored as a comma-separated string on each
//! `Evidence` node (`e.pattern_tags`). The graph schema does not promote
//! tags to first-class nodes, which keeps document ingestion simple but
//! pushes the parsing into every consumer. We do that parsing in two
//! places below — `available_filters` distinct-tag listing, and the
//! optional `pattern_tag` filter inside `run_query` — using `split` and
//! `trim` so leading/trailing whitespace and accidental empty tokens
//! never surface to the UI.

use std::collections::{BTreeMap, HashMap, HashSet};

use neo4rs::{query, Graph, Row};
use thiserror::Error;

use super::dto::{ActorOption, AvailableFilters, BiasInstance, BiasQueryFilters, DocumentRef};

// ─── Error ──────────────────────────────────────────────────────────────────

/// Errors emitted by the bias repository.
///
/// ## Rust Learning: thiserror::Error with #[from] and #[source]
///
/// `thiserror` lets us derive a typed error with one variant per failure
/// mode. `#[from]` auto-implements `From<E>` for that variant, so `?`
/// propagates `neo4rs::Error` and `neo4rs::DeError` upward without any
/// boilerplate. `#[source]` (implied by `#[from]` here) keeps the
/// underlying error in the chain so `tracing::error!(error = ?e, ...)`
/// shows the full cause and `e.source()` walks back to it.
#[derive(Debug, Error)]
pub enum BiasRepositoryError {
    /// Neo4j driver error — connection, query syntax, server response.
    #[error("Neo4j query failed: {0}")]
    Neo4j(#[from] neo4rs::Error),

    /// Row deserialization error — schema drift, missing field, bad type.
    #[error("Neo4j row deserialization failed: {0}")]
    Deserialize(#[from] neo4rs::DeError),
}

// ─── Repository ─────────────────────────────────────────────────────────────

/// Read-only repository for bias-related Cypher queries.
///
/// Holds a cloned `neo4rs::Graph`. The driver internally pools connections,
/// so cloning the handle is cheap and every handler gets its own clone via
/// `AppState::graph.clone()`.
#[derive(Clone)]
pub struct BiasRepository {
    graph: Graph,
}

impl BiasRepository {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }

    /// Return the dropdown contents for the Bias Explorer filter bar.
    ///
    /// Two separate queries run against Neo4j:
    ///
    /// 1. Actors that STATED_BY at least one tagged Evidence node, with
    ///    per-actor counts. Sorted by count DESC, then name ASC, so the
    ///    most active actor appears first in the dropdown.
    /// 2. Distinct, trimmed pattern tags across all tagged Evidence nodes.
    ///    Sorted alphabetically.
    ///
    /// Both queries hardcode the schema labels (`Evidence`) and edges
    /// (`STATED_BY`) — that is consistent with the rest of the
    /// repositories and reflects the data model contract; the prohibition
    /// on hardcoding applies to environment-specific values, not to the
    /// graph's own type names.
    pub async fn available_filters(&self) -> Result<AvailableFilters, BiasRepositoryError> {
        let actors = self.fetch_actors_with_tagged_statements().await?;
        let pattern_tags = self.fetch_distinct_pattern_tags().await?;

        Ok(AvailableFilters {
            actors,
            pattern_tags,
        })
    }

    /// Cypher: every actor that STATED_BY a tagged Evidence node.
    ///
    /// Note the lack of a label on `actor` in the MATCH — this lets the
    /// query pick up Persons, Organizations, or any future Actor-shaped
    /// label (Court, Agency, ...) without code changes. `labels(actor)[0]`
    /// returns the first (and, by convention in this schema, only) label
    /// as a string, which the frontend renders as the "Person" /
    /// "Organization" tag in the dropdown.
    async fn fetch_actors_with_tagged_statements(
        &self,
    ) -> Result<Vec<ActorOption>, BiasRepositoryError> {
        let cypher = "
            MATCH (e:Evidence)-[:STATED_BY]->(actor)
            WHERE e.pattern_tags IS NOT NULL AND e.pattern_tags <> ''
            WITH actor, count(e) AS tagged_count
            RETURN actor.id AS id,
                   actor.name AS name,
                   labels(actor)[0] AS actor_type,
                   tagged_count
            ORDER BY tagged_count DESC, actor.name ASC
        ";

        let mut result = self.graph.execute(query(cypher)).await?;
        let mut actors: Vec<ActorOption> = Vec::new();

        while let Some(row) = result.next().await? {
            // ## Rust Learning: row.get returns Result, .ok() converts to Option.
            // For required fields we use `.unwrap_or_default()` to keep parsing
            // resilient against the rare case of a node with a missing id (which
            // would indicate corrupt data, not a code bug). Per Rule 1, we still
            // surface that as an empty-string id rather than crashing — the UI
            // will visibly fail to navigate, which is the observable signal.
            let id: String = row.get("id").unwrap_or_default();
            let name: String = row.get("name").unwrap_or_default();
            let actor_type: String = row.get("actor_type").unwrap_or_default();
            let tagged_statement_count: i64 = row.get("tagged_count").unwrap_or(0);

            actors.push(ActorOption {
                id,
                name,
                actor_type,
                tagged_statement_count,
            });
        }

        Ok(actors)
    }

    /// Cypher: distinct trimmed pattern tags across all tagged Evidence.
    ///
    /// `split(e.pattern_tags, ',')` returns a list of raw tokens. `UNWIND`
    /// flattens them into rows; `trim()` strips surrounding whitespace
    /// (defensively — the extraction templates produce clean tags, but
    /// human review can leave stray spaces). The final filter drops any
    /// token that becomes empty after trimming.
    async fn fetch_distinct_pattern_tags(&self) -> Result<Vec<String>, BiasRepositoryError> {
        let cypher = "
            MATCH (e:Evidence)
            WHERE e.pattern_tags IS NOT NULL AND e.pattern_tags <> ''
            UNWIND split(e.pattern_tags, ',') AS raw_tag
            WITH trim(raw_tag) AS tag
            WHERE tag <> ''
            RETURN DISTINCT tag
            ORDER BY tag
        ";

        let mut result = self.graph.execute(query(cypher)).await?;
        let mut tags: Vec<String> = Vec::new();

        while let Some(row) = result.next().await? {
            if let Ok(tag) = row.get::<String>("tag") {
                tags.push(tag);
            }
        }

        Ok(tags)
    }

    /// Run the structured bias query with optional actor and pattern filters.
    ///
    /// The query is parameterised — values flow in via `query.param(...)` so
    /// neither `actor_id` nor `pattern_tag` is ever interpolated into the
    /// Cypher string. That is what makes the endpoint safe against Cypher
    /// injection.
    ///
    /// ## Result aggregation
    ///
    /// The Cypher returns one row per (Evidence, ABOUT-subject) pair. We
    /// group rows by `evidence_id` in Rust to build one `BiasInstance` per
    /// matching Evidence, with all distinct ABOUT subjects collected into
    /// the `about` list. (Doing the aggregation here rather than in Cypher
    /// keeps the query simple and avoids `collect()` semantics interacting
    /// poorly with multiple OPTIONAL MATCHes.)
    ///
    /// `total_count` is the deduped count of distinct Evidence nodes, not
    /// the raw row count.
    pub async fn run_query(
        &self,
        filters: &BiasQueryFilters,
    ) -> Result<(i64, Vec<BiasInstance>), BiasRepositoryError> {
        let cypher = "
            MATCH (e:Evidence)
            WHERE e.pattern_tags IS NOT NULL AND e.pattern_tags <> ''
            MATCH (e)-[:STATED_BY]->(actor)
            WHERE $actor_id IS NULL OR actor.id = $actor_id
            WITH e, actor
            WHERE $pattern_tag IS NULL
               OR ANY(t IN split(e.pattern_tags, ',') WHERE trim(t) = $pattern_tag)
            OPTIONAL MATCH (e)-[:ABOUT]->(subject)
            OPTIONAL MATCH (e)-[:CONTAINED_IN]->(d:Document)
            RETURN
              e.id AS evidence_id,
              coalesce(e.title, '') AS title,
              e.verbatim_quote AS verbatim_quote,
              e.page_number AS page_number,
              e.pattern_tags AS pattern_tags_raw,
              actor.id AS actor_id,
              coalesce(actor.name, '') AS actor_name,
              labels(actor)[0] AS actor_type,
              subject.id AS subject_id,
              subject.name AS subject_name,
              CASE WHEN subject IS NULL THEN NULL ELSE labels(subject)[0] END AS subject_type,
              d.id AS document_id,
              d.title AS document_title,
              d.document_type AS document_type
            ORDER BY actor.name, coalesce(d.title, ''), coalesce(e.page_number, 0)
        ";

        // ## Rust Learning: parameter binding with neo4rs
        // `.param(name, value)` binds a value the driver will substitute at
        // execution time. `Option<&str>` becomes a Cypher NULL when the
        // option is None, which is exactly the semantics our WHERE clauses
        // rely on (`$actor_id IS NULL OR ...`).
        let q = query(cypher)
            .param("actor_id", filters.actor_id.as_deref())
            .param("pattern_tag", filters.pattern_tag.as_deref());

        let mut result = self.graph.execute(q).await?;
        let mut state = AggregationState::new();

        while let Some(row) = result.next().await? {
            let extracted = match BiasRow::from_row(&row) {
                Some(r) => r,
                None => continue,
            };
            state.absorb(extracted);
        }

        Ok(state.finish())
    }
}

// ─── Row aggregation ─────────────────────────────────────────────────────────

/// One row from `run_query`'s Cypher, in extracted-but-unaggregated form.
///
/// The Cypher returns one row per (Evidence, ABOUT-subject) pair. We collect
/// each row into this flat struct, then `AggregationState::absorb` merges
/// rows that share an evidence_id into a single `BiasInstance`.
struct BiasRow {
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
    fn from_row(row: &Row) -> Option<Self> {
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

/// Aggregation state for `run_query`.
///
/// Keeps a sort-ordered map of evidence_id → BiasInstance plus a per-evidence
/// dedupe set so that an Evidence with multiple ABOUT edges produces exactly
/// one entry with each subject listed once.
struct AggregationState {
    /// Sorted by `(actor_name, document_title, page_number, evidence_id)`
    /// to mirror the Cypher's ORDER BY.
    by_evidence: BTreeMap<SortKey, BiasInstance>,
    /// Subject ids already merged into each evidence_id's `about` list.
    seen_about: HashMap<String, HashSet<String>>,
}

type SortKey = (String, String, i64, String);

impl AggregationState {
    fn new() -> Self {
        Self {
            by_evidence: BTreeMap::new(),
            seen_about: HashMap::new(),
        }
    }

    fn absorb(&mut self, row: BiasRow) {
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

    fn finish(self) -> (i64, Vec<BiasInstance>) {
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
