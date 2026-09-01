//! backend/tests/evidence_search_l1a_integration.rs
//!
//! ⚑ L1a — the shape proof for `evidence_search`.
//!
//! Two kinds of test live here, and the split is deliberate:
//!
//! 1. **One CI-runnable test that reads the migration file off disk** and
//!    asserts the load-bearing pieces are still in it. Everything else in this
//!    file is `#[ignore]`, so without this the whole migration would have no
//!    coverage that anyone actually runs. This is the Rule 21 pattern —
//!    disk/code consistency asserted by a test rather than by review.
//!
//! 2. **Three `#[ignore]` behavioural tests against a live database**, because
//!    the questions they ask ("does the generated column recompute on UPDATE?")
//!    can only be answered by Postgres. The project has no `#[sqlx::test]`
//!    fixture tier, so these follow the same convention as
//!    `timeline_subsets_integration.rs`: ignored by default, run by hand.
//!
//! ```text
//! EVIDENCE_SEARCH_TEST_DATABASE_URL=postgres://…/colossus_l1a_proof \
//!   cargo test -p colossus-legal-backend \
//!     --test evidence_search_l1a_integration -- --ignored --test-threads=1
//! ```
//!
//! ## Why this test refuses to use `PIPELINE_DATABASE_URL`
//!
//! Every other integration test in this directory reads the live pipeline URL
//! from `AppConfig` and writes to `colossus_legal_v2`. This one must not, and
//! the reason is that it INSERTS rows into a table whose whole purpose is to be
//! a faithful mirror of the graph. Three hand-built rows carrying the word
//! "Milster" in a table L1b is about to backfill and L2 is about to search is
//! not test residue, it is contamination of a search corpus.
//!
//! So it takes its own variable, `EVIDENCE_SEARCH_TEST_DATABASE_URL`, and
//! [`guarded_pool`] REFUSES to run against a database whose name is a known real
//! one. A test that can only run somewhere harmless cannot be run somewhere
//! harmful by accident at 2am.
//!
//! ## What is deliberately NOT here
//!
//! No `EXPLAIN ANALYZE`. The table is empty until L1b backfills it, and on three
//! rows the planner picks a sequential scan every time — a timing from that is
//! not information about the index, it is information about the row count. Those
//! numbers belong to L1b, against real data.

use sqlx::{PgPool, Row};

type TestResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// The migration this file is the proof of.
const MIGRATION: &str = "pipeline_migrations/20260901083845_evidence_search_table.sql";

/// Database names this test must never write to. A prefix on every seeded row
/// makes cleanup safe; this makes the cleanup unnecessary in the first place.
const FORBIDDEN_DATABASES: &[&str] = &["colossus_legal", "colossus_legal_v2"];

/// Every row this test inserts is keyed with this, so a failed run leaves rows
/// that are unmistakably ours and the next run can clear them.
const TEST_ID_PREFIX: &str = "__l1a_test:evidence:";

// ─── 1 · The CI-runnable shape test ──────────────────────────────────────────

