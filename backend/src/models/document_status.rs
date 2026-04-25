//! Authoritative string-constant module for the document pipeline.
//!
//! Centralizes every status, entity type, relationship type, and review
//! state that flows through the system as a string. The values defined
//! here match the strings persisted in PostgreSQL columns (`documents.status`,
//! `extraction_runs.status`, `pipeline_steps.status`, `pipeline_items.review_status`)
//! and Neo4j labels / relationship types.
//!
//! ## Why one module?
//!
//! Before consolidation these values were scattered across 12+ files. A
//! casing mismatch (`'COMPLETED'` for documents vs `'completed'` for
//! pipeline steps in the same SQL query) and three separate copies of the
//! `Party | Person | Organization` filter were the immediate triggers for
//! pulling everything here.
//!
//! ## Rust Learning — Module-Level Constants
//!
//! These are `&'static str` — string slices with a 'static lifetime,
//! meaning they live for the entire program. They're compiled into the
//! binary's read-only data section. Unlike `String` (heap-allocated),
//! these cost zero runtime allocation. We use `&str` constants instead
//! of an enum because the values are persisted as strings in PostgreSQL
//! and Neo4j and compared as strings — an enum would require conversion
//! at every database boundary.

// ── Document lifecycle statuses ─────────────────────────────────
//
// Persisted in `documents.status`. Casing is UPPERCASE — frontend
// rendering and SQL filters depend on this exact spelling. Changing a
// value here is a database-state migration, not a refactor.

pub const STATUS_NEW: &str = "NEW";
pub const STATUS_UPLOADED: &str = "UPLOADED";
pub const STATUS_PROCESSING: &str = "PROCESSING";
pub const STATUS_CLASSIFIED: &str = "CLASSIFIED";
pub const STATUS_TEXT_EXTRACTED: &str = "TEXT_EXTRACTED";
pub const STATUS_EXTRACTED: &str = "EXTRACTED";
pub const STATUS_VERIFIED: &str = "VERIFIED";
pub const STATUS_IN_REVIEW: &str = "IN_REVIEW";
pub const STATUS_APPROVED: &str = "APPROVED";
pub const STATUS_INGESTED: &str = "INGESTED";
pub const STATUS_INDEXED: &str = "INDEXED";
pub const STATUS_PUBLISHED: &str = "PUBLISHED";
pub const STATUS_COMPLETED: &str = "COMPLETED";
pub const STATUS_FAILED: &str = "FAILED";
pub const STATUS_CANCELLED: &str = "CANCELLED";

/// All valid lifecycle statuses, in approximate ordering.
///
/// Used by validators that accept "any of the known statuses." Order
/// matches the most common forward-progress sequence; back-edges
/// (FAILED, CANCELLED) are placed at the end.
pub const VALID_STATUSES: &[&str] = &[
    STATUS_NEW,
    STATUS_UPLOADED,
    STATUS_PROCESSING,
    STATUS_CLASSIFIED,
    STATUS_TEXT_EXTRACTED,
    STATUS_EXTRACTED,
    STATUS_VERIFIED,
    STATUS_IN_REVIEW,
    STATUS_APPROVED,
    STATUS_INGESTED,
    STATUS_INDEXED,
    STATUS_PUBLISHED,
    STATUS_COMPLETED,
    STATUS_FAILED,
    STATUS_CANCELLED,
];

// ── Extraction-run statuses (UPPERCASE) ─────────────────────────
//
// Persisted in `extraction_runs.status`. UPPERCASE matches the existing
// rows. Distinct from `STEP_STATUS_*` below — those use lowercase to
// match `pipeline_steps.status`. This split is intentional and was the
// source of the metrics.rs:231 bug; the constant names lock it down.

pub const RUN_STATUS_RUNNING: &str = "RUNNING";
pub const RUN_STATUS_COMPLETED: &str = "COMPLETED";
pub const RUN_STATUS_FAILED: &str = "FAILED";

