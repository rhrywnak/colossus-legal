//! Unit tests for `scan_runs.rs` — kept in a sibling file
//! (`#[cfg(test)] #[path = "..."] mod tests;`) so the parent module stays under
//! the 300-line limit (house pattern, see registry_tests.rs).

use super::*;

#[test]
fn bucket_column_maps_each_variant_to_its_count_column() {
    // These column names are interpolated into the progress UPDATE, so pin them:
    // a wrong mapping would advance the wrong live count. Each is a fixed literal
    // (no user input), which is why the format! is injection-safe.
    assert_eq!(bucket_column(ProgressBucket::Relevant), "relevant_count");
    assert_eq!(
        bucket_column(ProgressBucket::Irrelevant),
        "irrelevant_count"
    );
    assert_eq!(bucket_column(ProgressBucket::Failed), "failed_count");
}

/// SQL-shape guard for the history list. There is no `#[sqlx::test]`/live-DB
/// harness in this repo, so the behavioural ordering/scoping is asserted by the
/// live-DB integration test in `backend/tests/scan_run_history_integration.rs`
/// (not run in CI). This unit test pins the two properties the panel depends on
/// so a future edit that drops the scenario scope or reverses the order fails
/// here and names the regression.
#[test]
fn list_scan_runs_sql_scopes_by_scenario_and_orders_newest_first() {
    let sql = LIST_SCAN_RUNS_SQL;
    assert!(
        sql.contains("WHERE scenario_id = $1"),
        "history must be scoped to the scenario, got: {sql}"
    );
    assert!(
        sql.contains("ORDER BY started_at DESC"),
        "history must be newest-first, got: {sql}"
    );
    // The cost cast is load-bearing: a bare NUMERIC would fail to decode to f64.
    assert!(
        sql.contains("computed_cost::float8"),
        "computed_cost must be cast to float8 to decode, got: {sql}"
    );
    // Headers only — the heavy summary/verdict payload must NOT ride in the list.
    assert!(
        !sql.contains("summary_json"),
        "the history list must stay light (no summary_json), got: {sql}"
    );
    // The history must NOT fold in per-run merge counts. This assertion is
    // deliberately inverted from the one it replaces: under the unified merge
    // model, merge is pick-keyed, so "this run was merged N×" is an artifact of
    // the retired run-level model and no UI consumes it. Re-adding the subqueries
    // would quietly resurrect that model in the header, so it fails here.
    // (`scan_run_merges` itself is untouched — it is still written as the audit
    // trail; it simply must not feed the history list.)
    assert!(
        !sql.contains("scan_run_merges"),
        "the history list must not read run-level merge provenance, got: {sql}"
    );
    assert!(
        !sql.contains("merge_count") && !sql.contains("last_merged_at"),
        "no per-run merge counter may ride the header, got: {sql}"
    );
    // Likewise `dry_run`: nothing may branch on it after the benchmark model was
    // retired, and the surest way to keep that true is to stop reading it.
    assert!(
        !sql.contains("dry_run"),
        "the history list must not read the deprecated dry_run column, got: {sql}"
    );
}

/// SQL-shape guard for the delete. The `scenario_id` predicate is the case-fence:
/// dropping it would let any valid `run_id` delete a run in another scenario. Pin
/// both predicates so a future edit that widens the scope fails here and names it.
#[test]
fn delete_scan_run_sql_is_fenced_by_run_and_scenario() {
    let sql = DELETE_SCAN_RUN_SQL;
    assert!(
        sql.contains("DELETE FROM scan_runs"),
        "must delete from scan_runs, got: {sql}"
    );
    assert!(
        sql.contains("run_id = $1"),
        "must target the run by id ($1), got: {sql}"
    );
    assert!(
        sql.contains("scenario_id = $2"),
        "must fence the delete by scenario ($2), got: {sql}"
    );
}

