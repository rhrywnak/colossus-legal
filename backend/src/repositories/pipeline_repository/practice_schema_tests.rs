// Disk/code consistency for the practice repository's SQL (Rule 21).
//
// ## The defect this file exists to catch
//
// On 2026-08-18 every "Answer" press on DEV came back "Your answer was not
// recorded", because `insert_answer` names `read_raw_reply` and the v0 migration
// never created it. Nothing caught it before Marie's sitting:
//
// - `sqlx::query` / `query_as` take a `&str`. There is no compile-time schema
//   check — the string is opaque to the compiler, so a wrong column name is not
//   a build error. (This repo has no `.sqlx` offline data and uses no `query!`
//   macros, so that half of sqlx's safety is simply not in play here.)
// - The 2238 library tests never open a connection, so no test executed the
//   statement either.
// - The migration was proved in a `BEGIN … ROLLBACK` against DEV — which proves
//   the MIGRATION applies, and says nothing about whether the CODE agrees with
//   it. The column was added to the INSERT after that proof was taken.
//
// The gap is exactly the one `scan_runs_tests.rs` closed for `scan_runs` after
// the 2026-07-19 defect of the same family. This file is that guard, widened
// from one INSERT to every statement the practice repository ships, so the next
// practice query does not have to remember to ask for cover.
//
// ## Why parse the shipped artifacts rather than run the migrations
//
// A test that spins Postgres up, applies 81 migrations and executes the
// statements would be stronger and would also mean `cargo test --lib` needs a
// database. The cheapest honest check is to read the two artifacts that must
// agree — the repository source and the migration files — and assert they do.
// It cannot catch a type mismatch or a missing table in a database that drifted
// from its migrations by hand; it does catch every name the code invents, which
// is the whole of the defect family seen twice now.
//
// ## What is checked, and what is not
//
// Checked: every column an INSERT names, every column an UPDATE assigns, every
// `alias.column` whose alias resolves to a `practice_*` table, and the select
// list of a single-table `practice_*` SELECT. Not checked: expressions, column
// types, nullability, and tables outside the practice family (`list_points`
// reads `scenario_responses` and friends, which other guards own).
//
// ## Where the parsing lives
//
// In the sibling `practice_sql_shape`, so that this file is only the RULES. The
// two were one file until it passed the 300-line limit (Rule 17), and the seam
// they were cut on is the honest one: what the artifacts say, and what must be
// true of them.

use super::sql_shape::{
    declared, inserts, is_ident_char, is_practice_table, migration_columns, qualified_refs,
    sql_statements, tables_of, updates,
};

/// The column whose absence took the practice tool down on 2026-08-18.
///
/// Pinned by name as well as by the general rules below, so that deleting the
/// 20260818 migration fails with the sentence that explains it rather than with
/// a generic column-list diff.
#[test]
fn the_migrations_declare_read_raw_reply() {
    let columns = migration_columns("practice_answers");
    assert!(
        columns.contains(&"read_raw_reply".to_string()),
        "practice_answers has no `read_raw_reply` in the shipped migrations — this is the \
         2026-08-18 outage (every Answer press: rows_affected=0, \"column read_raw_reply … \
         does not exist\"). Declared: {columns:?}"
    );
}

/// The parse actually saw the repository's SQL.
///
/// Without this, every assertion below passes vacuously the day someone renames
/// the file, switches to raw strings, or breaks the literal scanner — a guard
/// that silently checks nothing is worse than no guard, because it is believed.
///
/// ## What this one covers
///
/// That the statements were FOUND, and that the migration side produced columns.
/// Finding them and reading nothing out of them is a second way to go blind, and
/// it has a test of its own below.
#[test]
fn the_parse_sees_the_repositorys_statements() {
    let statements = sql_statements();
    assert!(
        statements.len() >= 12,
        "expected at least the 12 SQL statements practice.rs shipped with, parsed {}: \
         the literal scanner has stopped seeing them",
        statements.len()
    );
    assert_eq!(
        inserts().len(),
        7,
        "two INSERTs in practice.rs, two in practice_editor.rs, one in \
         practice_notes.rs, the hidden-mark write's, and the seed's — the widest \
         column list in the codebase, and the one whose absence from this cover \
         let a `draft_by` \
         no migration created ship in Part A"
    );
    assert!(
        updates().len() >= 12,
        "the covered files ship at least twelve UPDATEs across practice.rs, \
         practice_flow.rs, practice_editor.rs, practice_notes.rs and the seed; \
         parsed {}",
        updates().len()
    );

    for (table, columns) in declared() {
        assert!(
            !columns.is_empty(),
            "parsed no columns at all for `{table}` out of the migrations"
        );
    }
}