/// The migration still carries every piece L2 and L1c depend on.
///
/// Not a re-implementation of the SQL — a set of claims about it that would
/// otherwise be held only by review. Each one names what breaks if it goes.
#[test]
fn the_migration_still_carries_every_load_bearing_piece() {
    let raw = std::fs::read_to_string(MIGRATION)
        .unwrap_or_else(|e| panic!("cannot read {MIGRATION}: {e}"));

    // Runs of whitespace collapse to one space before matching, so this test
    // asserts what the SQL SAYS rather than how its columns happen to be aligned.
    // Without it, `about             TEXT[]` would fail the day somebody
    // re-indented the column list — a failure on a migration that is still
    // completely correct, which is how a test earns a reputation for crying wolf.
    let sql: String = raw.split_whitespace().collect::<Vec<_>>().join(" ");

    for (needle, why) in [
        (
            "CREATE EXTENSION IF NOT EXISTS pg_trgm",
            "without the extension the trigram index cannot be created at all",
        ),
        (
            "GENERATED ALWAYS AS",
            "a trigger can be disabled or bypassed; a generated column cannot drift",
        ),
        (
            "STORED",
            "a VIRTUAL generated column cannot be indexed, so the GIN index would have nothing to index",
        ),
        (
            "to_tsvector('english'",
            "the one-argument form is only STABLE and Postgres rejects it in a generated column",
        ),
        (
            "'A')",
            "the quote must outrank the title and our own commentary",
        ),
        (
            "USING GIN (search_vector)",
            "the full-text half of the lexical gather",
        ),
        (
            "USING GIN (probe_text gin_trgm_ops)",
            "the trigram half — the one that can tell $50,000 from 50,000 — over quote, \
             title and significance, because 109 of 1209 quotes are a bare \"Admitted.\"",
        ),
        (
            // The haystack has been whitespace-normalised above, so the needle
            // must be single-spaced however the migration aligns its columns.
            "probe_text TEXT GENERATED ALWAYS AS (",
            "the trigram surface is generated, so it cannot drift from the row it mirrors",
        ),
        (
            "about TEXT[] NOT NULL DEFAULT '{}'",
            "L2's subject filter is a set-membership test; a joined string would have to be re-split",
        ),
        (
            "page BIGINT",
            "the source is Option<i64>; INTEGER would force L1c into a narrowing conversion (ruling R1)",
        ),
        (
            "RAISE EXCEPTION",
            "every statement is IF NOT EXISTS, so a pre-existing object of the same name would be a silent no-op",
        ),
    ] {
        assert!(
            sql.contains(needle),
            "{MIGRATION} no longer contains `{needle}` — {why}"
        );
    }
}

/// The table carries exactly ONE timestamp, and it is `synced_at`.
///
/// An earlier draft also had `source_updated_at`, for detecting rows that had
/// gone stale against the graph. Roman ruled on 2026-09-01 that staleness is
/// handled by whole-document re-sync instead — L1c rewrites every row for a
/// document and deletes the ones the graph no longer has — so a per-row source
/// stamp is unnecessary as well as unfillable.
///
/// This asserts the COLUMN is gone, not the word: the migration's prose still
/// explains why it was removed, and it should, because "why is there no source
/// timestamp?" is the first question the next reader will ask.
#[test]
fn the_table_carries_one_timestamp_and_it_is_synced_at() {
    let raw = std::fs::read_to_string(MIGRATION)
        .unwrap_or_else(|e| panic!("cannot read {MIGRATION}: {e}"));
    let sql: String = raw.split_whitespace().collect::<Vec<_>>().join(" ");

    assert!(
        !sql.contains("source_updated_at TIMESTAMPTZ"),
        "source_updated_at was removed by ruling: staleness is whole-document re-sync"
    );
    assert!(
        !sql.contains("'source_updated_at'"),
        "the shape assertion must not still be counting a column that no longer exists"
    );
    assert!(sql.contains("synced_at TIMESTAMPTZ NOT NULL DEFAULT now()"));
    assert!(
        sql.contains("the 10 expected columns"),
        "the DO block's count must match the columns the table actually has"
    );
}

/// The migration creates no foreign key, and that is a decision, not an omission.
///
/// The graph is the authority for what Evidence exists. A FK to `documents`
/// would let this derived mirror reject a row the graph accepted.
#[test]
fn the_mirror_has_no_foreign_key_to_veto_its_own_source() {
    let sql = std::fs::read_to_string(MIGRATION)
        .unwrap_or_else(|e| panic!("cannot read {MIGRATION}: {e}"));

    assert!(
        !sql.to_uppercase().contains("REFERENCES "),
        "evidence_search must carry no foreign key: it is derived from the graph, \
         and a constraint here could refuse a row the graph already accepted"
    );
}

// ─── 2 · The live behavioural tests ──────────────────────────────────────────

