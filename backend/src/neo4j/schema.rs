//! Canonical names of the Neo4j relationship types the data model defines.
//!
//! ## Why constants and not configuration
//!
//! These are graph-schema identifiers fixed at data-model time, not
//! environment- or case-specific configuration. They do not vary across
//! deployments; changing one requires a graph migration, not a config edit,
//! so they are deliberately compiled constants rather than config values
//! (Standing Rule 2 does not apply to schema identifiers). This mirrors the
//! node-label constant block in `canonical_elements::cypher`.
//!
//! Centralized here so that every read query, loader, and test references one
//! constant. A bare `-[:SOME_NAME]->` literal scattered across query strings
//! is how the previous name drifted out of sync during a rename; one constant
//! makes the next rename a single-line edit and lets the rules-enforcer ban
//! the literals (see `.claude/agents/rules-enforcer.md`, Rule 12).
//!
//! ## Rust Learning: `&'static str` constants
//!
//! A `pub const NAME: &str = "…"` is a compile-time constant string slice with
//! `'static` lifetime — it lives in the binary's read-only data, costs nothing
//! to reference, and can be interpolated into a `format!` Cypher template the
//! same way a literal would be. Because Cypher cannot *parameterize* a
//! relationship type (only node properties), interpolation is the only way to
//! keep the type name in one place; the values here are trusted, code-defined
//! literals, never user input, so that interpolation carries no injection risk.

/// `Allegation -[:BEARS_ON]-> Element`. The central reasoning relationship:
/// the Allegation's facts help establish (bear on) this specific Element of a
/// Count's legal theory. Persisted via `authored_relationships` (cross-tier:
/// extracted Allegation → canonical Element). At trial the attorney must prove
/// each Element, so this edge is the proof-chain backbone.
pub const BEARS_ON: &str = "BEARS_ON";

/// `LegalCount -[:HAS_ELEMENT]-> Element`. Structural: the Count is composed of
/// these Elements. Reconstructed mechanically from `Element.parent_count_id`.
pub const HAS_ELEMENT: &str = "HAS_ELEMENT";

/// `Element -[:HAS_THEORY]-> (BreachTheory | ImproperActTheory)`. One Element
/// can be satisfied by several theories of *how* it was met; the discriminator
/// property on the edge distinguishes the theory kind.
pub const HAS_THEORY: &str = "HAS_THEORY";

/// `LegalCount -[:SEEKS_DECLARATION]-> DeclarationSought`. The declaratory
/// relief the Count asks the court to grant.
pub const SEEKS_DECLARATION: &str = "SEEKS_DECLARATION";

/// `Allegation -[:ABOUT]-> Party`. The Allegation concerns (is against /
/// regarding) this party.
pub const ABOUT: &str = "ABOUT";

/// `MotionClaim -[:PROVES]-> Allegation`. A claim made in a motion proves /
/// asserts the Allegation.
pub const PROVES: &str = "PROVES";

/// `MotionClaim -[:RELIES_ON]-> Evidence`. The claim is supported by this piece
/// of evidence.
pub const RELIES_ON: &str = "RELIES_ON";

/// `Evidence -[:CORROBORATES]-> Allegation`. A discovery/evidence item
/// independently confirms (corroborates) a complaint Allegation.
///
/// Domain note: this is the cross-document proof edge the discovery pass-2
/// extraction authors — Phillips' sworn admission corroborating a complaint
/// paragraph. Combined with `BEARS_ON` + `HAS_ELEMENT` it forms the proof
/// chain `Evidence → Allegation → Element → LegalCount` the Proof Matrix walks.
/// Label-only (no edge properties).
pub const CORROBORATES: &str = "CORROBORATES";

/// `Evidence -[:CONTAINED_IN]-> Document`. The evidence item appears within
/// this source document.
pub const CONTAINED_IN: &str = "CONTAINED_IN";

/// `Statement -[:STATED_BY]-> Party`. The speaker who made the statement.
///
/// Domain note: `STATED_BY` is the speaker who made the statement under oath;
/// it is distinct from `ABOUT` (who the statement concerns) — different
/// relationships, different queries.
pub const STATED_BY: &str = "STATED_BY";

