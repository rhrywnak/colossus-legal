// =============================================================================
// backend/src/services/matrix_export_docx.rs — the model as a Word document
// =============================================================================
//
// PROOF_MATRIX_v2 §4's file format, and nothing else. Every decision about WHAT
// prints is in `matrix_export`; this file knows about fonts, bold and page order.
//
// ## Why `docx-rs` and not a hand-rolled OOXML package
//
// A `.docx` is a ZIP of related XML parts with a content-types manifest and a
// relationship graph. Writing that by hand is a week of work to get to the point
// where Word opens the file at all, and the failure mode is a document that will
// not open in front of Chuck. `docx-rs` 0.4.22 is pinned exactly (Rule 23's
// argument, applied to a crate) and pulled with `default-features = false` so the
// image decoder it can carry stays out of this build.
//
// ## Body text Calibri 11, headings bold (§4)
//
// `docx-rs` measures size in HALF-POINTS, so 11pt is `.size(22)`. Stated here
// once, as a named constant, rather than as a 22 sprinkled through the builder.

use docx_rs::{Docx, Paragraph, Run, RunFonts};

use super::matrix_export::{ExportDocument, ExportLine, ExportList};

// STRUCTURAL: the typography §4 specifies, and a property of the FILE FORMAT
// rather than of this deployment. `docx-rs` measures size in half-points, so 11pt
// is 22 — stated once here rather than as a bare 22 in the builder. Changing
// either is changing what the instruction asked the document to look like, which
// is a code change with a review, not a settings row a reader can turn.
//
// The confirmed MARK is deliberately NOT here: it is a user-visible character
// whose meaning is spelled out in the stored footer, so it is a stored row too
// and arrives on the document. See `ExportDocument::confirmed_mark`.
const BODY_FONT: &str = "Calibri";
const BODY_SIZE: usize = 22;

/// Render one [`ExportDocument`] to `.docx` bytes.
///
/// ## Rust Learning: `Vec<u8>` as the return, not a file path
///
/// The handler streams these bytes straight into the HTTP response. Writing a
/// temp file would put a path, a cleanup and a failure mode between the document
/// and the reader — and would leave proof material on a disk nobody is watching.
///
/// # Errors
/// Returns the packing error `docx-rs` raises if the ZIP cannot be written. There
/// is no partial success: either the whole document is built or none of it is.
pub fn render(document: &ExportDocument) -> Result<Vec<u8>, docx_rs::DocxError> {
    // Read once, for the whole document: both are stored strings, and a document
    // whose first accusation used one snapshot and its last another would be a
    // document that contradicts its own footer.
    let marks = Marks {
        empty_line: document.empty_line(),
        confirmed: document.confirmed_mark(),
    };
    let mut docx = Docx::new().add_paragraph(heading(&document.title, 32));

    for element in &document.elements {
        docx = docx.add_paragraph(heading(&element.heading, 26));
        for allegation in &element.allegations {
            docx = docx.add_paragraph(heading(
                &format!("{} {}", allegation.paragraph, allegation.text),
                BODY_SIZE,
            ));
            docx = add_list(docx, &allegation.supporting, &marks);
            docx = add_list(docx, &allegation.disputing, &marks);
        }
    }

    docx = docx.add_paragraph(body(&document.footer));

    // ## Rust Learning: `Cursor<Vec<u8>>` — an in-memory file
    //
    // `pack` writes a ZIP, which needs `Write + Seek`: a ZIP records each entry's
    // offset in a directory at the END, so the writer has to go back. A bare
    // `Vec<u8>` implements `Write` but not `Seek`. `std::io::Cursor` wraps the
    // vector with a position, supplying `Seek` over memory — the whole document
    // is built in RAM and never touches a disk.
    let mut buffer = std::io::Cursor::new(Vec::new());
    docx.build().pack(&mut buffer)?;
    Ok(buffer.into_inner())
}

/// One stance list: its bold heading, then a line per item — or the stored
/// "nothing in the corpus" sentence when there are none.
///
/// ## Why the empty case prints a sentence rather than nothing
///
/// A heading with nothing under it reads as a formatting fault, and a reader who
/// thinks the document is broken does not learn the thing it is telling them: no
/// evidence in this corpus bears on this accusation from this side. That gap is
/// among the most useful findings on the page, so it is printed as a finding.
fn add_list(docx: Docx, list: &ExportList, marks: &Marks) -> Docx {
    let mut docx = docx.add_paragraph(heading(&list.heading, BODY_SIZE));
    if list.lines.is_empty() {
        return docx.add_paragraph(body(&marks.empty_line));
    }
    for line in &list.lines {
        docx = docx.add_paragraph(body(&quote_text(line, &marks.confirmed)));
        let citation = citation_text(line);
        if !citation.is_empty() {
            docx = docx.add_paragraph(body(&citation));
        }
        if !line.reason.is_empty() {
            docx = docx.add_paragraph(body(&line.reason));
        }
    }
    docx
}

/// The two stored strings the renderer needs but the model's lines do not carry.
///
/// Bundled rather than passed as two `&str`s: both are stored sentences about
/// this document, they are read together on every list, and two adjacent `&str`
/// parameters is where a call site swaps them.
struct Marks {
    /// Printed under a heading with nothing beneath it.
    empty_line: String,
    /// Printed in front of an item a human confirmed.
    confirmed: String,
}

/// The quote, with the stored confirmed mark in front of it when a human kept
/// the item.
///
/// The mark is passed in rather than compiled in because the stored FOOTER
/// explains it. Editing one and not the other would print a document whose
/// footer describes a symbol that is not on the page.
pub(crate) fn quote_text(line: &ExportLine, confirmed_mark: &str) -> String {
    if line.confirmed {
        // The space is supplied HERE, not stored: the settings store trims every
        // text value on read, so a mark written as "✓ " arrives as "✓" and a
        // renderer that trusted the stored spacing would print "✓Admitted".
        format!("{confirmed_mark} {}", line.quote)
    } else {
        line.quote.clone()
    }
}

/// "Phillips Discovery Response, p. 22 · 2016-08-08" — whichever halves exist.
///
/// Built by joining only the non-empty parts, so an undated document prints a
/// citation with no date rather than one with a dangling separator, and an item
/// with neither prints no citation line at all.
pub(crate) fn citation_text(line: &ExportLine) -> String {
    [line.source.as_str(), line.date.as_str()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" · ")
}

/// A bold paragraph at the given half-point size.
fn heading(text: &str, size: usize) -> Paragraph {
    Paragraph::new().add_run(
        Run::new()
            .add_text(text)
            .bold()
            .size(size)
            .fonts(RunFonts::new().ascii(BODY_FONT)),
    )
}

/// A body paragraph: Calibri 11, not bold.
fn body(text: &str) -> Paragraph {
    Paragraph::new().add_run(
        Run::new()
            .add_text(text)
            .size(BODY_SIZE)
            .fonts(RunFonts::new().ascii(BODY_FONT)),
    )
}

#[cfg(test)]
#[path = "matrix_export_docx_tests.rs"]
mod tests;
