//! Tests for the package's pure half.

use super::*;

fn doc(id: &str, date: Option<NaiveDate>, pages: &[&str]) -> CorpusDocument {
    CorpusDocument {
        id: id.into(),
        title: id.to_uppercase(),
        document_date: date,
        pages: pages
            .iter()
            .enumerate()
            .map(|(i, t)| (i32::try_from(i).unwrap() + 1, (*t).to_string()))
            .collect(),
    }
}

#[test]
fn keys_become_words_and_only_whole_tokens_are_touched() {
    assert_eq!(
        translate_keys("P3 and S2 sit unanswered; R1 is the letter; S1 conflicts."),
        "talking point 3 and what they admitted under oath sit unanswered; the receipt \
         for talking point 1 is the letter; what they said conflicts."
    );
    assert_eq!(
        translate_keys("EP3, P3x, S9 and p3 stay"),
        "EP3, P3x, S9 and p3 stay"
    );
}

#[test]
fn the_key_scanner_finds_survivors_and_nothing_else() {
    assert!(has_internal_key("see P1"));
    assert!(has_internal_key("(S2)"));
    assert!(!has_internal_key("talking point 1, EP3, S9, Plan 2"));
    assert!(!has_internal_key(&translate_keys("P1, P2, R3, S1, S2")));
}

#[test]
fn the_primary_document_leads_and_says_so() {
    let corpus = vec![
        doc("doc-a", None, &["a"]),
        doc("doc-b", NaiveDate::from_ymd_opt(2016, 8, 8), &["b1", "b2"]),
    ];
    let docs = package_documents(&corpus, Some("doc-b"));
    assert_eq!(docs[0].id, "doc-b");
    let ctx = docs[0].block.context.as_deref().unwrap();
    assert!(ctx.contains("Dated August 8, 2016"));
    assert!(ctx.contains("built from this document"));
    assert!(docs[1]
        .block
        .context
        .as_deref()
        .unwrap()
        .contains("Date not recorded"));
    // Without a primary the corpus keeps its order.
    assert_eq!(package_documents(&corpus, None)[0].id, "doc-a");
}

#[test]
fn pages_are_joined_and_mapped_back() {
    let docs = package_documents(&[doc("d", None, &["One.", "Two."])], None);
    let d = &docs[0];
    assert_eq!(d.block.text, "One.\n\nTwo.");
    assert_eq!(d.page_at(0), Some(1));
    assert_eq!(d.page_at(5), Some(1)); // the join belongs to the page before
    assert_eq!(d.page_at(6), Some(2));
}

#[test]
fn the_estimate_is_conservative() {
    assert_eq!(estimate_tokens(&["abcdef", "abc"], 3), 3);
    assert_eq!(
        estimate_tokens(&["abcd"], 0),
        4,
        "a zero divisor is read as 1"
    );
}
