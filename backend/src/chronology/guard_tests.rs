//! THE PERMANENT VALIDATION GUARD (task A4).
//!
//! This is the test that would have caught the ten dead document links the day
//! they appeared. It reads the REAL seed file off disk — not a fixture — and
//! requires every event's phase, tag, date and document reference to be
//! accounted for, and the phase vocabulary to agree across all four places it
//! is written down.
//!
//! ## The two directions of the mutation proof
//!
//! A guard that passes is worthless unless you know it can fail, and a guard
//! that reads a fixture is worthless no matter what it says. So:
//!
//! 1. `the_real_seed_file_has_no_problems` runs the checker over the real file.
//! 2. `the_checker_catches_every_class_of_problem` runs the SAME checker over a
//!    deliberately corrupted copy and requires each class to be reported.
//! 3. `the_guard_reads_the_real_file_not_a_fixture` pins named sentinels that
//!    only the real corpus contains — the wording-fixture lesson, where a test
//!    passed happily against data nobody shipped.
//!
//! ## What this guard does NOT prove
//!
//! That a link's target exists **in the database**. A unit test has no Postgres.
//! What it proves is that every reference is ACCOUNTED FOR by the re-point map
//! or the known-absent list, with nothing falling through. The database half is
//! `seed_execute::check_targets`, which the one-shot runs in every mode
//! including the read-only one, and which refuses before writing anything.

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::chronology::seed::{
    build_plan, parse_source, SourceTimeline, NO_DOCUMENT_YET, REPOINT_MAP,
};
use crate::domain::case_phase::{CasePhase, ALL_CASE_PHASES};
use crate::domain::chronology::{is_known_tag, CHRONOLOGY_TAGS};

/// The seed file, relative to the backend crate root.
const SEED_RELATIVE_PATH: &str = "../frontend/public/data/timeline.json";

/// The migration that seeded `chronology_phases`.
const PHASES_MIGRATION: &str = "pipeline_migrations/20260825105447_chronology_phases.sql";

/// The migration that put the phase CHECK on `documents`.
const DOCUMENTS_PHASE_MIGRATION: &str = "pipeline_migrations/20260817150412_add_document_phase.sql";

fn crate_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// Read the real seed document, failing loudly if it has moved.
fn real_source() -> SourceTimeline {
    let path = crate_path(SEED_RELATIVE_PATH);
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "the chronology seed file is not readable at {}: {e}. If it moved, this \
             guard must move with it — deleting the guard is not the fix",
            path.display()
        )
    });
    parse_source(&path.to_string_lossy(), &raw).expect("the seed file parses")
}

