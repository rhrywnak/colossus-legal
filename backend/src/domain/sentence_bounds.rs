// =============================================================================
// backend/src/domain/sentence_bounds.rs — where a legal sentence starts and ends
// =============================================================================
//
// Task 1.7C, defect D6 (RULED 2026-08-02, design v2 §3). A candidate card shows
// the page text flanking its quote, and until now that flank was a hard
// character window: the last 240 characters before and the first 240 after. A
// fixed cut lands mid-word and mid-sentence, so the human read fragments like
// `"lle Hanley--Hanley is not here, but"` and had to reconstruct the sentence in
// their head before they could rule on the quote inside it.
//
// The law is now: **sentence boundaries decide, the char window only seeds.**
// Context starts at a sentence start and ends at a sentence end, expanding
// OUTWARD from the seed until both ends are complete.
//
// ## Why this is a hard problem in THIS corpus, measured
//
// A naive split-on-period is a defect here, and the measurement says so. Over
// the nine documents in the case corpus (160 pages of stored page text, 525
// Evidence nodes carrying a quote):
//
// | false boundary | occurrences | where |
// |---|---|---|
// | `Mr.` (mixed case) | 177 | everywhere — the corpus leader |
// | `MR.` (upper) | 127 | hearing-transcript speaker tags |
// | single-letter enumerators `a.` `b.` `c.` … | 229 line-initial | interrogatory and discovery sub-parts |
// | mid-name initials (`George S. Phillips`) | 17 | |
// | `SS.` | 16 | affidavit jurats ("STATE OF MICHIGAN ) SS.") |
// | `No.` | 14 | |
// | `Dr.` | 10 | |
// | `Ms.` | 7 | |
// | `v.` `Inc.` `Co.` `Hon.` `Mar.` `Dec.` | 1–4 each | |
//
// The 229 enumerators are the finding that matters most, because no
// abbreviation LIST would ever have caught them — `a.` is not an abbreviation,
// it is a numbering scheme. They need the structural rule below.
//
// Also measured, and worth stating because it removes a parameter the design
// expected to need: the design feared "a 4-page sentence-less stretch swallowing
// the card" and asked whether the expansion bound should be a setting or a
// constant. Neither. Context is sliced from ONE page's stored text, the longest
// page in the corpus is 4323 characters (transcripts average 1189), and the
// largest single-side expansion any real quote needed was 895 characters. **The
// page is the bound** — Roman's ruling R3, 2026-08-03. There is no new
// parameter here, and `quote_context_window_chars` keeps its 1.6 meaning
// unchanged: it is the seed width.
//
// ## Rust Learning: byte offsets, not character counts
//
// Every index in this module is a BYTE offset into a `&str`, because that is
// what `str` slicing takes and what `str::find` returns. Rust strings are UTF-8,
// so slicing at a byte index that falls inside a multi-byte character panics —
// and OCR'd legal PDFs are full of curly quotes, em dashes and accented names.
// The walks below therefore step with `char_indices()`, which only ever yields
// real character boundaries, rather than doing arithmetic on `usize`. Every
// offset this module returns is a boundary by construction, so the caller's
// slice cannot panic.

/// Abbreviations whose trailing period does NOT end a sentence.
///
/// Compared case-insensitively, so one entry covers `MR.` and `Mr.` both.
///
// CONST: this is legal-prose VOCABULARY, not a threshold — a fixed formatting
// invariant of the domain, exactly like the 1–2 digit gutter bound in
// `quote_match::is_gutter` and documented on the same ground (Roman's ruling R4,
// 2026-08-03; Rule-13 disclosure accepted). English legal abbreviations do not
// vary by environment, by case, or by deployment: `Mr.` is not a sentence end in
// DEV and a sentence end in PROD. It cannot live in `app_settings`, and making it
// configurable would invite a deployment where the card breaks mid-honorific.
//
// Ordered by measured corpus frequency, then the forward-compat tail.
pub const NON_TERMINAL_ABBREVIATIONS: &[&str] = &[
    // Measured in this corpus, most frequent first.
    "MR", "NO", "DR", "MS", "SS", "V", "INC", "CO", "HON", "MAR", "DEC",
    // Carried at ZERO corpus count, deliberately. The next transcript ingest is
    // a deposition, whose spine is the `Q.` / `A.` examination pattern, and
    // `MRS.` / `JR.` / `SR.` are ordinary in party names. Adding them after the
    // first deposition card breaks would be a defect we already knew about.
    "MRS", "JR", "SR", "Q", "A",
    // The rest of the ordinary legal-prose set, present in the wider case file
    // even where this corpus does not yet exercise them.
    "NOS", "CORP", "LTD", "ST", "AVE", "REV", "EX", "ESQ", "SEC", "ART", "VS", "ET", "AL", "JAN",
    "FEB", "APR", "JUN", "JUL", "AUG", "SEP", "SEPT", "OCT", "NOV",
];

/// The three characters that can end a sentence.
///
// CONST: sentence-terminating punctuation in English. Same standing as the
// abbreviation list above — orthography, not configuration.
const TERMINATORS: [char; 3] = ['.', '!', '?'];

