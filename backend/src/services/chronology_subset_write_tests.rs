//! ⚑ THE ONE WRITE PATH, PROVED BY SCANNING THE CRATE.
//!
//! T1.3's rule is that every write to the three subset tables goes through one
//! service, whose every operation ends at the seal that writes the history row.
//! Nothing in Rust's type system expresses that: a second
//! `INSERT INTO chronology_subset_events` in some future handler would compile,
//! serve, and land rows with no history and no attribution — and the only
//! symptom would be a history that was quietly incomplete, which is exactly the
//! kind of absence nobody notices.
//!
//! So the rule is enforced by reading the source. Every statement that writes
//! one of the four tables must be inside
//! `repositories::pipeline_repository::chronology_subset_write`, and every
//! caller of that module must be `services::chronology_subset_write`.
//!
//! The sibling `chronology_subset_guard_tests` proves what the seal RECORDS;
//! this proves nothing gets to skip it.

/// The one file allowed to hold a subset-writing SQL statement.
// STRUCTURAL: a repo-internal source path, not deployment configuration. A moved
// file fails this test rather than quietly leaving the rule unenforced.
const THE_WRITE_FLOOR: &str = "src/repositories/pipeline_repository/chronology_subset_write.rs";

/// The one module allowed to call it.
// STRUCTURAL: a repo-internal source path, not deployment configuration. A moved
// file fails this test rather than quietly leaving the rule unenforced.
const THE_WRITE_PATH: &str = "src/services/chronology_subset_write.rs";

/// The write statements no other file may contain.
///
/// Spelled as `<VERB> <table>` fragments rather than as a parser, because that
/// is what the SQL string literals in this codebase actually look like and a
/// real parser would be a second thing to get wrong.
const WRITE_STATEMENTS: &[&str] = &[
    "INSERT INTO chronology_subsets",
    "UPDATE chronology_subsets",
    "DELETE FROM chronology_subsets",
    "INSERT INTO chronology_subset_events",
    "UPDATE chronology_subset_events",
    "DELETE FROM chronology_subset_events",
    "INSERT INTO chronology_subset_history",
    "UPDATE chronology_subset_history",
    "DELETE FROM chronology_subset_history",
    "INSERT INTO scenario_subsets",
    "UPDATE scenario_subsets",
    "DELETE FROM scenario_subsets",
];

/// The functions the write floor exports, which only the write path may name.
const FLOOR_FUNCTIONS: &[&str] = &[
    "insert_subset",
    "update_subset",
    "soft_delete_subset",
    "undelete_subset",
    "touch_subset",
    "upsert_subset_event",
    "retain_subset_events",
    "insert_subset_history",
    "attach_subset_to_scenario",
    "detach_subset_from_scenario",
];

/// One file's source with its `//` comments removed.
///
/// ⚑ Required before any scan of this repository's source: this codebase
/// documents its rules next to its rules, so a scanner hunting for
/// `INSERT INTO chronology_subsets` finds the module header explaining that it
/// appears exactly once. The rule is stated once in `domain::wording_tests`.
fn without_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| match line.find("//") {
            Some(at) => &line[..at],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every `.rs` file under `src/`, recursively.
fn every_source_file() -> Vec<std::path::PathBuf> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// A path relative to the crate root, for a readable failure message.
fn relative(path: &std::path::Path) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[test]
fn no_file_but_the_floor_writes_a_subset_table() {
    let mut offenders = Vec::new();
    let mut floor_seen = false;

    for path in every_source_file() {
        let name = relative(&path);
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        let source = without_comments(&raw);
        for statement in WRITE_STATEMENTS {
            if !source.contains(statement) {
                continue;
            }
            if name == THE_WRITE_FLOOR {
                floor_seen = true;
                continue;
            }
            // The test files themselves quote the statements they forbid; the
            // integration proof does not, because it goes through the API.
            if name == relative(std::path::Path::new(file!())) || name.ends_with("_tests.rs") {
                continue;
            }
            offenders.push(format!("{name} contains `{statement}`"));
        }
    }

    assert!(
        offenders.is_empty(),
        "a subset table is written outside the one write floor ({THE_WRITE_FLOOR}). \
         Every one of these lands rows with no history row and no attribution:\n  {}",
        offenders.join("\n  ")
    );
    // ⚑ ANTI-VACUITY. The assertion above is satisfied by finding nothing, so a
    // renamed floor or a changed SQL spelling would leave it green while
    // checking no statement at all.
    assert!(
        floor_seen,
        "the scan found no write statement in {THE_WRITE_FLOOR} — either the file \
         moved or the SQL is spelled differently, and this proof has stopped proving"
    );
}

#[test]
fn no_module_but_the_write_path_calls_the_floor() {
    // The second half. A handler that imported the repository directly could
    // open its own transaction and commit without a seal, which the statement
    // scan above would not see — the SQL would still be in the right file.
    let mut offenders = Vec::new();
    let mut path_seen = false;

    for path in every_source_file() {
        let name = relative(&path);
        if name == THE_WRITE_FLOOR || name.ends_with("_tests.rs") {
            continue;
        }
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        let source = without_comments(&raw);
        for function in FLOOR_FUNCTIONS {
            // The floor's own names, as an import list or a call, only ever
            // appear where they are allowed: in the write path, and in the guard
            // for `insert_subset_history` — which IS the seal.
            let imported = source.contains(&format!("{function},"))
                || source.contains(&format!("{function}}}"))
                || source.contains(&format!("{function}("));
            if !imported {
                continue;
            }
            if name == THE_WRITE_PATH {
                path_seen = true;
                continue;
            }
            if name == "src/services/chronology_subset_guard.rs"
                && *function == "insert_subset_history"
            {
                // The seal is the one caller of the history insert, by design —
                // that is what makes "one history row per write" structural.
                continue;
            }
            offenders.push(format!("{name} names `{function}`"));
        }
    }

    assert!(
        offenders.is_empty(),
        "the subset write floor is called from outside {THE_WRITE_PATH}, so a write \
         could commit without its seal:\n  {}",
        offenders.join("\n  ")
    );
    assert!(
        path_seen,
        "the scan found no floor call in {THE_WRITE_PATH} — the module moved or was \
         renamed, and this proof has stopped proving"
    );
}
