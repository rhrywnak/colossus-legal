//! Shape tests for the lexical statements.
//!
//! No database: these assert what the SQL SAYS, which is the part that decides
//! whether the GIN indexes are used and whether the read can write. The
//! behaviour against real rows is measured by the L2b measurement harness.

use super::*;

/// The lexical reads cannot write.
#[test]
fn the_lexical_reads_cannot_write() {
    for sql in [FULL_TEXT_SQL, TRIGRAM_SQL, MEMBERSHIP_SQL] {
        let upper = sql.to_uppercase();
        for forbidden in [
            "INSERT", "UPDATE", "DELETE", "DROP", "ALTER", "TRUNCATE", "CREATE",
        ] {
            assert!(
                !upper.contains(forbidden),
                "a gather read must never contain {forbidden}: {sql}"
            );
        }
    }
}

/// ⚑ Both halves use the operator their index answers.
///
/// This is the difference between a read that uses a GIN index and one that
/// scans 1209 rows and gets slower with every document ingested. `ts_rank` and
/// `similarity` in an ORDER BY are NOT index-using on their own — the `@@` and
/// `%` in the WHERE are what make the indexes usable.
#[test]
fn each_half_uses_the_operator_its_index_answers() {
    assert!(
        FULL_TEXT_SQL.contains("search_vector @@ q.tsq"),
        "@@ against the generated tsvector is what idx_evidence_search_vector answers"
    );
    assert!(
        TRIGRAM_SQL.contains("quote % $1"),
        "% is what idx_evidence_search_quote_trgm answers; similarity() alone is not"
    );
    assert!(
        TRIGRAM_SQL.contains("similarity(quote, $1) DESC"),
        "and the same measure orders the result"
    );
}

/// The full-text half searches the WEIGHTED vector, not the raw quote.
///
/// L1a put quote at weight A, title at B, significance at C precisely so a
/// match in the quote outranks a match in a title that happens to echo the
/// query word. Searching `quote` directly here would throw that away.
#[test]
fn the_full_text_half_searches_the_weighted_vector() {
    assert!(FULL_TEXT_SQL.contains("search_vector"));
    assert!(
        !FULL_TEXT_SQL.contains("to_tsvector('english', quote)"),
        "the generated column is the one with the weights on it"
    );
}

/// ⚑ The composed query is ORed, not ANDed.
///
/// `websearch_to_tsquery` ANDs its terms. The composed query is a theme plus up
/// to nine verbatim allegations — a thousand characters — and no single card
/// contains all of it. Measured before the rewrite, on the real corpus: the
/// full-text half returned 0 rows for BOTH S-9 and S-11. It is not a subtle
/// degradation; the half simply does not work ANDed.
#[test]
fn the_composed_query_is_ored_rather_than_anded() {
    assert!(
        FULL_TEXT_SQL.contains("'&', '|'"),
        "the ANDs must be rewritten to ORs or the half returns nothing: {FULL_TEXT_SQL}"
    );
    assert!(
        FULL_TEXT_SQL.contains("'<->', '|'"),
        "the phrase operator too, so a quoted phrase degrades to OR rather than \
         silently ANDing again"
    );
    // The rewrite must reach BOTH uses — a WHERE that ORs and an ORDER BY that
    // ANDs would rank every row zero.
    assert_eq!(
        FULL_TEXT_SQL.matches("q.tsq").count(),
        2,
        "the same rewritten query must drive the filter AND the ranking"
    );
}

/// `websearch_to_tsquery`, not `plainto_tsquery` or `to_tsquery`.
///
/// The composed query is arbitrary prose lifted from a complaint. `to_tsquery`
/// raises a syntax error on an unbalanced quote or a stray `&`, which would be
/// a mid-gather failure no operator could fix; `websearch_to_tsquery` never
/// raises.
#[test]
fn the_query_parser_cannot_raise_on_punctuation() {
    assert!(FULL_TEXT_SQL.contains("websearch_to_tsquery"));
    assert!(!FULL_TEXT_SQL.contains("plainto_tsquery"));
    assert!(
        !FULL_TEXT_SQL.contains(" to_tsquery("),
        "to_tsquery raises on prose punctuation"
    );
}