/// Whether the character at `at` genuinely ends a sentence.
///
/// `at` must be a character boundary in `text`; the two walk functions below only
/// ever pass boundaries they got from `char_indices`.
///
/// ## The two guards, in the order they fire
///
/// 1. **What follows.** A terminator must be followed by whitespace, a closing
///    quote or bracket, or the end of the text. This is what keeps `$400.50` and
///    `No.5` from splitting — the period inside a number has a digit after it.
/// 2. **What precedes.** For `.` only, the preceding word decides:
///    * a **single alphabetic character** is never a sentence end. This is the
///      structural rule (Roman's ruling R4) and it is doing the heavy lifting:
///      it covers all 229 measured `a.` / `b.` / `c.` enumerators AND every
///      mid-name initial with no vocabulary at all. Measured over the whole
///      corpus: zero false negatives — no real sentence in these 160 pages ends
///      in a one-letter word.
///    * a word in [`NON_TERMINAL_ABBREVIATIONS`] is never a sentence end.
///    * **no** preceding word (a bare `...`, or a period after punctuation) is
///      not treated as a sentence end either: an ellipsis is prose trailing off,
///      and cutting the context there would reproduce the fragment problem this
///      module exists to remove.
///
/// `!` and `?` need no preceding guard — no abbreviation ends in either.
fn is_sentence_end(text: &str, at: usize) -> bool {
    let Some(ch) = text[at..].chars().next() else {
        return false;
    };
    if !TERMINATORS.contains(&ch) {
        return false;
    }

    // Guard 1 — what follows.
    let after = &text[at + ch.len_utf8()..];
    let follows_ok = match after.chars().next() {
        // End of the text: a sentence may certainly end there.
        None => true,
        Some(next) => next.is_whitespace() || matches!(next, '"' | '\'' | ')' | ']' | '\u{201D}'),
    };
    if !follows_ok {
        return false;
    }

    if ch != '.' {
        return true;
    }

    // Guard 2 — what precedes. Collect the alphabetic run immediately before the
    // period, skipping any interior periods so `U.S.` reads as the token `US`.
    let mut word = String::new();
    for c in text[..at].chars().rev() {
        if c.is_alphabetic() {
            word.push(c);
        } else if c == '.' {
            // An interior period inside an initialism; keep collecting.
            continue;
        } else {
            break;
        }
    }
    if word.is_empty() {
        return false;
    }
    if word.chars().count() == 1 {
        return false;
    }

    // `word` was built in reverse, which does not matter: the comparison is
    // against a set, and an abbreviation reversed is still uniquely identified
    // by its characters only if we un-reverse it. So un-reverse it.
    let token: String = word.chars().rev().collect::<String>().to_uppercase();
    !NON_TERMINAL_ABBREVIATIONS.contains(&token.as_str())
}

/// Where the sentence containing `seed` starts, searching BACKWARD from `seed`.
///
/// Returns the byte offset of the first character after the previous sentence's
/// terminator, with any whitespace between them skipped — so the caller's slice
/// begins on a word, never on a space or a stray newline.
///
/// # Errors
/// Returns `None` when no sentence end lies before `seed`: the text begins
/// mid-sentence, which for a page slice means the sentence started on the
/// previous page. The caller must render that honestly rather than pretend the
/// cut was a sentence start (Standing Rule 1) — see
/// [`crate::services::scenario_card_context`].
pub fn sentence_start_before(text: &str, seed: usize) -> Option<usize> {
    // Walk the characters before `seed` from the right. `char_indices` yields
    // boundaries, so every candidate index is safe to slice at.
    let boundaries: Vec<usize> = text[..seed].char_indices().map(|(i, _)| i).collect();
    for &at in boundaries.iter().rev() {
        if is_sentence_end(text, at) {
            // The sentence starts after this terminator, past any whitespace.
            let mut start = at + text[at..].chars().next().map_or(1, char::len_utf8);
            for (offset, c) in text[start..].char_indices() {
                if !c.is_whitespace() {
                    start += offset;
                    return Some(start);
                }
            }
            // Nothing but whitespace after the terminator — there is no sentence
            // to show, so report no boundary rather than an empty span.
            return None;
        }
    }
    None
}

/// Where the sentence containing `seed` ends, searching FORWARD from `seed`.
///
/// Returns the byte offset just PAST the terminator, so the caller's slice keeps
/// the full stop — a context ending `"…any longer"` reads as truncated where
/// `"…any longer."` reads as complete.
///
/// # Errors
/// Returns `None` when no sentence end lies at or after `seed` — the page ends
/// mid-sentence (the sentence continues onto the next page). Rendered honestly by
/// the caller, never papered over.
pub fn sentence_end_after(text: &str, seed: usize) -> Option<usize> {
    for (offset, ch) in text[seed..].char_indices() {
        let at = seed + offset;
        if is_sentence_end(text, at) {
            return Some(at + ch.len_utf8());
        }
    }
    None
}

/// Whether `text` already finishes on a real sentence end (ignoring trailing
/// whitespace).
///
/// ## Why this exists, and why it has no backward twin
///
/// A forward seed can overshoot the end of the page — the tail after a quote is
/// often shorter than the seed width. [`sentence_end_after`] then finds nothing,
/// because there is nothing at or after the seed to find, even though the tail it
/// skipped over ends in a perfectly good full stop. Reporting that tail as a
/// page-edge cut would be its own small lie: `"End."` is a finished sentence.
///
/// There is deliberately no `starts_with_sentence_start`. Forward completeness is
/// decidable from the text in hand — either the last character terminates a
/// sentence or it does not. BACKWARD completeness is not: a page beginning with a
/// capital letter may be continuing a sentence that started on the previous page,
/// and one `document_text` row cannot tell you which. So the flank that runs to
/// the top of a page reports the page edge, and the flank that runs to the bottom
/// checks first. The asymmetry is the honest reading of what each end knows.
pub fn ends_with_sentence_end(text: &str) -> bool {
    let trimmed = text.trim_end();
    match trimmed.char_indices().next_back() {
        Some((at, _)) => is_sentence_end(trimmed, at),
        None => false,
    }
}

#[cfg(test)]
#[path = "sentence_bounds_tests.rs"]
mod tests;
