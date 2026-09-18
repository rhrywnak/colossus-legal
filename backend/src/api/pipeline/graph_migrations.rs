//! Neo4j schema migrations — uniqueness constraints for entity nodes.
//!
//! ## Why uniqueness constraints are required
//!
//! MERGE semantics only provide safe idempotency when a uniqueness constraint
//! exists on the MERGE key. Without a constraint, two concurrent transactions
//! can both check for an existing node, both find none, and both CREATE —
//! producing duplicates even with MERGE syntax.
//!
//! This is documented in the Neo4j Knowledge Base ("Understanding how MERGE
//! works") and is why neo4j-graphrag-python creates an index automatically
//! before any writes (`__entity__tmp_internal_id`).
//!
//! ## Why at startup, not in a migration file
//!
//! Neo4j constraints are not managed by sqlx migrations (those are for
//! PostgreSQL). Running them at startup with IF NOT EXISTS is idempotent
//! and ensures the constraints exist before the first ingest attempt.
//! This pattern is used by production systems that manage Neo4j schema
//! as part of application initialization.

use neo4rs::Graph;

use crate::canonical_elements::cypher::{
    BREACH_THEORY_LABEL, DECLARATION_LABEL, IMPROPER_ACT_THEORY_LABEL,
};
use crate::neo4j::schema::{
    ABOUT, BEARS_ON, CAUSED_BY, CHARACTERIZES, CONTAINED_IN, CORROBORATES, DAMAGES_FOR,
    DERIVED_FROM, EVIDENCED_BY, HAS_ELEMENT, HAS_THEORY, REBUTS, SEEKS_DECLARATION, STATED_BY,
    SUFFERED_BY,
};

use crate::models::document_status::{
    ENTITY_COMPLAINT_ALLEGATION, ENTITY_DOCUMENT, ENTITY_HARM, ENTITY_LEGAL_COUNT,
    ENTITY_ORGANIZATION, ENTITY_PERSON,
};

/// Run all Neo4j schema constraints at application startup.
///
/// Safe to call repeatedly — all constraints use IF NOT EXISTS.
/// Logs success or failure for each constraint.
///
/// ## Constraint scope
///
/// We create constraints for every entity type produced by the extraction
/// pipeline plus the Tier-1 nodes written by the canonical Element loader
/// (`load_canonical_elements`). Each entry names the MERGE-key property
/// explicitly: most types key on `id`, but `BreachTheory` / `ImproperActTheory`
/// MERGE on `key` (see [`crate::canonical_elements::cypher`]). Without these
/// constraints, concurrent operations could produce duplicate nodes.
pub async fn run_graph_migrations(graph: &Graph) {
    // (Neo4j label, constraint name, MERGE-key property). Constraint names are
    // persisted in the database, so changing one is a migration; pair them with
    // the label explicitly rather than deriving from PascalCase to keep this
    // stable. The key property is carried per-row because it is not always `id`.
    let constraints: &[(&str, &str, &str)] = &[
        (ENTITY_DOCUMENT, "document_id_unique", "id"),
        (ENTITY_PERSON, "person_id_unique", "id"),
        (ENTITY_ORGANIZATION, "organization_id_unique", "id"),
        (
            ENTITY_COMPLAINT_ALLEGATION,
            "complaint_allegation_id_unique",
            "id",
        ),
        (ENTITY_LEGAL_COUNT, "legal_count_id_unique", "id"),
        (ENTITY_HARM, "harm_id_unique", "id"),
        // Canonical Tier-1 loader nodes. Theories MERGE on `key`;
        // declarations MERGE on `id`.
        (BREACH_THEORY_LABEL, "breach_theory_key_unique", "key"),
        (
            IMPROPER_ACT_THEORY_LABEL,
            "improper_act_theory_key_unique",
            "key",
        ),
        (DECLARATION_LABEL, "declaration_sought_id_unique", "id"),
    ];

    for (label, constraint_name, key_prop) in constraints {
        let cypher = format!(
            "CREATE CONSTRAINT {constraint_name} IF NOT EXISTS \
             FOR (n:{label}) REQUIRE (n.{key_prop}) IS UNIQUE"
        );
        match graph.run(neo4rs::query(&cypher)).await {
            Ok(_) => tracing::info!(
                label = %label,
                "Neo4j constraint created or already exists"
            ),
            Err(e) => tracing::error!(
                label = %label,
                error = %e,
                "Failed to create Neo4j uniqueness constraint — \
                 MERGE operations may not be safe"
            ),
        }
    }

    ensure_relationship_indexes(graph).await;
}