// ── Pipeline-step statuses (lowercase) ──────────────────────────
//
// Persisted in `pipeline_steps.status`. Lowercase per existing DB rows.

pub const STEP_STATUS_RUNNING: &str = "running";
pub const STEP_STATUS_COMPLETED: &str = "completed";
pub const STEP_STATUS_FAILED: &str = "failed";

// ── Review statuses (lowercase) ─────────────────────────────────
//
// Persisted in `pipeline_items.review_status`. Lowercase per existing
// DB rows.

pub const REVIEW_STATUS_PENDING: &str = "pending";
pub const REVIEW_STATUS_APPROVED: &str = "approved";
pub const REVIEW_STATUS_REJECTED: &str = "rejected";
pub const REVIEW_STATUS_EDITED: &str = "edited";

// ── Entity type names ───────────────────────────────────────────
//
// PascalCase. Used as Neo4j node labels and as the `entity_type`
// discriminator on `pipeline_items`. Renaming any of these is a
// data-model migration, not a refactor.

pub const ENTITY_PARTY: &str = "Party";
pub const ENTITY_PERSON: &str = "Person";
pub const ENTITY_ORGANIZATION: &str = "Organization";
pub const ENTITY_COMPLAINT_ALLEGATION: &str = "ComplaintAllegation";
pub const ENTITY_LEGAL_COUNT: &str = "LegalCount";
pub const ENTITY_HARM: &str = "Harm";
pub const ENTITY_EVIDENCE: &str = "Evidence";
pub const ENTITY_DOCUMENT: &str = "Document";

/// Entity-type discriminators that resolve from the generic `Party`
/// type into a concrete `Person` or `Organization` during ingest.
///
/// Used by ingest filters that need to treat all three labels as a
/// single "party-like" group (e.g., the canonical-verifier resolution
/// pass). Three call sites previously duplicated this allow-list as
/// `matches!(t, "Party" | "Person" | "Organization")`.
pub const PARTY_SUBTYPES: &[&str] = &[ENTITY_PARTY, ENTITY_PERSON, ENTITY_ORGANIZATION];

// ── Relationship type names ─────────────────────────────────────
//
// SCREAMING_SNAKE_CASE. Used as Neo4j relationship types. Most
// extraction relationships flow through as data (`rel.relationship_type`)
// rather than literal strings, but `CONTAINED_IN` is hardcoded as a
// structural edge from every node to its `Document`.

