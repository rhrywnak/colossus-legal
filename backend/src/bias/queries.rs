//! Cypher builders for the Bias Explorer reads.
//!
//! ## Why these live in their own module
//!
//! Two reasons, both mechanical:
//!
//! 1. **Relationship names must come from `neo4j::schema`** (Rule 12). Cypher
//!    cannot parameterize a relationship type, so the only way to keep the names
//!    in one place is to interpolate the constants — which means a `fn -> String`
//!    built with `format!`, because a Rust `const` cannot call it. Three of
//!    `repository.rs`'s queries were still raw string literals with the names
//!    typed inline.
//! 2. **`repository.rs` was over the 300-line module limit** (Rule 17). Moving
//!    the query text out is what makes room for the `format!` scaffolding rather
//!    than pushing the file further over.
//!
//! Nothing about the query LOGIC changed in the extraction: the same clauses in
//! the same order, with `STATED_BY` / `ABOUT` / `CONTAINED_IN` now read from the
//! schema constants instead of retyped. The repository keeps the execution,
//! parameter binding and row aggregation; this module is text only, which is why
//! every builder here is a pure function with a shape test beside it.
//!
//! ## Rust Learning: `{{` and `}}` inside `format!`
//!
//! `format!` treats `{` as the start of a substitution, so a Cypher block that
//! contains literal braces — here the `EXISTS { … }` subquery in
//! [`filtered_evidence`] — must double them: `{{` renders as one `{`. Miss one
//! and the failure is a compile error, not a malformed query, so the escaping is
//! checked by the compiler rather than left to review.

use crate::neo4j::schema;

/// Distinct actors (`STATED_BY` targets) that have at least one tagged
/// statement, ordered by how many they have.
///
/// The `actor` binding deliberately carries no label filter: a statement may be
/// attributed to a `Person` or an `Organization`, and both belong in the
/// dropdown. `labels(actor)[0]` returns the concrete label for the frontend tag.
pub(super) fn actors_with_tagged_statements() -> String {
    format!(
        "
            MATCH (e:Evidence)-[:{stated_by}]->(actor)
            WHERE e.pattern_tags IS NOT NULL AND e.pattern_tags <> ''
            WITH actor, count(e) AS tagged_count
            RETURN actor.id AS id,
                   actor.name AS name,
                   labels(actor)[0] AS actor_type,
                   tagged_count
            ORDER BY tagged_count DESC, actor.name ASC
        ",
        stated_by = schema::STATED_BY,
    )
}

/// Distinct subjects (`ABOUT` targets) that at least one tagged statement
/// concerns, ordered by how many concern them.
///
/// `count(DISTINCT e)` — not a plain `count` — because one statement may be
/// ABOUT the same subject through more than one edge, and the dropdown counts
/// statements, not edges.
pub(super) fn subjects_with_tagged_statements() -> String {
    format!(
        "
            MATCH (e:Evidence)-[:{about}]->(subject)
            WHERE e.pattern_tags IS NOT NULL AND e.pattern_tags <> ''
            WITH subject, count(DISTINCT e) AS tagged_count
            RETURN subject.id AS id,
                   subject.name AS name,
                   labels(subject)[0] AS actor_type,
                   tagged_count
            ORDER BY tagged_count DESC, subject.name ASC
        ",
        about = schema::ABOUT,
    )
}