/// Every relationship type this data model defines, for the traceability
/// indexes below.
///
/// ## Why the list is the WHOLE model and not the types that carry data today
///
/// Ruled 2026-09-17: a future writer must not be able to miss an index
/// silently. Three of these carried no traceability property on DEV when this
/// was written (`HAS_ELEMENT`, `HAS_THEORY`, `SEEKS_DECLARATION` — the
/// structural edges, reconstructed rather than extracted); indexing them costs
/// an empty index each and means the day one of them IS written by an
/// extraction pass, the index is already there.
// STRUCTURAL: which relationship types are indexed is a ruling (2026-09-17), not
// a deployment preference. Adding or removing one is a change to the GRAPH — a
// migration, never a settings edit — and the complete model is listed so a future
// extraction pass cannot silently reintroduce a traceability scan.
const INDEXED_RELATIONSHIPS: &[&str] = &[
    BEARS_ON,
    HAS_ELEMENT,
    HAS_THEORY,
    SEEKS_DECLARATION,
    ABOUT,
    CORROBORATES,
    CONTAINED_IN,
    STATED_BY,
    CHARACTERIZES,
    SUFFERED_BY,
    REBUTS,
    CAUSED_BY,
    EVIDENCED_BY,
    DAMAGES_FOR,
    DERIVED_FROM,
];

/// The two traceability properties every extracted relationship carries.
///
/// The first element is the property; the second is the slug that goes into the
/// index NAME, because an index name is persisted in the database and renaming
/// one is a migration — so it is written out rather than derived from the
/// property string.
// STRUCTURAL: the properties are Cypher wire vocabulary and the slugs are
// persisted Neo4j index names. Renaming either is a graph migration; neither
// varies by deployment.
const TRACEABILITY_PROPERTIES: &[(&str, &str)] = &[
    ("extraction_run_id", "extraction_run"),
    ("source_document_id", "source_document"),
];

