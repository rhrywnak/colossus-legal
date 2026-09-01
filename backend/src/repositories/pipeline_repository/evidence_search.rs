//! `evidence_search` — the ONE write path into the lexical mirror.
//!
//! ⚑ **Everything that writes this table calls [`upsert_evidence_search_rows`],
//! and nothing else may write it.** Today the only caller is the L1b backfill
//! bin; tomorrow the pipeline's index step (L1c) writes the same rows beside its
//! Qdrant upsert. If those two ever wrote the table by different SQL, the mirror
//! would drift from the graph in exactly the places nobody looks — and the only
//! symptom would be a lexical search silently missing a quote that is right
//! there in Neo4j.
//!
//! That is why this function lives in `pipeline_repository` rather than inside
//! the backfill binary. A binary's private helper cannot be called from
//! `api::pipeline::index`; this can, by the same
//! `repositories::pipeline_repository::*` glob every other pipeline write
//! already uses. L1c adds a call site, not a second implementation.
//!
//! ## What this module deliberately does NOT do
//!
//! It does not read Neo4j (that is `repositories::evidence_search_repository`),
//! it does not decide what a row means, and it never touches `search_vector` —
//! that column is `GENERATED ALWAYS AS … STORED` and Postgres refuses a direct
//! write to it. Recomputation on UPDATE is the database's job, which is the
//! whole reason L1a made it generated rather than trigger-maintained.

use serde::Serialize;
use sqlx::PgPool;

use super::PipelineRepoError;

/// One Evidence node, in the shape the mirror stores it.
///
/// ## Rust Learning: why `page` is `Option<i64>` and not `Option<i32>`
///
/// The graph hands this value over as `Option<i64>` (`BiasInstance::page_number`,
/// and the raw Neo4j integer behind it), and L1a's column is `BIGINT` on
/// Roman's ruling R1 for precisely this reason. So the value travels graph → row
/// → column with **no conversion anywhere**. An `i32` here would reintroduce the
/// narrowing the ruling removed, and a narrowing is either fallible (an error
/// path for a page number, which is absurd) or silent (which is worse).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EvidenceSearchRow {
    pub evidence_id: String,
    pub document_id: String,
    /// `None` when the graph node carries no title — distinct from an empty
    /// string somebody typed.
    pub title: Option<String>,
    /// The verbatim quote. Not optional: the column is `NOT NULL`, because a row
    /// with no quote could not be scored and would quietly shrink the
    /// denominator of every recall number computed off this table.
    pub quote: String,
    pub significance: Option<String>,
    pub page: Option<i64>,
    /// Party ids the Evidence is ABOUT. An empty vector is a real state (a node
    /// with no ABOUT edges), and it is stored as an empty array, never NULL.
    pub about: Vec<String>,
}

// STRUCTURAL: SQL text is wire vocabulary for the Postgres protocol, not a
// deployment-variable setting — there is no environment in which this project
// wants to write the mirror differently, and moving it to config would turn a
// compile-time fact into a runtime surprise. Held as a `const` at module scope
// so the shape tests below assert against THE TEXT THAT RUNS, not a copy.
//
// ## Why the whole batch arrives as ONE jsonb parameter
//
// The obvious shapes both fail here. Building `($1,$2,…),($8,$9,…)` by string
// concatenation makes the statement text depend on the batch length — a planner
// cache miss per distinct size, and the shape of code that grows an injection
// hole. Parallel arrays via `UNNEST` fix that for six of the seven columns, but
// not for `about`: it is a `text[]` PER ROW, so the parameter would have to be
// `text[][]`, and a Postgres multidimensional array requires every sub-array to
// be the SAME LENGTH. One node about two parties and its neighbour about three
// cannot be expressed. (sqlx declines to bind `Vec<Vec<String>>` at all, which
// is how this was caught rather than shipped.)
//
// `jsonb_to_recordset` takes one parameter of any batch size, names each column
// and its type in the statement itself, and hands `about` through as jsonb which
// `jsonb_array_elements_text` turns back into a real `text[]` of the row's own
// length. One bound value, one statement text, correct arrays.
//
// `search_vector` appears nowhere: it is GENERATED and Postgres rejects a write.
const UPSERT_SQL: &str = "\
    INSERT INTO evidence_search \
        (evidence_id, document_id, title, quote, significance, page, about, synced_at) \
    SELECT r.evidence_id, r.document_id, r.title, r.quote, r.significance, r.page, \
           coalesce(ARRAY(SELECT jsonb_array_elements_text(r.about)), '{}'::text[]), \
           now() \
    FROM jsonb_to_recordset($1::jsonb) AS r( \
        evidence_id text, document_id text, title text, quote text, \
        significance text, page bigint, about jsonb \
    ) \
    ON CONFLICT (evidence_id) DO UPDATE SET \
        document_id  = EXCLUDED.document_id, \
        title        = EXCLUDED.title, \
        quote        = EXCLUDED.quote, \
        significance = EXCLUDED.significance, \
        page         = EXCLUDED.page, \
        about        = EXCLUDED.about, \
        synced_at    = now()";