pub const REL_CONTAINED_IN: &str = "CONTAINED_IN";
pub const REL_STATED_BY: &str = "STATED_BY";
pub const REL_ABOUT: &str = "ABOUT";
pub const REL_SUPPORTS: &str = "SUPPORTS";
pub const REL_CORROBORATES: &str = "CORROBORATES";
pub const REL_CONTRADICTS: &str = "CONTRADICTS";
pub const REL_REBUTS: &str = "REBUTS";
pub const REL_CAUSED_BY: &str = "CAUSED_BY";
pub const REL_DAMAGES_FOR: &str = "DAMAGES_FOR";
pub const REL_SUFFERED_BY: &str = "SUFFERED_BY";
pub const REL_EVIDENCED_BY: &str = "EVIDENCED_BY";
pub const REL_DERIVED_FROM: &str = "DERIVED_FROM";

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_lifecycle_statuses_uppercase() {
        for s in VALID_STATUSES {
            assert!(
                s.chars().all(|c| c.is_uppercase() || c == '_'),
                "lifecycle status '{s}' must be UPPERCASE/underscore"
            );
        }
    }

    #[test]
    fn lifecycle_status_values_locked_in() {
        // These strings are persisted in the documents table and referenced
        // by frontend rendering logic. Lock the values.
        assert_eq!(STATUS_NEW, "NEW");
        assert_eq!(STATUS_UPLOADED, "UPLOADED");
        assert_eq!(STATUS_PROCESSING, "PROCESSING");
        assert_eq!(STATUS_TEXT_EXTRACTED, "TEXT_EXTRACTED");
        assert_eq!(STATUS_EXTRACTED, "EXTRACTED");
        assert_eq!(STATUS_VERIFIED, "VERIFIED");
        assert_eq!(STATUS_INGESTED, "INGESTED");
        assert_eq!(STATUS_INDEXED, "INDEXED");
        assert_eq!(STATUS_PUBLISHED, "PUBLISHED");
        assert_eq!(STATUS_COMPLETED, "COMPLETED");
        assert_eq!(STATUS_FAILED, "FAILED");
        assert_eq!(STATUS_CANCELLED, "CANCELLED");
    }

    #[test]
    fn run_status_uppercase_step_status_lowercase() {
        // Locks in the casing convention to prevent the metrics.rs:231 bug
        // (mixing 'COMPLETED' and 'completed' in the same SQL query).
        assert!(RUN_STATUS_RUNNING.chars().all(|c| c.is_uppercase()));
        assert!(RUN_STATUS_COMPLETED.chars().all(|c| c.is_uppercase()));
        assert!(RUN_STATUS_FAILED.chars().all(|c| c.is_uppercase()));
        assert!(STEP_STATUS_RUNNING.chars().all(|c| c.is_lowercase()));
        assert!(STEP_STATUS_COMPLETED.chars().all(|c| c.is_lowercase()));
        assert!(STEP_STATUS_FAILED.chars().all(|c| c.is_lowercase()));
    }

    #[test]
    fn review_status_values_are_lowercase() {
        assert!(REVIEW_STATUS_PENDING.chars().all(|c| c.is_lowercase()));
        assert!(REVIEW_STATUS_APPROVED.chars().all(|c| c.is_lowercase()));
        assert!(REVIEW_STATUS_REJECTED.chars().all(|c| c.is_lowercase()));
        assert!(REVIEW_STATUS_EDITED.chars().all(|c| c.is_lowercase()));
    }

    #[test]
    fn entity_type_values_are_pascal_case() {
        assert!(ENTITY_PARTY.starts_with(char::is_uppercase));
        assert!(ENTITY_PERSON.starts_with(char::is_uppercase));
        assert!(ENTITY_ORGANIZATION.starts_with(char::is_uppercase));
        assert!(ENTITY_COMPLAINT_ALLEGATION.starts_with(char::is_uppercase));
        assert!(ENTITY_LEGAL_COUNT.starts_with(char::is_uppercase));
        assert!(ENTITY_HARM.starts_with(char::is_uppercase));
        assert!(ENTITY_EVIDENCE.starts_with(char::is_uppercase));
        assert!(ENTITY_DOCUMENT.starts_with(char::is_uppercase));
    }

    #[test]
    fn party_subtypes_contains_all_party_types() {
        assert!(PARTY_SUBTYPES.contains(&ENTITY_PARTY));
        assert!(PARTY_SUBTYPES.contains(&ENTITY_PERSON));
        assert!(PARTY_SUBTYPES.contains(&ENTITY_ORGANIZATION));
        assert_eq!(PARTY_SUBTYPES.len(), 3);
    }

    #[test]
    fn relationship_type_values_are_screaming_snake() {
        for r in [
            REL_CONTAINED_IN,
            REL_STATED_BY,
            REL_ABOUT,
            REL_SUPPORTS,
            REL_CORROBORATES,
            REL_CONTRADICTS,
            REL_REBUTS,
            REL_CAUSED_BY,
            REL_DAMAGES_FOR,
            REL_SUFFERED_BY,
            REL_EVIDENCED_BY,
            REL_DERIVED_FROM,
        ] {
            assert!(
                r.chars().all(|c| c.is_uppercase() || c == '_'),
                "relationship type '{r}' must be SCREAMING_SNAKE_CASE"
            );
        }
    }
}
