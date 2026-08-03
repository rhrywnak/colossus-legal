//! Quote-in-context assembly for the §7 candidate card (task 1.7C, defect D6).
//!
//! ## What this module is for
//!
//! §7.1 says a card shows the page text flanking its quote, so a ruling is made
//! on a sentence rather than a fragment. Until 1.7C that flank was a hard
//! character window — the last 240 characters before and the first 240 after —
//! and a fixed cut lands wherever it lands. Measured on DEV, a real card read:
//!
//! ```text
//! "lle Hanley--Hanley is not here, but\n13\nOkay?\nher son James is present.\n14\n…"
//! ```
//!
//! Two defects in one string: it begins mid-word (`"lle Hanley"` is the tail of
//! `"And Camille Hanley"`), and it renders the court reporter's margin numerals.
//!
//! ## The law this module implements (design v2 §3, RULED 2026-08-02)
//!
//! | # | Rule | Where |
//! |---|---|---|
//! | 1 | Sentence boundaries decide; the char window only seeds | [`assemble`], expanding outward from `window` |
//! | 2 | Sentence detection must survive legal abbreviations | [`crate::domain::sentence_bounds`] |
//! | 3 | Expansion is bounded — **by the page** (ruling R3) | the slice source is one `document_text` row; no new parameter |
//! | 4 | Gutter numerals stripped from DISPLAY only | [`strip_gutter_line_numbers`], applied to the two flanks after slicing |
//! | 5 | The anchor quote stays verbatim | this module never touches it — it slices only OUTSIDE the located span |
//! | 6 | Context comes from ORIGINAL page bytes via the 1.7A index-mapped normalizer | [`locate`] |
//!
//! ## Why the ends can be honestly incomplete
//!
//! Measured over all 499 locatable corpus cards: backward expansion reaches the
//! page start with no sentence terminator in **154** of them (30.9%), and forward
//! expansion reaches the page end in **65** (13.0%). A page routinely begins and
//! ends mid-sentence, because the sentence continues on the page next door.
//!
//! Standing Rule 1 therefore forbids implying completeness the data does not
//! have. Each flank carries a `complete` flag AND a composed notice: the flag is
//! the state, the notice is the words. That pairing is the established shape in
//! this DTO family — `CardGrounding { state, label }` does exactly the same thing
//! for exactly the same reason (a machine token the client branches on, beside a
//! sentence the backend composed, because the language law puts every display
//! decision on this side of the wire).

use crate::domain::quote_match::{locate, strip_gutter_line_numbers};
use crate::domain::sentence_bounds::{
    ends_with_sentence_end, sentence_end_after, sentence_start_before,
};

/// The words shown where a context flank stops at the top of the page rather
/// than at a sentence start.
///
// CONST: display copy ratified by Roman 2026-08-03 (ruling R3), composed
// server-side per the language law. Not deployment-varying and not a tunable —
// it is a sentence about what the data is, in the product's own voice.
const PAGE_BEGINS_NOTICE: &str = "— page begins here";

/// The words shown where a context flank stops at the bottom of the page.
///
// CONST: see [`PAGE_BEGINS_NOTICE`].
const PAGE_ENDS_NOTICE: &str = "— page ends here";

/// One flank of the context, with the honest truth about how it terminated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextFlank {
    /// The page text, gutter numerals stripped for display. Empty when the page
    /// text is unavailable or the quote could not be located on it.
    pub text: String,
    /// Whether this flank terminated at a real sentence boundary.
    pub complete: bool,
    /// The composed page-edge notice, present exactly when `complete` is false
    /// AND there was a page to run out of. `None` when the flank is complete, and
    /// also `None` when there is no context at all — an absent context needs no
    /// explanation about page edges, and the card's `grounding` row already says
    /// why it is absent.
    pub notice: Option<String>,
}

impl ContextFlank {
    /// The no-context case: no page text stored, or the quote is not on this page.
    ///
    /// `complete` is `true` deliberately. The flag answers "did this text stop
    /// mid-sentence?", and an empty flank did not stop anywhere — marking it
    /// incomplete would put a page-edge notice on a card that has no context to
    /// explain, which is a different lie from the one this module removes.
    fn absent() -> Self {
        ContextFlank {
            text: String::new(),
            complete: true,
            notice: None,
        }
    }
}

/// Both flanks of one card's quote-in-context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QuoteContext {
    pub before: ContextFlank,
    pub after: ContextFlank,
}

impl QuoteContext {
    /// Both flanks absent — see [`ContextFlank::absent`].
    fn absent() -> Self {
        QuoteContext {
            before: ContextFlank::absent(),
            after: ContextFlank::absent(),
        }
    }
}