/// Open the throwaway database, or refuse.
///
/// ## Rust Learning: why this returns `TestResult` instead of panicking
///
/// A missing variable and a forbidden database are different outcomes and the
/// caller should be able to tell them apart in the failure text. Returning the
/// error lets the `?` in each test carry the sentence to the console verbatim,
/// which a bare `expect()` would flatten into "called Result::unwrap on an Err".
async fn guarded_pool() -> TestResult<PgPool> {
    let url = std::env::var("EVIDENCE_SEARCH_TEST_DATABASE_URL").map_err(|_| {
        "EVIDENCE_SEARCH_TEST_DATABASE_URL is not set. This test writes rows, so it \
         deliberately does NOT fall back to PIPELINE_DATABASE_URL — point it at a \
         throwaway database (see the module doc)."
    })?;

    // The database name is the last path segment, minus any query string. Parsed
    // rather than pattern-matched on the whole URL so a host called
    // `colossus_legal_v2.example.com` is not mistaken for the database.
    let name = url
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .split('?')
        .next()
        .unwrap_or_default();

    if FORBIDDEN_DATABASES.contains(&name) {
        return Err(format!(
            "refusing to run: '{name}' is a real database. This test INSERTs rows into \
             evidence_search, and three hand-built rows in a search corpus L2 is about \
             to query is contamination, not residue."
        )
        .into());
    }

    Ok(sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await?)
}

/// Clear this test's own rows. Runs at the start of every test as well as the
/// end, so a run that died half way does not fail the next one.
async fn clear(pool: &PgPool) -> TestResult<()> {
    sqlx::query("DELETE FROM evidence_search WHERE evidence_id LIKE $1")
        .bind(format!("{TEST_ID_PREFIX}%"))
        .execute(pool)
        .await?;
    Ok(())
}

