//! backend/tests/evidence_search_l1b_integration.rs
//!
//! ⚑ L1b — the behavioural proof for the ONE write path into `evidence_search`.
//!
//! These are `#[ignore]`d and need a live database carrying L1a's migration,
//! following the same convention as `evidence_search_l1a_integration.rs` and
//! `timeline_subsets_integration.rs`: the project has no `#[sqlx::test]` fixture
//! tier, so live tests are run by hand.
//!
//! ```text
//! EVIDENCE_SEARCH_TEST_DATABASE_URL=postgres://…/colossus_l1b_proof \
//!   cargo test -p colossus-legal-backend \
//!     --test evidence_search_l1b_integration -- --ignored --test-threads=1
//! ```
//!
//! ## The same refusal L1a shipped, for a sharper reason
//!
//! L1a's live tests wrote three hand-built rows into a search corpus and refused
//! to do it against a real database. That reasoning holds here and gets worse:
//! this file exercises the function the DEV backfill will call, so a careless
//! run against `colossus_legal_v2` would put rows carrying the word "Milster"
//! into the mirror L2 is about to search, indistinguishable from real evidence
//! except by their id prefix. [`guarded_pool`] refuses by database name, before
//! it connects.
//!
//! ## What these three tests are for
//!
//! The upsert is the one thing L1b builds that L1c will also use, so the
//! properties that matter are the ones a SECOND caller depends on: that running
//! it twice is safe, that it refreshes rather than duplicates, and that a
//! changed quote reaches the generated search column. Everything else about the
//! backfill — the counts, the paging, the Qdrant comparison — is pure and tested
//! in the binary itself, without a database.

use sqlx::{PgPool, Row};

use colossus_legal_backend::repositories::pipeline_repository::{
    count_evidence_search_rows, upsert_evidence_search_rows, EvidenceSearchRow,
};

type TestResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Database names this test must never write to.
const FORBIDDEN_DATABASES: &[&str] = &["colossus_legal", "colossus_legal_v2"];

/// Every row this test writes is keyed with this, so cleanup can never reach a
/// row it did not create.
const TEST_ID_PREFIX: &str = "__l1b_test:evidence:";

/// Open the throwaway database, or refuse.
async fn guarded_pool() -> TestResult<PgPool> {
    let url = std::env::var("EVIDENCE_SEARCH_TEST_DATABASE_URL").map_err(|_| {
        "EVIDENCE_SEARCH_TEST_DATABASE_URL is not set. This test WRITES rows through the \
         same function the DEV backfill calls, so it deliberately does not fall back to \
         PIPELINE_DATABASE_URL — point it at a throwaway database."
    })?;

    // The database name is the last path segment minus any query string, parsed
    // rather than matched against the whole URL so a HOST containing the name is
    // not mistaken for the database.
    let name = url
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .split('?')
        .next()
        .unwrap_or_default();

    if FORBIDDEN_DATABASES.contains(&name) {
        return Err(format!(
            "refusing to run: '{name}' is a real database. This test writes through the same \
             upsert the DEV backfill uses; its rows would be indistinguishable from evidence."
        )
        .into());
    }

    Ok(sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await?)
}

async fn clear(pool: &PgPool) -> TestResult<()> {
    sqlx::query("DELETE FROM evidence_search WHERE evidence_id LIKE $1")
        .bind(format!("{TEST_ID_PREFIX}%"))
        .execute(pool)
        .await?;
    Ok(())
}

/// One row, in the shape the graph read produces.
fn row(suffix: &str, quote: &str, about: &[&str]) -> EvidenceSearchRow {
    EvidenceSearchRow {
        evidence_id: format!("{TEST_ID_PREFIX}{suffix}"),
        document_id: "doc-l1b-test".to_string(),
        // Deliberately does NOT echo `suffix`. The generated column weights
        // title at B beside the quote at A, so a title carrying the same word as
        // the quote keeps that word searchable after the quote changes — which
        // is correct behaviour and would make the "old terms are gone" assertion
        // below test nothing. The probe word must live in exactly one column.
        title: Some("Phillips admissions extract".to_string()),
        quote: quote.to_string(),
        // Same discipline as `title` and `significance`: no probe word here
        // either. `question` joined the row on 2026-09-04 and feeds both
        // generated columns, so a probe word placed here would make the
        // "old terms are gone" assertion pass for the wrong reason.
        question: Some("Admit that the extract is accurate.".to_string()),
        // Same discipline as `title`: no probe word here either.
        significance: Some("why it matters".to_string()),
        // i64 all the way down: BIGINT column, Option<i64> field, no conversion.
        page: Some(22),
        about: about.iter().map(|s| (*s).to_string()).collect(),
    }
}