/// SQL with every `--` comment removed, so a scan cannot be fooled by a token
/// that only appears in prose.
fn sql_without_comments(relative: &str) -> String {
    let raw = std::fs::read_to_string(crate_path(relative)).expect("the migration is readable");
    raw.lines()
        .map(|line| match line.find("--") {
            Some(at) => &line[..at],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every problem the checker can find, as human-readable lines.
///
/// Returns a list rather than a `bool` so a failure names WHICH event and WHY,
/// and so the mutation proof can assert that each class is detected.
fn problems(source: &SourceTimeline) -> Vec<String> {
    let mut found = Vec::new();
    let phase_ids: BTreeSet<&str> = source.phases.iter().map(|p| p.id.as_str()).collect();

    if source.phases.len() != ALL_CASE_PHASES.len() {
        found.push(format!(
            "phase count is {}, the Rust vocabulary has {}",
            source.phases.len(),
            ALL_CASE_PHASES.len()
        ));
    }
    for phase in &source.phases {
        if CasePhase::from_slug(&phase.id).is_none() {
            found.push(format!("phase '{}' is not a CasePhase slug", phase.id));
        }
    }

    let file_tags: BTreeSet<&str> = source.categories.keys().map(String::as_str).collect();
    let code_tags: BTreeSet<&str> = CHRONOLOGY_TAGS.iter().map(|(t, _)| *t).collect();
    if file_tags != code_tags {
        found.push(format!(
            "the file's tag vocabulary {file_tags:?} differs from the code's {code_tags:?}"
        ));
    }

    for event in &source.events {
        if chrono::NaiveDate::parse_from_str(&event.date, "%Y-%m-%d").is_err() {
            found.push(format!("{}: date '{}' is not ISO", event.id, event.date));
        }
        if !is_known_tag(&event.category) {
            found.push(format!("{}: tag '{}' is unknown", event.id, event.category));
        }
        if !phase_ids.contains(event.phase.as_str()) {
            found.push(format!("{}: phase '{}' is unknown", event.id, event.phase));
        }
        if let Some(doc) = event.document_id.as_deref() {
            let mapped = REPOINT_MAP.iter().any(|(from, _)| *from == doc);
            if !mapped && !NO_DOCUMENT_YET.contains(&doc) {
                found.push(format!("{}: document '{doc}' is unaccounted for", event.id));
            }
        }
    }
    found
}

#[test]
fn the_real_seed_file_has_no_problems() {
    let found = problems(&real_source());
    assert!(
        found.is_empty(),
        "the real chronology seed file failed validation:\n  {}",
        found.join("\n  ")
    );
}

#[test]
fn the_guard_reads_the_real_file_not_a_fixture() {
    let source = real_source();

    // NAMED SENTINEL — the Tighe post-appeal order. Design R13 rules that this
    // event stays in `appeals` even though the document it points at is tagged
    // `probate`, so it is the row most likely to be "corrected" by mistake.
    let tighe = source
        .events
        .iter()
        .find(|e| e.id == "e016")
        .expect("e016 is in the real corpus; a fixture would not have it");
    assert_eq!(tighe.title, "Judge Tighe Issues Post-Appeal Order");
    assert_eq!(tighe.date, "2012-04-12");
    assert_eq!(tighe.phase, "appeals", "design R13: trust the date");
    assert_eq!(
        tighe.document_id.as_deref(),
        Some("doc-tighe-opinion-041212")
    );

    // NAMED SENTINEL — the phase whose date_range carries a U+2013 EN-DASH and
    // the word Present. A hyphen here would be a silent visual regression.
    let last = source
        .phases
        .iter()
        .find(|p| p.id == "civil_lawsuit")
        .expect("the real corpus has a civil_lawsuit phase");
    assert_eq!(last.label, "COMPLAINT");
    assert_eq!(last.date_range, "2014\u{2013}Present");
}

#[test]
fn the_checker_catches_every_class_of_problem() {
    let mut broken = real_source();

    // 1 · a date that is not ISO
    broken.events[0].date = "18 August 2008".to_string();
    // 2 · a tag outside the vocabulary
    broken.events[1].category = "hearsay".to_string();
    // 3 · a phase that is not one of the case's
    broken.events[2].phase = "mediation".to_string();
    // 4 · a document reference in neither list
    broken.events[3].document_id = Some("doc-nobody-mapped-this".to_string());
    // 5 · a phase slug the Rust enum does not know
    broken.phases[0].id = "pre_probate".to_string();
    // 6 · a tag vocabulary that has drifted from the code's
    broken.categories.remove("personal");

    let found = problems(&broken);
    let joined = found.join("\n");

    assert!(
        joined.contains("is not ISO"),
        "date class missed:\n{joined}"
    );
    assert!(
        joined.contains("tag 'hearsay' is unknown"),
        "tag class missed:\n{joined}"
    );
    assert!(
        joined.contains("phase 'mediation' is unknown"),
        "phase class missed:\n{joined}"
    );
    assert!(
        joined.contains("unaccounted for"),
        "document class missed:\n{joined}"
    );
    assert!(
        joined.contains("is not a CasePhase slug"),
        "slug class missed:\n{joined}"
    );
    assert!(
        joined.contains("tag vocabulary"),
        "vocabulary class missed:\n{joined}"
    );
}

#[test]
fn the_real_file_plans_cleanly_and_every_reference_is_decided() {
    let source = real_source();
    let plan = build_plan(&source).expect("the real seed file must plan without refusal");

    // Exact, not a threshold: the file is FROZEN and retires after the seed
    // (design R15), so a change to these numbers is a change to a file nobody
    // should be editing — which is exactly what this guard should notice.
    assert_eq!(plan.events.len(), source.events.len());
    assert_eq!(
        plan.link_count(),
        REPOINT_MAP.len(),
        "one link per mapped id"
    );
    assert_eq!(plan.unlinkable().len(), NO_DOCUMENT_YET.len());

    // Every reference is decided one way or the other — nothing fell through.
    let referenced = source
        .events
        .iter()
        .filter(|e| e.document_id.is_some())
        .count();
    assert_eq!(referenced, plan.link_count() + plan.unlinkable().len());
}

#[test]
fn the_phase_vocabulary_agrees_across_all_four_places_it_is_written() {
    let source = real_source();
    let file: BTreeSet<&str> = source.phases.iter().map(|p| p.id.as_str()).collect();
    let code: BTreeSet<&str> = ALL_CASE_PHASES.iter().map(|p| p.slug()).collect();
    assert_eq!(file, code, "the seed file and the Rust enum disagree");

    let phases_sql = sql_without_comments(PHASES_MIGRATION);
    let documents_sql = sql_without_comments(DOCUMENTS_PHASE_MIGRATION);
    for slug in &code {
        let quoted = format!("'{slug}'");
        assert!(
            phases_sql.contains(&quoted),
            "{slug} is missing from the chronology_phases migration (comments stripped)"
        );
        assert!(
            documents_sql.contains(&quoted),
            "{slug} is missing from the documents phase CHECK (comments stripped)"
        );
    }
}

#[test]
fn the_migration_seeds_every_phase_label_and_range_byte_for_byte() {
    let source = real_source();
    let sql = sql_without_comments(PHASES_MIGRATION);
    for phase in &source.phases {
        for (what, value) in [
            ("label", &phase.label),
            ("date_range", &phase.date_range),
            ("color", &phase.color),
        ] {
            // SQL doubles an embedded single quote; nothing else is escaped.
            let needle = value.replace('\'', "''");
            assert!(
                sql.contains(&needle),
                "phase {}: {what} '{value}' is not seeded verbatim by the migration",
                phase.id
            );
        }
        if let Some(description) = &phase.description {
            let needle = description.replace('\'', "''");
            assert!(
                sql.contains(&needle),
                "phase {}: description is not seeded verbatim",
                phase.id
            );
        }
    }
}