/// Upsert a batch of mirror rows, keyed by `evidence_id`.
///
/// Returns the number of rows written (inserted + updated). An empty batch is a
/// legitimate no-op and returns `0` without a round trip.
///
/// ## Why upsert rather than delete-then-insert
///
/// A re-run must be safe at any moment, including halfway through. Truncating
/// first would leave the mirror empty for the length of the backfill, and a
/// crash in the middle would leave it *permanently* short with no error anywhere
/// — the table would simply have fewer rows than the graph and every later
/// search would quietly under-return. `ON CONFLICT DO UPDATE` means a partial
/// run leaves a partially-refreshed mirror that is still complete, and the next
/// run finishes the job.
///
/// `synced_at` is refreshed on both paths — it records when WE last wrote the
/// row, which is knowable with certainty. It is the table's only timestamp:
/// staleness is handled by whole-document re-sync in L1c (Roman, 2026-09-01),
/// not by comparing a per-row stamp against the graph.
///
/// ## Rust Learning: `impl PgExecutor<'_>` so a caller can enrol this in a transaction
///
/// The parameter widened from `&PgPool` to `impl PgExecutor<'_>` when L1c needed
/// the upsert and the ghost-row delete to be ONE transaction. `&PgPool` is
/// itself a `PgExecutor` (each call takes its own connection), so **every
/// existing call site compiles unchanged** — the backfill bin still passes
/// `&pool`. `&mut PgConnection`, which is what a live transaction hands out, is
/// also one. The body did not change; only who is allowed to run it.
///
/// This is the house pattern — `scenario_store::insert_scenario` and its
/// siblings take the same parameter for the same reason.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the statement fails.
pub async fn upsert_evidence_search_rows(
    executor: impl sqlx::PgExecutor<'_>,
    rows: &[EvidenceSearchRow],
) -> Result<u64, PipelineRepoError> {
    if rows.is_empty() {
        return Ok(0);
    }

    // ## Rust Learning: `serde_json::to_value` on a slice, not a hand-built string
    //
    // The batch is serialized by the same derive that already describes the
    // struct, so a field renamed on the Rust side changes the JSON key and the
    // `jsonb_to_recordset` column list stops matching — a loud failure on the
    // first row rather than a column silently arriving NULL. Building the JSON
    // by hand here would be a second description of the same shape, free to
    // drift from the first.
    let batch = serde_json::to_value(rows).map_err(|e| {
        PipelineRepoError::Database(format!("could not serialize an evidence_search batch: {e}"))
    })?;

    let result = sqlx::query(UPSERT_SQL)
        .bind(&batch)
        .execute(executor)
        .await?;

    Ok(result.rows_affected())
}

// STRUCTURAL: SQL text is wire vocabulary for the Postgres protocol, not a
// deployment-variable setting. Held at module scope so the shape test asserts
// against the text that runs.
//
// `<> ALL($2)` and not `NOT IN`: with an EMPTY array `evidence_id <> ALL('{}')`
// is TRUE for every row, so the empty-set case deletes the document's rows
// exactly as it must. `NOT IN` on an empty list is also true, but `NOT IN` turns
// the whole predicate NULL the moment one element is NULL, which would silently
// delete nothing. The ids come from the graph and are never null today; `ALL`
// means that stays true by construction rather than by luck.
//
// Scoped by `document_id` — this statement can never touch another document's
// rows, which is the property that makes it safe to run from two paths.
const DELETE_GHOSTS_SQL: &str = "\
    DELETE FROM evidence_search \
    WHERE document_id = $1 AND evidence_id <> ALL($2::text[])";

