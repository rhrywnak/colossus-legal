//! Tests for `buckets`, in their own file so the module stays inside
//! Rule 17's 300-line ceiling.

use super::*;

fn card(id: &str, quote: &str, doc: &str, page: Option<i64>) -> Card {
    Card {
        id: id.to_string(),
        source_document: doc.to_string(),
        page_number: page,
        quote: quote.to_string(),
        title: "t".to_string(),
        question: None,
        statement_type: None,
        grounding_status: "exact".to_string(),
        party_count: 1,
        unnamed_party_count: 0,
        allegation_count: 1,
        document_node_count: 1,
        doc_page_count: Some(10),
        doc_row_exists: true,
        template_name: Some("v5_5".to_string()),
        model_name: Some("m".to_string()),
    }
}

fn rules<'a>(tokens: &'a [String], dropped: &'a [String]) -> Rules<'a> {
    Rules {
        answer_tokens: tokens,
        dropped_statement_types: dropped,
        near_duplicate_min_ratio: 0.5,
        mirror_ok_ids: None,
    }
}

#[test]
fn b1_flags_blank_and_answer_tokens_only() {
    let tokens = vec!["Admitted.".to_string()];
    let dropped: Vec<String> = vec![];
    let cards = vec![
        card("a", "   ", "d", Some(1)),
        card("b", "Admitted.", "d", Some(1)),
        card("c", "The court ordered the money returned.", "d", Some(1)),
    ];
    let (flags, _) = assign(&cards, &rules(&tokens, &dropped));
    assert!(flags[0].0[0] && flags[1].0[0] && !flags[2].0[0]);
}

#[test]
fn b2_groups_only_repeated_quotes_and_ignores_case_and_spacing() {
    let tokens: Vec<String> = vec![];
    let dropped: Vec<String> = vec![];
    let cards = vec![
        card("a", "The Court  ordered it", "d1", Some(1)),
        card("b", "the court ordered it", "d2", Some(3)),
        card("c", "something else entirely", "d1", Some(2)),
    ];
    let (flags, dup) = assign(&cards, &rules(&tokens, &dropped));
    assert!(flags[0].0[1] && flags[1].0[1] && !flags[2].0[1]);
    assert_eq!(dup.clusters.len(), 1);
    assert_eq!(dup.card_count(), 2);
    // Different documents -> a cross-reference, not a twin.
    assert_eq!(dup.twin_split(&cards), (0, 2));
}

#[test]
fn b2_twin_split_counts_same_document_same_page_as_twins() {
    let cards = vec![
        card("a", "identical text here", "d1", Some(4)),
        card("b", "identical text here", "d1", Some(4)),
    ];
    let dup = DuplicateIndex::build(&cards);
    assert_eq!(dup.twin_split(&cards), (2, 0));
}

#[test]
fn b5_flags_no_party_unnamed_party_and_more_than_four() {
    let tokens: Vec<String> = vec![];
    let dropped: Vec<String> = vec![];
    let mut none = card("a", "text one", "d", Some(1));
    none.party_count = 0;
    let mut unnamed = card("b", "text two", "d", Some(1));
    unnamed.unnamed_party_count = 1;
    let mut many = card("c", "text three", "d", Some(1));
    many.party_count = 5;
    let ok = card("d", "text four", "d", Some(1));
    let cards = vec![none, unnamed, many, ok];
    let (flags, _) = assign(&cards, &rules(&tokens, &dropped));
    assert!(flags[0].0[4] && flags[1].0[4] && flags[2].0[4] && !flags[3].0[4]);
}

#[test]
fn b7_uses_the_stored_dropped_list_case_insensitively() {
    let tokens: Vec<String> = vec![];
    let dropped = vec!["referral".to_string()];
    let mut referral = card("a", "See the prior response.", "d", Some(1));
    referral.statement_type = Some("Referral".to_string());
    let mut other = card("b", "A real assertion.", "d", Some(1));
    other.statement_type = Some("assertion".to_string());
    let cards = vec![referral, other];
    let (flags, _) = assign(&cards, &rules(&tokens, &dropped));
    assert!(flags[0].0[6] && !flags[1].0[6]);
}

#[test]
fn clean_card_lands_in_no_bucket() {
    let tokens: Vec<String> = vec![];
    let dropped: Vec<String> = vec![];
    let cards = vec![card(
        "a",
        "A perfectly ordinary sentence of evidence.",
        "d",
        Some(2),
    )];
    let (flags, _) = assign(&cards, &rules(&tokens, &dropped));
    assert!(flags[0].clean(), "flags were {:?}", flags[0].0);
}

