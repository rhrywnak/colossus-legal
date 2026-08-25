//! THE PERMANENT VALIDATION GUARD, after the JSON retired.
//!
//! ## What changed on 2026-08-25, and why the file half is gone
//!
//! Phase A's version of this file read `frontend/public/data/timeline.json` off
//! disk and validated it end to end. That was the right guard while the file was
//! the product's data. Phase B moved the phases and the tags into tables, loaded
//! the events, and DELETED the file (ruling R15, task §B7) — so the tests that
//! read it retired WITH it. Their deletion condition was written down when they
//! were, and it has arrived.
//!
//! ## What stayed, and why
//!
//! Everything that pins one VOCABULARY across the places it is written down.
//! That coupling did not go away when the file did — it moved. The phase slugs
//! now live in a Rust enum, a `chronology_phases` CHECK, a `documents_phase_valid`
//! CHECK and a table of seeded rows; the tags live in a Rust list and a
//! `chronology_tags` seed. Four places and two, and a drift in any of them is
//! still a page that renders blank or a row nothing can be tagged with.
//!
//! `domain::case_phase_tests` holds the enum↔rows half. This file holds the
//! CHECK↔CHECK half and the tag half.
//!
//! ## ⚑ Comments are stripped before every scan
//!
//! This codebase documents its rules next to its rules, so a scanner hunting for
//! a token finds the DOCUMENTATION first. See `wording_tests`'s "prose versus
//! parser" note — and the chronology's own instance of it, where a migration's
//! header explaining an alignment rule was parsed as a stored value.

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::domain::case_phase::ALL_CASE_PHASES;
use crate::domain::chronology::CHRONOLOGY_TAGS;

/// The migration that creates the chronology tables and seeds the phases.
// STRUCTURAL: repo-internal pointers to immutable, version-controlled
// migrations. Identical in every environment; nothing here varies by deployment.
const TABLES_MIGRATION: &str = "pipeline_migrations/20260825105447_chronology_tables.sql";

/// The migration that seeds the tag vocabulary (ruling R-F).
const TAGS_MIGRATION: &str = "pipeline_migrations/20260825150937_chronology_tags.sql";

/// The migration that put the phase CHECK on `documents`.
const DOCUMENTS_PHASE_MIGRATION: &str = "pipeline_migrations/20260817150412_add_document_phase.sql";

