//! Tests for `services::chronology_guard` — the one guarded write path.
//!
//! Three things are pinned here, and each of them is a claim the Phase C report
//! makes out loud:
//!
//!  1. the four action words are the four the migration's CHECK allows;
//!  2. the snapshot carries every column of the event, `deleted_at` included;
//!  3. the seal is the ONLY committer and the ONLY history writer, proved by
//!     scanning this crate's source with its comments stripped.

use super::*;
use crate::repositories::pipeline_repository::chronology_write::ChronologyEventStateRow;
use chrono::{DateTime, NaiveDate, Utc};

/// The migration that declares the action CHECK.
// STRUCTURAL: a repo-internal pointer to one immutable, version-controlled
// migration. Identical in every environment.
const TABLES_MIGRATION: &str = "pipeline_migrations/20260825105447_chronology_tables.sql";

/// One event row, so the snapshot can be asserted without a database.
fn row(deleted: bool) -> ChronologyEventStateRow {
    let at: DateTime<Utc> = "2026-08-26T10:00:00Z".parse().expect("a real timestamp");
    ChronologyEventStateRow {
        id: uuid::uuid!("11111111-2222-3333-4444-555555555555"),
        case_slug: "awad".to_string(),
        event_date: NaiveDate::from_ymd_opt(2012, 4, 12).expect("a real date"),
        date_precision: "day".to_string(),
        approximate: false,
        phase: "appeals".to_string(),
        title: "Judge Tighe Issues Post-Appeal Order".to_string(),
        fact: Some("Judge Tighe issues Opinion and Order.".to_string()),
        attributes: serde_json::json!({ "tags": ["court_action"], "source": "legacy_json" }),
        created_by: Some("roman".to_string()),
        created_at: at,
        updated_by: Some("marie".to_string()),
        updated_at: at,
        deleted_at: if deleted { Some(at) } else { None },
    }
}

fn read_migration() -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(root.join(TABLES_MIGRATION))
        .unwrap_or_else(|cause| panic!("{TABLES_MIGRATION} is not on disk: {cause}"))
}

/// Every `.rs` file in this crate's `src`, recursively — INCLUDING test files.
///
/// The vacuity guard counts these, so it must see everything on disk.
/// [`production_sources`] is what the three scans below actually read.
fn crate_sources() -> Vec<std::path::PathBuf> {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
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
    let mut out = Vec::new();
    walk(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut out,
    );
    out.sort();
    out
}

/// The crate's PRODUCTION source: everything except test modules.
///
/// ## Why the test files are excluded, and why that is not a loophole
///
/// This very file quotes `INSERT INTO chronology_events (` and `insert_history(`
/// inside its own assertion messages, so a scan that read it would report
/// itself. Test modules cannot reach a database in this project — there is no
/// integration tier, and every one of them runs without a pool — so a SQL string
/// in a `_tests.rs` file is a fixture or a message, never a statement anything
/// executes. What the scans are protecting is the set of statements a request
/// can reach, and that set is exactly the production files.
fn production_sources() -> Vec<std::path::PathBuf> {
    crate_sources()
        .into_iter()
        .filter(|path| {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            !name.contains("_tests.") && !name.starts_with("test_")
        })
        .collect()
}

/// Source with its `//` comments removed.
///
/// ⚑ Required before any scan of this repository's source. This codebase
/// documents its rules next to its rules, so a scanner searching for a statement
/// finds the DOCUMENTATION of that statement first. The rule and its instances
/// are stated once in `domain::wording_tests`.
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

// ── 1 · the action vocabulary ───────────────────────────────────────────────

