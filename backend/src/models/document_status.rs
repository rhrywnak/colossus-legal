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
/// v5.1 complaint-schema variant of [`ENTITY_COMPLAINT_ALLEGATION`].
///
/// The v5.1 complaint schema emits entities under the shorter label
/// `"Allegation"` rather than `"ComplaintAllegation"`. Both names must
/// be recognised by anything that filters on allegation-shaped items
/// (cross-document context, downstream graph queries) so the v5.1
/// pipeline and any older v4-era data are both readable. Renaming
/// either constant is a data-model migration, not a refactor.
pub const ENTITY_ALLEGATION: &str = "Allegation";
pub const ENTITY_LEGAL_COUNT: &str = "LegalCount";
pub const ENTITY_HARM: &str = "Harm";
pub const ENTITY_EVIDENCE: &str = "Evidence";
pub const ENTITY_DOCUMENT: &str = "Document";
pub const ENTITY_ELEMENT: &str = "Element";

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

// ── Evidence statement_type / evidence_strength vocabulary ──────
//
// lowercase snake_case. These are *property values* carried on `Evidence`
// nodes (the `statement_type` and `evidence_strength` properties), emitted by
// the discovery v5_2 extraction schema — the answer-classification vocabulary
// the Proof-Review reads group and filter on.
//
// They live here, alongside the node-label and review-status constants, for
// the same reason those do: they are fixed graph vocabulary, not environment-
// or case-specific configuration. The v5_2 schema decides these strings;
// changing one is a data-model migration, not a config edit (so Standing Rule
// 2 — "no hardcoded values" — does not pull them into runtime config). They
// are gathered here so every Cypher read and Rust filter references one
// constant instead of a bare `"partial_admission"` literal scattered across
// query strings (Rule 12).

/// `statement_type` for a discovery answer that admits the matter asserted.
pub const STMT_ADMISSION: &str = "admission";
/// `statement_type` for an answer that admits in part / with qualification.
/// This is also the v1 "borderline" (hedged-partial) queue discriminator —
/// see `proof_review_builder`.
pub const STMT_PARTIAL_ADMISSION: &str = "partial_admission";
/// `statement_type` for an evasive (non-responsive) answer.
pub const STMT_EVASIVE: &str = "evasive";
/// `statement_type` for an answer that is purely an objection.
pub const STMT_OBJECTION: &str = "objection";
/// `statement_type` for an answer that refers the question elsewhere.
pub const STMT_REFERRAL: &str = "referral";
/// `statement_type` for an answer that denies the matter asserted.
pub const STMT_DENIAL: &str = "denial";

/// `evidence_strength` carried by a sworn party admission — the strongest
/// corroboration tier. Defined here as named graph vocabulary so a future
/// query that filters on it does not reintroduce a bare literal.
///
/// Domain note: the original Proof-Review design also named an
/// `evidence_strength = "sworn_party_evasion"` value for the borderline queue.
/// That value does **not** exist in the graph, so v1 keys borderline off
/// `STMT_PARTIAL_ADMISSION` instead (see `proof_review_builder`); no constant
/// is defined for the non-existent value, by design.
pub const EVIDENCE_STRENGTH_SWORN_PARTY_ADMISSION: &str = "sworn_party_admission";

/// The two `statement_type` values that an `Evidence`→`Allegation`
/// `CORROBORATES` edge carries: a corroboration is either a full or a partial
/// admission. The Proof-Review summary groups corroborations by these.
///
/// ## Rust Learning: `&[&str]` set constants
///
/// A `pub const NAME: &[&str] = &[A, B]` is a compile-time slice of string
/// slices living in the binary's read-only data. Cypher reads bind it directly
/// as a list parameter (`WHERE x IN $set`, via `.param("set", SET.to_vec())`),
/// and Rust filters call `SET.contains(&value)`. Mirrors `PARTY_SUBTYPES`
/// above. Referencing the *set* — never a re-typed list of literals — is what
/// keeps the vocabulary in exactly one place.
pub const CORROBORATING_STATEMENT_TYPES: &[&str] = &[STMT_ADMISSION, STMT_PARTIAL_ADMISSION];