// ─── The start INSERT, checked against the real schema on disk ───────────────
//
// These read the shipped source and the shipped migration rather than a fixture,
// so they measure what will actually run. See the note in
// `pipeline_repository::sql_invariants` for the 2026-07-19 defect that motivated
// them: a dropped `.bind()` shifted every later value one column left and put a
// boolean into the `resolved_params` jsonb column, which only Postgres could
// report and only at runtime.
//
// The start INSERT is now the STUB insert — the row is born `failed` and is
// promoted to `running` by a separate UPDATE (see the module doc). It is still
// the only `INSERT INTO scan_runs` in the file, so every shape test below reads
// it unchanged; the promote UPDATE has its own tests further down.

/// The shipped source of `scan_runs.rs`.
///
/// `pub(super)` so the sibling `scan_run_start_tests` module can read the same
/// artifact without a second copy of this helper. Both test modules are children
/// of `scan_runs`, so `super` visibility is exactly the reach required — no wider.
pub(super) fn scan_runs_source() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/repositories/pipeline_repository/scan_runs.rs");
    std::fs::read_to_string(path).expect("scan_runs.rs is readable")
}

/// The body of one `pub async fn`, from its signature to the closing brace at
/// column 0. Good enough to answer "which constant does THIS function bind",
/// which is the only question asked of it. `pub(super)` for the same reason as
/// [`scan_runs_source`].
pub(super) fn fn_body(name: &str) -> String {
    let text = scan_runs_source();
    let needle = format!("pub async fn {name}");
    let start = text
        .find(&needle)
        .unwrap_or_else(|| panic!("`{name}` is present in scan_runs.rs"));
    let rest = &text[start + needle.len()..];
    let end = rest.find("\n}").map(|i| i + 2).unwrap_or(rest.len());
    rest[..end].to_string()
}

/// The `INSERT INTO scan_runs (...) VALUES (...)` block, read from the shipped
/// source file.
fn start_insert_sql() -> String {
    let text = scan_runs_source();
    let start = text
        .find("INSERT INTO scan_runs")
        .expect("the start INSERT is present");
    // ASSUMES: the SQL is a one-hash raw string, `r#"…"#`. With `r##"…"##` this
    // terminator matches the first `"#` INSIDE the closing `"##`, returning a
    // truncated fragment — the parse would then measure the wrong text and the
    // assertions would silently check nothing. Loud here, silent there, so the
    // assumption is written down rather than left to be rediscovered.
    let end = text[start..]
        .find("\"#")
        .expect("the raw string terminates")
        + start;
    text[start..end].to_string()
}

/// `(columns, values)` from the INSERT, in declaration order. `pub(super)` so the
/// sibling `scan_run_start_tests` can assert the stub row's birth state against
/// the same parse.
pub(super) fn columns_and_values() -> (Vec<String>, Vec<String>) {
    let sql = start_insert_sql();
    let open = sql.find('(').expect("column list opens");
    let close = sql.find(')').expect("column list closes");
    let columns: Vec<String> = sql[open + 1..close]
        .split(',')
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
        .collect();

    let values_at = sql.find("VALUES").expect("VALUES clause present");
    let vopen = sql[values_at..].find('(').expect("value list opens") + values_at;
    let vclose = sql[vopen..].find(')').expect("value list closes") + vopen;
    let values: Vec<String> = sql[vopen + 1..vclose]
        .split(',')
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .collect();

    (columns, values)
}

