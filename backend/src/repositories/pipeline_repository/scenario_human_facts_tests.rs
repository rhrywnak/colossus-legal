//! Tests for `scenario_human_facts` (C4) AND the §8 write-path invariant.
//!
//! The headline is `scan_and_merge_paths_write_only_their_own_tables`: v2 §8 says
//! "re-gathering never edits human content", and this is where that stops being a
//! promise and becomes a build failure.
//!
//! ## Why the allowlist is stated POSITIVELY
//!
//! A test that only denies ("no scan file mentions `scenario_human_facts`")
//! passes vacuously the day someone renames the table, and says nothing about
//! what the scan paths ARE allowed to touch. Naming the permitted table families
//! instead means a scan path that grows a write to ANY new table — human content
//! or otherwise — trips the test and has to be looked at. The denial is then a
//! consequence of the allowlist rather than a separate rule to remember.
//!
//! Standing Rule 21: when an invariant must hold across many files, assert it by
//! scanning the files.

use std::fs;
use std::path::{Path, PathBuf};

use super::*;

// ─── The insert's shape ──────────────────────────────────────────────────────

/// The VALUES list, as a vector of placeholder tokens.
///
/// ## Why parse rather than match the literal
///
/// The first version of these tests asserted `contains("VALUES ($1, …, $7, $7)")`
/// and asserted `!contains("$8")`. Both broke the moment task 1.5 added the
/// `kind` column and shifted every number — and they broke by reporting the
/// stamp invariant as violated when it was perfectly intact. A test that fails
/// for the wrong reason teaches the reader to distrust it. Reading the tokens out
/// and comparing the LAST TWO pins the actual property (whatever their numbers
/// are, they are the same one) and survives the next column.
fn values_placeholders() -> Vec<String> {
    let start = INSERT_HUMAN_FACT_SQL
        .find("VALUES (")
        .expect("the insert has a VALUES list");
    let rest = &INSERT_HUMAN_FACT_SQL[start + "VALUES (".len()..];
    let end = rest.find(')').expect("the VALUES list is closed");
    rest[..end]
        .split(',')
        .map(|t| t.trim().to_string())
        .collect()
}

/// A fresh fact has equal created/updated stamps, so "edited" is readable
/// without a separate flag.
#[test]
fn a_new_fact_has_equal_created_and_updated_stamps() {
    let placeholders = values_placeholders();
    let last = placeholders.last().expect("at least one placeholder");
    let second_last = &placeholders[placeholders.len() - 2];
    assert_eq!(
        last, second_last,
        "created_at and updated_at must bind the SAME parameter on insert, so an \
         untouched fact is distinguishable from an edited one: {INSERT_HUMAN_FACT_SQL}"
    );
}

/// Every column the table requires is written.
#[test]
fn the_insert_writes_every_column_a_human_fact_needs() {
    for column in [
        "scenario_id",
        "text",
        "occurred_on",
        "date_type",
        "person_refs",
        "authored_by",
        // Task 1.5. Without it every note would insert as the DEFAULT 'fact',
        // and a watch-list note would silently file itself as a human fact —
        // the wrong kind of statement, shown in the wrong place.
        "kind",
        "created_at",
        "updated_at",
    ] {
        assert!(
            INSERT_HUMAN_FACT_SQL.contains(column),
            "the human-fact insert must write {column}"
        );
    }

    // One placeholder per column, with the two stamps sharing the last one.
    let columns = INSERT_HUMAN_FACT_SQL
        .split_once('(')
        .and_then(|(_, rest)| rest.split_once(')'))
        .map(|(cols, _)| cols.split(',').count())
        .expect("the insert names its columns");
    let placeholders = values_placeholders();
    assert_eq!(
        placeholders.len(),
        columns,
        "every column needs a value: {INSERT_HUMAN_FACT_SQL}"
    );
    // Distinct numbers = columns - 1, because the stamps share one.
    let distinct: std::collections::BTreeSet<&String> = placeholders.iter().collect();
    assert_eq!(
        distinct.len(),
        columns - 1,
        "exactly one placeholder is reused (the two stamps); the rest are \
         distinct: {INSERT_HUMAN_FACT_SQL}"
    );
}

/// A delete is fenced by scenario, not just by id.
#[test]
fn a_delete_is_scoped_to_its_scenario() {
    // Without the scenario fence, guessing a UUID would delete another
    // scenario's authored content — and a human fact has no citation to
    // reconstruct it from.
    //
    // Asserted against the PRODUCTION statement. This test used to declare its
    // own copy of the SQL as a local and assert on that, which proved only that
    // the test file contained the string it had just written — the delete could
    // have lost its fence entirely and this would still have passed.
    assert!(
        DELETE_HUMAN_FACT_SQL.contains("scenario_id = $2"),
        "the scenario is the fence: {DELETE_HUMAN_FACT_SQL}"
    );
}

// ─── The §8 write-path invariant ─────────────────────────────────────────────

