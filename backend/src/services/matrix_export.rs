// =============================================================================
// backend/src/services/matrix_export.rs — one Count as a printable document
// =============================================================================
//
// PROOF_MATRIX_v2 §4, minus the file format. This turns the Count's Elements and
// their ordered evidence into a MODEL of the document — headings, paragraphs and
// lines — and stops there. `matrix_export_docx` turns that model into a `.docx`.
//
// ## Why the model and the format are two modules
//
// Every decision worth arguing about is in this file: what prints, in what order,
// how many, and what a line says. None of them needs a Word document to test, and
// a test that had to unzip an OOXML package to assert "hidden items never print"
// would be a test nobody trusts. The other module knows about fonts.
//
// ## The one rule this file exists to enforce
//
// HIDDEN ITEMS NEVER PRINT. The page can reveal them behind a toggle; the
// document cannot, because paper has no toggle. An item a human removed, or one
// the machine retracted, is absent from the export — and because absence in a
// printed proof summary is invisible, that rule is asserted directly rather than
// left to a filter someone might reorder.

use crate::domain::wording_matrix::MatrixWording;
use crate::repositories::element_detail_repository::{AllegationSummary, EvidenceRef};

/// A whole exported Count.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportDocument {
    /// "Count 1 — Breach of Fiduciary Duty", from the stored title template.
    pub title: String,
    pub elements: Vec<ExportElement>,
    /// The generated-on line, from the stored footer template.
    pub footer: String,
    /// The stored sentence printed under an empty heading. See
    /// [`ExportDocument::empty_line`].
    empty_line: String,
    /// The stored mark printed in front of a confirmed item. See
    /// [`ExportDocument::confirmed_mark`].
    confirmed_mark: String,
}

/// One Element of the Count: a heading and the accusations under it.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportElement {
    pub heading: String,
    pub allegations: Vec<ExportAllegation>,
}

/// One accusation: the complaint's own words, then the two stance lists.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportAllegation {
    /// "¶41".
    pub paragraph: String,
    /// The complaint's `verbatim_quote`, or its summary when the quote is
    /// missing. An accusation printed with NEITHER would be a heading over
    /// evidence for something the document never states.
    pub text: String,
    /// Heading + lines. Present even when empty — the empty case prints the
    /// stored "no evidence" sentence, because a bare heading reads as a
    /// formatting fault rather than as a finding.
    pub supporting: ExportList,
    pub disputing: ExportList,
}

/// One stance list under an accusation.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportList {
    pub heading: String,
    /// Empty means "nothing in the corpus", which the renderer prints as
    /// [`ExportDocument`]'s stored empty sentence rather than as blank space.
    pub lines: Vec<ExportLine>,
}

/// One evidence line: the words, then where they came from, then why.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportLine {
    /// The quote, or the composed RFA form when the item is a Q&A card.
    pub quote: String,
    /// "Phillips Discovery Response, p. 22" — the document and the page.
    pub source: String,
    /// The document's date, or empty when it has none. Kept separate from
    /// `source` so an undated document prints as a citation with no date rather
    /// than as a citation with a dangling comma.
    pub date: String,
    /// The pass's `rank_reason`, or its older `why`, or empty.
    pub reason: String,
    /// Whether a human confirmed this item — the ✓ the footer explains.
    pub confirmed: bool,
}

impl ExportDocument {
    /// The sentence printed under a heading with nothing beneath it.
    ///
    /// Carried on the document rather than on each list because it is the same
    /// sentence everywhere and because the renderer, which has no wording handle
    /// of its own, needs it — the alternative was threading `MatrixWording`
    /// through the format layer, which would give the file-format module a reason
    /// to know about the settings store.
    pub fn empty_line(&self) -> String {
        self.empty_line.clone()
    }

    /// The mark printed in front of an item a human confirmed.
    ///
    /// Carried here for the same reason as [`ExportDocument::empty_line`], and
    /// for one more: the FOOTER names this mark — "items marked ✓ confirmed by
    /// Roman" — and both halves of that sentence are stored rows. A mark compiled
    /// into the renderer while its explanation lives in the settings store is two
    /// halves free to drift apart, and the drift would print a document whose
    /// footer describes a symbol that is not on the page.
    pub fn confirmed_mark(&self) -> String {
        self.confirmed_mark.clone()
    }
}

