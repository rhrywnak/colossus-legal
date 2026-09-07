// Tests for `services::matrix_export_docx`.
//
// A `.docx` is a ZIP, so these assert two different things at two different
// levels: the two pure helpers by value, and the packed bytes by their structure
// — that the file really is a Word package and that the text reached it.
//
// What is deliberately NOT tested here is which items print: that is
// `matrix_export`'s decision and `matrix_export_tests` proves it against a model
// nobody has to unzip.

use super::*;
use crate::domain::wording_matrix::MatrixWording;
use crate::services::matrix_export::{
    build_document, ExportAllegation, ExportDocument, ExportElement,
};

fn line(quote: &str, confirmed: bool) -> ExportLine {
    ExportLine {
        quote: quote.to_string(),
        source: "Phillips Discovery Response, p. 22".to_string(),
        date: "2016-08-08".to_string(),
        reason: "It is his own sworn answer.".to_string(),
        confirmed,
    }
}

/// A document with one Element, one accusation and the given lines.
fn document(supporting: Vec<ExportLine>, disputing: Vec<ExportLine>) -> ExportDocument {
    let words = MatrixWording::for_test();
    // Built through the production builder so the stored strings — including the
    // empty-line sentence the renderer reads — are the real ones.
    let mut doc = build_document(1, "Breach of Fiduciary Duty", &[], 5, "2026-09-06", &words);
    doc.elements = vec![ExportElement {
        heading: "1.1 Duty".to_string(),
        allegations: vec![ExportAllegation {
            paragraph: "¶41".to_string(),
            text: "The Defendants ignored Plaintiff's arguments…".to_string(),
            supporting: ExportList {
                heading: words.export_supporting_heading.clone(),
                lines: supporting,
            },
            disputing: ExportList {
                heading: words.export_disputing_heading.clone(),
                lines: disputing,
            },
        }],
    }];
    doc
}

/// A kept item carries the ✓ the footer explains; an unruled one does not.
#[test]
fn only_a_confirmed_line_carries_the_tick() {
    let mark = MatrixWording::for_test().export_confirmed_mark;
    assert_eq!(quote_text(&line("Admitted", true), &mark), "✓ Admitted");
    assert_eq!(quote_text(&line("Admitted", false), &mark), "Admitted");
}

/// The mark comes from the STORE, not from a constant — and the SPACE does not.
///
/// The store trims every text value on read, so a mark written as "✓ " arrives as
/// "✓". A renderer that trusted the stored spacing would print "✓Admitted".
///
/// The footer explains the mark — "items marked ✓ confirmed by Roman" — and both
/// halves of that sentence are stored rows. A mark compiled into the renderer
/// would let the footer be re-worded to describe a symbol the document does not
/// print, with nothing failing.
#[test]
fn the_confirmed_mark_is_whatever_the_store_holds() {
    assert_eq!(quote_text(&line("Admitted", true), ">>"), ">> Admitted");
    let doc = document(vec![line("Admitted", true)], vec![]);
    assert_eq!(
        doc.confirmed_mark(),
        MatrixWording::for_test().export_confirmed_mark
    );
    // And the footer, which explains it, is the same snapshot's.
    assert!(doc.footer.contains("✓"), "{}", doc.footer);
}

/// The citation joins only the halves that exist.
#[test]
fn the_citation_joins_only_the_parts_that_exist() {
    let both = line("q", false);
    assert_eq!(
        citation_text(&both),
        "Phillips Discovery Response, p. 22 · 2016-08-08"
    );

    let mut undated = line("q", false);
    undated.date = String::new();
    assert_eq!(
        citation_text(&undated),
        "Phillips Discovery Response, p. 22"
    );

    let mut uncited = line("q", false);
    uncited.source = String::new();
    assert_eq!(citation_text(&uncited), "2016-08-08");

    let mut neither = line("q", false);
    neither.source = String::new();
    neither.date = String::new();
    assert_eq!(
        citation_text(&neither),
        "",
        "an item with no source and no date prints no citation line at all"
    );
}

/// The rendered bytes ARE a ZIP — a Word package, not a text file with a `.docx`
/// name.
///
/// `PK\x03\x04` is the ZIP local-file-header magic. A document Word refuses to
/// open is the failure mode this whole module exists to avoid, and it is not
/// something anyone would notice from a green build.
#[test]
fn the_rendered_document_is_a_zip_package() {
    let bytes =
        render(&document(vec![line("Admitted", true)], vec![])).expect("the document packs");
    assert!(
        bytes.len() > 1_000,
        "a real package is not a handful of bytes"
    );
    assert_eq!(&bytes[..4], b"PK\x03\x04", "not a ZIP local file header");
}

/// The package carries the OOXML parts Word looks for.
///
/// Asserted by scanning the ZIP's central directory for the file names, which is
/// enough to catch a build that produced a valid but EMPTY archive.
#[test]
fn the_package_carries_the_word_document_parts() {
    let bytes =
        render(&document(vec![line("Admitted", false)], vec![])).expect("the document packs");
    let raw = String::from_utf8_lossy(&bytes);
    for part in ["[Content_Types].xml", "word/document.xml"] {
        assert!(raw.contains(part), "the package is missing {part}");
    }
}

/// An empty leg reaches the document as the stored sentence, not as blank space.
///
/// The bytes are deflated, so this asserts the sentence the renderer was GIVEN
/// rather than searching the compressed stream — the packing itself is covered by
/// the two tests above.
#[test]
fn an_empty_leg_is_rendered_from_the_stored_sentence() {
    let doc = document(vec![], vec![]);
    assert_eq!(doc.empty_line(), "No evidence in the corpus.");
    render(&doc).expect("a document whose lists are empty still packs");
}

/// A document with no Elements at all still produces a valid file.
///
/// A Count whose Elements have not been loaded is a real state, and handing the
/// reader a corrupt download would say nothing about it.
#[test]
fn an_empty_count_still_produces_a_valid_document() {
    let words = MatrixWording::for_test();
    let doc = build_document(4, "Abuse of Process", &[], 5, "2026-09-06", &words);
    let bytes = render(&doc).expect("an empty Count still packs");
    assert_eq!(&bytes[..4], b"PK\x03\x04");
}

/// Both stance lists reach the renderer.
#[test]
fn both_stance_lists_are_rendered() {
    let doc = document(vec![line("supports", false)], vec![line("disputes", false)]);
    assert_eq!(doc.elements[0].allegations[0].supporting.lines.len(), 1);
    assert_eq!(doc.elements[0].allegations[0].disputing.lines.len(), 1);
    render(&doc).expect("both legs pack");
}
