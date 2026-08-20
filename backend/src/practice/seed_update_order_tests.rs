//! The re-ordering defect, against a REAL Postgres and the REAL constraint.
//!
//! ## Why this test has to touch a database
//!
//! The .403 defect — `--update --apply` failing
//! `practice_questions_order_unique` mid-transaction on S-5 — was invisible to
//! every test this repo had, and it was invisible for a structural reason worth
//! stating: the dry run never writes, so it cannot meet a constraint, and the
//! unit tests cover the PLAN (which key maps to which row) rather than the
//! writes. The bug lived in the sequence of `UPDATE`s and only a database that
//! enforces the constraint can see it.
//!
//! So this one is gated, per the repo's live-test convention
//! (`rig_llm_bridge::test_rig_bridge_live`): `#[ignore]` plus a self-skip when
//! the environment does not name a database. It never runs in the normal gate.
//!
//! ```text
//! podman run -d --name pgtest-order -e POSTGRES_PASSWORD=test \
//!     -e POSTGRES_DB=ordertest -p 55432:5432 docker.io/library/postgres:16.4
//! ORDER_TEST_DATABASE_URL=postgres://postgres:test@127.0.0.1:55432/ordertest \
//!     cargo test --lib seed_update_order -- --ignored --nocapture
//! ```
//!
//! ## Point it at a THROWAWAY database, never at DEV
//!
//! It creates a scenario and a deck and then drops the whole schema. The env var
//! is deliberately NOT `PIPELINE_DATABASE_URL`: that variable is already set in
//! any shell an operator uses for the real one-shots, and a test that ran itself
//! against DEV because the wrong terminal was in focus would write to a witness's
//! practice record.
//!
//! ## The table is built from the SHIPPED migration
//!
//! `practice_questions` is created by extracting its `CREATE TABLE` from
//! `pipeline_migrations/` on disk, plus every later `ALTER TABLE … ADD COLUMN`.
//! A hand-copied definition would be a second source of truth that could drift
//! from the real one — and drifting in the constraint is the one drift that would
//! make this test pass while production still failed.

use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::deck_file::{DeckFile, DeckQuestion};
use super::seed_update::run_update;

/// The env var that names a throwaway database, or the test skips.
const URL_VAR: &str = "ORDER_TEST_DATABASE_URL";

/// Every pipeline migration, oldest first.
fn migrations() -> Vec<String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("pipeline_migrations");
    let mut files: Vec<_> = std::fs::read_dir(&root)
        .expect("pipeline_migrations is readable")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "sql"))
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|p| std::fs::read_to_string(&p).unwrap_or_default())
        .collect()
}

/// The shipped `CREATE TABLE practice_questions (…);`, exactly as it is on disk.
fn create_practice_questions() -> String {
    for sql in migrations() {
        let Some(at) = sql.find("CREATE TABLE practice_questions (") else {
            continue;
        };
        // The statement ends at the first `);` that closes it at column 0 — the
        // shape every table in these files is written in.
        let end = sql[at..]
            .find("\n);")
            .expect("the CREATE TABLE is terminated")
            + at
            + 3;
        return sql[at..end].to_string();
    }
    panic!("no migration creates practice_questions");
}

/// Every later `ALTER TABLE practice_questions ADD COLUMN …;` in order.
fn later_columns() -> Vec<String> {
    let mut out = Vec::new();
    for sql in migrations() {
        for line in sql.lines() {
            let line = line.trim();
            if line.starts_with("ALTER TABLE practice_questions ADD COLUMN") {
                out.push(line.to_string());
            }
        }
    }
    out
}