/// The batch every test starts from — deliberately uneven `about` lengths,
/// because that is precisely what a multidimensional array parameter could not
/// have expressed and what the jsonb batch exists to carry.
fn three_rows() -> Vec<EvidenceSearchRow> {
    vec![
        row(
            "custody",
            "CFS took custody of the $50,000 check for two and a half months.",
            &["org-catholic-family-services", "person-emil-awad"],
        ),
        row(
            "milster",
            "Richard Milster prepared the pleadings filed under Form 1724.",
            &["person-george-phillips"],
        ),
        // No ABOUT edges at all: an empty list is a real state and must store as
        // an empty array, never NULL.
        row("orphan", "A quote about nobody in particular.", &[]),
    ]
}

async fn synced_at_of(pool: &PgPool, id: &str) -> TestResult<chrono::DateTime<chrono::Utc>> {
    let row = sqlx::query("SELECT synced_at FROM evidence_search WHERE evidence_id = $1")
        .bind(id)
        .fetch_one(pool)
        .await?;
    Ok(row.get("synced_at"))
}

/// Running the same batch twice does not duplicate, and DOES refresh.
///
/// This is the property the whole backfill rests on: it must be safe to re-run
/// after an interruption, and safe for L1c to call on every document forever.
#[tokio::test]
#[ignore = "requires EVIDENCE_SEARCH_TEST_DATABASE_URL pointing at a throwaway database"]
async fn the_upsert_is_idempotent_and_refreshes_synced_at() -> TestResult<()> {
    let pool = guarded_pool().await?;
    clear(&pool).await?;

    let rows = three_rows();
    let first = upsert_evidence_search_rows(&pool, &rows).await?;
    assert_eq!(first, 3, "three rows inserted");
    let after_first = count_evidence_search_rows(&pool).await?;
    let stamp_first = synced_at_of(&pool, &rows[0].evidence_id).await?;

    // A second identical batch. Row count must not move; synced_at must.
    let second = upsert_evidence_search_rows(&pool, &rows).await?;
    assert_eq!(second, 3, "three rows updated, not inserted");
    assert_eq!(
        count_evidence_search_rows(&pool).await?,
        after_first,
        "a re-run must not duplicate — the conflict target is the primary key"
    );

    let stamp_second = synced_at_of(&pool, &rows[0].evidence_id).await?;
    assert!(
        stamp_second > stamp_first,
        "synced_at records when WE last wrote the row and must move on every write \
         ({stamp_first} -> {stamp_second})"
    );

    // The uneven `about` lengths survived, including the empty one.
    let abouts: Vec<Vec<String>> = sqlx::query_scalar(
        "SELECT about FROM evidence_search WHERE evidence_id LIKE $1 ORDER BY evidence_id",
    )
    .bind(format!("{TEST_ID_PREFIX}%"))
    .fetch_all(&pool)
    .await?;
    assert_eq!(abouts.len(), 3);
    assert_eq!(abouts[0].len(), 2, "custody is about two parties");
    assert_eq!(abouts[1].len(), 1, "milster is about one");
    assert!(
        abouts[2].is_empty(),
        "orphan is about none — empty, not NULL"
    );

    clear(&pool).await?;
    Ok(())
}

