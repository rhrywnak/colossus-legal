// Tests for `services::matrix_export`.
//
// This model becomes a document somebody files. The tests are weighted towards
// what must NOT reach paper: a removed item, a retracted one, more than the
// stored limit, an invented source, a ✓ nobody earned.

use super::*;

fn words() -> MatrixWording {
    MatrixWording::for_test()
}

/// One evidence row with the fields the export reads and nothing else.
fn line_item(id: &str) -> EvidenceRef {
    EvidenceRef {
        id: id.to_string(),
        verbatim_quote: Some(format!("the words of {id}")),
        page_number: Some(22),
        paragraph: None,
        page_note: None,
        source_document_id: Some("doc-phillips".to_string()),
        source_document_title: Some("Phillips Discovery Response".to_string()),
        statement_type: None,
        evidence_strength: None,
        speaker: None,
        question: None,
        answer: None,
        rank: None,
        role: None,
        confidence: None,
        rank_reason: Some("It is his own sworn answer.".to_string()),
        why: None,
        conflict: false,
        duplicate_of_card_id: None,
        document_date: Some("2016-08-08".to_string()),
        ruling: None,
        ruled_by: None,
        hidden_reason: None,
        rfa_line: None,
        occurrences: 1,
    }
}

fn allegation(supporting: Vec<EvidenceRef>, disputing: Vec<EvidenceRef>) -> AllegationSummary {
    AllegationSummary {
        allegation_id: "allegation-41".to_string(),
        paragraph_number: "41".to_string(),
        summary: Some("They ignored her".to_string()),
        title: None,
        verbatim_quote: Some("The Defendants ignored Plaintiff's arguments…".to_string()),
        source_section: "Common",
        supporting_evidence: supporting,
        disputing_evidence: disputing,
    }
}

fn document(allegations: Vec<AllegationSummary>, limit: usize) -> ExportDocument {
    build_document(
        1,
        "Breach of Fiduciary Duty",
        &[("1.1 Duty".to_string(), allegations)],
        limit,
        "2026-09-06",
        &words(),
    )
}

fn first_supporting(doc: &ExportDocument) -> &ExportList {
    &doc.elements[0].allegations[0].supporting
}

/// The title and footer come from the stored templates, filled.
#[test]
fn the_title_and_footer_are_filled_from_the_stored_templates() {
    let doc = document(vec![allegation(vec![line_item("a")], vec![])], 5);
    assert_eq!(doc.title, "Count 1 — Breach of Fiduciary Duty");
    assert!(doc.footer.contains("2026-09-06"), "{}", doc.footer);
    assert!(!doc.footer.contains("{date}"), "{}", doc.footer);
    assert!(!doc.title.contains("{number}"), "{}", doc.title);
    assert!(!doc.title.contains("{name}"), "{}", doc.title);
}

/// THE RULE THIS FILE EXISTS FOR: a hidden item never prints.
///
/// The page can reveal it behind a toggle. Paper has no toggle, and an item
/// missing from a printed proof summary is invisible — so the filter is asserted
/// here directly, for both hidden reasons.
#[test]
fn a_hidden_item_never_prints() {
    let mut removed = line_item("removed");
    removed.hidden_reason = Some("removed".to_string());
    let mut retracted = line_item("retracted");
    retracted.hidden_reason = Some("does_not_belong".to_string());

    let doc = document(
        vec![allegation(
            vec![removed, line_item("printed"), retracted],
            vec![],
        )],
        5,
    );
    let quotes: Vec<&str> = first_supporting(&doc)
        .lines
        .iter()
        .map(|l| l.quote.as_str())
        .collect();
    assert_eq!(quotes, vec!["the words of printed"]);
}

/// AT MOST `limit` LINES, and the hidden ones are removed BEFORE the count.
///
/// Truncating first would print two lines where the page shows five, with nothing
/// to say the document short-changed the reader.
#[test]
fn the_limit_counts_only_what_is_printable() {
    let mut items: Vec<EvidenceRef> = (0..3)
        .map(|n| {
            let mut hidden = line_item(&format!("hidden-{n}"));
            hidden.hidden_reason = Some("removed".to_string());
            hidden
        })
        .collect();
    items.extend((0..7).map(|n| line_item(&format!("visible-{n}"))));

    let doc = document(vec![allegation(items, vec![])], 5);
    assert_eq!(first_supporting(&doc).lines.len(), 5);
    assert!(first_supporting(&doc)
        .lines
        .iter()
        .all(|l| l.quote.contains("visible")));
}

/// The limit is the STORED number, not a compiled-in five.
#[test]
fn the_limit_is_the_number_it_is_given() {
    let items: Vec<EvidenceRef> = (0..9).map(|n| line_item(&format!("i-{n}"))).collect();
    for limit in [1usize, 3, 9, 20] {
        let doc = document(vec![allegation(items.clone(), vec![])], limit);
        assert_eq!(first_supporting(&doc).lines.len(), limit.min(9));
    }
}

/// An empty leg keeps its heading and prints NO lines — the renderer supplies
/// the stored "no evidence in the corpus" sentence.
///
/// The list is present rather than omitted so the document reports the gap as a
/// finding instead of silently having no section.
#[test]
fn an_empty_leg_keeps_its_heading_and_no_lines() {
    let doc = document(vec![allegation(vec![], vec![])], 5);
    let supporting = first_supporting(&doc);
    assert_eq!(supporting.heading, "Supporting");
    assert!(supporting.lines.is_empty());
    assert_eq!(
        doc.elements[0].allegations[0].disputing.heading,
        "Disputing"
    );
}