/// `Statement -[:CHARACTERIZES]-> target`. The statement frames or
/// characterizes the target (e.g. a party, act, or event).
pub const CHARACTERIZES: &str = "CHARACTERIZES";

/// `Harm -[:SUFFERED_BY]-> Party`. Who bore the harm.
///
/// Domain note: the direction runs FROM the harm TO the party, which is the
/// opposite of what the English reads like. A party "suffered" the harm, but the
/// edge is written on the harm — so a query for everything a person suffered
/// walks INCOMING edges on the party. Added 2026-08-16 for the party merge,
/// which has to move all five party-incident types and refuses to run on one it
/// does not know.
pub const SUFFERED_BY: &str = "SUFFERED_BY";

/// `MotionClaim -[:CONTRADICTS]-> target`. The claim directly contradicts the
/// target statement or allegation.
pub const CONTRADICTS: &str = "CONTRADICTS";

/// `Evidence -[:REBUTS]-> (Evidence | Allegation)`. A sworn statement directly
/// counters / defeats what a *different* speaker asserted: either that speaker's
/// foreign Evidence in another document (impeachment) or a complaint Allegation
/// whose fact this statement denies.
///
/// Domain note: produced by the evidence-anchoring pass-2 extraction (discovery,
/// affidavit) over cross-document context — the same context+prompt+id-resolution
/// path that carries `CORROBORATES -> Allegation`. The pairing is the key
/// distinction: against an Allegation, CORROBORATES means the answer *confirms*
/// the alleged fact, REBUTS means it *counters* it. Label-only (no edge
/// properties). The legacy `MotionClaim -[:REBUTS]->` framing predated the
/// three-tier model and no longer names the actual producer.
pub const REBUTS: &str = "REBUTS";

/// `Allegation -[:CAUSED_BY]-> cause`. The harm or event was caused by the
/// linked node.
pub const CAUSED_BY: &str = "CAUSED_BY";

/// `entity -[:APPEARS_IN]-> Document`. The entity is mentioned in / appears in
/// the document.
pub const APPEARS_IN: &str = "APPEARS_IN";

/// `Allegation -[:SUPPORTS]-> LegalCount`. Legacy count-level support edge;
/// also the *rendered* label the graph view shows for the synthetic
/// Allegation→Count link that masks the `BEARS_ON`+`HAS_ELEMENT` hops.
pub const SUPPORTS: &str = "SUPPORTS";

#[cfg(test)]
mod tests {
    use super::*;

    /// Disk/code invariant (Standing Rule 21, applied to schema identifiers):
    /// the constant *value* is the wire/graph string. A typo here would silently
    /// build queries that match nothing, so assert the exact spelling.
    #[test]
    fn relationship_constant_values_are_exact() {
        assert_eq!(BEARS_ON, "BEARS_ON");
        assert_eq!(HAS_ELEMENT, "HAS_ELEMENT");
        assert_eq!(HAS_THEORY, "HAS_THEORY");
        assert_eq!(SEEKS_DECLARATION, "SEEKS_DECLARATION");
        assert_eq!(ABOUT, "ABOUT");
        assert_eq!(PROVES, "PROVES");
        assert_eq!(RELIES_ON, "RELIES_ON");
        assert_eq!(CORROBORATES, "CORROBORATES");
        assert_eq!(CONTAINED_IN, "CONTAINED_IN");
        assert_eq!(STATED_BY, "STATED_BY");
        assert_eq!(SUFFERED_BY, "SUFFERED_BY");
        assert_eq!(CHARACTERIZES, "CHARACTERIZES");
        assert_eq!(CONTRADICTS, "CONTRADICTS");
        assert_eq!(REBUTS, "REBUTS");
        assert_eq!(CAUSED_BY, "CAUSED_BY");
        assert_eq!(APPEARS_IN, "APPEARS_IN");
        assert_eq!(SUPPORTS, "SUPPORTS");
    }