/// Make the mirror match the graph for ONE document, atomically.
///
/// Upserts every current row and deletes the rows this document used to have and
/// no longer does. Returns `(written, deleted)`.
///
/// ## Why this exists rather than two calls at the call site
///
/// A caller that did the upsert and forgot the delete would leave ghost rows —
/// evidence deleted from the graph still answering searches — and a caller that
/// did them in the wrong order, or in two transactions, would leave a window
/// where the mirror is missing rows it should have. Wrapping the pair means a
/// caller cannot do half of it. Both of L1c's two call sites use this and
/// neither carries any SQL of its own.
///
/// ## Whole-document re-sync IS the staleness strategy
///
/// Roman's ruling of 2026-09-01. There is no per-row staleness comparison and no
/// `source_updated_at` column to make one with: every index of a document
/// rewrites every row it has and removes every row it does not. A row therefore
/// cannot be stale with respect to its document — which is why the empty case
/// below is not an edge case but the mechanism.
///
/// ## The empty set is the point, not a special case
///
/// `rows` empty means the graph now holds no Evidence for this document. The
/// upsert does nothing (L1b short-circuits) and the delete removes everything
/// the mirror still had. Skipping the call on an empty list — the obvious
/// "optimisation" — is precisely how ghost rows would survive for ever.
///
/// ## Rust Learning: `&mut *tx` — reborrowing a transaction for two statements
///
/// `sqlx::Transaction` yields its connection through `&mut *tx`, and each
/// `execute` wants that mutable borrow. Passing `&mut *tx` **reborrows** rather
/// than moving, so the first statement gives the borrow back and the second can
/// take it — and `tx` is still owned afterwards to `commit()`. Passing `tx`
/// itself would move it into the first call and the second would not compile.
///
/// # Errors
/// Returns [`PipelineRepoError`] if either statement or the commit fails. On any
/// error the transaction is dropped, which rolls it back: the mirror is left
/// exactly as it was, never half-synced.
pub async fn sync_document_evidence_search(
    pool: &PgPool,
    document_id: &str,
    rows: &[EvidenceSearchRow],
) -> Result<(u64, u64), PipelineRepoError> {
    let ids: Vec<&str> = rows.iter().map(|r| r.evidence_id.as_str()).collect();

    let mut tx = pool.begin().await?;
    let written = upsert_evidence_search_rows(&mut *tx, rows).await?;
    let deleted = sqlx::query(DELETE_GHOSTS_SQL)
        .bind(document_id)
        .bind(&ids)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    tx.commit().await?;

    Ok((written, deleted))
}