/// The parse reads real columns and aliases OUT of the statements it found.
///
/// Finding two INSERTs and reading NO columns out of either is a shape the other
/// tests cannot tell from success: `for column in &[]` runs zero assertions and
/// `0 == 0` compares equal. Three tests would go green having checked nothing —
/// `read_raw_reply` included. So each level the parse can empty out at is pinned
/// here: the column lists, the SET targets, and the alias resolution that the
/// qualified-reference check rides on.
#[test]
fn the_parse_reads_columns_and_aliases_out_of_them() {
    for (table, columns, values) in inserts() {
        assert!(
            !columns.is_empty(),
            "parsed no columns out of INSERT INTO {table} — \
             the column-list parse has gone blind"
        );
        // A SELECT-form insert has no VALUES clause and legitimately parses to
        // an empty value list. Everything else must have parsed values, or the
        // arity test below is comparing 0 to 0 and calling it a pass.
        assert!(
            !values.is_empty() || select_form_inserts() > 0,
            "parsed no values out of INSERT INTO {table}, and no SELECT-form \
             insert exists to explain it — the VALUES parse has gone blind"
        );
    }

    let answers = inserts()
        .into_iter()
        .find(|(t, _, _)| t == "practice_answers")
        .expect("practice.rs ships an INSERT INTO practice_answers");
    assert_eq!(
        answers.1.len(),
        16,
        "the practice_answers INSERT names 16 columns; parsed {:?}",
        answers.1
    );

    for (table, columns) in updates() {
        assert!(
            !columns.is_empty(),
            "parsed no assigned column out of UPDATE {table} — the SET parse has gone blind"
        );
    }

    // `sheet_rows` reads `a.answer_text` through `FROM practice_answers a`. If
    // either half regresses, `check_qualified_refs` returns 0 for every
    // statement and says nothing about it.
    let resolves_alias = sql_statements().iter().any(|s| {
        let (aliases, _) = tables_of(s);
        aliases.get("a").map(String::as_str) == Some("practice_answers")
            && qualified_refs(s).contains(&("a".to_string(), "answer_text".to_string()))
    });
    assert!(
        resolves_alias,
        "no statement resolved `a` to practice_answers with an `a.answer_text` \
         reference — the alias parse has gone blind"
    );
}

/// Every column an INSERT names, and every column an UPDATE assigns, exists.
///
/// This is the assertion that would have failed on 2026-08-17, in the same
/// `cargo test --lib` run that reported 2238 passing tests.
#[test]
fn writes_name_only_columns_the_migrations_create() {
    let declared = declared();

    for (table, columns, _) in inserts() {
        if !is_practice_table(&table) {
            continue;
        }
        let have = declared.get(&table).unwrap_or_else(|| {
            panic!("INSERT writes `{table}`, which no shipped migration creates")
        });
        for column in &columns {
            assert!(
                have.contains(column),
                "INSERT INTO {table} names `{column}`, which the migrations do not create. \
                 Declared: {have:?}"
            );
        }
    }

    for (table, columns) in updates() {
        if !is_practice_table(&table) {
            continue;
        }
        let have = declared
            .get(&table)
            .unwrap_or_else(|| panic!("UPDATE writes `{table}`, which no migration creates"));
        for column in &columns {
            assert!(
                have.contains(column),
                "UPDATE {table} SET names `{column}`, which the migrations do not create. \
                 Declared: {have:?}"
            );
        }
    }
}

/// How many parsed INSERTs are SELECT-form — i.e. carry no VALUES clause.
///
/// A count and not a boolean: it is asserted exactly below, so adding a second
/// SELECT-form insert is a decision somebody makes on purpose rather than a way
/// to opt a statement out of the arity check quietly.
fn select_form_inserts() -> usize {
    inserts().iter().filter(|(_, _, v)| v.is_empty()).count()
}