/// Create one relationship-property index per (type, property).
///
/// ## The 2026-09-17 defect this repairs
///
/// These indexes NEVER existed. The previous form asked for one index across
/// every relationship type at once:
///
/// ```cypher
/// CREATE INDEX rel_extraction_run IF NOT EXISTS FOR ()-[r]-() ON (r.extraction_run_id)
/// ```
///
/// Neo4j 5 has no such form. The server answered
/// `SyntaxError: Invalid input ']': expected ':'` on EVERY boot since the code
/// was written, the error was logged and swallowed, and the traceability
/// queries it was meant to serve went on doing full relationship scans. Probed
/// against DEV's own server (5.26.21 community) before this was written: the
/// typeless form is refused, a single type is accepted, and there is no
/// multi-type `[r:A|B]` form for an index — so ONE INDEX PER TYPE is not a
/// style choice, it is the only shape the server accepts.
///
/// Idempotent (`IF NOT EXISTS`) and logged per index, as the constraints above
/// are: a failure here degrades query performance rather than breaking a write,
/// so it must be visible in the log rather than silent.
async fn ensure_relationship_indexes(graph: &Graph) {
    for rel_type in INDEXED_RELATIONSHIPS {
        for (property, slug) in TRACEABILITY_PROPERTIES {
            let index_name = format!("rel_{slug}_{}", rel_type.to_lowercase());
            // ## Rust Learning: why the type is interpolated and not a parameter
            //
            // Cypher parameterizes VALUES, never schema identifiers — there is
            // no `$relType` that could stand where `:ABOUT` stands. These come
            // from `neo4j::schema`'s compiled constants, never from a request,
            // so the interpolation carries no injection risk.
            let cypher = format!(
                "CREATE INDEX {index_name} IF NOT EXISTS \
                 FOR ()-[r:{rel_type}]-() ON (r.{property})"
            );
            match graph.run(neo4rs::query(&cypher)).await {
                // DEBUG and not INFO: thirty routine "created or already exists"
                // lines at every boot is the volume a real failure hides in —
                // and the failure below is the whole reason this loop is logged
                // at all. The error arm keeps its level.
                Ok(_) => tracing::debug!(
                    index = %index_name,
                    rel_type = %rel_type,
                    property = %property,
                    "Neo4j relationship index created or already exists"
                ),
                Err(e) => tracing::error!(
                    index = %index_name,
                    rel_type = %rel_type,
                    property = %property,
                    error = %e,
                    "Failed to create Neo4j relationship index — \
                     traceability queries may degrade to relationship scans"
                ),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{INDEXED_RELATIONSHIPS, TRACEABILITY_PROPERTIES};

    /// The typeless index form must never come back.
    ///
    /// ## Why this is a source guard and not a behavioural test
    ///
    /// The old form COMPILED, ran, and logged an error the server had already
    /// refused — for a month, on every boot, with nobody the wiser. Nothing a
    /// unit test can call would have gone red, because the failure lived in a
    /// string the server rejected at parse time. What can be asserted here is
    /// the shape of the string this module builds.
    #[test]
    fn no_relationship_index_is_asked_for_without_a_type() {
        // Comments STRIPPED: the doc above quotes the broken form on purpose,
        // and a guard that cannot tell an explanation from an instruction would
        // force the explanation out of the file — which is how the reason a
        // thing is forbidden gets lost.
        let source: String = include_str!("graph_migrations.rs")
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        // The literal that was wrong, composed rather than written so this test
        // does not trip over its own assertion.
        let typeless = format!("()-[r]{}()", "-");
        assert!(
            !source.contains(&typeless),
            "Neo4j 5 refuses a relationship index with no type — probed against \
             5.26.21 on 2026-09-17. Name the type: FOR ()-[r:TYPE]-()"
        );
        assert!(
            source.contains("FOR ()-[r:{rel_type}]-() ON (r.{property})"),
            "the index template must name a type and a property"
        );
    }

    /// Both traceability properties, across every relationship the model defines.
    #[test]
    fn every_relationship_type_is_indexed_on_both_properties() {
        assert_eq!(
            INDEXED_RELATIONSHIPS.len(),
            15,
            "the data model defines fifteen relationship types; ruled 2026-09-17 \
             that every one is indexed, so a future writer cannot miss one"
        );
        assert_eq!(TRACEABILITY_PROPERTIES.len(), 2);

        let mut sorted = INDEXED_RELATIONSHIPS.to_vec();
        sorted.sort_unstable();
        let mut unique = sorted.clone();
        unique.dedup();
        assert_eq!(
            sorted, unique,
            "a type named twice is two identical indexes"
        );
    }

    /// An index name is persisted in the database, so it must be stable and
    /// distinct — a collision would silently index one property and not the other.
    #[test]
    fn every_index_name_is_distinct_and_lowercase() {
        let mut names: Vec<String> = Vec::new();
        for rel_type in INDEXED_RELATIONSHIPS {
            for (_, slug) in TRACEABILITY_PROPERTIES {
                names.push(format!("rel_{slug}_{}", rel_type.to_lowercase()));
            }
        }
        assert_eq!(names.len(), 30, "fifteen types on two properties");
        assert!(
            names.contains(&"rel_extraction_run_about".to_string()),
            "{names:?}"
        );
        assert!(
            names.iter().all(|n| n == &n.to_lowercase()),
            "Neo4j index names are case-sensitive; keep one casing"
        );

        let mut unique = names.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), names.len(), "two indexes cannot share a name");
    }
}