    /// No Cypher anywhere in `src/` may read `document_type` off a `Document`
    /// node — the property does not exist there and never has.
    ///
    /// ## Why this is a filesystem scan and not a review item
    ///
    /// Every write path sets **`doc_type`**
    /// (`api::pipeline::ingest_helpers::create_document_node`,
    /// `repositories::document_repository`). The Postgres column of the same
    /// name — `documents.document_type` — is a DIFFERENT store, which is exactly
    /// what makes the mistake so easy: the name is right somewhere. Reading it
    /// from the graph returns NULL rather than an error, so the failure mode is a
    /// plausible-looking empty field, not a visible fault.
    ///
    /// That is the class of defect review misses. SIX separate queries carried it
    /// — three in `bias`, one in `embedding_repository`, one in
    /// `graph_expansion_minor`, one in `graph_expansion_cypher` — and none was
    /// noticed until a metric inventory went looking. The sixth was found only
    /// because it used the alias `doc` instead of `d`, and a first version of
    /// this very test missed it. Standing rule 21: when an invariant must hold
    /// across many files, assert it by scanning the files.
    ///
    /// ## What the scan matches, and its one known limit
    ///
    /// It flags `.document_type AS ` — the Cypher RETURN-projection form, which
    /// is how every one of the six sites was written and how a property read is
    /// always spelled in this codebase's queries. Matching on the alias (` AS `)
    /// rather than on a node-binding name is what makes it alias-agnostic: `d`,
    /// `doc` or anything else is caught, while Rust field access
    /// (`document.document_type`) and SQL without an alias cannot match, because
    /// neither contains ` AS ` on the same line.
    ///
    /// Known limit, stated rather than hidden: a Cypher predicate that reads the
    /// property without projecting it (`WHERE d.document_type = $x`) would slip
    /// through. No such read exists today; widen the pattern if one appears.
    #[test]
    fn no_cypher_reads_document_type_from_a_document_node() {
        use std::fs;
        use std::path::Path;

        /// The forbidden spelling: a Cypher projection of `document_type`.
        const FORBIDDEN: &str = ".document_type AS ";

        /// Path fragments whose occurrences are legitimate: SQL against the
        /// Postgres column of the same name, or prose about this rule.
        ///
        /// `repositories/pipeline_repository/` is exempt as a whole DIRECTORY,
        /// not file by file: that module is the Postgres layer by construction
        /// (it talks to `state.pipeline_pool` and nothing else), so
        /// `documents.document_type` there is the real column. SQL uses ` AS `
        /// too, which is why the projection pattern alone cannot separate them.
        const ALLOWED: &[&str] = &[
            "repositories/pipeline_repository/", // Postgres-only module
            "bias/dto.rs",                       // the doc comment explaining the fix
            "neo4j/schema.rs",                   // this test
            // Asserts the same invariant for the Case Health queries, so it
            // names the forbidden spelling in its own failure message.
            "repositories/case_health_repository_tests.rs",
        ];

        fn scan(dir: &Path, forbidden: &str, allowed: &[&str], hits: &mut Vec<String>) {
            let entries = fs::read_dir(dir).expect("src/ is readable");
            for entry in entries {
                let path = entry.expect("dir entry readable").path();
                if path.is_dir() {
                    scan(&path, forbidden, allowed, hits);
                } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                    let rel = path
                        .strip_prefix(env!("CARGO_MANIFEST_DIR"))
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .replace("src/", "");
                    if allowed.iter().any(|a| rel.contains(a)) {
                        continue;
                    }
                    let text = fs::read_to_string(&path).expect("source file is UTF-8");
                    for (i, line) in text.lines().enumerate() {
                        if line.contains(forbidden) {
                            hits.push(format!("{rel}:{}", i + 1));
                        }
                    }
                }
            }
        }

        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut hits = Vec::new();
        scan(&src, FORBIDDEN, ALLOWED, &mut hits);
        assert!(
            hits.is_empty(),
            "`document_type` is not a property of a Document node — the write \
             paths set `doc_type`. Found at: {hits:?}"
        );
    }
}