/// Build the model for one Count.
///
/// `elements` is `(heading, allegations)` per Element, already in the order the
/// document should print them and already ordered/ruled by
/// [`crate::services::matrix_detail::apply_rulings_and_order`]. `limit` is the
/// stored `matrix_visible_items` — the SAME number the page shows before "N
/// more", so the document is the top of the list the reader was looking at.
pub fn build_document(
    count_number: i64,
    count_name: &str,
    elements: &[(String, Vec<AllegationSummary>)],
    limit: usize,
    generated_on: &str,
    wording: &MatrixWording,
) -> ExportDocument {
    ExportDocument {
        title: wording
            .export_title_template
            .replace("{number}", &count_number.to_string())
            .replace("{name}", count_name),
        elements: elements
            .iter()
            .map(|(heading, allegations)| ExportElement {
                heading: heading.clone(),
                allegations: allegations
                    .iter()
                    .map(|a| build_allegation(a, limit, wording))
                    .collect(),
            })
            .collect(),
        footer: wording
            .export_footer_template
            .replace("{date}", generated_on),
        empty_line: wording.export_empty_line.clone(),
        confirmed_mark: wording.export_confirmed_mark.clone(),
    }
}

/// One accusation and both of its stance lists.
fn build_allegation(
    allegation: &AllegationSummary,
    limit: usize,
    wording: &MatrixWording,
) -> ExportAllegation {
    ExportAllegation {
        paragraph: format!("¶{}", allegation.paragraph_number),
        // The complaint's own words first; its summary only if the quote is
        // missing; and an explicit blank rather than a panic if both are.
        text: allegation
            .verbatim_quote
            .clone()
            .or_else(|| allegation.summary.clone())
            .unwrap_or_default(),
        supporting: build_list(
            &allegation.supporting_evidence,
            limit,
            &wording.export_supporting_heading,
        ),
        disputing: build_list(
            &allegation.disputing_evidence,
            limit,
            &wording.export_disputing_heading,
        ),
    }
}

/// The top `limit` VISIBLE items of one leg, as printable lines.
///
/// ## Why the filter comes before the truncation
///
/// Taking five and then dropping the hidden ones would print two lines where the
/// page shows five, silently, with the reader having no way to tell the document
/// short-changed them. Hidden items are removed first; the count is of what
/// remains.
fn build_list(leg: &[EvidenceRef], limit: usize, heading: &str) -> ExportList {
    ExportList {
        heading: heading.to_string(),
        lines: leg
            .iter()
            .filter(|item| item.hidden_reason.is_none())
            .take(limit)
            .map(build_line)
            .collect(),
    }
}

/// One evidence line.
fn build_line(item: &EvidenceRef) -> ExportLine {
    ExportLine {
        // The RFA form when there is one: a bare "Admitted." in a printed proof
        // summary is an affirmation of nothing.
        quote: item
            .rfa_line
            .clone()
            .or_else(|| item.verbatim_quote.clone())
            .unwrap_or_default(),
        source: source_line(item),
        date: item
            .document_date
            .clone()
            .filter(|d| !d.trim().is_empty())
            .unwrap_or_default(),
        // `rank_reason` is the linking pass's; `why` is the older pass's. A line
        // with neither prints no reason rather than an empty dash — §4 asks for
        // "one-line reason", and there are 36 edges that have none.
        reason: item
            .rank_reason
            .clone()
            .or_else(|| item.why.clone())
            .unwrap_or_default(),
        confirmed: item.ruling.as_deref() == Some("keep"),
    }
}

/// "Document title, p. 22" — or the title alone when there is no page, or an
/// empty string when there is no document.
///
/// ## Domain note: no invented placeholder
///
/// An earlier surface printed "Source document" for an item with no
/// `CONTAINED_IN` edge. On screen that is a harmless muted label; in a document
/// somebody files, it is a citation to a source with that name. So a missing
/// document prints NOTHING, and the quote stands uncited — which is visibly
/// incomplete, and therefore fixable.
fn source_line(item: &EvidenceRef) -> String {
    let Some(title) = item
        .source_document_title
        .as_deref()
        .filter(|t| !t.trim().is_empty())
    else {
        return String::new();
    };
    match item.page_number {
        Some(page) => format!("{title}, p. {page}"),
        None => title.to_string(),
    }
}

#[cfg(test)]
#[path = "matrix_export_tests.rs"]
mod tests;