/// The `statement_type` values for preserved-but-unlinked *non-answers*:
/// answers the corroboration bar deliberately excluded (they produce no
/// `CORROBORATES` edge). The Proof-Review "excluded" read selects Evidence with
/// one of these types and no outgoing `CORROBORATES`, proving the bar excluded
/// the right things and deleted nothing.
pub const NON_ANSWER_STATEMENT_TYPES: &[&str] =
    &[STMT_EVASIVE, STMT_OBJECTION, STMT_REFERRAL, STMT_DENIAL];

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
        assert!(ENTITY_ALLEGATION.starts_with(char::is_uppercase));
        assert!(ENTITY_LEGAL_COUNT.starts_with(char::is_uppercase));
        assert!(ENTITY_HARM.starts_with(char::is_uppercase));
        assert!(ENTITY_EVIDENCE.starts_with(char::is_uppercase));
        assert!(ENTITY_DOCUMENT.starts_with(char::is_uppercase));
        assert!(ENTITY_ELEMENT.starts_with(char::is_uppercase));
    }

    #[test]
    fn party_subtypes_contains_all_party_types() {
        assert!(PARTY_SUBTYPES.contains(&ENTITY_PARTY));
        assert!(PARTY_SUBTYPES.contains(&ENTITY_PERSON));
        assert!(PARTY_SUBTYPES.contains(&ENTITY_ORGANIZATION));
        assert_eq!(PARTY_SUBTYPES.len(), 3);
    }

    #[test]
    fn statement_type_values_are_lowercase() {
        for s in [
            STMT_ADMISSION,
            STMT_PARTIAL_ADMISSION,
            STMT_EVASIVE,
            STMT_OBJECTION,
            STMT_REFERRAL,
            STMT_DENIAL,
        ] {
            assert!(
                s.chars().all(|c| c.is_lowercase() || c == '_'),
                "statement_type '{s}' must be lowercase/underscore"
            );
        }
        assert!(EVIDENCE_STRENGTH_SWORN_PARTY_ADMISSION
            .chars()
            .all(|c| c.is_lowercase() || c == '_'));
    }

    /// The two statement_type sets must be disjoint: a `statement_type` is
    /// either a corroboration (admission/partial) or a non-answer
    /// (evasive/objection/referral/denial), never both. If they ever overlap,
    /// the Proof-Review summary would double-count an Evidence node in both the
    /// corroborating and the excluded buckets — a real, silent miscount this
    /// test catches at `cargo test` time.
    #[test]
    fn corroborating_and_non_answer_sets_are_disjoint() {
        for c in CORROBORATING_STATEMENT_TYPES {
            assert!(
                !NON_ANSWER_STATEMENT_TYPES.contains(c),
                "statement_type '{c}' is in both the corroborating and non-answer sets"
            );
        }
        // Pin the membership so an accidental edit to either set is caught.
        assert_eq!(CORROBORATING_STATEMENT_TYPES.len(), 2);
        assert_eq!(NON_ANSWER_STATEMENT_TYPES.len(), 4);
        assert!(CORROBORATING_STATEMENT_TYPES.contains(&STMT_ADMISSION));
        assert!(CORROBORATING_STATEMENT_TYPES.contains(&STMT_PARTIAL_ADMISSION));
    }

    #[test]
    fn relationship_type_values_are_screaming_snake() {
        // CONTAINED_IN is the only relationship type carried as a named
        // constant: it is the structural edge hardcoded from every node to
        // its Document. All other relationship types flow through as data
        // (`rel.relationship_type`), so there are no further constants to
        // assert here.
        assert!(
            REL_CONTAINED_IN
                .chars()
                .all(|c| c.is_uppercase() || c == '_'),
            "relationship type '{REL_CONTAINED_IN}' must be SCREAMING_SNAKE_CASE"
        );
    }
}
