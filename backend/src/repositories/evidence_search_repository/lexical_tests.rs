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

/// ⚑ The trigram half must never regress to `%`.
///
/// `%` is `similarity()`, normalised over the union of both trigram sets, so a
/// short probe against a long text scores ~0.02 and matches nothing at any
/// threshold. Measured: `quote % '$50,000'` returned 0 against 40 rows that
/// literally contain it. This is the single most reversible-looking mistake in
/// the module — the operators differ by one character.
#[test]
fn the_trigram_half_does_not_use_the_similarity_operator() {
    assert!(
        !TRIGRAM_SQL.contains(" % "),
        "the bare similarity operator cannot match a short needle in a long haystack"
    );
    assert!(
        !TRIGRAM_SQL.contains("similarity(quote"),
        "and it must not order by it either"
    );
    // Operand order is not symmetric: probe on the left, text on the right.
    assert!(
        TRIGRAM_SQL.contains("$1 <% probe_text") && !TRIGRAM_SQL.contains("probe_text <% $1"),
        "reversed, <% asks whether the TEXT appears in the PROBE"
    );
}

/// Both lexical halves search the SAME text, which is what makes their ranks
/// comparable — full text over the weighted vector, trigram over the flat
/// concatenation of the same three fields.
#[test]
fn both_halves_search_the_same_three_fields() {
    assert!(FULL_TEXT_SQL.contains("search_vector"));
    assert!(
        TRIGRAM_SQL.contains("probe_text"),
        "quote alone could not answer for the 109 cards whose quote is 'Admitted.'"
    );
    assert!(
        !TRIGRAM_SQL.contains("quote gin_trgm") && !TRIGRAM_SQL.contains(" quote "),
        "the trigram half no longer reads the quote column directly"
    );
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
        TRIGRAM_SQL.contains("$1 <% probe_text"),
        "<% is what idx_evidence_search_probe_trgm answers, and it is what can find a \
         SHORT probe inside a LONG text — `%` measured 0 hits where 40 rows matched"
    );
    assert!(
        TRIGRAM_SQL.contains("word_similarity($1, probe_text) DESC"),
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

// ---------------------------------------------------------------------------
// The count and the read must agree
// ---------------------------------------------------------------------------

/// ⚑ The count asks EXACTLY the question the read answers.
///
/// The selectivity rule decides on the count and the gather then acts on the
/// read. If the two ever asked different questions — a different operator, a
/// different column, a different filter — the rule would be dropping probes on
/// one basis while the gather searched on another, and nothing would fail
/// loudly enough to notice.
#[test]
fn the_probe_count_and_the_probe_read_ask_the_same_question() {
    assert!(
        TRIGRAM_COUNT_SQL.contains("p.probe <% probe_text"),
        "same operator, same surface, same operand order as the read — the probe on \
         the left, the text on the right"
    );
    assert!(
        TRIGRAM_SQL.contains("$1 <% probe_text"),
        "and the read must still be the thing it is a twin of"
    );
    assert!(
        TRIGRAM_COUNT_SQL.contains("JOIN evidence_search")
            && TRIGRAM_SQL.contains("FROM evidence_search"),
        "and the same table — the count JOINs it against the probe array, the read \
         selects from it directly, but it is the same rows either way"
    );
}

/// Both carry the same party filter, so they count and read the same universe.
///
/// The placeholder NUMBERS differ — the read binds the limit at $2 and so its
/// filter is $3/$4 — which is why this compares the shape rather than the text.
#[test]
fn the_count_and_the_read_filter_the_same_admitted_set() {
    assert!(TRIGRAM_COUNT_SQL.contains("($3 OR about && $2::text[])"));
    assert!(TRIGRAM_SQL.contains("($4 OR about && $3::text[])"));
}

/// ⚑ The count is NOT limited. That is the entire reason it exists.
///
/// A `LIMIT` here would cap the count at the read depth, and a probe matching
/// 534 rows would report the same number as one matching exactly 200 — which is
/// the measurement the selectivity rule cannot be built on.
#[test]
fn the_count_is_unbounded_because_a_capped_count_decides_nothing() {
    assert!(
        !TRIGRAM_COUNT_SQL.to_uppercase().contains("LIMIT"),
        "a capped count cannot tell a saturating probe from a good one"
    );
    assert!(
        TRIGRAM_COUNT_SQL.contains("count(*)"),
        "and it counts rather than returning rows, so a dropped probe costs no read"
    );
    // The read, by contrast, must stay bounded.
    assert!(TRIGRAM_SQL.contains("LIMIT $2"));
}

/// The count cannot write either.
#[test]
fn the_probe_count_cannot_write() {
    let upper = TRIGRAM_COUNT_SQL.to_uppercase();
    for forbidden in [
        "INSERT", "UPDATE", "DELETE", "DROP", "ALTER", "TRUNCATE", "CREATE",
    ] {
        assert!(!upper.contains(forbidden), "{forbidden} in a count query");
    }
}

/// The count's placeholders are dense from $1, like every other statement here.
#[test]
fn the_count_numbers_its_placeholders_densely() {
    let used: std::collections::BTreeSet<usize> = (1..=9)
        .filter(|n| TRIGRAM_COUNT_SQL.contains(&format!("${n}")))
        .collect();
    assert_eq!(used.iter().max(), Some(&3));
    assert_eq!(
        used.len(),
        3,
        "a gap would make Postgres demand a bind nobody supplies"
    );
}

/// ⚑ The count is ONE round trip, not one per probe.
///
/// A loop of N statements would put 31 sequential calls in front of every S-11
/// gather before a single row was read, on a path a human is waiting on. The
/// `unnest` join answers all of them at once.
#[test]
fn the_count_asks_for_every_probe_in_one_statement() {
    assert!(
        TRIGRAM_COUNT_SQL.contains("unnest($1::text[])"),
        "every probe is bound as one array: {TRIGRAM_COUNT_SQL}"
    );
    assert!(
        TRIGRAM_COUNT_SQL.contains("GROUP BY p.probe"),
        "and grouped back per probe"
    );
    assert!(
        TRIGRAM_COUNT_SQL.contains("p.probe AS probe"),
        "the probe is projected, so the caller can pair counts back by name rather \
         than trusting the row order of a GROUP BY"
    );
}