/// A Q&A card prints its composed RFA line, not its bare answer.
///
/// "Admitted." in a printed proof summary is an affirmation of nothing.
#[test]
fn a_q_and_a_card_prints_its_composed_line() {
    let mut rfa = line_item("rfa");
    rfa.verbatim_quote = Some("Admitted".to_string());
    rfa.rfa_line = Some("RFA 8 — Admit that you were engaged. — Admitted".to_string());

    let doc = document(vec![allegation(vec![rfa], vec![])], 5);
    assert_eq!(
        first_supporting(&doc).lines[0].quote,
        "RFA 8 — Admit that you were engaged. — Admitted"
    );
}

/// The source line is "title, p. N", and drops the page when there is none.
#[test]
fn the_source_line_names_the_document_and_the_page() {
    let doc = document(vec![allegation(vec![line_item("a")], vec![])], 5);
    assert_eq!(
        first_supporting(&doc).lines[0].source,
        "Phillips Discovery Response, p. 22"
    );

    let mut unpaged = line_item("b");
    unpaged.page_number = None;
    let doc = document(vec![allegation(vec![unpaged], vec![])], 5);
    assert_eq!(
        first_supporting(&doc).lines[0].source,
        "Phillips Discovery Response"
    );
}

/// AN ITEM WITH NO SOURCE DOCUMENT PRINTS NO CITATION — not a placeholder.
///
/// On screen a muted "Source document" is a harmless label. In a filed document
/// it is a citation to a source with that name.
#[test]
fn an_item_with_no_source_document_prints_no_citation() {
    let mut orphan = line_item("orphan");
    orphan.source_document_id = None;
    orphan.source_document_title = None;

    let doc = document(vec![allegation(vec![orphan], vec![])], 5);
    let line = &first_supporting(&doc).lines[0];
    assert_eq!(line.source, "");
    assert_eq!(line.quote, "the words of orphan", "the quote still prints");
}

/// An empty document_date prints as no date, not as an empty field with punctuation.
#[test]
fn an_empty_document_date_prints_as_no_date() {
    let mut blank = line_item("blank");
    blank.document_date = Some("   ".to_string());
    let doc = document(vec![allegation(vec![blank], vec![])], 5);
    assert_eq!(first_supporting(&doc).lines[0].date, "");
}

/// The reason falls back from `rank_reason` to the older `why`, then to nothing.
#[test]
fn the_reason_falls_back_from_rank_reason_to_why() {
    let mut older = line_item("older");
    older.rank_reason = None;
    older.why = Some("Supports the allegation.".to_string());
    let doc = document(vec![allegation(vec![older], vec![])], 5);
    assert_eq!(
        first_supporting(&doc).lines[0].reason,
        "Supports the allegation."
    );

    let mut neither = line_item("neither");
    neither.rank_reason = None;
    neither.why = None;
    let doc = document(vec![allegation(vec![neither], vec![])], 5);
    assert_eq!(first_supporting(&doc).lines[0].reason, "");
}

/// THE ✓ IS EARNED. Only an item a human kept is marked confirmed.
///
/// The footer says "items marked ✓ confirmed by Roman". A tick on an unruled item
/// would make that sentence false in a document going to a hearing.
#[test]
fn only_a_kept_item_is_marked_confirmed() {
    let mut kept = line_item("kept");
    kept.ruling = Some("keep".to_string());
    let unruled = line_item("unruled");

    let doc = document(vec![allegation(vec![kept, unruled], vec![])], 5);
    let lines = &first_supporting(&doc).lines;
    assert!(lines[0].confirmed, "a kept item is confirmed");
    assert!(!lines[1].confirmed, "an unruled item is not");
}

/// The accusation prints the complaint's own words, falling back to its summary.
#[test]
fn the_accusation_prints_the_complaints_words_then_its_summary() {
    let doc = document(vec![allegation(vec![], vec![])], 5);
    assert_eq!(doc.elements[0].allegations[0].paragraph, "¶41");
    assert!(doc.elements[0].allegations[0]
        .text
        .starts_with("The Defendants ignored"));

    let mut quoteless = allegation(vec![], vec![]);
    quoteless.verbatim_quote = None;
    let doc = document(vec![quoteless], 5);
    assert_eq!(doc.elements[0].allegations[0].text, "They ignored her");
}

/// BOTH legs are built, with their own headings and their own limit.
#[test]
fn both_stance_lists_are_built() {
    let doc = document(
        vec![allegation(
            vec![line_item("s1"), line_item("s2")],
            vec![line_item("d1")],
        )],
        5,
    );
    let a = &doc.elements[0].allegations[0];
    assert_eq!(a.supporting.lines.len(), 2);
    assert_eq!(a.disputing.lines.len(), 1);
    assert_eq!(a.disputing.lines[0].quote, "the words of d1");
}

/// An Element with no accusations still prints its heading.
///
/// "This Element has nothing mapped to it" is a finding a reader needs; an
/// Element silently absent from the document is not.
#[test]
fn an_element_with_no_allegations_still_prints_its_heading() {
    let doc = document(vec![], 5);
    assert_eq!(doc.elements.len(), 1);
    assert_eq!(doc.elements[0].heading, "1.1 Duty");
    assert!(doc.elements[0].allegations.is_empty());
}

/// A limit of zero prints no lines rather than panicking — a stored value of 0 is
/// refused by the settings min, but the model must not depend on that.
#[test]
fn a_limit_of_zero_prints_nothing_rather_than_panicking() {
    let doc = document(vec![allegation(vec![line_item("a")], vec![])], 0);
    assert!(first_supporting(&doc).lines.is_empty());
}
