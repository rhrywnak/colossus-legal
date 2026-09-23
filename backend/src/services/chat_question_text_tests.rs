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

/// The regression test for the whole 62% (CC_TASK_CHAT_COST_AUDIT_v1).
///
/// The document blocks are the prompt cache's prefix. If ANY byte of them varies
/// by question, the provider cannot reuse the cache and re-writes ~370k tokens —
/// which is what the deleted `primary`-first sort did. Packaging the same corpus
/// can only ever produce one answer now, so the assertion is simply that the
/// function has no second input left to vary on.
#[test]
fn the_corpus_order_never_depends_on_the_question() {
    let corpus = vec![
        doc("doc-a", None, &["a"]),
        doc("doc-b", NaiveDate::from_ymd_opt(2016, 8, 8), &["b1", "b2"]),
        doc("doc-c", NaiveDate::from_ymd_opt(2011, 4, 11), &["c"]),
    ];
    let first = package_documents(&corpus);
    let second = package_documents(&corpus);
    let ids: Vec<&str> = first.iter().map(|d| d.id.as_str()).collect();
    assert_eq!(ids, ["doc-a", "doc-b", "doc-c"], "the corpus's own order");
    for (a, b) in first.iter().zip(second.iter()) {
        assert_eq!(a.id, b.id);
        assert_eq!(a.block.title, b.block.title);
        assert_eq!(a.block.context, b.block.context);
        assert_eq!(a.block.text, b.block.text);
    }
}

/// The other half of the deleted behaviour: the sentence that used to be appended
/// to one document's `context`. A document block must say nothing about the
/// question being asked — that belongs in the uncached per-question block.
#[test]
fn no_document_context_mentions_the_question() {
    let corpus = vec![
        doc("doc-a", None, &["a"]),
        doc("doc-b", NaiveDate::from_ymd_opt(2016, 8, 8), &["b1", "b2"]),
    ];
    for packaged in package_documents(&corpus) {
        let ctx = packaged.block.context.as_deref().unwrap();
        assert!(
            !ctx.to_lowercase().contains("question"),
            "a document block's context named the question: {ctx}"
        );
        assert!(!ctx.contains("built from this document"), "{ctx}");
    }
}

/// The context line still carries what the `get_document` tool and the model need:
/// the date (or its absence, said plainly), the page count and the id.
#[test]
fn a_document_context_carries_its_date_pages_and_id() {
    let corpus = vec![
        doc("doc-a", None, &["a"]),
        doc("doc-b", NaiveDate::from_ymd_opt(2016, 8, 8), &["b1", "b2"]),
    ];
    let docs = package_documents(&corpus);
    assert_eq!(
        docs[0].block.context.as_deref().unwrap(),
        "Date not recorded. 1 pages. Document id: doc-a."
    );
    assert_eq!(
        docs[1].block.context.as_deref().unwrap(),
        "Dated August 8, 2016. 2 pages. Document id: doc-b."
    );
}

#[test]
fn pages_are_joined_and_mapped_back() {
    let docs = package_documents(&[doc("d", None, &["One.", "Two."])]);
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