/// The filtered Evidence list — the Bias Explorer's main read.
///
/// Every filter is an `IS NULL OR …` predicate over a bound parameter, so one
/// query serves all eight filter combinations and an absent filter is a Cypher
/// NULL rather than a different query.
///
/// The subject filter is an `EXISTS { … }` subquery rather than a constraining
/// `MATCH`, deliberately: the `OPTIONAL MATCH (e)-[:ABOUT]->(subject)` below it
/// must still return **every** subject of a matching statement, not only the one
/// filtered on. A constraining MATCH would silently narrow the `about` list on
/// each card to the filtered subject.
///
/// `document_type` is projected from **`d.doc_type`** — the property the write
/// paths actually set. See `bias::dto::DocumentRef` for why the wire name and
/// the graph property differ.
pub(super) fn filtered_evidence() -> String {
    format!(
        "
            MATCH (e:Evidence)
            WHERE e.pattern_tags IS NOT NULL AND e.pattern_tags <> ''
            MATCH (e)-[:{stated_by}]->(actor)
            WHERE $actor_id IS NULL OR actor.id = $actor_id
            WITH e, actor
            WHERE $pattern_tag IS NULL
               OR ANY(t IN split(e.pattern_tags, ',') WHERE trim(t) = $pattern_tag)
            WITH e, actor
            WHERE $subject_id IS NULL
               OR EXISTS {{
                    MATCH (e)-[:{about}]->(s)
                    WHERE s.id = $subject_id
               }}
            OPTIONAL MATCH (e)-[:{about}]->(subject)
            OPTIONAL MATCH (e)-[:{contained_in}]->(d:Document)
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
              d.doc_type AS document_type
            ORDER BY actor.name, coalesce(d.title, ''), coalesce(e.page_number, 0)
        ",
        stated_by = schema::STATED_BY,
        about = schema::ABOUT,
        contained_in = schema::CONTAINED_IN,
    )
}

/// Hydrate a known set of Evidence ids — the curation panel's read.
///
/// RETURN columns are a SUPERSET of [`filtered_evidence`]'s: this query and
/// [`all_evidence_about_subject`] additionally project `e.question`, the
/// interrogatory a discovery answer responds to. The SAME `BiasRow::from_row`
/// decoder serves all three unchanged, because it reads every optional column
/// with `.ok()` — a column present here but absent there decodes to `None` on
/// that path, with no code change.
///
/// `STATED_BY` is an **OPTIONAL MATCH**, deliberately: a plain MATCH would
/// silently drop Evidence with no speaker edge. `coalesce(.., '')` then lets a
/// null actor decode to an empty string rather than a decode error.
///
/// Domain note: discovery Evidence stores the question it answers in
/// `e.question`; documentary evidence has none (Cypher null → `None`). Returning
/// it is what lets a candidate card pair a bare "Yes" with the question that
/// makes it interpretable.
pub(super) fn evidence_by_ids() -> String {
    format!(
        "
            MATCH (e:Evidence)
            WHERE e.id IN $ids
            OPTIONAL MATCH (e)-[:{stated_by}]->(actor)
            OPTIONAL MATCH (e)-[:{about}]->(subject)
            OPTIONAL MATCH (e)-[:{contained_in}]->(d:Document)
            RETURN
              e.id AS evidence_id,
              coalesce(e.title, '') AS title,
              e.verbatim_quote AS verbatim_quote,
              e.question AS question,
              e.page_number AS page_number,
              e.pattern_tags AS pattern_tags_raw,
              coalesce(actor.id, '') AS actor_id,
              coalesce(actor.name, '') AS actor_name,
              CASE WHEN actor IS NULL THEN '' ELSE labels(actor)[0] END AS actor_type,
              subject.id AS subject_id,
              subject.name AS subject_name,
              CASE WHEN subject IS NULL THEN NULL ELSE labels(subject)[0] END AS subject_type,
              d.id AS document_id,
              d.title AS document_title,
              d.doc_type AS document_type
    ",
        stated_by = schema::STATED_BY,
        about = schema::ABOUT,
        contained_in = schema::CONTAINED_IN,
    )
}