/// How many rows the mirror holds. The second of the three counts L1b asserts.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the query fails.
pub async fn count_evidence_search_rows(pool: &PgPool) -> Result<i64, PipelineRepoError> {
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM evidence_search")
        .fetch_one(pool)
        .await?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The statement never writes the generated column.
    ///
    /// Postgres would reject it at runtime, but that failure would arrive on the
    /// first live backfill rather than on the first `cargo test`, and by then
    /// somebody has already waited for a graph read to finish.
    #[test]
    fn the_upsert_never_writes_the_generated_column() {
        assert!(
            !UPSERT_SQL.contains("search_vector"),
            "search_vector is GENERATED ALWAYS; Postgres refuses a direct write to it"
        );
    }

    /// The INSERT's column list is EXACTLY the eight writable columns.
    ///
    /// ## What this replaced, and why the replacement is stronger
    ///
    /// This was a pair of assertions: that each of the eight appears somewhere in
    /// the statement, and that `source_updated_at` does not. The second went
    /// vacuous when Roman's whole-document-re-sync ruling removed that column
    /// from the table (L1c.0) — a `!contains` for a column that no longer exists
    /// anywhere cannot fail. The first was weak in its own way: `contains` is
    /// satisfied by a column name appearing in the `ON CONFLICT` clause even if
    /// the INSERT list has lost it.
    ///
    /// So it now reads the INSERT's parenthesised list and compares it exactly.
    /// That catches both directions of drift, which is what the original pair was
    /// reaching for: a column DROPPED from the list would be NULL on every row
    /// forever with nothing to say so, and a column ADDED to the table but not to
    /// the list would be the same defect on the next migration. `search_vector`
    /// must never appear — it is GENERATED and Postgres rejects the write.
    #[test]
    fn the_insert_names_exactly_the_writable_columns() {
        let open = UPSERT_SQL
            .find("INSERT INTO evidence_search (")
            .expect("the statement inserts into the mirror")
            + "INSERT INTO evidence_search (".len();
        let close = UPSERT_SQL[open..]
            .find(')')
            .expect("the column list is parenthesised")
            + open;
        let written: Vec<&str> = UPSERT_SQL[open..close]
            .split(',')
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .collect();

        assert_eq!(
            written,
            vec![
                "evidence_id",
                "document_id",
                "title",
                "quote",
                "significance",
                "page",
                "about",
                "synced_at",
            ],
            "the INSERT list must be exactly the writable columns, in order — a column \
             missing from it is NULL on every row forever, and search_vector must never \
             be in it because Postgres refuses a write to a generated column"
        );
    }

    /// It is an UPSERT, and the conflict target is the primary key.
    ///
    /// A plain INSERT would fail the second time the backfill ran, which would
    /// make the tool un-rerunnable — and an un-rerunnable backfill is one nobody
    /// dares run when it matters.
    #[test]
    fn a_re_run_updates_rather_than_failing() {
        assert!(UPSERT_SQL.contains("ON CONFLICT (evidence_id) DO UPDATE"));
        assert!(
            UPSERT_SQL.contains("synced_at    = now()"),
            "an update must refresh synced_at, or the mirror cannot say when it was last written"
        );
    }

    /// The ghost delete is scoped to one document and survives an empty set.
    ///
    /// Two properties, and both are load-bearing. The `document_id = $1` scope is
    /// what makes it safe for two paths to call this — one document's sync can
    /// never reach another's rows. And `<> ALL` is what makes the EMPTY set
    /// clear the document rather than do nothing: `evidence_id <> ALL('{}')` is
    /// true for every row, whereas the same predicate written with `NOT IN` goes
    /// NULL the moment any element is NULL and would silently delete nothing.
    #[test]
    fn the_ghost_delete_is_document_scoped_and_empty_safe() {
        assert!(
            DELETE_GHOSTS_SQL.contains("WHERE document_id = $1"),
            "the delete must be scoped to one document — two paths call this"
        );
        assert!(
            DELETE_GHOSTS_SQL.contains("evidence_id <> ALL($2::text[])"),
            "must be <> ALL, so an empty incoming set clears the document's rows"
        );
        assert!(
            !DELETE_GHOSTS_SQL.contains("NOT IN"),
            "NOT IN goes NULL on a NULL element and would delete nothing"
        );
    }

    /// The sync's delete never touches rows it was not given a document for.
    ///
    /// A `DELETE` with no `WHERE` — or one scoped only by the id list — would
    /// empty the mirror for every other document the first time it ran. Cheap to
    /// assert, catastrophic to get wrong.
    #[test]
    fn the_ghost_delete_cannot_be_unscoped() {
        let where_clause = DELETE_GHOSTS_SQL
            .find("WHERE")
            .expect("the delete must have a WHERE clause");
        let doc_scope = DELETE_GHOSTS_SQL
            .find("document_id = $1")
            .expect("scoped by document");
        assert!(
            doc_scope > where_clause,
            "the document scope must be part of the WHERE, not incidental text"
        );
    }

    /// The statement text does not depend on the batch size.
    ///
    /// One bound parameter, whatever the batch holds — so Postgres plans it once
    /// and no part of the SQL is assembled from data.
    #[test]
    fn the_statement_text_is_independent_of_the_batch_size() {
        assert!(UPSERT_SQL.contains("jsonb_to_recordset($1::jsonb)"));
        assert_eq!(
            UPSERT_SQL.matches('$').count(),
            1,
            "exactly one bound parameter, regardless of how many rows the batch holds"
        );
        assert!(
            UPSERT_SQL.contains("page bigint"),
            "page must land as BIGINT — ruling R1, no narrowing anywhere on this path"
        );
        assert!(
            UPSERT_SQL.contains("jsonb_array_elements_text(r.about)"),
            "about must become a real text[] of this row's own length"
        );
    }

    /// A row serializes to the keys `jsonb_to_recordset` names.
    ///
    /// This is the join between the two halves: the derive produces the keys and
    /// the SQL names the columns, and nothing else checks that they agree.
    #[test]
    fn a_row_serializes_to_the_column_names_the_sql_expects() {
        let row = EvidenceSearchRow {
            evidence_id: "doc-x:evidence:1".to_string(),
            document_id: "doc-x".to_string(),
            title: None,
            quote: "the check was never deposited".to_string(),
            significance: None,
            page: Some(22),
            about: vec!["org-catholic-family-services".to_string()],
        };
        let json = serde_json::to_value([&row]).expect("a row serializes");
        let first = &json[0];
        for column in [
            "evidence_id",
            "document_id",
            "title",
            "quote",
            "significance",
            "page",
            "about",
        ] {
            assert!(
                first.get(column).is_some(),
                "the SQL names `{column}` in its recordset; the struct must serialize it"
            );
            assert!(UPSERT_SQL.contains(column), "the SQL must name `{column}`");
        }
        // An absent title is JSON null, which `jsonb_to_recordset` reads as SQL
        // NULL — not the empty string. The distinction is the whole reason the
        // field is an Option.
        assert!(first["title"].is_null());
        assert_eq!(first["page"], serde_json::json!(22));
        assert_eq!(
            first["about"],
            serde_json::json!(["org-catholic-family-services"])
        );
    }
}
