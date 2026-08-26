//! Document TITLE lookups — reading a document's name without its record.
//!
//! ## Why this is not in `document_records`
//!
//! That module owns the full `DocumentRecord` CRUD, and it sits at 288 non-comment
//! lines against a 300-line limit (Rule 17). Adding a query to it put it over, and
//! growing a module past the limit is not something a task gets to do — so the new
//! query lives here, in the module its purpose names.
//!
//! The seam is real rather than convenient. `document_records` answers "give me
//! everything about this document" and returns a ~40-field row. This answers "what
//! is this document CALLED", for a caller holding ids and needing labels — a
//! projection so narrow it would be wasteful to satisfy with a record read, and one
//! whose consumers care about the ids that come back MISSING.

use sqlx::PgPool;

use super::PipelineRepoError;

/// Titles for a set of document ids — only the ids that still exist come back.
///
/// Used by the orphan strip (task 1.7C, defect D9) to label a group of orphaned
/// references with the document they came from.
///
/// ## The absences are the point
///
/// An id ABSENT from the result is an answer, not a failure: that document no
/// longer exists. Measured on DEV 2026-08-03, two of scenario S-2's 26 orphaned
/// refs point at `doc-court-of-appeals-ruling-01-12-2012` while the live document
/// is `doc-court-of-appeals-**rulling**-01-12-2012` — double `l`, the misspelling
/// OCR produced from the cover page. Those refs predate the re-titling, so their
/// document id genuinely does not exist any more.
///
/// Design §2.8 requires the strip say so rather than invent a title, so the caller
/// (`services::scenario_orphans`) turns an absence into an id-derived slug. This
/// function must therefore return the ids it FOUND, never pad the result.
///
/// ## Rust Learning: `= ANY($1)` instead of a built-up `IN (…)` list
///
/// Postgres accepts an array parameter with `= ANY`, so one bound `&[String]`
/// replaces a hand-assembled `IN ($1, $2, $3…)`. That matters for more than
/// tidiness: string-building an `IN` list is where SQL injection lives, and a
/// parameter count that varies per call defeats statement caching. `sqlx` binds the
/// whole slice as a single `text[]`.
pub async fn document_titles_by_ids(
    pool: &PgPool,
    document_ids: &[String],
) -> Result<Vec<(String, String)>, PipelineRepoError> {
    // An empty `= ANY(ARRAY[]::text[])` is valid SQL and returns nothing, but the
    // round trip is pointless — skip it. Distinct from an error (Standing Rule 1):
    // the orphan strip still renders its groups, just with no titles resolved.
    if document_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query_as::<_, (String, String)>(DOCUMENT_TITLES_BY_IDS_SQL)
        .bind(document_ids)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

/// The title-lookup query. Extracted as a `const` so its shape can be asserted by a
/// unit test without a live database (the house pattern — see
/// `scan_runs::LIST_SCAN_RUNS_SQL`).
///
/// The `= ANY($1)` form is the load-bearing part and the reason this is pinned. The
/// tempting "simplification" is an `IN ($1, $2, $3…)` list built by concatenation,
/// which is both an injection site and a statement-cache defeat. A test that fails
/// when `= ANY` disappears is cheaper than rediscovering why it was there.
///
/// Query text, not config — Rule 13 does not apply.
const DOCUMENT_TITLES_BY_IDS_SQL: &str = "SELECT id, title FROM documents WHERE id = ANY($1)";

/// How many documents matched a search, and the page of them being offered.
///
/// ## Why the total travels with the page
///
/// The picker offers a bounded short list (`chronology_document_picker_max`).
/// A truncated list that looked complete is how somebody links the wrong
/// document with no idea a better match was cut off — so the count of ALL
/// matches comes back beside the ones being shown, and the surface says so when
/// they differ. A silent cap is a silent failure (Standing Rule 1).
#[derive(Debug, Clone)]
pub struct DocumentSearchPage {
    /// The documents being offered, by title.
    pub matches: Vec<(String, String)>,
    /// How many documents matched in total, before the cap.
    pub total: i64,
}

/// Documents whose title contains `needle`, case-insensitively, title-ordered.
///
/// Serves the chronology's document picker (design R9: links are HUMAN-MADE —
/// the author searches the store and picks the target).
///
/// ## ⚑ It reads the SAME table the link resolver reads
///
/// `chronology_links::existing_document_ids` answers "does this link point at
/// something real?" from `documents` in the pipeline database. If the picker
/// searched anywhere else — the Neo4j document list, say — an author could pick
/// a document that the very next page render marked "⚠ no document yet". Pick
/// and resolve are the same question asked twice, so they ask the same table.
///
/// ## Rust Learning: `ILIKE` with a bound pattern, not a formatted one
///
/// The `%` wildcards are concatenated in SQL (`'%' || $1 || '%'`) rather than in
/// Rust, so the needle stays a bound PARAMETER. Building `format!("%{needle}%")`
/// would still bind safely here, but it puts the pattern's syntax in two
/// languages; keeping it in the statement means a needle containing `%` matches
/// a literal-looking search the same way whichever caller sends it.
///
/// # Errors
/// Any database failure, verbatim — the caller names the operation in its log.
pub async fn search_document_titles(
    pool: &PgPool,
    needle: &str,
    limit: i64,
) -> Result<DocumentSearchPage, PipelineRepoError> {
    let total = sqlx::query_scalar::<_, i64>(DOCUMENT_SEARCH_COUNT_SQL)
        .bind(needle)
        .fetch_one(pool)
        .await?;
    let matches = sqlx::query_as::<_, (String, String)>(DOCUMENT_SEARCH_SQL)
        .bind(needle)
        .bind(limit)
        .fetch_all(pool)
        .await?;
    Ok(DocumentSearchPage { matches, total })
}

/// The picker's page query. Pinned by a unit test, like its sibling above.
const DOCUMENT_SEARCH_SQL: &str = "SELECT id, title FROM documents \
     WHERE title ILIKE '%' || $1 || '%' ORDER BY title, id LIMIT $2";

/// The same predicate, counted. Two statements rather than a window function
/// because the count must be over ALL matches while the rows are capped, and a
/// reader should be able to see at a glance that the two agree on the WHERE.
const DOCUMENT_SEARCH_COUNT_SQL: &str =
    "SELECT COUNT(*) FROM documents WHERE title ILIKE '%' || $1 || '%'";

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;
    use std::time::Duration;

    /// A pool aimed at a dead port: any real query fails fast, so a test can prove a
    /// code path did NOT touch the database. Mirrors `scan_run_verdicts`' helper.
    fn dead_pool() -> PgPool {
        PgPoolOptions::new()
            .acquire_timeout(Duration::from_millis(500))
            .connect_lazy("postgres://127.0.0.1:1/nodb")
            .expect("connect_lazy builds a pool without connecting")
    }

    #[tokio::test]
    async fn no_document_ids_is_a_no_op_ok_without_touching_the_pool() {
        // A legitimate no-op: every orphaned ref carried an unparseable id, so there
        // is no document to look up. Must return Ok WITHOUT connecting — the dead
        // pool would error on any real query.
        let result = document_titles_by_ids(&dead_pool(), &[]).await;
        assert!(
            result.is_ok(),
            "an empty id list must be a no-op Ok, got {result:?}"
        );
        assert!(result.expect("checked ok above").is_empty());
    }

    #[test]
    fn the_title_lookup_binds_an_array_rather_than_building_an_in_list() {
        // The injection-safe form. A future "simplification" to an `IN (…)` list
        // built by string concatenation fails here and is told why.
        let sql = DOCUMENT_TITLES_BY_IDS_SQL;
        assert!(
            sql.contains("= ANY($1)"),
            "the id set must bind as one array parameter, got: {sql}"
        );
        assert!(
            !sql.contains(" IN ("),
            "an IN list is where injection lives; bind an array instead, got: {sql}"
        );
        // The two columns the orphan strip needs, from the table that holds them.
        assert!(sql.contains("SELECT id, title"), "got: {sql}");
        assert!(sql.contains("FROM documents"), "got: {sql}");
    }

    #[test]
    fn the_picker_search_and_its_count_agree_on_the_predicate() {
        // The cap is only honest if the total is counted over the SAME matches
        // the page is drawn from. Two statements can drift; this is what stops
        // them — an edit to one predicate that forgets the other fails here.
        let predicate = "title ILIKE '%' || $1 || '%'";
        assert!(
            DOCUMENT_SEARCH_SQL.contains(predicate),
            "got: {DOCUMENT_SEARCH_SQL}"
        );
        assert!(
            DOCUMENT_SEARCH_COUNT_SQL.contains(predicate),
            "got: {DOCUMENT_SEARCH_COUNT_SQL}"
        );
        // The page is capped and ordered; the count is neither, or it would be
        // counting the cap rather than the matches.
        assert!(
            DOCUMENT_SEARCH_SQL.contains("LIMIT $2"),
            "got: {DOCUMENT_SEARCH_SQL}"
        );
        assert!(
            DOCUMENT_SEARCH_SQL.contains("ORDER BY title, id"),
            "got: {DOCUMENT_SEARCH_SQL}"
        );
        assert!(
            !DOCUMENT_SEARCH_COUNT_SQL.contains("LIMIT"),
            "got: {DOCUMENT_SEARCH_COUNT_SQL}"
        );
    }

    #[test]
    fn the_picker_search_binds_its_needle_rather_than_formatting_it() {
        // The wildcards are concatenated IN SQL so the needle stays a bound
        // parameter. A `format!("%{needle}%")` in Rust would put the pattern's
        // syntax in two languages and is the shape this pins against.
        assert!(
            DOCUMENT_SEARCH_SQL.contains("'%' || $1 || '%'"),
            "got: {DOCUMENT_SEARCH_SQL}"
        );
        assert!(
            !DOCUMENT_SEARCH_SQL.contains("{"),
            "the needle must not be interpolated: {DOCUMENT_SEARCH_SQL}"
        );
    }
}
