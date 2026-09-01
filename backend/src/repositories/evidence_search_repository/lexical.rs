//! The lexical half of a ranked gather: full text and trigram, over L1's
//! `evidence_search` mirror.
//!
//! **READ-ONLY.** Two `SELECT`s; a shape test asserts they can never be
//! anything else.
//!
//! ## Why two reads and not one
//!
//! They fail in opposite directions, and the corpus needs both.
//!
//! - **Full text** (`websearch_to_tsquery` over the weighted `search_vector`)
//!   stems and stopwords, so it matches "deposited the money" against "money
//!   was deposited". It also throws away exactly what a legal record turns on:
//!   `to_tsvector` reduces `$50,000` and `$50,000.00` and `50000` to tokens that
//!   no longer carry the dollar sign, so it cannot tell one figure from another.
//! - **Trigram** (`quote % $query`, backed by the `gin_trgm_ops` index) matches
//!   on character runs, so `$50,000` finds `$50,000` and not `$15,000`. It has
//!   no idea what a word means, so on its own it is noise.
//!
//! L1b measured 16 real false positives on `$50,000` alone from full text. The
//! trigram half is what the seven admissions AT-2 needs are found by; the full
//! text half is what everything else is found by.
//!
//! ## How the two halves become one ranking
//!
//! They are fused by the same reciprocal-rank rule that later fuses lexical
//! against vector — see [`crate::services::gather_fusion`]. Not by adding
//! `ts_rank` to `similarity()`: those are different quantities on different
//! scales (a `ts_rank` depends on the document's length and the query's term
//! count; `similarity` is a ratio of trigram sets), and any weighting between
//! them would be a number nobody has measured. Reciprocal rank needs only the
//! order, which is the part both halves mean the same way.

use sqlx::{PgPool, Row};

/// Errors the lexical read can raise, naming the statement that raised them.
///
/// Deliberately NOT the module's existing `EvidenceSearchReadError`: that type
/// wraps `neo4rs::Error`, because everything else here reads the GRAPH to fill
/// the mirror. This reads the mirror itself, over Postgres, and folding a
/// `sqlx::Error` into a graph error would tell an operator to go and look at
/// Neo4j when the failure was in a `SELECT`.
#[derive(Debug, thiserror::Error)]
pub enum LexicalReadError {
    #[error(
        "lexical read '{operation}' failed against evidence_search: {source} — check the \
         L1a migration has been applied and that pg_trgm is installed"
    )]
    Query {
        operation: &'static str,
        #[source]
        source: sqlx::Error,
    },
}

// STRUCTURAL: SQL is wire vocabulary for the Postgres protocol, not a
// deployment-variable setting. Held at module scope so the shape tests assert
// against the text that actually runs.
//
// ## ⚑ Why the query is turned into an OR
//
// `websearch_to_tsquery('english', 'a b c')` yields `a & b & c` — it ANDs every
// term. That is right for a search box, where a user types four words and means
// all four. It is catastrophic here: the composed query is a scenario's theme
// plus the verbatim text of up to nine allegations, so a thousand characters
// and a hundred and fifty distinct lexemes, and requiring a single card to
// contain ALL of them matches nothing. Measured before this fix, on the real
// corpus: the full-text half returned 0 rows for both S-9 and S-11.
//
// So the `&`s are rewritten to `|`s. The query becomes "any of these lexemes",
// `ts_rank` then does the work of ordering by how many matched and how heavily
// they are weighted, and the GIN index still answers it. `<->` (the phrase
// operator, which only appears for quoted input) is rewritten too, so a future
// caller quoting a phrase degrades to OR rather than silently ANDing again.
//
// The rewrite is done in SQL rather than in Rust because `websearch_to_tsquery`
// is what parses the prose — doing it here means the parsing and the rewrite
// cannot drift, and the query text stays one auditable statement.
//
// `$3::text[]` is the party filter and `$4::bool` disables it, rather than two
// separate statements: one query text means one plan, one place to audit, and
// no chance of the filtered and unfiltered paths drifting apart.
const FULL_TEXT_SQL: &str = "\
    WITH q AS (SELECT replace(replace(\
                 websearch_to_tsquery('english', $1)::text, '<->', '|'), '&', '|')::tsquery AS tsq) \
    SELECT evidence_id \
      FROM evidence_search, q \
     WHERE search_vector @@ q.tsq \
       AND ($4 OR about && $3::text[]) \
     ORDER BY ts_rank(search_vector, q.tsq) DESC, evidence_id ASC \
     LIMIT $2";

const TRIGRAM_SQL: &str = "\
    SELECT evidence_id \
      FROM evidence_search \
     WHERE quote % $1 \
       AND ($4 OR about && $3::text[]) \
     ORDER BY similarity(quote, $1) DESC, evidence_id ASC \
     LIMIT $2";