/// The four tables `read_sources` reads, with only the columns it selects.
///
/// Hand-written, unlike `practice_questions`, because only a few columns of each
/// are touched and the test needs them EMPTY — a stub that drifted would fail
/// this test loudly rather than let a wrong result through.
const STUBS: &str = "
CREATE TABLE scenarios (
    scenario_id   UUID PRIMARY KEY,
    code_ordinal  INTEGER NOT NULL
);
CREATE TABLE scenario_human_facts (
    id                    UUID PRIMARY KEY,
    scenario_id           UUID NOT NULL,
    kind                  TEXT NOT NULL,
    anchor_graph_node_id  TEXT,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE TABLE scenario_responses (
    id           UUID PRIMARY KEY,
    scenario_id  UUID NOT NULL
);
CREATE TABLE response_items (
    id           UUID PRIMARY KEY,
    response_id  UUID NOT NULL,
    item_index   INTEGER NOT NULL
);
";

/// One manual question. `manual` so `resolve_refs` needs no instances or points.
fn question(key: &str, text: &str) -> DeckQuestion {
    DeckQuestion {
        key: Some(key.to_string()),
        side: super::deck_file::DeckSide::George,
        kind: Some(super::deck_file::DeckKind::Cross),
        text: text.to_string(),
        tactic: None,
        braid_rows: None,
        source_kind: super::deck_file::DeckSourceKind::Manual,
        source_index: None,
        receipt: None,
        watch_for: None,
        stronger: None,
        stronger_lean: None,
        pair_said: None,
        pair_admitted: None,
        follows: None,
        draft_by: None,
        source_line: None,
    }
}

fn deck_of(keys: &[&str]) -> DeckFile {
    DeckFile {
        scenario_code: "S-5".to_string(),
        points: Vec::new(),
        questions: keys
            .iter()
            .map(|k| question(k, &format!("question {k}")))
            .collect(),
    }
}

/// Build the schema, then a five-question deck in key order g1..g5.
async fn seed(pool: &PgPool, scenario: Uuid) {
    // `raw_sql`, not `query`: `STUBS` is four statements, and `query` prepares —
    // Postgres refuses multiple commands in one prepared statement.
    sqlx::raw_sql(STUBS).execute(pool).await.expect("stubs");
    sqlx::raw_sql(&create_practice_questions())
        .execute(pool)
        .await
        .expect("the shipped practice_questions DDL applies");
    for alter in later_columns() {
        sqlx::query(&alter)
            .execute(pool)
            .await
            .expect("a later column applies");
    }
    sqlx::query("INSERT INTO scenarios (scenario_id, code_ordinal) VALUES ($1, 5)")
        .bind(scenario)
        .execute(pool)
        .await
        .expect("the scenario row");

    for (i, key) in ["g1", "g2", "g3", "g4", "g5"].iter().enumerate() {
        sqlx::query(
            "INSERT INTO practice_questions \
                 (id, scenario_id, side, text, source_kind, sort_order, created_by, deck_key) \
             VALUES ($1, $2, 'george', $3, 'manual', $4, 'test', $5)",
        )
        .bind(Uuid::new_v4())
        .bind(scenario)
        .bind(format!("question {key}"))
        .bind(i as i32 + 1)
        .bind(key)
        .execute(pool)
        .await
        .expect("a seeded question");
    }
}

/// The stored order, by deck key.
async fn stored_order(pool: &PgPool, scenario: Uuid) -> Vec<String> {
    sqlx::query(
        "SELECT deck_key FROM practice_questions WHERE scenario_id = $1 ORDER BY sort_order",
    )
    .bind(scenario)
    .fetch_all(pool)
    .await
    .expect("the stored order")
    .into_iter()
    .map(|r| r.get::<String, _>("deck_key"))
    .collect()
}

/// A GENUINE permutation is applied — the .403 defect, reproduced and fixed.
///
/// The order asserted is the one Roman's run failed on: S-5's ruled sequence
/// g3 · g4 · g2 · g1 · g5, which is a permutation with a cycle. Before the
/// parking phase this call failed with
/// `duplicate key value violates unique constraint "practice_questions_order_unique"`
/// on the first write, because g3 was assigned `sort_order = 1` while g1 still
/// held it.
///
/// A no-op re-run is asserted afterwards for the reason the fix could otherwise
/// hide: parking every row and writing the same numbers back must still leave
/// the deck exactly as it was.
#[tokio::test]
#[ignore]
async fn a_reordered_deck_is_written_without_violating_the_order_constraint() {
    let Ok(url) = std::env::var(URL_VAR) else {
        eprintln!("{URL_VAR} is not set — skipping the live re-order test");
        return;
    };
    let pool = PgPool::connect(&url).await.expect("the throwaway database");
    sqlx::raw_sql("DROP SCHEMA public CASCADE; CREATE SCHEMA public;")
        .execute(&pool)
        .await
        .expect("a clean schema");

    let scenario = Uuid::new_v4();
    seed(&pool, scenario).await;
    assert_eq!(
        stored_order(&pool, scenario).await,
        ["g1", "g2", "g3", "g4", "g5"],
        "the fixture starts in key order"
    );

    // THE REAL CODE PATH, applying for real.
    let report = run_update(&pool, &deck_of(&["g3", "g4", "g2", "g1", "g5"]), true)
        .await
        .expect("the re-order applies");
    assert!(report.written, "the run reports that it wrote");

    assert_eq!(
        stored_order(&pool, scenario).await,
        ["g3", "g4", "g2", "g1", "g5"],
        "the stored order is the file's order"
    );

    // Re-running the SAME order must be a no-op that still succeeds — the park
    // rewrites every row whether or not its number changed.
    run_update(&pool, &deck_of(&["g3", "g4", "g2", "g1", "g5"]), true)
        .await
        .expect("a no-op re-run applies");
    assert_eq!(
        stored_order(&pool, scenario).await,
        ["g3", "g4", "g2", "g1", "g5"],
        "a re-run changes nothing"
    );

    // And no parked value survived the transaction.
    let (negatives,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM practice_questions WHERE scenario_id = $1 AND sort_order < 0",
    )
    .bind(scenario)
    .fetch_one(&pool)
    .await
    .expect("the parked-row count");
    assert_eq!(negatives, 0, "a parked sort_order must never be visible");

    sqlx::raw_sql("DROP SCHEMA public CASCADE; CREATE SCHEMA public;")
        .execute(&pool)
        .await
        .expect("cleanup");
}