#[test]
fn overlap_matrix_diagonal_is_the_bucket_count() {
    let mut a = Flags::default();
    a.0[0] = true;
    a.0[3] = true;
    let mut b = Flags::default();
    b.0[0] = true;
    let matrix = overlap_matrix(&[a, b]);
    assert_eq!(matrix[0][0], 2);
    assert_eq!(matrix[3][3], 1);
    assert_eq!(matrix[0][3], 1);
    assert_eq!(matrix[3][0], 1);
}
/// **B3, the true-positive path.** `norm::is_near_duplicate` is tested in
/// its own module; this exercises the wrapper around it.
///
/// ## Why the wrapper needs its own test
///
/// `near_duplicate_flags` does not compare every pair — it buckets cards by
/// their first and last word first, and only compares within a bucket. That
/// optimisation is where a B3 count can silently go to zero: a predicate
/// that is perfectly correct, called on pairs that never meet. Before this
/// test nothing anywhere asserted `flags[i].0[2]` was ever `true`.
#[test]
fn b3_fires_when_one_quote_contains_another_end_to_end() {
    let cards = vec![
        card(
            "a",
            "the court ordered the money returned",
            "doc-x",
            Some(1),
        ),
        card(
            "b",
            "the court ordered the money returned to the estate",
            "doc-x",
            Some(2),
        ),
    ];
    let tokens: Vec<String> = Vec::new();
    let dropped: Vec<String> = Vec::new();
    let (flags, _) = assign(&cards, &rules(&tokens, &dropped));
    assert!(flags[0].0[2], "the shorter card must be flagged B3");
    assert!(flags[1].0[2], "the longer card must be flagged B3 too");
}

/// **B6** — a card with no allegation edge. Coverage, not damage, but the
/// flag still has to fire.
#[test]
fn b6_fires_when_a_card_reaches_no_allegation() {
    let mut c = card("a", "a real finding", "doc-x", Some(1));
    c.allegation_count = 0;
    let tokens: Vec<String> = Vec::new();
    let dropped: Vec<String> = Vec::new();
    let (flags, _) = assign(&[c], &rules(&tokens, &dropped));
    assert!(flags[0].0[5]);
}

/// **B10** — the orphan test, both halves. A card whose `source_document`
/// matches no `documents` row, and a card with no `CONTAINED_IN` edge, are
/// each orphaned in a different store; the bucket covers both.
#[test]
fn b10_fires_for_a_missing_document_row_and_for_a_missing_document_node() {
    let tokens: Vec<String> = Vec::new();
    let dropped: Vec<String> = Vec::new();

    let mut no_row = card("a", "a real finding", "doc-gone", Some(1));
    no_row.doc_row_exists = false;
    let (flags, _) = assign(&[no_row], &rules(&tokens, &dropped));
    assert!(flags[0].0[9], "no documents row must flag B10");

    let mut no_node = card("b", "a real finding", "doc-x", Some(1));
    no_node.document_node_count = 0;
    let (flags, _) = assign(&[no_node], &rules(&tokens, &dropped));
    assert!(flags[0].0[9], "no CONTAINED_IN edge must flag B10 as well");
}

/// **B11** — provenance missing. Either half alone is enough: a card whose
/// extraction run recorded no template, or none that recorded no model,
/// cannot be traced back to what produced it.
#[test]
fn b11_fires_when_either_half_of_the_provenance_is_absent() {
    let tokens: Vec<String> = Vec::new();
    let dropped: Vec<String> = Vec::new();

    let mut no_template = card("a", "a real finding", "doc-x", Some(1));
    no_template.template_name = None;
    let (flags, _) = assign(&[no_template], &rules(&tokens, &dropped));
    assert!(flags[0].0[10], "a missing template must flag B11");

    let mut no_model = card("b", "a real finding", "doc-x", Some(1));
    no_model.model_name = None;
    let (flags, _) = assign(&[no_model], &rules(&tokens, &dropped));
    assert!(flags[0].0[10], "a missing model must flag B11");
}

/// **B12** — the mirror bucket, and the distinction that makes it safe.
///
/// `mirror_ok_ids: None` means the `evidence_search` table does not exist on
/// this database, and then NO card may be flagged — an absent table is not
/// 1,209 defects. Only when the table exists (`Some`) does a card missing
/// from it become a finding. Both halves are asserted, because getting this
/// backwards would have condemned the whole corpus on the 09-03 run, where
/// the table genuinely did not exist.
#[test]
fn b12_fires_only_when_the_mirror_table_exists() {
    let tokens: Vec<String> = Vec::new();
    let dropped: Vec<String> = Vec::new();
    let c = card("a", "a real finding", "doc-x", Some(1));

    let (flags, _) = assign(std::slice::from_ref(&c), &rules(&tokens, &dropped));
    assert!(
        !flags[0].0[11],
        "no mirror table means no B12 — an absent table is not a corpus of defects"
    );

    let present: std::collections::HashSet<String> = std::collections::HashSet::new();
    let with_mirror = Rules {
        answer_tokens: &tokens,
        dropped_statement_types: &dropped,
        near_duplicate_min_ratio: 0.5,
        mirror_ok_ids: Some(&present),
    };
    let (flags, _) = assign(std::slice::from_ref(&c), &with_mirror);
    assert!(
        flags[0].0[11],
        "the table exists and this id is not in it — that IS a finding"
    );
}
