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

use neo4rs::{query, Graph};
use thiserror::Error;

use super::aggregation::{AggregationState, BiasRow};
use super::dto::{ActorOption, AvailableFilters, BiasInstance, BiasQueryFilters};

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
    /// Three separate queries run against Neo4j and resolve concurrently:
    ///
    /// 1. Actors that STATED_BY at least one tagged Evidence node, with
    ///    per-actor counts. Sorted by count DESC, then name ASC, so the
    ///    most active actor appears first in the dropdown.
    /// 2. Distinct, trimmed pattern tags across all tagged Evidence nodes.
    ///    Sorted alphabetically.
    /// 3. Subjects that are the target of an ABOUT edge from a tagged
    ///    Evidence node, with per-subject counts. Same ordering as actors.
    ///
    /// After the three fetches, `default_subject_name` (sourced from the
    /// `CASE_DEFAULT_SUBJECT_NAME` env var) is matched against the
    /// subjects list to compute `default_subject_id`. Doing the match
    /// server-side keeps case-specific data — the plaintiff's name —
    /// out of the JS bundle (Standing Rule 2).
    ///
    /// All Cypher hardcodes the schema labels (`Evidence`) and edges
    /// (`STATED_BY`, `ABOUT`) — that is consistent with the rest of the
    /// repositories and reflects the data model contract; the prohibition
    /// on hardcoding applies to environment-specific values, not to the
    /// graph's own type names.
    ///
    /// ## Rust Learning: `tokio::try_join!` for fail-fast parallelism
    ///
    /// `try_join!` polls multiple futures concurrently; when any of them
    /// returns `Err`, the macro returns that error and drops the others.
    /// We use it (rather than the non-fallible `join!`) so a Neo4j outage
    /// surfaces as a single error from this function instead of three
    /// independent failures. The futures don't compete for the same
    /// connection — `neo4rs::Graph` has an internal connection pool and
    /// hands out a fresh session per `.execute()` call.
    pub async fn available_filters(
        &self,
        default_subject_name: Option<&str>,
    ) -> Result<AvailableFilters, BiasRepositoryError> {
        let (actors, pattern_tags, subjects) = tokio::try_join!(
            self.fetch_actors_with_tagged_statements(),
            self.fetch_distinct_pattern_tags(),
            self.fetch_subjects_with_tagged_statements(),
        )?;

        let default_subject_id = resolve_default_subject_id(&subjects, default_subject_name);

        Ok(AvailableFilters {
            actors,
            pattern_tags,
            subjects,
            default_subject_id,
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

    /// Cypher: every subject that is the target of an ABOUT edge from a
    /// tagged Evidence node, with how many distinct Evidence nodes are
    /// "about" them.
    ///
    /// The shape mirrors `fetch_actors_with_tagged_statements` (same
    /// `ActorOption` shape, same sort order) so the frontend can render
    /// the About dropdown identically to the Speaker dropdown. The
    /// difference is the relationship traversed (`[:ABOUT]` instead of
    /// `[:STATED_BY]`) and `count(DISTINCT e)` — a single Evidence may
    /// have multiple ABOUT targets, and we want each Evidence counted
    /// once per subject, not once per ABOUT edge.
    async fn fetch_subjects_with_tagged_statements(
        &self,
    ) -> Result<Vec<ActorOption>, BiasRepositoryError> {
        let cypher = "
            MATCH (e:Evidence)-[:ABOUT]->(subject)
            WHERE e.pattern_tags IS NOT NULL AND e.pattern_tags <> ''
            WITH subject, count(DISTINCT e) AS tagged_count
            RETURN subject.id AS id,
                   subject.name AS name,
                   labels(subject)[0] AS actor_type,
                   tagged_count
            ORDER BY tagged_count DESC, subject.name ASC
        ";

        let mut result = self.graph.execute(query(cypher)).await?;
        let mut subjects: Vec<ActorOption> = Vec::new();

        while let Some(row) = result.next().await? {
            let id: String = row.get("id").unwrap_or_default();
            let name: String = row.get("name").unwrap_or_default();
            let actor_type: String = row.get("actor_type").unwrap_or_default();
            let tagged_statement_count: i64 = row.get("tagged_count").unwrap_or(0);

            subjects.push(ActorOption {
                id,
                name,
                actor_type,
                tagged_statement_count,
            });
        }

        Ok(subjects)
    }

    /// Count the unfiltered total of tagged Evidence nodes in the graph.
    ///
    /// Used by the "Filtered: X of Y" counter so the user knows how many
    /// total instances exist regardless of current filter selections.
    /// Cheap because Neo4j caches a single label-and-property scan.
    async fn count_total_unfiltered(&self) -> Result<i64, BiasRepositoryError> {
        let cypher = "
            MATCH (e:Evidence)
            WHERE e.pattern_tags IS NOT NULL AND e.pattern_tags <> ''
            RETURN count(e) AS total_unfiltered
        ";

        let mut result = self.graph.execute(query(cypher)).await?;
        if let Some(row) = result.next().await? {
            return Ok(row.get::<i64>("total_unfiltered").unwrap_or(0));
        }
        Ok(0)
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

    /// Run the structured bias query with optional actor, pattern, and
    /// subject filters, and report the total unfiltered count alongside.
    ///
    /// The filtered query and the total-unfiltered count run **concurrently**
    /// via `tokio::try_join!` (see the `available_filters` doc comment for
    /// the rationale). Both queries hit the same Neo4j instance through
    /// the pooled `Graph` handle.
    ///
    /// The filter Cypher is parameterised — values flow in via
    /// `query.param(...)` so none of `actor_id`, `pattern_tag`, or
    /// `subject_id` is ever interpolated into the Cypher string. That is
    /// what makes the endpoint safe against Cypher injection.
    ///
    /// ## Why `EXISTS { ... }` for the subject filter
    ///
    /// The OPTIONAL MATCH `(e)-[:ABOUT]->(subject)` below pulls **every**
    /// ABOUT subject for display on each card. If we filtered subjects
    /// with a constraining MATCH (e.g. `MATCH (e)-[:ABOUT]->(target {id: $subject_id})`),
    /// the displayed `about` list would silently shrink to just the
    /// matching subject — Marie's name would appear on the card, but
    /// Phillips and CFS, who the same Evidence is also about, would
    /// vanish from the rendered "About:" line. EXISTS is a presence
    /// check that does not bind into the result, so the displayed
    /// subjects stay complete. (Standing Rule 1: "Marie present alongside
    /// other subjects" must remain observable.)
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
    /// `total_count` is the deduped count of distinct Evidence nodes the
    /// filter matched. `total_unfiltered` is the count of all tagged
    /// Evidence regardless of filters.
    pub async fn run_query(
        &self,
        filters: &BiasQueryFilters,
    ) -> Result<(i64, i64, Vec<BiasInstance>), BiasRepositoryError> {
        let (filtered, total_unfiltered) = tokio::try_join!(
            self.execute_filtered_query(filters),
            self.count_total_unfiltered(),
        )?;
        let (total_count, instances) = filtered;
        Ok((total_count, total_unfiltered, instances))
    }

    /// Execute just the filtered Cypher and return `(total_count, instances)`.
    ///
    /// Split out from `run_query` so the parallel total-unfiltered count
    /// can run alongside it under `tokio::try_join!`. Carries no public
    /// surface — `run_query` is the only caller.
    async fn execute_filtered_query(
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
            WITH e, actor
            WHERE $subject_id IS NULL
               OR EXISTS {
                    MATCH (e)-[:ABOUT]->(s)
                    WHERE s.id = $subject_id
               }
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
            .param("pattern_tag", filters.pattern_tag.as_deref())
            .param("subject_id", filters.subject_id.as_deref());

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

// ─── Pure helpers (no I/O — easy to unit-test) ──────────────────────────────

/// Resolve the default-subject id from a list of subjects and an optional
/// configured name.
///
/// - `name = None` (env var unset) → `None`. The frontend renders
///   "All subjects" as the default.
/// - `name = Some("")` after trimming → `None` (treat empty config the
///   same as unset).
/// - `name = Some(n)` and exactly one subject in the list has
///   `subject.name == n` → `Some(subject.id)`.
/// - `name = Some(n)` and **multiple** subjects share that name → return
///   the first match (the input is sorted by `tagged_count DESC`, then by
///   name) and emit a `tracing::warn!` so the duplicate is observable.
/// - `name = Some(n)` and no subject matches → `None`, plus a
///   `tracing::warn!` noting the configured name was not found in the
///   current data. (Standing Rule 1: configured-but-missing must be
///   distinguishable from unset.)
///
/// Match is case-sensitive exact equality. Case-insensitive matching
/// would be friendlier but would create surprises when two case-specific
/// names share a prefix differing only in case — exact-match keeps the
/// contract obvious.
pub(crate) fn resolve_default_subject_id(
    subjects: &[ActorOption],
    name: Option<&str>,
) -> Option<String> {
    let configured = match name {
        Some(n) if !n.trim().is_empty() => n,
        _ => return None,
    };

    let matches: Vec<&ActorOption> = subjects.iter().filter(|s| s.name == configured).collect();

    match matches.as_slice() {
        [] => {
            tracing::warn!(
                configured_name = configured,
                "bias.available_filters: CASE_DEFAULT_SUBJECT_NAME did not match any subject; \
                 frontend will default to All subjects"
            );
            None
        }
        [only] => Some(only.id.clone()),
        many => {
            tracing::warn!(
                configured_name = configured,
                duplicates = many.len(),
                "bias.available_filters: CASE_DEFAULT_SUBJECT_NAME matched multiple subjects; \
                 picking first by sort order"
            );
            Some(many[0].id.clone())
        }
    }
}