/// The table families the scan / gather / merge paths are permitted to write.
///
/// Stated positively (see the module doc). Each entry is a table these paths
/// legitimately own:
///
/// * `scenario_fact_refs` — the candidate's workbench STATE;
/// * `scan_runs` / `scan_run_verdicts` / `scan_run_merges` — the scan's own
///   record of what it did;
/// * `scenario_candidate_ordinals` — candidate identity, memoized on gather.
///
/// Anything else appearing in a write statement in those modules is a change to
/// what the scan can reach, and must be looked at rather than absorbed.
const SCAN_WRITABLE_TABLES: &[&str] = &[
    "scenario_fact_refs",
    "scan_runs",
    "scan_run_verdicts",
    "scan_run_merges",
    "scenario_candidate_ordinals",
];

/// The modules that make up the scan / gather / merge write surface.
const SCAN_PATH_FILES: &[&str] = &[
    "src/services/theme_scan.rs",
    "src/services/theme_scan_persist.rs",
    "src/services/scan_run_enrich.rs",
    "src/api/scenario_gather.rs",
    "src/api/scenario_theme_scan.rs",
    "src/repositories/pipeline_repository/scan_runs.rs",
    "src/repositories/pipeline_repository/scan_run_verdicts.rs",
    "src/repositories/pipeline_repository/scan_run_merges.rs",
    "src/repositories/pipeline_repository/scenario_candidate_ordinals.rs",
];

/// The tables recording HUMAN acts, which no scan path may ever write (v2 §8).
///
/// The first three hold human-authored CONTENT. `scenario_status_transitions`
/// (task 1.5) holds something adjacent and equally off-limits: the record of who
/// declared a scenario ready. A scan able to append there could manufacture an
/// attribution, and a scan able to write it at all could put a scenario in front
/// of a witness with no human having said so.
const HUMAN_AUTHORED_TABLES: &[&str] = &[
    "scenario_human_facts",
    "scenario_responses",
    "response_items",
    "scenario_status_transitions",
    // Task 1.7F: a human's correction of a machine-written interrogatory
    // question. It belongs here for the sharpest possible reason — the row exists
    // BECAUSE the machine got it wrong, so a scan that rewrote it would restore
    // the very text a human had already rejected, and would do it silently.
    "evidence_summary_overrides",
    // Task 2.10: a human's link from a statement to the accusation it bears on,
    // and the ledger of every such act. Sharper still than the row above: this one
    // exists because the extraction linked NOTHING, and it is what decides whether
    // a card can be ruled at all. A scan that overwrote it would re-lock 94 cards
    // a human had unlocked, with no error anywhere — and a scan that could append
    // to the ledger could manufacture an attribution for a decision nobody made.
    "evidence_allegation_links",
    "evidence_allegation_link_events",
];

fn read_source(relative: &str) -> Option<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
    fs::read_to_string(&path).ok()
}

/// Every `INSERT INTO` / `UPDATE` / `DELETE FROM` target in a source file.
///
/// Deliberately crude — a substring walk, not a SQL parser. It only has to find
/// the table NAME after a write keyword, and being crude means it also catches a
/// write hidden in a `format!`ed string, which a parser keyed on valid SQL would
/// miss.
///
/// COMMENT LINES ARE STRIPPED FIRST. Without that, prose trips it: `theme_scan`
/// has a doc comment reading "The promote-to-`running` UPDATE matched zero rows",
/// which a raw walk reports as a write to a table called `matched`. A test that
/// cries wolf on comments is a test people learn to ignore.
fn write_targets(source: &str) -> Vec<String> {
    let code: String = source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !(trimmed.starts_with("//") || trimmed.starts_with('*'))
        })
        .collect::<Vec<_>>()
        .join("\n");
    let source = code.as_str();

    let mut targets = Vec::new();
    for keyword in ["INSERT INTO ", "UPDATE ", "DELETE FROM "] {
        let mut cursor = 0usize;
        while let Some(at) = source[cursor..].find(keyword) {
            let start = cursor + at + keyword.len();
            let table: String = source[start..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !table.is_empty() {
                targets.push(table);
            }
            cursor = start;
        }
    }
    targets
}

/// THE §8 INVARIANT. Scan and merge paths write only their own tables.
///
/// This is the structural half of "re-gathering never edits human content". A
/// scan that grew a write into `scenario_human_facts` — or into anything else
/// not on the allowlist — fails here, with the file and the table named.
#[test]
fn scan_and_merge_paths_write_only_their_own_tables() {
    let mut offenders: Vec<String> = Vec::new();
    let mut files_read = 0usize;

    for relative in SCAN_PATH_FILES {
        let Some(source) = read_source(relative) else {
            continue;
        };
        files_read += 1;
        for table in write_targets(&source) {
            if !SCAN_WRITABLE_TABLES.contains(&table.as_str()) {
                offenders.push(format!("{relative} writes {table}"));
            }
        }
    }

    // Anti-vacuity: a renamed or moved module would empty the scan set and this
    // test would pass while checking nothing.
    assert!(
        files_read >= 5,
        "expected the scan/gather/merge modules on disk; found {files_read} — the \
         file list is stale and this test is no longer checking anything"
    );

    assert!(
        offenders.is_empty(),
        "a scan/gather/merge path writes a table outside its allowlist. If the new \
         table is human-authored content this is a v2 §8 violation (re-gathering \
         must never edit human content); if it is legitimately the scan's own, add \
         it to SCAN_WRITABLE_TABLES deliberately:\n  {}",
        offenders.join("\n  ")
    );
}