// STRUCTURAL: the id-ordered membership read behind the conservation baseline.
// No text, no ranking — just "which cards is this party filter allowed to
// reach", which is what `strict` has to reproduce exactly.
// The placeholders are $1/$2 here and $3/$4 in the ranked reads above,
// because those two bind the query and the limit first. Numbered per statement
// rather than kept uniform: a placeholder number is positional in the bind
// list, and Postgres counts the HIGHEST one to decide how many parameters it
// expects — leaving a gap makes it demand a parameter that is never bound.
const MEMBERSHIP_SQL: &str = "\
    SELECT evidence_id \
      FROM evidence_search \
     WHERE ($2 OR about && $1::text[]) \
     ORDER BY evidence_id ASC";

/// One lexical read: full text and trigram, each a ranked id list.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LexicalHits {
    /// Ranked best-first by `ts_rank`.
    pub full_text: Vec<String>,
    /// Ranked best-first by trigram `similarity`.
    pub trigram: Vec<String>,
}

/// Run both lexical halves for one query.
///
/// `parties` is `None` for "no filter" and `Some(list)` for "only these" — the
/// same distinction [`crate::domain::gather_filter::GatherSubjectFilter`] draws,
/// carried through rather than flattened, because `Some(empty)` (reach nothing)
/// and `None` (reach everything) are opposite states.
///
/// # Errors
/// Returns [`LexicalReadError`] if either statement fails.
pub async fn lexical_search(
    pool: &PgPool,
    query: &str,
    parties: Option<&[&str]>,
    limit: i64,
) -> Result<LexicalHits, LexicalReadError> {
    let (party_list, unfiltered) = filter_args(parties);

    Ok(LexicalHits {
        full_text: ranked_ids(pool, FULL_TEXT_SQL, query, &party_list, unfiltered, limit).await?,
        trigram: ranked_ids(pool, TRIGRAM_SQL, query, &party_list, unfiltered, limit).await?,
    })
}

/// Every evidence id the party filter admits, ignoring the query entirely.
///
/// This is the conservation baseline and the bound the vector read is given:
/// under `strict` it is exactly today's subject-only pool.
///
/// # Errors
/// Returns [`LexicalReadError`] if the statement fails.
pub async fn party_membership(
    pool: &PgPool,
    parties: Option<&[&str]>,
) -> Result<Vec<String>, LexicalReadError> {
    let (party_list, unfiltered) = filter_args(parties);
    let rows = sqlx::query(MEMBERSHIP_SQL)
        .bind(&party_list)
        .bind(unfiltered)
        .fetch_all(pool)
        .await
        .map_err(|source| LexicalReadError::Query {
            operation: "party_membership",
            source,
        })?;
    collect_ids(rows, "party_membership")
}

/// `(the bound list, whether the filter is disabled)`.
///
/// ## Rust Learning: why the empty list is not the "no filter" signal
///
/// It would be tempting to pass an empty array and let `about && '{}'` mean
/// "match everything". It does not — array overlap with an empty array is
/// always false, so that spelling would silently return zero rows. The boolean
/// carries the distinction the SQL cannot.
fn filter_args(parties: Option<&[&str]>) -> (Vec<String>, bool) {
    match parties {
        Some(list) => (list.iter().map(|p| (*p).to_string()).collect(), false),
        None => (Vec::new(), true),
    }
}

/// Run one ranked statement and collect its ids in order.
async fn ranked_ids(
    pool: &PgPool,
    sql: &'static str,
    query: &str,
    parties: &[String],
    unfiltered: bool,
    limit: i64,
) -> Result<Vec<String>, LexicalReadError> {
    let operation = if sql == FULL_TEXT_SQL {
        "lexical_full_text"
    } else {
        "lexical_trigram"
    };
    let rows = sqlx::query(sql)
        .bind(query)
        .bind(limit)
        .bind(parties)
        .bind(unfiltered)
        .fetch_all(pool)
        .await
        .map_err(|source| LexicalReadError::Query { operation, source })?;
    collect_ids(rows, operation)
}

/// Decode the single column, naming the operation if a row will not decode.
fn collect_ids(
    rows: Vec<sqlx::postgres::PgRow>,
    operation: &'static str,
) -> Result<Vec<String>, LexicalReadError> {
    rows.into_iter()
        .map(|row| {
            row.try_get::<String, _>("evidence_id")
                .map_err(|source| LexicalReadError::Query { operation, source })
        })
        .collect()
}

#[cfg(test)]
#[path = "lexical_tests.rs"]
mod tests;