/// Every column the live `scan_runs` table has, read from the shipped migrations.
///
/// ## Why this reads EVERY migration, not just the CREATE
///
/// The table's real shape is the union of its migrations: the 2026-07-15 CREATE
/// plus the `ALTER TABLE … ADD COLUMN` that the background-progress migration
/// applied hours later (`status`, `candidates_total`, `candidates_judged`,
/// `error`, `summary_json`, `last_progress_at`). Reading only the CREATE would
/// declare half the columns the INSERT legitimately writes to be typos.
///
/// ## Why the CREATE block is terminated on a line-anchored `);`
///
/// A first version of this helper searched for the first `");"` anywhere and
/// stopped inside a COMMENT — "…(decodes cleanly to i64); a scan never…" — which
/// truncated the column list before `duration_ms` and failed the test with a
/// false accusation. Same defect family as the bug this file exists to catch: a
/// naive textual match against a real artifact. Anchoring to a line that is
/// exactly `);` is what makes it right.
fn migration_columns() -> Vec<String> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("pipeline_migrations");
    let mut columns = Vec::new();

    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("pipeline_migrations is readable")
        .map(|e| e.expect("dir entry readable").path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("sql"))
        .collect();
    files.sort();

    for path in files {
        let sql = std::fs::read_to_string(&path).expect("migration is UTF-8");

        // CREATE TABLE scan_runs ( … );  — body ends at a line that is just `);`
        if let Some(start) = sql.find("CREATE TABLE scan_runs") {
            for line in sql[start..].lines().skip(1) {
                let line = line.trim();
                if line == ");" {
                    break;
                }
                if line.is_empty() || line.starts_with("--") {
                    continue;
                }
                if let Some(name) = line.split_whitespace().next() {
                    if name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
                        columns.push(name.to_string());
                    }
                }
            }
        }

        // ALTER TABLE scan_runs ADD COLUMN [IF NOT EXISTS] <name> …
        for line in sql.lines() {
            let line = line.trim();
            let Some(rest) = line.strip_prefix("ALTER TABLE scan_runs ADD COLUMN ") else {
                continue;
            };
            let rest = rest.strip_prefix("IF NOT EXISTS ").unwrap_or(rest);
            if let Some(name) = rest.split_whitespace().next() {
                columns.push(name.to_string());
            }
        }
    }

    columns.sort();
    columns.dedup();
    columns
}

/// Every column the INSERT names exists in the migration that creates the table.
/// A typo here is not a compile error and not a test failure anywhere else — it
/// is a runtime `column "..." does not exist` on the first real scan.
#[test]
fn start_insert_only_names_columns_the_migration_creates() {
    let (columns, _) = columns_and_values();
    let declared = migration_columns();

    assert!(
        !declared.is_empty(),
        "failed to parse any column out of the scan_runs migration"
    );
    for column in &columns {
        assert!(
            declared.contains(column),
            "INSERT names `{column}`, which the scan_runs migration does not create. \
             Declared columns: {declared:?}"
        );
    }
}

/// The column list and the VALUES list are the same length.
///
/// Postgres would reject a mismatch, but only at runtime on a live connection —
/// the same blind spot that let the 2026-07-19 defect ship.
#[test]
fn start_insert_column_and_value_lists_are_the_same_length() {
    let (columns, values) = columns_and_values();
    assert_eq!(
        columns.len(),
        values.len(),
        "INSERT declares {} columns but supplies {} values\ncolumns: {columns:?}\nvalues: {values:?}",
        columns.len(),
        values.len()
    );
}

/// **The regression test.** `resolved_params` is `jsonb` and must receive a bound
/// placeholder, never an inline literal.
///
/// This is the assertion that would have caught the 2026-07-19 defect at the
/// point it was introduced: the dropped bind left `$4` — the slot this column
/// sits in — receiving the literal `false` intended for `dry_run`, and Postgres
/// answered *"column resolved_params is of type jsonb but expression is of type
/// boolean"*. Checking the pairing positionally means a future bind shuffle
/// cannot quietly reintroduce it.
#[test]
fn jsonb_columns_receive_a_bound_placeholder_not_a_literal() {
    let (columns, values) = columns_and_values();
    // The jsonb columns THIS statement writes. `summary_json` is deliberately
    // absent: it belongs to the finalize UPDATE, and listing it here would imply
    // a check that the zip below can never reach — a test reading stronger than
    // it is. It has its own test, `finalize_update_assigns_summary_json_a_placeholder`.
    const JSONB_COLUMNS: &[&str] = &["resolved_params"];

    for (column, value) in columns.iter().zip(values.iter()) {
        if JSONB_COLUMNS.contains(&column.as_str()) {
            assert!(
                value.starts_with('$'),
                "`{column}` is jsonb but the INSERT supplies the literal `{value}` \
                 — a bind was dropped or the binding order shifted"
            );
        }
    }
}