fn crate_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// SQL with every `--` comment removed.
fn sql_without_comments(relative: &str) -> String {
    let raw = std::fs::read_to_string(crate_path(relative))
        .unwrap_or_else(|e| panic!("{relative} is not on disk: {e}"));
    raw.lines()
        .map(|line| match line.find("--") {
            Some(at) => &line[..at],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The single-quoted literals of one `INSERT INTO <table>` statement, row by row.
///
/// Returns each row's literals in order, so a caller can take the first as an id
/// and the second as a label without a SQL parser.
fn seeded_rows(sql: &str, table: &str) -> Vec<Vec<String>> {
    let Some(at) = sql.find(&format!("INSERT INTO {table}")) else {
        return Vec::new();
    };
    let block = &sql[at..];
    let end = block.find(';').unwrap_or(block.len());
    block[..end]
        .split("\n    (")
        .skip(1)
        .map(|row| {
            row.split('\'')
                .skip(1)
                .step_by(2)
                .map(str::to_string)
                .collect()
        })
        .collect()
}

// ─── the phase vocabulary, across the places it is written ───────────────────

#[test]
fn every_phase_slug_appears_in_both_check_constraints() {
    let tables = sql_without_comments(TABLES_MIGRATION);
    let documents = sql_without_comments(DOCUMENTS_PHASE_MIGRATION);

    for phase in ALL_CASE_PHASES {
        let quoted = format!("'{}'", phase.slug());
        assert!(
            tables.contains(&quoted),
            "{} is missing from the chronology_phases CHECK (comments stripped)",
            phase.slug()
        );
        assert!(
            documents.contains(&quoted),
            "{} is missing from the documents phase CHECK (comments stripped)",
            phase.slug()
        );
    }
}

#[test]
fn the_phases_table_seeds_exactly_the_enum_and_nothing_else() {
    let rows = seeded_rows(&sql_without_comments(TABLES_MIGRATION), "chronology_phases");
    let seeded: BTreeSet<&str> = rows
        .iter()
        .filter_map(|row| row.first().map(String::as_str))
        .collect();
    let declared: BTreeSet<&str> = ALL_CASE_PHASES.iter().map(|p| p.slug()).collect();

    assert_eq!(
        seeded, declared,
        "the seeded phase rows and the Rust enum are one vocabulary"
    );
    // Vacuity: an empty read would satisfy the set comparison against an empty
    // set, so the count is what proves the reader saw anything at all.
    assert_eq!(rows.len(), ALL_CASE_PHASES.len());
}

// ─── the tag vocabulary (ruling R-F) ─────────────────────────────────────────

#[test]
fn the_tags_table_seeds_exactly_the_declared_vocabulary() {
    let rows = seeded_rows(&sql_without_comments(TAGS_MIGRATION), "chronology_tags");
    let seeded: BTreeSet<&str> = rows
        .iter()
        .filter_map(|row| row.first().map(String::as_str))
        .collect();
    let declared: BTreeSet<&str> = CHRONOLOGY_TAGS.iter().map(|(id, _)| *id).collect();

    assert_eq!(
        seeded, declared,
        "chronology_tags and domain::chronology::CHRONOLOGY_TAGS are one \
         vocabulary — an event tagged with a token that has no row renders in a \
         neutral chip with no label, and nothing else notices"
    );
    assert_eq!(rows.len(), CHRONOLOGY_TAGS.len(), "vacuity guard");
}

#[test]
fn every_seeded_tag_carries_the_label_this_build_expects() {
    // The LABEL is asserted here and the phases' is not, deliberately: a phase
    // label is Roman's to rename at will, while a tag label is also the chip's
    // text AND the word the code's vocabulary carries, so the two must agree.
    let rows = seeded_rows(&sql_without_comments(TAGS_MIGRATION), "chronology_tags");
    for (id, label) in CHRONOLOGY_TAGS {
        let found = rows
            .iter()
            .find(|row| row.first().map(String::as_str) == Some(*id))
            .unwrap_or_else(|| panic!("{id} has no row in the tags migration"));
        assert_eq!(
            found.get(1).map(String::as_str),
            Some(*label),
            "the tags migration and the code disagree about {id}'s label"
        );
    }
}

#[test]
fn every_seeded_tag_carries_a_colour_a_browser_can_use() {
    let rows = seeded_rows(&sql_without_comments(TAGS_MIGRATION), "chronology_tags");
    assert!(!rows.is_empty(), "vacuity guard");
    for row in &rows {
        let colour = row.get(2).map(String::as_str).unwrap_or("");
        assert!(
            colour.starts_with('#') && colour.len() == 7,
            "tag {:?} has colour {colour:?}, which is not a #rrggbb value — the \
             chip and the dot both read it raw",
            row.first()
        );
    }
}

// ─── the seed's re-point map, which outlives the file ────────────────────────

#[test]
fn the_repoint_map_still_accounts_for_every_reference_it_ever_saw() {
    // The file is gone; the MAP is history that must stay readable. Eleven
    // references were decided — seven re-pointed (one to itself), four marked as
    // having no document. Nothing may quietly leave either list.
    use crate::chronology::seed::{NO_DOCUMENT_YET, REPOINT_MAP};

    assert_eq!(REPOINT_MAP.len(), 7);
    assert_eq!(NO_DOCUMENT_YET.len(), 4);
    for (from, _) in REPOINT_MAP {
        assert!(
            !NO_DOCUMENT_YET.contains(from),
            "{from} is in both lists; `plan_link` would be order-dependent"
        );
    }
}