#[test]
fn the_action_words_match_the_migrations_check() {
    // A fifth action would be refused by Postgres at write time, as a 500 in
    // front of whoever pressed the button. This is where it is refused instead.
    let sql = without_comments(&read_migration());
    let at = sql
        .find("chronology_event_history_action_valid\n    CHECK (action IN (")
        .or_else(|| sql.find("CHECK (action IN ("))
        .expect("the history CHECK is in the migration");
    let clause = &sql[at..at + 120];

    for action in HistoryAction::ALL {
        let quoted = format!("'{}'", action.as_str());
        assert!(
            clause.contains(&quoted),
            "{quoted} is spellable in Rust but not allowed by the CHECK: {clause}"
        );
    }
    // The other direction: an action the CHECK allows that Rust cannot spell is
    // a state nothing can ever write, which is a row shape nobody maintains.
    let allowed = clause.matches('\'').count() / 2;
    assert_eq!(
        allowed,
        HistoryAction::ALL.len(),
        "the CHECK allows {allowed} actions and this build can spell {}: {clause}",
        HistoryAction::ALL.len()
    );
}

#[test]
fn every_action_word_is_distinct() {
    let mut seen: Vec<&str> = HistoryAction::ALL.iter().map(|a| a.as_str()).collect();
    seen.sort_unstable();
    let before = seen.len();
    seen.dedup();
    assert_eq!(before, seen.len(), "two actions share a stored token");
}

// ── 2 · the snapshot ────────────────────────────────────────────────────────

#[test]
fn the_snapshot_carries_every_column_of_the_event() {
    // ⚑ A snapshot missing a field is a history that quietly forgets it, and
    // nothing else in the system would notice — the row is written, the count
    // is right, and the diff between two snapshots simply never mentions it.
    let snapshot = snapshot_of(&row(false));
    let object = snapshot.as_object().expect("a snapshot is an object");
    for column in [
        "id",
        "case_slug",
        "event_date",
        "date_precision",
        "approximate",
        "phase",
        "title",
        "fact",
        "attributes",
        "created_by",
        "created_at",
        "updated_by",
        "updated_at",
        "deleted_at",
    ] {
        assert!(object.contains_key(column), "the snapshot drops {column}");
    }
    assert_eq!(
        object.len(),
        14,
        "the snapshot carries a field the column list does not: {snapshot}"
    );
}

#[test]
fn the_snapshot_keeps_the_whole_attribute_bag_verbatim() {
    // The change rule (design R4) again: a key this build has never heard of
    // must survive into the history, or the history is only as complete as the
    // build that wrote it.
    let snapshot = snapshot_of(&row(false));
    assert_eq!(
        snapshot["attributes"]["source"],
        serde_json::json!("legacy_json")
    );
    assert_eq!(
        snapshot["attributes"]["tags"],
        serde_json::json!(["court_action"])
    );
}

#[test]
fn a_deleted_snapshot_and_a_live_one_differ_in_their_content() {
    // The action word says what happened; the snapshot must SHOW it. If
    // `deleted_at` were absent from the snapshot, a delete and an edit would be
    // byte-identical records distinguished only by a label.
    let live = snapshot_of(&row(false));
    let gone = snapshot_of(&row(true));
    assert_ne!(live, gone);
    assert_eq!(live["deleted_at"], serde_json::Value::Null);
    assert!(!gone["deleted_at"].is_null());
}

// ── 3 · the one write path, proved by scanning the crate ────────────────────