/// `dry_run` keeps receiving a value, AND the parser above actually parsed
/// something.
///
/// Two jobs, deliberately in one test. The first is the domain assertion: the
/// column is NOT NULL and the literal `false` is bound for it on purpose (see the
/// comment at the bind site), so this pins that it did not simply fall out of the
/// INSERT when the flag was retired — the very edit that broke the neighbouring
/// bind.
///
/// The second is the ANTI-VACANCY GUARD for every other test in this section. If
/// `columns_and_values()` ever returned empty vectors — a parser regression, a
/// renamed statement, a moved file — then
/// `start_insert_column_and_value_lists_are_the_same_length` would pass on
/// `0 == 0` and both loop-based tests would pass without executing a single
/// iteration. Three green tests guarding nothing. This assertion fails first in
/// that case, which is why it is not merely a nice-to-have.
#[test]
fn start_insert_supplies_dry_run_and_the_parser_found_real_columns() {
    let (columns, values) = columns_and_values();
    assert!(
        columns.len() >= 10 && values.len() >= 10,
        "parser returned {} columns / {} values — it is no longer reading the \
         INSERT, and the sibling tests in this section would pass vacuously",
        columns.len(),
        values.len()
    );
    assert!(
        columns.iter().any(|c| c == "dry_run"),
        "dry_run is NOT NULL in the migration and must still be written"
    );
}

/// The finalize UPDATE writes `summary_json` (jsonb) as its last bind — the
/// symmetric risk to the start INSERT's `resolved_params`.
///
/// The arity scan in `pipeline_repository::sql_invariants` would catch a dropped
/// bind here by count. This adds the type-shaped half: the jsonb column must
/// receive a placeholder, never an inline literal. Being the LAST bind makes it
/// lower-risk than `resolved_params` was (dropping it shifts nothing into it),
/// but "lower risk" is how the original defect was allowed to exist unexamined.
#[test]
fn finalize_update_assigns_summary_json_a_placeholder() {
    let text = scan_runs_source();
    // ANCHOR: `duration_ms =` appears in the finalize UPDATE and nowhere else in
    // this file. Anchoring on the shared "UPDATE scan_runs SET" prefix would be
    // an ordering dependency — `fail_scan_run` and `sweep_running_scan_runs`
    // share it, so reordering the functions would silently retarget this test.
    let start = text
        .find("duration_ms = $")
        .expect("the finalize UPDATE assigns duration_ms via a placeholder");
    let stmt_start = text[..start]
        .rfind("UPDATE scan_runs SET")
        .expect("the finalize UPDATE opens with the SET clause");
    // ASSUMES: the SQL is a one-hash raw string, `r#"…"#`. Promoting it to `r##`
    // would make this terminator match inside the delimiter and silently
    // truncate the statement — see the same note on `start_insert_sql`.
    let end = text[stmt_start..]
        .find("\"#")
        .expect("raw string terminates")
        + stmt_start;
    let sql = &text[stmt_start..end];

    let assignment = sql
        .lines()
        .flat_map(|l| l.split(','))
        .map(str::trim)
        .find(|a| a.starts_with("summary_json"))
        .expect("the finalize UPDATE assigns summary_json");

    let value = assignment
        .split_once('=')
        .map(|(_, v)| v.trim())
        .expect("assignment has a right-hand side");
    assert!(
        value.starts_with('$'),
        "summary_json is jsonb but the UPDATE assigns the literal `{value}`"
    );
}