/// Every Evidence node ABOUT a subject — the Theme Scan and candidate-gather
/// pool. Deliberately **ungated by `pattern_tags`**, unlike the other builders
/// here: this read backs a 100%-recall pool, so a tag filter would silently
/// shrink it.
///
/// `EXISTS { … }` (not a second top-level MATCH) fixes the subject as the scope
/// WITHOUT multiplying `e` rows — the same shape [`filtered_evidence`] uses,
/// minus its `$subject_id IS NULL OR` (the subject is required here).
///
/// `STATED_BY` is an **OPTIONAL MATCH**, deliberately: a plain MATCH would
/// silently drop Evidence with no speaker edge and break the exhaustive-recall
/// guarantee. Documentary evidence (letters, records) legitimately has no
/// speaker — that is metadata, not a filter. A speaker-less row decodes to an
/// empty-name actor via the `coalesce` / `CASE WHEN actor IS NULL` projection,
/// which the DTO already types as `Option`.
///
/// Domain note: this is the discovery Q&A pairing. Pass-1 stores the ANSWER in
/// `e.verbatim_quote` and the interrogatory in `e.question`; projecting the
/// latter lets the theme-scan judge read a bare Yes/No in light of what was
/// asked.
pub(super) fn all_evidence_about_subject() -> String {
    format!(
        "
            MATCH (e:Evidence)
            WHERE EXISTS {{ MATCH (e)-[:{about}]->(s) WHERE s.id = $subject_id }}
            OPTIONAL MATCH (e)-[:{stated_by}]->(actor)
            OPTIONAL MATCH (e)-[:{about}]->(subject)
            OPTIONAL MATCH (e)-[:{contained_in}]->(d:Document)
            RETURN
              e.id AS evidence_id,
              coalesce(e.title, '') AS title,
              e.verbatim_quote AS verbatim_quote,
              e.question AS question,
              e.page_number AS page_number,
              e.pattern_tags AS pattern_tags_raw,
              coalesce(actor.id, '') AS actor_id,
              coalesce(actor.name, '') AS actor_name,
              CASE WHEN actor IS NULL THEN '' ELSE labels(actor)[0] END AS actor_type,
              subject.id AS subject_id,
              subject.name AS subject_name,
              CASE WHEN subject IS NULL THEN NULL ELSE labels(subject)[0] END AS subject_type,
              d.id AS document_id,
              d.title AS document_title,
              d.doc_type AS document_type
            ORDER BY e.id
    ",
        about = schema::ABOUT,
        stated_by = schema::STATED_BY,
        contained_in = schema::CONTAINED_IN,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every relationship name is interpolated from `neo4j::schema`, so a rename
    /// there flows here at compile time and no query carries a retyped literal
    /// that could drift (Rule 12). Asserting the rendered form also proves the
    /// `format!` arguments are wired to the right placeholders.
    #[test]
    fn every_query_interpolates_its_relationship_names_from_schema() {
        assert!(actors_with_tagged_statements().contains(&format!("-[:{}]->", schema::STATED_BY)));
        assert!(subjects_with_tagged_statements().contains(&format!("-[:{}]->", schema::ABOUT)));

        let filtered = filtered_evidence();
        assert!(filtered.contains(&format!("-[:{}]->(actor)", schema::STATED_BY)));
        assert!(filtered.contains(&format!("-[:{}]->(subject)", schema::ABOUT)));
        assert!(filtered.contains(&format!("-[:{}]->(d:Document)", schema::CONTAINED_IN)));
    }

    /// The `EXISTS { … }` braces survive `format!` as single braces. A missed
    /// escape is a compile error, but a doubled one that rendered `{{` into the
    /// query would be a runtime Cypher syntax error — so pin the rendered text.
    #[test]
    fn exists_subquery_renders_single_braces() {
        let q = filtered_evidence();
        assert!(
            q.contains("OR EXISTS {\n"),
            "EXISTS block must open with one brace"
        );
        assert!(
            !q.contains("{{"),
            "no doubled braces may reach the query text"
        );
        assert!(
            !q.contains("}}"),
            "no doubled braces may reach the query text"
        );
    }

    /// The subject filter must stay an `EXISTS` subquery. Rewriting it as a
    /// constraining `MATCH (e)-[:ABOUT]->(target {id: $subject_id})` would also
    /// narrow the `about` list rendered on each card — a silent data loss, not a
    /// visible failure.
    #[test]
    fn subject_filter_stays_an_exists_subquery_not_a_constraining_match() {
        let q = filtered_evidence();
        assert!(q.contains("$subject_id IS NULL"));
        assert!(q.contains("OR EXISTS {"));
        assert!(
            q.contains(&format!(
                "OPTIONAL MATCH (e)-[:{}]->(subject)",
                schema::ABOUT
            )),
            "the unfiltered subject projection must survive"
        );
    }

    /// Every filter is optional and NULL-tolerant, so one query serves all eight
    /// combinations. A filter that lost its `IS NULL` guard would silently return
    /// nothing when unset instead of everything.
    #[test]
    fn every_filter_is_null_tolerant() {
        let q = filtered_evidence();
        for guard in [
            "$actor_id IS NULL OR actor.id = $actor_id",
            "$pattern_tag IS NULL",
            "$subject_id IS NULL",
        ] {
            assert!(q.contains(guard), "missing NULL guard: {guard}");
        }
    }

    /// The document type is read from `doc_type`, the property the write paths
    /// set — never `document_type`, which has never existed on a Document node.
    /// The repo-wide scan in `neo4j::schema` enforces this globally; this pins it
    /// at the query that regressed.
    #[test]
    fn document_type_is_projected_from_the_real_property() {
        let q = filtered_evidence();
        assert!(q.contains("d.doc_type AS document_type"));
        assert!(!q.contains("d.document_type"));
    }

    /// Tagged-only is the defining scope of this surface: an Evidence node with
    /// no `pattern_tags` is not a bias instance and must not appear in any of the
    /// three reads.
    #[test]
    fn every_query_is_scoped_to_tagged_evidence() {
        // `all_evidence_about_subject` is deliberately excluded — see its own
        // test; it is the exhaustive pool and must NOT be tag-gated.
        for q in [
            actors_with_tagged_statements(),
            subjects_with_tagged_statements(),
            filtered_evidence(),
        ] {
            assert!(q.contains("e.pattern_tags IS NOT NULL AND e.pattern_tags <> ''"));
        }
    }

    /// The hydrate read keeps the properties its callers depend on, and the
    /// speaker leg stays OPTIONAL — a plain MATCH would silently drop Evidence
    /// with no `STATED_BY` edge, which documentary evidence legitimately lacks.
    #[test]
    fn evidence_by_ids_keeps_optional_speaker_and_the_question_projection() {
        let q = evidence_by_ids();
        assert!(q.contains(&format!(
            "OPTIONAL MATCH (e)-[:{}]->(actor)",
            schema::STATED_BY
        )));
        assert!(!q.contains(&format!(
            "\n            MATCH (e)-[:{}]->(actor)",
            schema::STATED_BY
        )));
        assert!(
            q.contains("e.question AS question"),
            "the Q&A pairing must survive"
        );
        assert!(q.contains("d.doc_type AS document_type"));
        assert!(q.contains("WHERE e.id IN $ids"));
    }

    /// The gather/scan pool is deliberately UNGATED by `pattern_tags` — it backs a
    /// 100%-recall candidate pool, and a tag filter would silently shrink it. This
    /// is the one builder here that must NOT carry the tagged-only scope, so it is
    /// asserted as a negative.
    #[test]
    fn subject_pool_is_exhaustive_and_not_gated_by_pattern_tags() {
        let q = all_evidence_about_subject();
        assert!(
            !q.contains("pattern_tags IS NOT NULL"),
            "the scan pool must not be narrowed to tagged Evidence"
        );
        assert!(q.contains(&format!("EXISTS {{ MATCH (e)-[:{}]->(s)", schema::ABOUT)));
        assert!(q.contains(&format!(
            "OPTIONAL MATCH (e)-[:{}]->(actor)",
            schema::STATED_BY
        )));
        assert!(q.contains("e.question AS question"));
        assert!(q.contains("d.doc_type AS document_type"));
    }

    /// Read-only. A write reaching the graph from the Bias Explorer would break
    /// the surface's defining constraint, so scan the generated Cypher rather
    /// than trusting review.
    #[test]
    fn every_query_is_read_only() {
        for q in [
            actors_with_tagged_statements(),
            subjects_with_tagged_statements(),
            filtered_evidence(),
            evidence_by_ids(),
            all_evidence_about_subject(),
        ] {
            let upper = q.to_uppercase();
            for forbidden in ["CREATE", "MERGE", " SET ", "DELETE", "REMOVE", "DETACH"] {
                assert!(
                    !upper.contains(forbidden),
                    "read-only violation: `{forbidden}`"
                );
            }
        }
    }
}