/// Assemble the quote-in-context flanks for one card. Pure — no I/O.
///
/// `page_text` is the ORIGINAL stored page (`document_text.text_content`), not a
/// normalized copy. `window` is `Settings::quote_context_window_chars` — the
/// SEED width, per design §3.1; expansion runs outward from it.
///
/// Returns [`QuoteContext::absent`] when the page text is unavailable or the
/// quote cannot be located on the page it cites. The latter is a real, measured
/// state — 26 of the corpus's 525 page-bearing quotes (5.0%), the cross-page pair
/// case (grounding records the LEFT page and this loads that page alone) plus the
/// OCR line-transposition case. An honest empty box, never a window drawn around
/// the nearest similar words, which would read as evidence.
///
/// ## Rust Learning: why the seed is measured in CHARACTERS and sliced in BYTES
///
/// `window` is a count of characters, because that is what a human means by "240
/// characters of context". Rust strings are UTF-8, so a byte offset `window`
/// bytes away may land inside a multi-byte character and slicing there panics.
/// `char_indices()` walks real character boundaries, so counting characters and
/// taking the boundary they sit on gives a seed offset that is safe to slice —
/// which matters here because OCR'd legal PDFs are full of curly quotes, em
/// dashes and accented names.
pub(crate) fn assemble(page_text: Option<&str>, quote: &str, window: usize) -> QuoteContext {
    let Some(page) = page_text else {
        return QuoteContext::absent();
    };
    let Some(span) = locate(page, quote) else {
        return QuoteContext::absent();
    };

    QuoteContext {
        before: flank_before(page, span.start, window),
        after: flank_after(page, span.end, window),
    }
}

/// The flank before the quote: seed back `window` characters, then expand
/// BACKWARD to the start of the sentence that seed lands in.
///
/// Note the expansion direction. The seed is a floor, not a ceiling: the returned
/// text is always at least as long as the seed, because a sentence start found
/// before the seed offset moves the slice's beginning further left. That is what
/// "expanding OUTWARD from the configured char window" means in §3.1 — the window
/// is a guide, and the sentence is the law.
fn flank_before(page: &str, quote_start: usize, window: usize) -> ContextFlank {
    // Step back `window` characters, stopping at the page start.
    let seed = page[..quote_start]
        .char_indices()
        .rev()
        .nth(window.saturating_sub(1))
        .map(|(i, _)| i)
        .unwrap_or(0);

    match sentence_start_before(page, seed) {
        Some(start) => complete_flank(&page[start..quote_start]),
        // No sentence terminator anywhere before the seed: this page begins
        // mid-sentence. Show everything there is and say so.
        None => incomplete_flank(&page[..quote_start], PAGE_BEGINS_NOTICE),
    }
}

/// The flank after the quote: seed forward `window` characters, then expand
/// FORWARD to the end of the sentence that seed lands in (terminator included).
fn flank_after(page: &str, quote_end: usize, window: usize) -> ContextFlank {
    let tail = &page[quote_end..];
    let seed = tail
        .char_indices()
        .nth(window)
        .map(|(i, _)| quote_end + i)
        .unwrap_or(page.len());

    match sentence_end_after(page, seed) {
        Some(end) => complete_flank(&page[quote_end..end]),
        // Nothing terminates at or after the seed. Two different situations, and
        // they must not be collapsed: the seed may have OVERSHOT a tail that
        // already ends in a full stop (a short page), or the page may genuinely
        // stop mid-sentence. `ends_with_sentence_end` tells them apart — see its
        // doc for why only the forward end can be checked this way.
        None if ends_with_sentence_end(tail) => complete_flank(tail),
        None => incomplete_flank(tail, PAGE_ENDS_NOTICE),
    }
}

/// A flank that ended on a sentence boundary: strip the gutter, keep no notice.
fn complete_flank(slice: &str) -> ContextFlank {
    ContextFlank {
        text: display_text(slice),
        complete: true,
        notice: None,
    }
}

/// A flank that ran out of page: strip the gutter, carry the composed notice.
///
/// A slice that strips to nothing gets [`ContextFlank::absent`] rather than a
/// bare notice — "— page begins here" with no text above it explains an absence
/// the reader cannot see, and the quote genuinely sitting at the very top of a
/// page is an ordinary, honest state that needs no annotation.
fn incomplete_flank(slice: &str, notice: &str) -> ContextFlank {
    let text = display_text(slice);
    if text.is_empty() {
        return ContextFlank::absent();
    }
    ContextFlank {
        text,
        complete: false,
        notice: Some(notice.to_string()),
    }
}

/// The display transform: strip gutter numerals, trim the edges.
///
/// Trimmed because a slice that begins at a page start (or ends at a page end)
/// carries that page's leading/trailing whitespace, and a context that opens with
/// three blank lines reads as a rendering fault.
fn display_text(slice: &str) -> String {
    strip_gutter_line_numbers(slice).trim().to_string()
}

#[cfg(test)]
#[path = "scenario_card_context_tests.rs"]
mod tests;