/// Both ranked reads break ties on the id, so two runs return the same list.
///
/// Without it, two rows of equal rank come back in whatever order the plan
/// produced, the fused ranks shift, and a measurement is not reproducible.
#[test]
fn both_ranked_reads_break_ties_on_the_id() {
    for sql in [FULL_TEXT_SQL, TRIGRAM_SQL] {
        assert!(
            sql.contains("evidence_id ASC"),
            "a ranked read with no tiebreak is not reproducible: {sql}"
        );
    }
    assert!(MEMBERSHIP_SQL.contains("ORDER BY evidence_id ASC"));
}

/// ⚑ `None` disables the filter; `Some(empty)` matches nothing.
///
/// Array overlap against an empty array is always FALSE in Postgres, so an
/// empty list cannot be the "no filter" spelling — it would silently return
/// zero rows. The boolean is what carries the distinction the SQL cannot.
#[test]
fn no_filter_and_an_empty_filter_are_different_arguments() {
    let (list, unfiltered) = filter_args(None);
    assert!(unfiltered, "None disables the filter");
    assert!(list.is_empty());

    let (list, unfiltered) = filter_args(Some(&[]));
    assert!(
        !unfiltered,
        "Some(empty) is a filter that matches nothing — a real, reportable state"
    );
    assert!(list.is_empty());

    let (list, unfiltered) =
        filter_args(Some(&["person-emil-awad", "org-catholic-family-services"]));
    assert!(!unfiltered);
    assert_eq!(
        list,
        vec!["person-emil-awad", "org-catholic-family-services"]
    );
}

/// The filter is applied by array overlap, and the disable flag short-circuits
/// it in SQL rather than by swapping in a second statement.
#[test]
fn every_statement_carries_the_same_party_filter() {
    for sql in [FULL_TEXT_SQL, TRIGRAM_SQL] {
        assert!(
            sql.contains("($4 OR about && $3::text[])"),
            "one filter expression, one plan, nothing to drift: {sql}"
        );
    }
    assert!(MEMBERSHIP_SQL.contains("($2 OR about && $1::text[])"));
}

/// ⚑ Every statement's placeholders are dense from $1, with no gap.
///
/// Postgres decides how many parameters a prepared statement takes from the
/// HIGHEST placeholder number, not from how many distinct ones appear. A
/// statement using $1 and $3 demands three binds and fails at execution with
/// "bind message supplies 2 parameters, but prepared statement requires 3" —
/// which is a runtime error no compiler and no shape test catches unless it
/// looks for exactly this.
#[test]
fn every_statement_numbers_its_placeholders_densely_from_one() {
    for (name, sql, expected) in [
        ("full text", FULL_TEXT_SQL, 4),
        ("trigram", TRIGRAM_SQL, 4),
        ("membership", MEMBERSHIP_SQL, 2),
    ] {
        let used: std::collections::BTreeSet<usize> =
            (1..=9).filter(|n| sql.contains(&format!("${n}"))).collect();
        let highest = *used.iter().max().expect("every statement binds something");
        assert_eq!(
            highest, expected,
            "{name} binds {expected} parameters, so its highest placeholder must be \
             ${expected} — Postgres counts the highest, not the distinct ones"
        );
        assert_eq!(
            used.len(),
            highest,
            "{name} has a GAP in its placeholders: {used:?}. Postgres will demand \
             {highest} binds and the call supplies {}",
            used.len()
        );
    }
}

/// Both ranked reads are bounded. An unbounded gather read would pull the whole
/// mirror into memory and rank it.
#[test]
fn the_ranked_reads_are_bounded() {
    assert!(FULL_TEXT_SQL.contains("LIMIT $2"));
    assert!(TRIGRAM_SQL.contains("LIMIT $2"));
}

/// A failure names the statement and what to check.
#[test]
fn a_lexical_failure_names_the_statement_and_the_likely_cause() {
    let rendered = LexicalReadError::Query {
        operation: "lexical_trigram",
        source: sqlx::Error::RowNotFound,
    }
    .to_string();

    assert!(rendered.contains("lexical_trigram"), "{rendered}");
    assert!(rendered.contains("evidence_search"), "{rendered}");
    assert!(
        rendered.contains("L1a migration") && rendered.contains("pg_trgm"),
        "the two things that are actually ever wrong here: {rendered}"
    );
}