/// Seed one row. `about` goes in as a real `TEXT[]`, which is half the point of
/// the column — sqlx binds a `Vec<String>` straight to it with no join.
async fn seed(
    pool: &PgPool,
    suffix: &str,
    quote: &str,
    title: &str,
    significance: &str,
    about: &[&str],
) -> TestResult<String> {
    let id = format!("{TEST_ID_PREFIX}{suffix}");
    let about: Vec<String> = about.iter().map(|s| (*s).to_string()).collect();
    sqlx::query(
        "INSERT INTO evidence_search \
             (evidence_id, document_id, title, quote, significance, page, about) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(&id)
    .bind("doc-l1a-test")
    .bind(title)
    .bind(quote)
    .bind(significance)
    // `i64`, matching the BIGINT column and `BiasInstance.page_number:
    // Option<i64>` — the binding L1c will use, with no narrowing anywhere.
    .bind(22_i64)
    .bind(&about)
    .execute(pool)
    .await?;
    Ok(id)
}

/// The three rows every live test below works from.
///
/// Domain note: row 2 is the one that makes the two indexes necessary. It says
/// "50,000" with NO dollar sign — which the english analyzer cannot distinguish
/// from row 1's "$50,000", because it discards the sign and splits on the comma.
async fn seed_three(pool: &PgPool) -> TestResult<(String, String, String)> {
    // "custody" is the WEIGHT PROBE: it appears in this row's QUOTE (weight A)
    // and in the next row's SIGNIFICANCE (weight C), and nowhere else. One term,
    // two rows of comparable length — which is what makes a rank comparison
    // between them a measurement of the weights rather than of the text.
    let dollars = seed(
        pool,
        "dollars",
        "CFS took custody of the $50,000 check for two and a half months without depositing it.",
        "Phillips admits CFS held the check",
        "The admission at the spine of the S-11 theme.",
        &["org-catholic-family-services", "person-emil-awad"],
    )
    .await?;
    let bare = seed(
        pool,
        "bare",
        "The auction netted 50,000 in scrap with no dollar sign anywhere in it.",
        "Auction proceeds",
        "Not the custody question; included to prove the analyzer cannot see a dollar sign.",
        &["org-catholic-family-services"],
    )
    .await?;
    let milster = seed(
        pool,
        "milster",
        "Richard Milster prepared the pleadings filed under Form 1724.",
        "Milster prepared the pleadings",
        "Names and form numbers are typed from memory and half-remembered.",
        &[],
    )
    .await?;
    Ok((dollars, bare, milster))
}

/// A full-text query reaches the quote, and stemming does the work it is there
/// for: the query says "deposit", the quote says "depositing".
#[tokio::test]
#[ignore = "requires EVIDENCE_SEARCH_TEST_DATABASE_URL pointing at a throwaway database"]
async fn a_full_text_query_matches_on_the_quote() -> TestResult<()> {
    let pool = guarded_pool().await?;
    clear(&pool).await?;
    let (dollars, _, _) = seed_three(&pool).await?;

    let hits: Vec<String> = sqlx::query_scalar(
        "SELECT evidence_id FROM evidence_search \
         WHERE evidence_id LIKE $1 \
           AND search_vector @@ websearch_to_tsquery('english', 'deposit') \
         ORDER BY evidence_id",
    )
    .bind(format!("{TEST_ID_PREFIX}%"))
    .fetch_all(&pool)
    .await?;

    assert_eq!(
        hits,
        vec![dollars],
        "the english stemmer must reach 'depositing' from a query for 'deposit'"
    );

    // The weighting is not decoration: the same word found in the record must
    // outrank it found in our own commentary. ONE term, "custody", carried by two
    // rows of comparable length — in `dollars`.quote at weight A and in
    // `bare`.significance at weight C, and nowhere else. Comparing two DIFFERENT
    // terms across two rows would measure term frequency and document length as
    // much as the weights, and could pass for the wrong reason.
    let rank_of = |id: String| {
        let pool = pool.clone();
        async move {
            sqlx::query_scalar::<_, f32>(
                "SELECT ts_rank(search_vector, websearch_to_tsquery('english', 'custody')) \
                 FROM evidence_search WHERE evidence_id = $1",
            )
            .bind(id)
            .fetch_one(&pool)
            .await
        }
    };
    let in_the_quote = rank_of(format!("{TEST_ID_PREFIX}dollars")).await?;
    let in_our_commentary = rank_of(format!("{TEST_ID_PREFIX}bare")).await?;
    assert!(
        in_the_quote > in_our_commentary,
        "\"custody\" in the quote (weight A, {in_the_quote}) must outrank the same word \
         in our own significance note (weight C, {in_our_commentary})"
    );

    clear(&pool).await?;
    Ok(())
}

/// A trigram query finds `$50,000` EXACTLY — and the full-text index, on the
/// same two rows, cannot tell it from a bare `50,000`.
///
/// This test is the entire justification for there being two indexes, so it
/// asserts both halves: what trigrams do, and what full text does instead.
#[tokio::test]
#[ignore = "requires EVIDENCE_SEARCH_TEST_DATABASE_URL pointing at a throwaway database"]
async fn a_trigram_query_matches_a_dollar_amount_exactly() -> TestResult<()> {
    let pool = guarded_pool().await?;
    clear(&pool).await?;
    let (dollars, bare, milster) = seed_three(&pool).await?;

    let literal: Vec<String> = sqlx::query_scalar(
        "SELECT evidence_id FROM evidence_search \
         WHERE evidence_id LIKE $1 AND quote LIKE '%$50,000%' ORDER BY evidence_id",
    )
    .bind(format!("{TEST_ID_PREFIX}%"))
    .fetch_all(&pool)
    .await?;
    assert_eq!(
        literal,
        vec![dollars.clone()],
        "the trigram half must match the characters — dollar sign, comma and all"
    );

    // The contrast. Same intent, expressed as full text: it matches BOTH, because
    // by index time '$50,000' and '50,000' are the same two tokens.
    let analyzed: Vec<String> = sqlx::query_scalar(
        "SELECT evidence_id FROM evidence_search \
         WHERE evidence_id LIKE $1 \
           AND search_vector @@ websearch_to_tsquery('english', '$50,000') \
         ORDER BY evidence_id",
    )
    .bind(format!("{TEST_ID_PREFIX}%"))
    .fetch_all(&pool)
    .await?;
    let mut both = vec![bare, dollars];
    both.sort();
    assert_eq!(
        analyzed, both,
        "full text discards the dollar sign, so it cannot distinguish the two — \
         which is precisely why the trigram index exists"
    );

    // Substrings, the other thing whole-token matching cannot do.
    let partial: Vec<String> = sqlx::query_scalar(
        "SELECT evidence_id FROM evidence_search \
         WHERE evidence_id LIKE $1 AND quote ILIKE '%Milste%' ORDER BY evidence_id",
    )
    .bind(format!("{TEST_ID_PREFIX}%"))
    .fetch_all(&pool)
    .await?;
    assert_eq!(partial, vec![milster.clone()], "trigrams reach substrings");

    let partial_full_text: Vec<String> = sqlx::query_scalar(
        "SELECT evidence_id FROM evidence_search \
         WHERE evidence_id LIKE $1 \
           AND search_vector @@ websearch_to_tsquery('english', 'Milste') \
         ORDER BY evidence_id",
    )
    .bind(format!("{TEST_ID_PREFIX}%"))
    .fetch_all(&pool)
    .await?;
    assert!(
        partial_full_text.is_empty(),
        "whole-token matching cannot reach 'Milster' from 'Milste' — measured, not assumed"
    );

    clear(&pool).await?;
    Ok(())
}

/// The generated column recomputes on UPDATE. This is the property that makes it
/// a generated column rather than a trigger: nothing has to remember.
#[tokio::test]
#[ignore = "requires EVIDENCE_SEARCH_TEST_DATABASE_URL pointing at a throwaway database"]
async fn the_generated_column_follows_an_update() -> TestResult<()> {
    let pool = guarded_pool().await?;
    clear(&pool).await?;
    let (dollars, _, _) = seed_three(&pool).await?;

    let before = sqlx::query(
        "SELECT search_vector::text AS vec, \
                search_vector @@ websearch_to_tsquery('english', 'deposit') AS has_deposit, \
                search_vector @@ websearch_to_tsquery('english', 'escrow')  AS has_escrow \
         FROM evidence_search WHERE evidence_id = $1",
    )
    .bind(&dollars)
    .fetch_one(&pool)
    .await?;
    assert!(before.get::<bool, _>("has_deposit"));
    assert!(!before.get::<bool, _>("has_escrow"));

    sqlx::query("UPDATE evidence_search SET quote = $2 WHERE evidence_id = $1")
        .bind(&dollars)
        .bind("CFS placed the funds in escrow and told nobody.")
        .execute(&pool)
        .await?;

    let after = sqlx::query(
        "SELECT search_vector::text AS vec, \
                search_vector @@ websearch_to_tsquery('english', 'deposit') AS has_deposit, \
                search_vector @@ websearch_to_tsquery('english', 'escrow')  AS has_escrow \
         FROM evidence_search WHERE evidence_id = $1",
    )
    .bind(&dollars)
    .fetch_one(&pool)
    .await?;

    assert!(
        after.get::<bool, _>("has_escrow"),
        "the new quote's terms must be searchable immediately after the UPDATE"
    );
    assert!(
        !after.get::<bool, _>("has_deposit"),
        "the OLD quote's terms must be GONE — a stale term left behind is the drift \
         a generated column exists to make impossible"
    );
    assert_ne!(
        before.get::<String, _>("vec"),
        after.get::<String, _>("vec"),
        "the vector itself must have changed"
    );

    // A generated column cannot be written directly, and proving that is worth one
    // statement: it is what stops L1c from ever setting it by hand and diverging.
    let direct_write = sqlx::query(
        "UPDATE evidence_search SET search_vector = to_tsvector('english', 'anything') \
         WHERE evidence_id = $1",
    )
    .bind(&dollars)
    .execute(&pool)
    .await;
    assert!(
        direct_write.is_err(),
        "Postgres must refuse a direct write to a generated column"
    );

    clear(&pool).await?;
    Ok(())
}