/// The same invariant from the other side, named explicitly.
///
/// Redundant with the allowlist by construction — and worth stating anyway,
/// because this is the sentence v2 §8 actually contains, and a reader looking for
/// it should find it rather than have to derive it.
#[test]
fn no_scan_path_mentions_a_human_authored_table() {
    let mut offenders: Vec<String> = Vec::new();

    for relative in SCAN_PATH_FILES {
        let Some(source) = read_source(relative) else {
            continue;
        };
        for table in HUMAN_AUTHORED_TABLES {
            if source.contains(table) {
                offenders.push(format!("{relative} names {table}"));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "v2 §8: re-gathering never edits human content. A scan path naming a \
         human-authored table is either that violation or a comment that should \
         not be there:\n  {}",
        offenders.join("\n  ")
    );
}

/// The human-fact writer is reachable from exactly one place.
///
/// The augmentation service is the only legitimate caller. This is the C4
/// equivalent of the anchor choke point task 1.1 established for rulings.
/// Walk every source file, returning those that call any of `needles`.
///
/// Shared by the two caller-family tests below. The definition's own module and
/// its tests are not callers.
fn callers_of(needles: &[&str], skip_containing: &str) -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }

    let mut files = Vec::new();
    walk(&root, &mut files);
    files.sort();

    let mut callers = Vec::new();
    for path in files {
        let rel = path
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        // Skip the definition's own module family, and skip TEST files: a test
        // naming a writer is not a write path, and this file itself lists the
        // needles it searches for.
        if rel.contains(skip_containing) || rel.contains("_tests.rs") {
            continue;
        }
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };
        if needles.iter().any(|n| source.contains(n)) {
            callers.push(rel);
        }
    }
    callers
}

/// The TALKING-POINT writers are reachable from exactly one place too.
///
/// C4 had this guard from the start; C5 did not, and the asymmetry was real: a
/// scan module that grew a call to `insert_scenario_response` would have violated
/// §8 with nothing failing, because the SQL-level allowlist only reads the files
/// on its own list.
#[test]
fn the_talking_point_writers_have_one_caller_family() {
    for caller in callers_of(
        &[
            "insert_scenario_response(",
            "insert_response_item(",
            "delete_responses_for_scenario(",
        ],
        "scenario_responses",
    ) {
        assert!(
            caller.contains("augmentation"),
            "only the augmentation service may write talking points; {caller} calls \
             a C5 writer (v2 §8)"
        );
    }
}

#[test]
fn the_human_fact_writer_has_one_caller_family() {
    // Uses the shared `callers_of` walk. This test carried its OWN copy of the
    // directory walk until task 1.5 — two implementations of one rule, and the
    // copy did not skip `_tests.rs`, so it was one test file away from failing
    // for a reason that was not a violation.
    for caller in callers_of(&["insert_human_fact("], "scenario_human_facts") {
        assert!(
            caller.contains("augmentation"),
            "only the augmentation service may write human facts; {caller} calls \
             insert_human_fact"
        );
    }
}

/// The READY GATE has one caller family too (task 1.5).
///
/// Both halves of the act are guarded: appending a transition, and changing the
/// status it describes. A module that could do either on its own could produce a
/// scenario that is ready with nobody's name against it, or a record of a
/// promotion that never happened. §5/§6 make readiness a human act; this is where
/// "only one path performs it" stops being a convention.
#[test]
fn the_readiness_writers_have_one_caller_family() {
    for caller in callers_of(
        &["insert_status_transition(", "update_scenario_status("],
        "scenario_status_transitions",
    ) {
        assert!(
            caller.contains("readiness"),
            "only the readiness service may change a scenario's readiness; \
             {caller} calls a ready-gate writer (v2 §5/§6)"
        );
    }
}

/// Anti-vacuity for the caller-family guards.
///
/// Every one of them passes trivially if `callers_of` finds nothing — a renamed
/// function, a moved module, or a bug in the walk would silence all three at
/// once and they would keep reporting green. Asserting that the expected caller
/// IS found makes the walk prove itself.
#[test]
fn the_caller_family_scan_actually_finds_the_callers() {
    for (needles, skip, expected) in [
        (
            vec!["insert_human_fact("],
            "scenario_human_facts",
            "augmentation",
        ),
        (
            vec!["insert_scenario_response("],
            "scenario_responses",
            "augmentation",
        ),
        (
            vec!["insert_status_transition("],
            "scenario_status_transitions",
            "readiness",
        ),
    ] {
        let callers = callers_of(&needles, skip);
        assert!(
            callers.iter().any(|c| c.contains(expected)),
            "the scan for {needles:?} found no caller containing '{expected}' \
             (found {callers:?}) — the walk is checking nothing"
        );
    }
}