/// Exactly one INSERT in the covered files is SELECT-form.
///
/// `practice_hidden_queue::record_hidden_marks`, which writes one `hidden` row
/// per queued-but-hidden question from a join rather than from bind parameters.
/// It is the only statement the arity test skips, and this is what stops that
/// skip from becoming a habit.
#[test]
fn exactly_one_insert_is_select_form() {
    assert_eq!(
        select_form_inserts(),
        1,
        "expected only record_hidden_marks to be SELECT-form; a new one must be \
         added here deliberately, because each is a statement the arity check \
         cannot see"
    );
}

/// Each INSERT supplies exactly one value per column it names.
///
/// Postgres would reject a mismatch — but only at runtime, on a live
/// connection, which is precisely the blind spot that let this ship.
#[test]
fn inserts_supply_one_value_per_column() {
    for (table, columns, values) in inserts() {
        // SELECT-form: no VALUES clause to count against. Skipped by SHAPE and
        // not by table name, so a VALUES-form insert can never slip through by
        // being written in a file this test was told to ignore. The count of
        // them is pinned below, so a skip cannot appear unnoticed.
        if values.is_empty() {
            continue;
        }
        assert_eq!(
            columns.len(),
            values.len(),
            "INSERT INTO {table} declares {} columns and supplies {} values\n\
             columns: {columns:?}\nvalues: {values:?}",
            columns.len(),
            values.len()
        );
    }
}

/// Every column a SELECT reads exists too.
///
/// A wrong name in a SELECT fails exactly the way the INSERT did — at runtime,
/// on the first real request — so the reads are held to the same rule as the
/// writes. Qualified `a.mark` references resolve through the statement's own
/// alias list; an unaliased single-table SELECT has its select list checked
/// directly. Expressions and aliased computations are skipped, not guessed at.
/// ## Why the two counters are separate
///
/// One total would let either half go to zero unnoticed: the single-table
/// SELECTs alone contribute 31 checks, so a `checked >= 20` floor is already
/// satisfied before a single qualified reference is looked at. `sheet_rows` and
/// `last_ended_session` are read ENTIRELY through aliases, so that is exactly
/// the half a combined floor would stop covering.
#[test]
fn reads_name_only_columns_the_migrations_create() {
    let statements = sql_statements();
    let qualified: usize = statements.iter().map(|s| check_qualified_refs(s)).sum();
    let listed: usize = statements
        .iter()
        .map(|s| check_single_table_select(s))
        .sum();

    assert!(
        qualified >= 20,
        "only {qualified} qualified `alias.column` references were checked — \
         the alias parse has gone blind"
    );
    assert!(
        listed >= 25,
        "only {listed} select-list columns were checked — the select-list parse has gone blind"
    );
}

/// Assert every `alias.column` in one statement, and return how many were
/// checked. References through an alias this file does not resolve to a
/// practice table are skipped — the count is what proves the skipping did not
/// swallow everything.
fn check_qualified_refs(statement: &str) -> usize {
    let declared = declared();
    let (aliases, _) = tables_of(statement);
    let mut checked = 0;

    for (alias, column) in qualified_refs(statement) {
        let Some(have) = aliases.get(&alias).and_then(|t| declared.get(t)) else {
            continue;
        };
        let table = &aliases[&alias];
        assert!(
            have.contains(&column),
            "`{alias}.{column}` reads {table}, which has no such column. \
             Declared: {have:?}\nstatement: {statement}"
        );
        checked += 1;
    }
    checked
}

/// Assert the select list of an unaliased single-table practice SELECT —
/// `SELECT a, b FROM practice_x WHERE …` — and return how many were checked.
/// Anything that is not a bare identifier is an expression or an `AS` rename,
/// and is skipped rather than guessed at.
fn check_single_table_select(statement: &str) -> usize {
    let declared = declared();
    let (_, tables) = tables_of(statement);
    let (Some(select_at), Some(from_at)) = (statement.find("SELECT "), statement.find(" FROM "))
    else {
        return 0;
    };
    if tables.len() != 1 || !is_practice_table(&tables[0]) {
        return 0;
    }
    let Some(have) = declared.get(&tables[0]) else {
        return 0;
    };

    let mut checked = 0;
    for item in statement[select_at + "SELECT ".len()..from_at].split(',') {
        let item = item.trim();
        if item.is_empty() || !item.chars().all(is_ident_char) {
            continue;
        }
        assert!(
            have.contains(&item.to_string()),
            "SELECT reads `{item}` from {}, which has no such column. Declared: {have:?}",
            tables[0]
        );
        checked += 1;
    }
    checked
}