/// An answer-only card is findable by the request it answers — through BOTH
/// halves of the lexical surface.
///
/// ## Domain note: the 86 cards this exists for
///
/// 86 of the 1,209 Evidence nodes have a `verbatim_quote` that is nothing but
/// the answer: `Admitted.`, `Denied as untrue.`, `No.` Against quote/title/
/// significance alone there is no retrievable text on those cards at all — the
/// vector for "Admitted." is the vector for every other "Admitted."
///
/// ## Why this test is separate from the one below
///
/// `row()` deliberately keeps probe words OUT of `question`, because the
/// quote-staleness test needs the probe word to live in exactly one column. That
/// discipline is right, and it means none of the other database tests would
/// notice if `question` were missing from the generated columns: they all probe
/// with words that come from the quote. This row inverts it — a generic quote
/// nobody would search for, and the only distinctive words in the request. If
/// the migration's `probe_text` or `search_vector` ever stops reading
/// `question`, this fails and the others do not.
#[tokio::test]
#[ignore = "requires EVIDENCE_SEARCH_TEST_DATABASE_URL pointing at a throwaway database"]
async fn an_answer_only_card_is_findable_by_its_request() -> TestResult<()> {
    let pool = guarded_pool().await?;
    clear(&pool).await?;

    let mut answer_only = row("answeronly", "Admitted.", &["person-emil-awad"]);
    answer_only.question =
        Some("Admit that the promissory note was never recorded with the register.".to_string());
    upsert_evidence_search_rows(&pool, &[answer_only]).await?;

    let found = sqlx::query(
        "SELECT search_vector @@ websearch_to_tsquery('english', 'promissory') AS full_text, \
                probe_text ILIKE '%promissory note%'                           AS trigram_surface, \
                quote                                                          AS quote \
         FROM evidence_search WHERE evidence_id = $1",
    )
    .bind(format!("{TEST_ID_PREFIX}answeronly"))
    .fetch_one(&pool)
    .await?;

    assert!(
        found.get::<bool, _>("full_text"),
        "the full-text half must reach a word that appears only in the request \
         (search_vector weights question at D)"
    );
    assert!(
        found.get::<bool, _>("trigram_surface"),
        "the trigram half must reach it too (probe_text concatenates question first)"
    );
    // And the card still says what the witness said, unchanged.
    assert_eq!(found.get::<String, _>("quote"), "Admitted.");
    clear(&pool).await?;
    Ok(())
}

/// A quote that changes in the graph changes in the mirror, and the generated
/// search column follows it without anything remembering to update it.
#[tokio::test]
#[ignore = "requires EVIDENCE_SEARCH_TEST_DATABASE_URL pointing at a throwaway database"]
async fn a_changed_quote_updates_the_row_and_its_search_vector() -> TestResult<()> {
    let pool = guarded_pool().await?;
    clear(&pool).await?;
    upsert_evidence_search_rows(&pool, &three_rows()).await?;

    let id = format!("{TEST_ID_PREFIX}custody");
    let before = sqlx::query(
        "SELECT search_vector @@ websearch_to_tsquery('english', 'custody') AS has_custody, \
                search_vector @@ websearch_to_tsquery('english', 'escrow')  AS has_escrow \
         FROM evidence_search WHERE evidence_id = $1",
    )
    .bind(&id)
    .fetch_one(&pool)
    .await?;
    assert!(before.get::<bool, _>("has_custody"));
    assert!(!before.get::<bool, _>("has_escrow"));

    // Re-extraction rephrased the quote. The mirror is told by the same upsert.
    let mut changed = three_rows();
    changed[0].quote = "CFS placed the funds in escrow and told nobody.".to_string();
    upsert_evidence_search_rows(&pool, &changed).await?;

    let after = sqlx::query(
        "SELECT quote, \
                search_vector @@ websearch_to_tsquery('english', 'custody') AS has_custody, \
                search_vector @@ websearch_to_tsquery('english', 'escrow')  AS has_escrow \
         FROM evidence_search WHERE evidence_id = $1",
    )
    .bind(&id)
    .fetch_one(&pool)
    .await?;

    assert_eq!(
        after.get::<String, _>("quote"),
        "CFS placed the funds in escrow and told nobody."
    );
    assert!(
        after.get::<bool, _>("has_escrow"),
        "the new quote's terms must be searchable immediately"
    );
    assert!(
        !after.get::<bool, _>("has_custody"),
        "the OLD quote's terms must be gone — a stale term left behind is exactly the drift \
         a generated column exists to make impossible"
    );

    // And the trigram half sees the new characters too.
    let literal: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM evidence_search WHERE evidence_id = $1 AND quote LIKE '%escrow%'",
    )
    .bind(&id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(literal, 1);

    clear(&pool).await?;
    Ok(())
}

/// An empty batch is a no-op that costs nothing and touches nothing.
///
/// The backfill's last page is frequently empty, and L1c will call this for
/// documents that produced no Evidence at all.
#[tokio::test]
#[ignore = "requires EVIDENCE_SEARCH_TEST_DATABASE_URL pointing at a throwaway database"]
async fn an_empty_batch_writes_nothing() -> TestResult<()> {
    let pool = guarded_pool().await?;
    clear(&pool).await?;
    upsert_evidence_search_rows(&pool, &three_rows()).await?;
    let before = count_evidence_search_rows(&pool).await?;

    let written = upsert_evidence_search_rows(&pool, &[]).await?;
    assert_eq!(written, 0);
    assert_eq!(count_evidence_search_rows(&pool).await?, before);

    clear(&pool).await?;
    Ok(())
}