#[test]
fn the_seal_is_the_only_place_a_chronology_transaction_commits() {
    // ⚑ THE STRUCTURAL HALF OF "one history row per write". `seal_and_commit`
    // consumes the transaction, so a handler cannot commit without it — unless
    // somebody adds a second commit inside the chronology's own modules, which
    // is what this finds.
    let mut offenders = Vec::new();
    for path in production_sources() {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        // The chronology's own write surface. Every other module in the crate
        // commits its own transactions and always has.
        let is_chronology_write =
            name.starts_with("timeline_write") || name.starts_with("chronology_");
        if !is_chronology_write {
            continue;
        }
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        if without_comments(&raw).contains(".commit()") && name != "chronology_guard.rs" {
            offenders.push(name.to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "a chronology write module commits its own transaction, so it can write \
         without a history row — the seal must be the only committer. Found in: {offenders:?}"
    );
}

#[test]
fn nothing_outside_the_guard_writes_a_history_row() {
    // The other half: `insert_history` exists once and is called from one place.
    // A second caller would be a second definition of "what gets recorded", and
    // the two would drift the first time an action word was added.
    let mut callers = Vec::new();
    for path in production_sources() {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string();
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        let source = without_comments(&raw);
        if source.contains("insert_history(") {
            callers.push(name);
        }
    }
    callers.sort();
    callers.dedup();
    assert_eq!(
        callers,
        vec![
            "chronology_guard.rs".to_string(),
            "chronology_write.rs".to_string()
        ],
        "history is written by exactly two files — the guard that calls it and \
         the repository that defines it. Found: {callers:?}"
    );
}

#[test]
fn the_only_insert_into_chronology_events_is_the_repositorys() {
    // ⚑ THE PROOF PHASE C ASKS FOR BY NAME: "the write path is the ONLY path —
    // a grep-with-comments-stripped that no second INSERT into
    // chronology_events exists outside it".
    let mut writers = Vec::new();
    for path in production_sources() {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string();
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        let source = without_comments(&raw);
        // The table name is a prefix of `chronology_event_links`,
        // `chronology_event_notes` and `chronology_event_history`, so the space
        // and the open paren after it are load-bearing: without them this test
        // would report three files that write entirely different tables.
        if source.contains("INSERT INTO chronology_events \\")
            || source.contains("INSERT INTO chronology_events (")
            || source.contains("UPDATE chronology_events ")
        {
            writers.push(name);
        }
    }
    writers.sort();
    writers.dedup();
    assert_eq!(
        writers,
        vec!["chronology_write.rs".to_string()],
        "every statement that writes an event must live in the repository module \
         that is the one write path's floor. Found: {writers:?}"
    );
}

#[test]
fn the_scan_can_actually_see_this_crates_source() {
    // ⚑ THE VACUITY GUARD. Every assertion above is satisfied by finding
    // nothing, so a broken walker would leave this file green while reading no
    // source at all.
    let files = crate_sources();
    assert!(
        files.len() > 100,
        "the crate walker found only {} files — it has stopped working",
        files.len()
    );
    let names: Vec<&str> = files
        .iter()
        .filter_map(|p| p.file_name().and_then(|n| n.to_str()))
        .collect();
    for sentinel in ["chronology_write.rs", "chronology_guard.rs", "timeline.rs"] {
        assert!(
            names.contains(&sentinel),
            "the walker did not find {sentinel}"
        );
    }

    // And the production filter removed the test files WITHOUT removing the
    // production ones — a filter that dropped everything would leave all three
    // scans above green while reading nothing at all.
    let production: Vec<&str> = production_sources()
        .iter()
        .filter_map(|p| p.file_name().and_then(|n| n.to_str()))
        .map(|n| Box::leak(n.to_string().into_boxed_str()) as &str)
        .collect();
    assert!(
        production.contains(&"chronology_write.rs"),
        "the production filter dropped the module the scans exist to protect"
    );
    assert!(
        !production.contains(&"chronology_guard_tests.rs"),
        "the production filter is not excluding test modules"
    );
    assert!(
        production.len() > files.len() / 2,
        "the production filter kept only {} of {} files; it is dropping real source",
        production.len(),
        files.len()
    );
}

#[test]
fn stripping_comments_hides_a_documented_statement_from_the_scan() {
    // The stripper itself, proved: this crate documents its SQL next to its SQL,
    // and a scanner that read comments would report every module header that
    // MENTIONS an INSERT as if it performed one.
    let documented = "// INSERT INTO chronology_events (x) VALUES (1)\nlet x = 1;";
    assert!(!without_comments(documented).contains("INSERT INTO chronology_events"));
    // And it does not eat real code on the same line as a trailing comment.
    assert!(without_comments("let x = 1; // note").contains("let x = 1;"));
}
