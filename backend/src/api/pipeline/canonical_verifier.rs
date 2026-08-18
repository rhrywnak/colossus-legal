//! Canonical text verification for the pipeline Verify step.
//!
//! Searches extraction item snippets against the stored `document_text`
//! representation rather than the original file. This makes verification
//! format-agnostic: text PDFs, scanned PDFs (OCR), and future formats
//! all verify against the same canonical text the LLM saw during extraction.
//!
//! ## Why this exists
//!
//! The original PageGrounder approach opened the raw PDF and searched its
//! native text layer. Scanned PDFs have no native text — their OCR output
//! lives only in `document_text`. Since the LLM extracted quotes FROM
//! `document_text`, verification must search IN `document_text`.

use crate::domain::quote_gap::{
    bare_numerals, locate_with_gap, normalize_without_foreign_numerals, GapPolicy,
};

/// `verification_reason` stamped on an item matched only after stray numeral
/// tokens were removed from the page text.
///
/// Stored rather than merely logged: months later, the question "why does this
/// statement claim page 12?" has to be answerable from the row itself.
pub const REASON_WITHOUT_NUMERALS: &str = "verified_without_stray_numerals";

/// `verification_reason` stamped on an item matched in two halves around one
/// interruption. The Review panel reads this to say "matched around a
/// footnote" instead of presenting it as an ordinary match — honesty law: an
/// operator must be able to see that this page number came the harder way.
pub const REASON_WITH_GAP: &str = "verified_with_gap";

/// `verification_reason` stamped on an item whose stored quote is nothing but
/// bare numerals. The item is ungrounded, like a not-found quote, but for a
/// different reason and with a different fix — inspect the stored quote and
/// re-extract, rather than hunt the page for words that are not there.
pub const REASON_STRIPPED_TO_EMPTY: &str = "quote_is_only_numerals";

/// Result of searching for a snippet in canonical text.
#[derive(Debug, Clone, PartialEq)]
pub struct CanonicalGroundingResult {
    pub match_type: CanonicalMatchType,
    pub page_number: Option<u32>,
}

/// How the snippet was matched against canonical text.
#[derive(Debug, Clone, PartialEq)]
pub enum CanonicalMatchType {
    /// Case-sensitive exact substring match.
    Exact,
    /// Case-insensitive match after whitespace collapse and character
    /// normalization (smart quotes, dashes, ligatures, invisible chars,
    /// hyphenated line breaks, transcript gutter line-numbers). See
    /// `normalize_text` for the full list.
    Normalized,
    /// Matched only after stray numeral tokens (footnote markers, gutter
    /// numerals) were removed from the page text. The quote's own numerals are
    /// never removed, so a date still has to match a date.
    NormalizedWithoutNumerals,
    /// Matched in two contiguous halves around ONE interruption — a footnote
    /// body spliced into the sentence by the PDF's text layer.
    ///
    /// Both halves together are the whole quote: nothing partial is accepted.
    /// Carried as its own variant so the Review panel can say "matched around a
    /// footnote" rather than presenting it as an ordinary match (honesty law).
    NormalizedWithGap {
        /// Characters between the halves — how much was spliced in.
        gap_chars: usize,
    },
    /// The snippet consisted ENTIRELY of bare numerals, so stripping them left
    /// nothing to search for.
    ///
    /// Kept apart from [`NotFound`](Self::NotFound) because the two are
    /// different failures with different remedies: a not-found quote means the
    /// extractor invented words that are not on the page, while this means the
    /// extractor stored a quote that is only footnote markers. One is a
    /// grounding problem, the other is an extraction problem, and a reviewer
    /// must be able to tell them apart from the row (Standing Rule 1).
    EmptyAfterStripping,
    /// Snippet not found in any page of canonical text.
    NotFound,
}

/// Strip a trailing page-number line from text for cross-page concatenation.
///
/// PDF text extraction embeds page numbers as standalone lines at page
/// boundaries (e.g., page 3 text ends with `"...his checking\n3"`).
/// These artifacts break substring matching when adjacent pages are
/// concatenated. This function removes the last non-empty line if it
/// consists entirely of digits (after trimming whitespace).
///
/// Returns a slice of the original text with the page-number line
/// (and its preceding newline) removed, or the full text if no
/// page-number line is found.
fn strip_trailing_page_number(text: &str) -> &str {
    let trimmed = text.trim_end();
    if let Some(newline_pos) = trimmed.rfind('\n') {
        let last_line = trimmed[newline_pos + 1..].trim();
        if !last_line.is_empty() && last_line.chars().all(|c| c.is_ascii_digit()) {
            return &trimmed[..newline_pos];
        }
    }
    text
}

/// Strip a leading page-number line from text for cross-page concatenation.
///
/// Same rationale as `strip_trailing_page_number` but for the start
/// of the next page (e.g., page 4 text starts with `"4\naccount on..."`).
fn strip_leading_page_number(text: &str) -> &str {
    let trimmed = text.trim_start();
    if let Some(newline_pos) = trimmed.find('\n') {
        let first_line = trimmed[..newline_pos].trim();
        if !first_line.is_empty() && first_line.chars().all(|c| c.is_ascii_digit()) {
            return &trimmed[newline_pos + 1..];
        }
    }
    text
}

/// How a search result is written to `extraction_items`: the status string, the
/// page, and the reason (if any).
///
/// ## Why this is a function and not two copies of a `match`
///
/// Two code paths run verify — the Restate pipeline step and the Re-verify API
/// handler — and they must write the SAME row for the same result. When each
/// carried its own `match` over [`CanonicalMatchType`], adding a variant meant
/// remembering to add it twice, and a divergence would have been invisible until
/// an operator noticed two documents grounded differently for no reason.
///
/// ## Rust Learning: `&'static str` in the return, not `String`
///
/// Every value returned here is a compile-time literal that lives for the whole
/// program, so there is nothing to allocate and nothing to free. `'static` is
/// the lifetime that says exactly that, and it lets the caller hand the value
/// straight to a `&str` parameter with no clone.
pub fn grounding_write_for(
    item_id: i32,
    result: &CanonicalGroundingResult,
) -> (&'static str, Option<i32>, Option<&'static str>) {
    let page = result.page_number.map(|p| p as i32);
    if let CanonicalMatchType::NormalizedWithGap { gap_chars } = result.match_type {
        // Logged here rather than at each call site: one place decides what a
        // gap match is, so one place says so.
        tracing::info!(
            item_id,
            gap_chars,
            page = ?page,
            "verify: matched around an interruption of {gap_chars} characters"
        );
    }
    match result.match_type {
        CanonicalMatchType::Exact => ("exact", page, None),
        CanonicalMatchType::Normalized => ("normalized", page, None),
        // Counted and stored as normalized matches: they ARE normalized
        // matches, of a page whose stray numerals were removed. The reason
        // column is what distinguishes them, and it is stored rather than
        // merely logged so "why does this claim page 12?" stays answerable.
        CanonicalMatchType::NormalizedWithoutNumerals => {
            ("normalized", page, Some(REASON_WITHOUT_NUMERALS))
        }
        CanonicalMatchType::NormalizedWithGap { .. } => ("normalized", page, Some(REASON_WITH_GAP)),
        // No page, ever, on a refusal. Both refusals write `not_found` — the
        // status a reviewer filters on — and the reason column is what says
        // WHICH refusal it was.
        CanonicalMatchType::EmptyAfterStripping => {
            ("not_found", None, Some(REASON_STRIPPED_TO_EMPTY))
        }
        CanonicalMatchType::NotFound => ("not_found", None, None),
    }
}

/// True when the result grounded the item somewhere — any tier.
///
/// Kept beside [`grounding_write_for`] for the same reason: one definition, two
/// callers, no chance of the counts drifting apart from the writes.
pub fn is_grounded(match_type: &CanonicalMatchType) -> bool {
    !matches!(
        match_type,
        CanonicalMatchType::NotFound | CanonicalMatchType::EmptyAfterStripping
    )
}

/// The three counts every verify run reports.
///
/// ## Rust Learning: `#[derive(Default)]` on a struct of counters
///
/// `Default` gives `GroundingTally::default()` with every field zeroed, which is
/// what "nothing counted yet" means. Deriving it beats writing `0, 0, 0` at each
/// call site, and it cannot get out of step when a field is added.
#[derive(Debug, Default, Clone, Copy)]
pub struct GroundingTally {
    pub exact: usize,
    pub normalized: usize,
    pub not_found: usize,
}

impl GroundingTally {
    /// Count one result. Every tier that produced a page counts as
    /// `normalized` except an exact hit — the second-chance tiers ARE
    /// normalized matches, distinguished by `verification_reason`, not by a
    /// separate count that every downstream reader would have to learn about.
    pub fn record(&mut self, match_type: &CanonicalMatchType) {
        match match_type {
            CanonicalMatchType::Exact => self.exact += 1,
            _ if is_grounded(match_type) => self.normalized += 1,
            _ => self.not_found += 1,
        }
    }
}

/// Search for a snippet in the canonical text representation.
///
/// Tries exact match first (case-sensitive substring), then normalized
/// match (whitespace-collapsed, case-insensitive). Returns the page
/// number of the first match found.
///
/// # Arguments
/// * `snippet` — the verbatim quote or name to search for
/// * `document_pages` — canonical text pages as (page_number, text_content)
///
/// # Why two-tier matching
///
/// The LLM produces "clean" quotes with straight punctuation, but stored
/// document text can contain smart quotes (U+2019), em dashes, ligatures,
/// ¶ markers, and line breaks mid-word. Normalized matching bridges the gap
/// by character-normalizing BOTH sides before comparison — see
/// `normalize_text` for the full set of substitutions.
///
/// ## Why this is test-only
///
/// Production has exactly one caller shape — "verify with the configured
/// policy" — so a second production entry point would only be a way for the two
/// verify paths to drift apart. It survives as the *no-second-chance* entry the
/// existing suite calls, which is how those tests keep proving that the four
/// contiguous tiers behave today exactly as they did before the second chance
/// was added.
#[cfg(test)]
pub fn find_in_canonical_text(
    snippet: &str,
    document_pages: &[(u32, String)],
) -> CanonicalGroundingResult {
    find_in_canonical_text_with_policy(snippet, document_pages, None)
}

/// [`find_in_canonical_text`], plus the two second-chance tiers.
///
/// `policy` of `None` reproduces the historical behaviour exactly — four tiers,
/// no numeral stripping, no gap. Passing a policy adds tiers 5 and 6, which run
/// ONLY after all four contiguous tiers have failed, so no quote that grounds
/// today can change how it grounds.
pub fn find_in_canonical_text_with_policy(
    snippet: &str,
    document_pages: &[(u32, String)],
    policy: Option<GapPolicy>,
) -> CanonicalGroundingResult {
    if snippet.is_empty() {
        return CanonicalGroundingResult {
            match_type: CanonicalMatchType::NotFound,
            page_number: None,
        };
    }

    // 1. Try exact match on each page (case-sensitive substring)
    for (page_num, text) in document_pages {
        if text.contains(snippet) {
            return CanonicalGroundingResult {
                match_type: CanonicalMatchType::Exact,
                page_number: Some(*page_num),
            };
        }
    }

    // 2. Try normalized match (collapse whitespace, case-insensitive)
    let normalized_snippet = normalize_text(snippet);
    if normalized_snippet.is_empty() {
        // Two different reasons to arrive here, and they are not the same
        // problem. Whitespace only → nothing was ever stored, which the
        // `missing_quote` path already covers. Digits only → the extractor
        // stored a quote consisting of footnote or gutter markers, which
        // `normalize_text` strips as page furniture. The second is an
        // EXTRACTION fault surfacing at verification time, and reporting it as
        // an ordinary "not found" would send a reviewer hunting the page for
        // words that were never claimed to be there (Standing Rule 1).
        let only_numerals = snippet.chars().any(|c| c.is_ascii_digit());
        if only_numerals {
            tracing::warn!(
                snippet = %snippet,
                "verify: the stored quote is nothing but bare numerals, so there is \
                 nothing to search for. The item stays ungrounded. Inspect the quote \
                 on the Review tab; if it is only footnote markers, this item needs \
                 re-extracting, not re-verifying."
            );
        }
        return CanonicalGroundingResult {
            match_type: if only_numerals {
                CanonicalMatchType::EmptyAfterStripping
            } else {
                CanonicalMatchType::NotFound
            },
            page_number: None,
        };
    }

    for (page_num, text) in document_pages {
        let normalized_text = normalize_text(text);
        if normalized_text.contains(&normalized_snippet) {
            return CanonicalGroundingResult {
                match_type: CanonicalMatchType::Normalized,
                page_number: Some(*page_num),
            };
        }
    }

    // NOTE: Paragraphs in legal documents occasionally span a page boundary
    // (e.g., a sentence starts on page 3 and finishes on page 4). Per-page
    // search misses these because neither page contains the full snippet.
    // We concatenate adjacent page pairs with a space separator and re-run
    // both match tiers. A single space prevents word-merging at the boundary
    // without introducing characters that would break normalized matching.
    // Adjacent pairs are sufficient — a single paragraph won't span 3+ pages
    // in a legal filing. If that assumption is ever wrong, extending to
    // triplets is a one-line change to the window size.
    if document_pages.len() >= 2 {
        // 3. Try exact match across adjacent page pairs.
        //    Strip embedded page-number lines from the boundary before
        //    joining — PDF extraction often appends/prepends the page
        //    number as a standalone digit line.
        for pair in document_pages.windows(2) {
            let clean_left = strip_trailing_page_number(&pair[0].1);
            let clean_right = strip_leading_page_number(&pair[1].1);
            let combined = format!("{} {}", clean_left.trim_end(), clean_right.trim_start());
            if combined.contains(snippet) {
                return CanonicalGroundingResult {
                    match_type: CanonicalMatchType::Exact,
                    page_number: Some(pair[0].0),
                };
            }
        }

        // 4. Try normalized match across adjacent page pairs
        for pair in document_pages.windows(2) {
            let clean_left = strip_trailing_page_number(&pair[0].1);
            let clean_right = strip_leading_page_number(&pair[1].1);
            let combined = format!("{} {}", clean_left.trim_end(), clean_right.trim_start());
            let normalized_combined = normalize_text(&combined);
            if normalized_combined.contains(&normalized_snippet) {
                return CanonicalGroundingResult {
                    match_type: CanonicalMatchType::Normalized,
                    page_number: Some(pair[0].0),
                };
            }
        }
    }

    // Tiers 5 and 6 are the second chance, and they run only here — after every
    // contiguous tier has failed. A quote that grounds today cannot reach them.
    let Some(policy) = policy else {
        return CanonicalGroundingResult {
            match_type: CanonicalMatchType::NotFound,
            page_number: None,
        };
    };

    // The quote's OWN numerals are the keep set: they are never stripped from
    // the page, so a date must still match a date.
    let keep = bare_numerals(snippet);
    let snippet_stripped = normalize_without_foreign_numerals(snippet, &keep);
    let needle = snippet_stripped.text();
    if needle.is_empty() {
        // Defensive: the guard above already catches a snippet that normalizes
        // to nothing, and stripping only removes numerals the quote does not
        // itself contain, so this should be unreachable. It refuses rather than
        // searching for an empty string, which would match every page.
        return CanonicalGroundingResult {
            match_type: CanonicalMatchType::EmptyAfterStripping,
            page_number: None,
        };
    }

    // The same surfaces the contiguous tiers use: each page, then adjacent
    // pairs, because a sentence interrupted by a footnote is also often a
    // sentence that crosses a page break — measured, all six of the motion's
    // failures matched on a PAIR, not a single page.
    let mut surfaces: Vec<(u32, String)> = document_pages.to_vec();
    for pair in document_pages.windows(2) {
        let clean_left = strip_trailing_page_number(&pair[0].1);
        let clean_right = strip_leading_page_number(&pair[1].1);
        surfaces.push((
            pair[0].0,
            format!("{} {}", clean_left.trim_end(), clean_right.trim_start()),
        ));
    }

    // 5. Contiguous match once the foreign numerals are gone.
    for (page_num, text) in &surfaces {
        let hay = normalize_without_foreign_numerals(text, &keep);
        if hay.text().contains(needle) {
            return CanonicalGroundingResult {
                match_type: CanonicalMatchType::NormalizedWithoutNumerals,
                page_number: Some(*page_num),
            };
        }
    }

    // 6. One gap — a footnote body spliced into the sentence.
    for (page_num, text) in &surfaces {
        let hay = normalize_without_foreign_numerals(text, &keep);
        if let Some(m) = locate_with_gap(hay.text(), needle, policy) {
            return CanonicalGroundingResult {
                match_type: CanonicalMatchType::NormalizedWithGap {
                    gap_chars: m.gap_chars,
                },
                page_number: Some(*page_num),
            };
        }
    }

    // 7. Not found — the item stays exactly as ungrounded as it is today.
    CanonicalGroundingResult {
        match_type: CanonicalMatchType::NotFound,
        page_number: None,
    }
}

// ── Normalization lives in `domain::quote_match` ─────────────────────────────
//
// It moved there in task 1.7A (defect D3) so the card assembler could reuse it
// WITH an origin map — the same normalization, plus the byte span each
// normalized character came from, which is what lets a normalized match point
// back at the original page text.
//
// Re-exported rather than duplicated, because two normalizers would be two
// definitions of "grounded": a quote could match at ingest and not at display,
// or the reverse, and nothing would say which one was right. There is one
// implementation; `normalize_text` and `normalize_with_map` are two views of it.
//
// The behaviour is unchanged — the tests in this module's `mod tests` are the
// proof, and they still address `strip_gutter_line_numbers` by name.
pub use crate::domain::quote_match::normalize_text;

#[cfg(test)]
mod tests {
    use super::*;

    // The gutter-numeral helper moved to `domain::quote_match` with
    // `normalize_text` (task 1.7A). These tests stayed: they are the proof that
    // the move changed no behaviour, so they must keep addressing the SAME
    // function the grounding path now calls, not a copy of it.
    use crate::domain::quote_match::strip_gutter_line_numbers;

    // ── normalize_text tests ─────────────────────────────────────

    #[test]
    fn test_normalize_collapses_whitespace() {
        assert_eq!(normalize_text("hello   world"), "hello world");
    }

    #[test]
    fn test_normalize_handles_newlines_and_tabs() {
        assert_eq!(normalize_text("hello\n\tworld"), "hello world");
    }

    // ── find_in_canonical_text tests ─────────────────────────────

    fn sample_pages() -> Vec<(u32, String)> {
        vec![
            (
                1,
                "This is page one with Milton Higgs as plaintiff.".to_string(),
            ),
            (
                2,
                "Page two discusses the defendant George Phillips.".to_string(),
            ),
            (
                3,
                "Page three contains\nmulti-line\ntext about damages.".to_string(),
            ),
        ]
    }

    #[test]
    fn test_exact_match_found_on_correct_page() {
        let pages = sample_pages();
        let result = find_in_canonical_text("Milton Higgs", &pages);
        assert_eq!(result.match_type, CanonicalMatchType::Exact);
        assert_eq!(result.page_number, Some(1));
    }

    #[test]
    fn test_exact_match_on_page_two() {
        let pages = sample_pages();
        let result = find_in_canonical_text("George Phillips", &pages);
        assert_eq!(result.match_type, CanonicalMatchType::Exact);
        assert_eq!(result.page_number, Some(2));
    }

    #[test]
    fn test_normalized_match_case_insensitive() {
        let pages = sample_pages();
        let result = find_in_canonical_text("milton higgs", &pages);
        assert_eq!(result.match_type, CanonicalMatchType::Normalized);
        assert_eq!(result.page_number, Some(1));
    }

    #[test]
    fn test_normalized_match_whitespace_differences() {
        let pages = sample_pages();
        let result = find_in_canonical_text("contains multi-line text", &pages);
        assert_eq!(result.match_type, CanonicalMatchType::Normalized);
        assert_eq!(result.page_number, Some(3));
    }

    #[test]
    fn test_not_found() {
        let pages = sample_pages();
        let result = find_in_canonical_text("this text does not exist anywhere", &pages);
        assert_eq!(result.match_type, CanonicalMatchType::NotFound);
        assert_eq!(result.page_number, None);
    }

    #[test]
    fn test_empty_snippet_returns_not_found() {
        let pages = sample_pages();
        let result = find_in_canonical_text("", &pages);
        assert_eq!(result.match_type, CanonicalMatchType::NotFound);
        assert_eq!(result.page_number, None);
    }

    #[test]
    fn test_empty_pages_returns_not_found() {
        let pages: Vec<(u32, String)> = vec![];
        let result = find_in_canonical_text("anything", &pages);
        assert_eq!(result.match_type, CanonicalMatchType::NotFound);
        assert_eq!(result.page_number, None);
    }

    #[test]
    fn test_exact_match_takes_priority_over_normalized() {
        let pages = vec![(1, "Milton Higgs is here.".to_string())];
        let result = find_in_canonical_text("Milton Higgs", &pages);
        assert_eq!(result.match_type, CanonicalMatchType::Exact);
    }

    #[test]
    fn test_first_page_match_wins() {
        let pages = vec![
            (1, "The defendant is George Phillips.".to_string()),
            (2, "George Phillips appeared in court.".to_string()),
        ];
        let result = find_in_canonical_text("George Phillips", &pages);
        assert_eq!(result.match_type, CanonicalMatchType::Exact);
        assert_eq!(result.page_number, Some(1));
    }

    #[test]
    fn test_whitespace_only_snippet_returns_not_found() {
        let pages = sample_pages();
        let result = find_in_canonical_text("   \n  ", &pages);
        assert_eq!(result.match_type, CanonicalMatchType::NotFound);
    }

    // ── character normalization tests ────────────────────────────
    // These mirror the real-world failure mode where stored document
    // text contains typographic characters the LLM's quote does not.

    #[test]
    fn test_normalized_match_smart_apostrophe_in_stored_text() {
        // Stored text: smart apostrophe (U+2019). LLM quote: straight apostrophe.
        let pages = vec![(
            4,
            "The plaintiff alleges Awad\u{2019}s conduct caused harm.".to_string(),
        )];
        let result = find_in_canonical_text("Awad's conduct", &pages);
        assert_eq!(result.match_type, CanonicalMatchType::Normalized);
        assert_eq!(result.page_number, Some(4));
    }

    #[test]
    fn test_normalized_match_smart_double_quotes_in_stored_text() {
        let pages = vec![(
            1,
            "The memo was marked \u{201C}confidential\u{201D} on page one.".to_string(),
        )];
        let result = find_in_canonical_text("\"confidential\"", &pages);
        assert_eq!(result.match_type, CanonicalMatchType::Normalized);
        assert_eq!(result.page_number, Some(1));
    }

    #[test]
    fn test_normalized_match_em_dash_in_stored_text() {
        // Stored text: em dash (U+2014). LLM quote: plain hyphen.
        let pages = vec![(
            2,
            "Damages\u{2014}both economic and non-economic\u{2014}were claimed.".to_string(),
        )];
        let result = find_in_canonical_text("Damages-both economic", &pages);
        assert_eq!(result.match_type, CanonicalMatchType::Normalized);
        assert_eq!(result.page_number, Some(2));
    }

    #[test]
    fn test_normalized_match_en_dash_in_stored_text() {
        // Stored text: en dash (U+2013). LLM quote: plain hyphen.
        let pages = vec![(3, "pages 12\u{2013}15 of the deposition".to_string())];
        let result = find_in_canonical_text("pages 12-15", &pages);
        assert_eq!(result.match_type, CanonicalMatchType::Normalized);
        assert_eq!(result.page_number, Some(3));
    }

    #[test]
    fn test_normalized_match_fi_ligature_in_stored_text() {
        // Stored text: fi ligature (U+FB01). LLM quote: plain "fi".
        let pages = vec![(
            5,
            "The \u{FB01}nal judgment was issued on that date.".to_string(),
        )];
        let result = find_in_canonical_text("final judgment", &pages);
        assert_eq!(result.match_type, CanonicalMatchType::Normalized);
        assert_eq!(result.page_number, Some(5));
    }

    #[test]
    fn test_normalized_match_fl_ligature_in_stored_text() {
        let pages = vec![(6, "a \u{FB02}ash of insight".to_string())];
        let result = find_in_canonical_text("a flash of insight", &pages);
        assert_eq!(result.match_type, CanonicalMatchType::Normalized);
        assert_eq!(result.page_number, Some(6));
    }

    #[test]
    fn test_normalized_match_hyphenated_line_break_in_stored_text() {
        // Stored text: word split across a line by a trailing hyphen.
        // LLM quote: the unbroken word.
        let pages = vec![(
            7,
            "The defendant's coun-\nsel failed to object in time.".to_string(),
        )];
        let result = find_in_canonical_text("counsel failed to object", &pages);
        assert_eq!(result.match_type, CanonicalMatchType::Normalized);
        assert_eq!(result.page_number, Some(7));
    }

    #[test]
    fn test_normalized_match_quote_with_line_break_mid_phrase() {
        // Stored text contains a newline mid-phrase. LLM quote is a flat string.
        let pages = vec![(
            8,
            "The parties agreed to\nthe settlement terms in full.".to_string(),
        )];
        let result = find_in_canonical_text("agreed to the settlement terms", &pages);
        assert_eq!(result.match_type, CanonicalMatchType::Normalized);
        assert_eq!(result.page_number, Some(8));
    }

    #[test]
    fn test_normalize_strips_invisible_characters() {
        // Soft hyphen, zero-width space, BOM all removed.
        let s = "foo\u{00AD}bar\u{200B}baz\u{FEFF}qux".to_string();
        assert_eq!(normalize_text(&s), "foobarbazqux");
    }

    #[test]
    fn test_normalize_expands_ellipsis() {
        assert_eq!(normalize_text("wait\u{2026}done"), "wait...done");
    }

    #[test]
    fn test_normalize_replaces_paragraph_marker() {
        assert_eq!(normalize_text("part one¶part two"), "part one part two");
    }

    // ── cross-page boundary tests ───────────────────────────────

    #[test]
    fn test_cross_page_exact_match() {
        let pages = vec![
            (
                3,
                "The guardianship proceeding involved allegations of".to_string(),
            ),
            (4, "theft with regard to funds held in trust.".to_string()),
        ];
        let snippet = "involved allegations of theft with regard to funds";
        let result = find_in_canonical_text(snippet, &pages);
        assert_eq!(result.match_type, CanonicalMatchType::Exact);
        assert_eq!(result.page_number, Some(3));
    }

    #[test]
    fn test_cross_page_normalized_match() {
        let pages = vec![
            (
                5,
                "The defendant\u{2019}s counsel argued that the".to_string(),
            ),
            (6, "fiduciary duty was not breached.".to_string()),
        ];
        let snippet = "defendant's counsel argued that the fiduciary duty";
        let result = find_in_canonical_text(snippet, &pages);
        assert_eq!(result.match_type, CanonicalMatchType::Normalized);
        assert_eq!(result.page_number, Some(5));
    }

    #[test]
    fn test_single_page_preferred_over_cross_page() {
        let pages = vec![
            (
                1,
                "The contract was signed by both parties on that date.".to_string(),
            ),
            (
                2,
                "Both parties on that date agreed to the terms.".to_string(),
            ),
        ];
        let snippet = "Both parties on that date";
        let result = find_in_canonical_text(snippet, &pages);
        assert_eq!(result.match_type, CanonicalMatchType::Exact);
        assert_eq!(result.page_number, Some(2));
    }

    #[test]
    fn test_cross_page_with_one_page_only() {
        let pages = vec![(1, "Only one page of text here.".to_string())];
        let snippet = "text that spans nowhere because there is only one page";
        let result = find_in_canonical_text(snippet, &pages);
        assert_eq!(result.match_type, CanonicalMatchType::NotFound);
        assert_eq!(result.page_number, None);
    }

    #[test]
    fn test_cross_page_boundary_word_split() {
        let pages = vec![
            (
                5,
                "The guardianship proceeding involved allegations of".to_string(),
            ),
            (
                6,
                "theft with regard to funds held by the trustee.".to_string(),
            ),
        ];
        let snippet = "guardianship proceeding involved allegations of theft with regard to funds";
        let result = find_in_canonical_text(snippet, &pages);
        assert_eq!(result.match_type, CanonicalMatchType::Exact);
        assert_eq!(result.page_number, Some(5));
    }

    // ── page-number stripping tests ─────────────────────────────

    #[test]
    fn test_strip_trailing_page_number() {
        let text = "his checking\n3";
        assert_eq!(strip_trailing_page_number(text), "his checking");
    }

    #[test]
    fn test_strip_trailing_page_number_with_whitespace() {
        let text = "some content here\n  17  \n";
        assert_eq!(strip_trailing_page_number(text), "some content here");
    }

    #[test]
    fn test_strip_trailing_page_number_not_a_number() {
        let text = "some content here\nsome text";
        assert_eq!(strip_trailing_page_number(text), text);
    }

    #[test]
    fn test_strip_trailing_page_number_mixed_line() {
        let text_dot = "content\n3.";
        assert_eq!(strip_trailing_page_number(text_dot), text_dot);

        let text_prefix = "content\nPage 3";
        assert_eq!(strip_trailing_page_number(text_prefix), text_prefix);

        let text_range = "content\n3 of 17";
        assert_eq!(strip_trailing_page_number(text_range), text_range);
    }

    #[test]
    fn test_strip_leading_page_number() {
        let text = "4\naccount on August 18, 2008";
        assert_eq!(
            strip_leading_page_number(text),
            "account on August 18, 2008"
        );
    }

    #[test]
    fn test_cross_page_match_with_page_number_artifact() {
        let pages = vec![
            (
                3,
                "that Mr. Awad had deposited in his checking\n3".to_string(),
            ),
            (
                4,
                "account on August 18, 2008 but had been removed".to_string(),
            ),
        ];
        let snippet = "his checking account on August 18, 2008";
        let result = find_in_canonical_text(snippet, &pages);
        assert_eq!(result.match_type, CanonicalMatchType::Exact);
        assert_eq!(result.page_number, Some(3));
    }

    // ── gutter line-number stripping (transcript margin numerals) ────
    // FIX 1 for the court_transcript maiden-run grounding failures: OCR
    // interleaves the 1–25 gutter numerals as standalone lines, breaking
    // the normalized substring for multi-line quotes.

    #[test]
    fn test_strip_gutter_line_numbers_removes_standalone_numeral() {
        assert_eq!(strip_gutter_line_numbers("alpha\n14\nbeta"), "alpha\nbeta");
    }

    #[test]
    fn test_strip_gutter_line_numbers_handles_surrounding_whitespace() {
        // Gutter numeral padded by spaces (indented margin) and a 1-digit case.
        assert_eq!(
            strip_gutter_line_numbers("alpha\n  7  \nbeta"),
            "alpha\nbeta"
        );
    }

    #[test]
    fn test_strip_gutter_line_numbers_keeps_three_digit_lines() {
        // 3+ digit standalone numbers are OUT of the gutter range (1–25) and
        // could be a real page number or figure a quote spans — never stripped.
        assert_eq!(
            strip_gutter_line_numbers("alpha\n122\nbeta"),
            "alpha\n122\nbeta"
        );
    }

    #[test]
    fn test_strip_gutter_line_numbers_keeps_mixed_numeral_lines() {
        // Anything but a bare 1–2 digit line survives: numbered-paragraph
        // markers, "Page N", ranges, citations.
        for text in [
            "content\n3.",        // has a dot
            "content\nPage 3",    // has letters
            "content\n3 of 17",   // has words
            "content\nMCR 2.114", // citation
        ] {
            assert_eq!(strip_gutter_line_numbers(text), text, "must keep: {text:?}");
        }
    }

    #[test]
    fn test_strip_gutter_line_numbers_leading_position() {
        // The filter is position-agnostic: a gutter numeral on the FIRST line
        // (a page that continues mid-transcript) is stripped like any other.
        assert_eq!(strip_gutter_line_numbers("1\nalpha\nbeta"), "alpha\nbeta");
    }

    #[test]
    fn test_strip_gutter_line_numbers_trailing_position() {
        // Gutter numeral as the terminal line with NO trailing newline —
        // `str::lines()` yields it as a final line and it is stripped.
        assert_eq!(strip_gutter_line_numbers("alpha\nbeta\n5"), "alpha\nbeta");
    }

    #[test]
    fn test_strip_gutter_line_numbers_consecutive_numerals() {
        // Two adjacent gutter lines with no intervening text — a common OCR
        // artifact in dense passages (a blank transcript line prints a bare
        // numeral). Both are removed.
        assert_eq!(
            strip_gutter_line_numbers("alpha\n7\n8\nbeta"),
            "alpha\nbeta"
        );
    }

    #[test]
    fn test_strip_gutter_line_numbers_two_vs_three_digit_boundary() {
        // The tightest expression of the 1–2 digit bound: "99" (2 digits) is
        // stripped; "100" (3 digits) is kept. This is the exact boundary the
        // scoping exists to hold.
        assert_eq!(strip_gutter_line_numbers("alpha\n99\nbeta"), "alpha\nbeta");
        assert_eq!(
            strip_gutter_line_numbers("alpha\n100\nbeta"),
            "alpha\n100\nbeta"
        );
    }

    #[test]
    fn test_gutter_numeral_between_lines_now_grounds() {
        // The maiden-run failure: an appearance line OCR split across gutter
        // numerals 14/15. Before the strip, those broke the normalized
        // substring; now the multi-line quote grounds (Normalized tier).
        let pages = vec![(
            3,
            "George Phillips on behalf of\n14\nCatholic Family Service,\n15\nthe personal representative."
                .to_string(),
        )];
        let result = find_in_canonical_text(
            "George Phillips on behalf of Catholic Family Service, the personal representative.",
            &pages,
        );
        assert_eq!(result.match_type, CanonicalMatchType::Normalized);
        assert_eq!(result.page_number, Some(3));
    }

    #[test]
    fn test_short_transcript_turn_still_grounds_exact() {
        // The "short quotes verify" half of the pattern must keep working:
        // a single-line turn matches exactly even with a gutter numeral nearby.
        let pages = vec![(
            2,
            "THE WITNESS: Yes, I can.\n7\nTHE COURT: Thank you.".to_string(),
        )];
        let result = find_in_canonical_text("Yes, I can.", &pages);
        assert_eq!(result.match_type, CanonicalMatchType::Exact);
        assert_eq!(result.page_number, Some(2));
    }

    /// MANDATED behavior test (Roman's ruling): the six already-verified,
    /// non-transcript documents' grounding must be provably unchanged.
    ///
    /// The rigorous form of that guarantee: `strip_gutter_line_numbers` is
    /// the IDENTITY function on any text that contains no standalone 1–2
    /// digit line. If it returns such text byte-for-byte, then
    /// `normalize_text` — and therefore every grounding decision — is
    /// unchanged for those documents. This corpus mirrors the number-bearing
    /// content of the six verified doc types (discovery Q&A, complaint
    /// allegations, court-ruling citations, affidavit dates, dollar figures,
    /// deposition page ranges, multi-digit record pages). None carries a bare
    /// gutter numeral, so the strip is a no-op on all of them.
    #[test]
    fn test_gutter_strip_is_identity_on_non_transcript_documents() {
        let corpus = [
            // discovery_response — sworn Q&A with an interrogatory number inline
            "Answer to Q74: That is my recollection.",
            // court_ruling — a citation and a finding
            "Many of those objections were frivolous, and sanctions under MCR 2.114 are in order.",
            // complaint — numbered allegation paragraphs (dot-suffixed, multi-line)
            "10. During the accounting period the trustee failed to disclose the fees.\n11. The auctioneer realized $6,000 less than the costs.",
            // affidavit — a dated statement with a multi-digit day
            "I sent a certified letter on November 16, 2009 requesting an accounting.",
            // discovery — dollar figures and 3-digit page markers a quote may span
            "the $50,000 was not gifted, per page 122 of the record",
            // appellate_brief — a deposition page range
            "pages 12-15 of the deposition were entered as Exhibit 7",
            // correspondence — a bare year on its own is 4 digits, out of range
            "2009\nwas the operative year for the accounting",
        ];
        for page in corpus {
            assert_eq!(
                strip_gutter_line_numbers(page),
                page,
                "gutter strip must be identity on non-transcript text (verified docs unchanged): {page:?}"
            );
        }
    }

    /// Companion to the identity test, one level up: for the same
    /// non-transcript corpus, a representative quote still grounds exactly
    /// as it did before the change — the numbers survive normalization.
    #[test]
    fn test_non_transcript_grounding_unchanged_end_to_end() {
        let pages = vec![
            (
                1,
                "the $50,000 was not gifted, per page 122 of the record".to_string(),
            ),
            (2, "sanctions under MCR 2.114 are in order".to_string()),
            (
                3,
                "certified letter on November 16, 2009 requesting an accounting".to_string(),
            ),
        ];
        // Dollar figure + multi-digit page number preserved:
        let r1 = find_in_canonical_text("$50,000 was not gifted, per page 122", &pages);
        assert_eq!(r1.match_type, CanonicalMatchType::Exact);
        assert_eq!(r1.page_number, Some(1));
        // Citation number preserved (case-normalized match):
        let r2 = find_in_canonical_text("MCR 2.114", &pages);
        assert_eq!(r2.match_type, CanonicalMatchType::Exact);
        // Date with a 2-digit day is NOT a standalone gutter line — preserved:
        let r3 = find_in_canonical_text("November 16, 2009", &pages);
        assert_eq!(r3.match_type, CanonicalMatchType::Exact);
        assert_eq!(r3.page_number, Some(3));
    }

    // ── Second-chance tiers (2026-08-17) ───────────────────────────

    /// The policy the build ships with, so these tests measure DEV's behaviour.
    fn shipped_policy() -> GapPolicy {
        GapPolicy {
            max_gap_chars: 240,
            min_half_fraction: 0.05,
            min_half_words: 3,
        }
    }

    #[test]
    fn footnote_marker_mid_sentence_now_grounds_and_reports_its_page() {
        let pages = vec![
            (11u32, "irrelevant opening page".to_string()),
            (
                12u32,
                "the fee arrangement 24 the record seems to indicate 25 was never disclosed"
                    .to_string(),
            ),
        ];
        let quote = "the record seems to indicate was never disclosed";

        // Today: no contiguous match anywhere, so no page.
        let before = find_in_canonical_text(quote, &pages);
        assert_eq!(before.match_type, CanonicalMatchType::NotFound);
        assert_eq!(before.page_number, None);

        // With the second chance: grounded, on the page it is actually on.
        let after = find_in_canonical_text_with_policy(quote, &pages, Some(shipped_policy()));
        assert_eq!(
            after.match_type,
            CanonicalMatchType::NormalizedWithoutNumerals
        );
        assert_eq!(after.page_number, Some(12));
    }

    #[test]
    fn gutter_numeral_at_line_start_now_grounds() {
        let pages = vec![(
            7u32,
            "1 the guardian ad litem 2 billed the estate 3 for a title search".to_string(),
        )];
        let quote = "the guardian ad litem billed the estate for a title search";

        assert_eq!(
            find_in_canonical_text(quote, &pages).match_type,
            CanonicalMatchType::NotFound
        );
        let after = find_in_canonical_text_with_policy(quote, &pages, Some(shipped_policy()));
        assert_eq!(
            after.match_type,
            CanonicalMatchType::NormalizedWithoutNumerals
        );
        assert_eq!(after.page_number, Some(7));
    }

    #[test]
    fn a_footnote_body_spliced_into_the_sentence_matches_with_a_gap() {
        let pages = vec![(
            15u32,
            "the guardian ad litem billed for the title \
             See Exhibit 11 Transcript of the hearing held that same afternoon \
             search and he is not entitled to that fee"
                .to_string(),
        )];
        let quote =
            "the guardian ad litem billed for the title search and he is not entitled to that fee";

        assert_eq!(
            find_in_canonical_text(quote, &pages).match_type,
            CanonicalMatchType::NotFound
        );
        let after = find_in_canonical_text_with_policy(quote, &pages, Some(shipped_policy()));
        match after.match_type {
            CanonicalMatchType::NormalizedWithGap { gap_chars } => {
                assert!(gap_chars > 0 && gap_chars <= 240, "gap was {gap_chars}");
            }
            other => panic!("expected a gap match, got {other:?}"),
        }
        assert_eq!(after.page_number, Some(15));
    }

    #[test]
    fn a_gap_match_is_refused_when_one_half_is_a_single_common_word() {
        // The shape of item 9402: the head is one word that occurs by
        // coincidence well before the rest of the quote. Refused — and refused
        // means NO page, never a guessed one.
        let pages = vec![(
            9u32,
            "For an entirely different reason set out at length below \
             the estate paid the disputed fee in full"
                .to_string(),
        )];
        let quote = "For the estate paid the disputed fee in full";

        let after = find_in_canonical_text_with_policy(quote, &pages, Some(shipped_policy()));
        assert_eq!(after.match_type, CanonicalMatchType::NotFound);
        assert_eq!(
            after.page_number, None,
            "a refused match must not carry a page"
        );
    }

    #[test]
    fn a_quote_that_is_genuinely_absent_still_fails() {
        let pages = vec![
            (1u32, "the estate paid the fee 12 in full".to_string()),
            (2u32, "13 counsel then withdrew from the matter".to_string()),
        ];
        let quote = "the trustee admitted forging the signature on the deed";

        let after = find_in_canonical_text_with_policy(quote, &pages, Some(shipped_policy()));
        assert_eq!(after.match_type, CanonicalMatchType::NotFound);
        assert_eq!(after.page_number, None);
    }

    #[test]
    fn a_quote_carrying_its_own_numerals_must_still_match_the_right_year() {
        let right = vec![(
            4u32,
            "the estate paid $50,000.00 on July 23, 2009 7 to the guardian".to_string(),
        )];
        let wrong = vec![(
            4u32,
            "the estate paid $50,000.00 on July 23, 2011 7 to the guardian".to_string(),
        )];
        let quote = "the estate paid $50,000.00 on July 23, 2009 to the guardian";

        let hit = find_in_canonical_text_with_policy(quote, &right, Some(shipped_policy()));
        assert_eq!(
            hit.match_type,
            CanonicalMatchType::NormalizedWithoutNumerals
        );
        assert_eq!(hit.page_number, Some(4));

        let miss = find_in_canonical_text_with_policy(quote, &wrong, Some(shipped_policy()));
        assert_eq!(
            miss.match_type,
            CanonicalMatchType::NotFound,
            "the year in the quote must not be stripped away"
        );
    }

    #[test]
    fn a_quote_that_already_grounds_is_unaffected_by_the_second_chance() {
        // The Humphrey case: an ordinary exact match must stay an EXACT match
        // on the same page, policy or no policy.
        let pages = vec![
            (1u32, "unrelated".to_string()),
            (
                2u32,
                "the affiant states that he was present throughout".to_string(),
            ),
        ];
        let quote = "the affiant states that he was present throughout";

        let without = find_in_canonical_text(quote, &pages);
        let with = find_in_canonical_text_with_policy(quote, &pages, Some(shipped_policy()));
        assert_eq!(without.match_type, CanonicalMatchType::Exact);
        assert_eq!(with.match_type, CanonicalMatchType::Exact);
        assert_eq!(with.page_number, without.page_number);
    }

    #[test]
    fn re_verifying_the_same_item_twice_yields_the_same_answer() {
        // Idempotence: the matcher is pure, so a second Re-verify over an
        // already-recovered item must not move its page or change its tier.
        let pages = vec![(
            12u32,
            "the fee arrangement 24 the record seems to indicate 25 was never disclosed"
                .to_string(),
        )];
        let quote = "the record seems to indicate was never disclosed";

        let first = find_in_canonical_text_with_policy(quote, &pages, Some(shipped_policy()));
        let second = find_in_canonical_text_with_policy(quote, &pages, Some(shipped_policy()));
        assert_eq!(first.match_type, second.match_type);
        assert_eq!(first.page_number, second.page_number);
        assert_eq!(second.page_number, Some(12));
    }

    #[test]
    fn tier_five_matches_across_a_page_pair_when_neither_page_holds_the_whole_quote() {
        // The real shape, and the one the single-page tests do not reach: on the
        // Phillips default motion ALL FOUR recoveries are on an adjacent PAIR,
        // because a sentence a footnote interrupts is often also a sentence that
        // crosses a page break. Tiers 5 and 6 build their own surface list, so
        // the pre-existing cross-page tests for tiers 3 and 4 do not cover this.
        let pages = vec![
            (
                7u32,
                "preamble the guardian ad litem billed 12 for the title search and he is"
                    .to_string(),
            ),
            (
                8u32,
                "not entitled to that fee. Further matters.".to_string(),
            ),
        ];
        let quote =
            "the guardian ad litem billed for the title search and he is not entitled to that fee";

        // Neither page holds it, and no contiguous tier finds it — the inline
        // `12` is not a gutter LINE, so the existing normalization leaves it.
        assert_eq!(
            find_in_canonical_text(quote, &pages).match_type,
            CanonicalMatchType::NotFound
        );

        let after = find_in_canonical_text_with_policy(quote, &pages, Some(shipped_policy()));
        assert_eq!(
            after.match_type,
            CanonicalMatchType::NormalizedWithoutNumerals
        );
        // The pair is reported under its FIRST page, which is where the quote
        // starts — the page a citation should carry.
        assert_eq!(after.page_number, Some(7));
    }

    #[test]
    fn tier_six_matches_across_a_page_pair_around_a_footnote_body() {
        let pages = vec![
            (
                3u32,
                "the estate paid the disputed fee \
                 See the note below regarding the parties \
                 in full to counsel"
                    .to_string(),
            ),
            (4u32, "of record and the matter was closed".to_string()),
        ];
        let quote = "the estate paid the disputed fee in full to counsel of record";

        assert_eq!(
            find_in_canonical_text(quote, &pages).match_type,
            CanonicalMatchType::NotFound
        );

        let after = find_in_canonical_text_with_policy(quote, &pages, Some(shipped_policy()));
        match after.match_type {
            CanonicalMatchType::NormalizedWithGap { gap_chars } => {
                assert!(gap_chars > 0 && gap_chars <= 240, "gap was {gap_chars}");
            }
            other => panic!("expected a gap match across the pair, got {other:?}"),
        }
        assert_eq!(after.page_number, Some(3));
    }

    #[test]
    fn a_quote_that_is_only_numerals_is_its_own_outcome_not_a_not_found() {
        // An extraction fault, not a grounding fault: the stored quote is
        // nothing but footnote markers, so there is nothing to search for. It
        // must not look identical to a quote whose words are genuinely absent —
        // the two have different remedies (re-extract vs. re-read the page).
        // Each marker on its own line, which is how the extractor stores one it
        // mistook for a quote — and which is exactly what `normalize_text`'s
        // gutter-line stripping removes, leaving nothing behind.
        let pages = vec![(5u32, "the estate paid the fee 24 in full".to_string())];
        let result = find_in_canonical_text_with_policy("24\n25\n", &pages, Some(shipped_policy()));

        // Whitespace only is a DIFFERENT emptiness and keeps the old answer:
        // nothing was ever stored, which the missing-quote path covers.
        assert_eq!(
            find_in_canonical_text_with_policy("   \n  ", &pages, Some(shipped_policy()))
                .match_type,
            CanonicalMatchType::NotFound
        );
        assert_eq!(result.match_type, CanonicalMatchType::EmptyAfterStripping);
        assert_eq!(result.page_number, None);

        let (status, page, reason) = grounding_write_for(1, &result);
        assert_eq!(status, "not_found", "it is ungrounded, and filters on that");
        assert_eq!(page, None);
        assert_eq!(
            reason,
            Some(REASON_STRIPPED_TO_EMPTY),
            "the reason column is what distinguishes it from a genuine miss"
        );
    }

    #[test]
    fn every_match_type_maps_to_exactly_one_write() {
        // The table both verify paths depend on. A new variant that nobody
        // decided how to write would fail to compile in `grounding_write_for`;
        // this pins what the existing ones write.
        let cases = [
            (CanonicalMatchType::Exact, "exact", Some(4), None),
            (CanonicalMatchType::Normalized, "normalized", Some(4), None),
            (
                CanonicalMatchType::NormalizedWithoutNumerals,
                "normalized",
                Some(4),
                Some(REASON_WITHOUT_NUMERALS),
            ),
            (
                CanonicalMatchType::NormalizedWithGap { gap_chars: 99 },
                "normalized",
                Some(4),
                Some(REASON_WITH_GAP),
            ),
            (
                CanonicalMatchType::EmptyAfterStripping,
                "not_found",
                None,
                Some(REASON_STRIPPED_TO_EMPTY),
            ),
            (CanonicalMatchType::NotFound, "not_found", None, None),
        ];
        for (match_type, want_status, want_page, want_reason) in cases {
            let result = CanonicalGroundingResult {
                match_type: match_type.clone(),
                page_number: Some(4),
            };
            let (status, page, reason) = grounding_write_for(1, &result);
            assert_eq!(status, want_status, "status for {match_type:?}");
            assert_eq!(page, want_page, "page for {match_type:?}");
            assert_eq!(reason, want_reason, "reason for {match_type:?}");
        }
    }

    #[test]
    fn the_tally_counts_every_second_chance_tier_as_normalized() {
        // The second-chance tiers ARE normalized matches. Giving them their own
        // count would mean every downstream reader of grounding_pct had to learn
        // about them or silently under-report.
        let mut tally = GroundingTally::default();
        tally.record(&CanonicalMatchType::Exact);
        tally.record(&CanonicalMatchType::Normalized);
        tally.record(&CanonicalMatchType::NormalizedWithoutNumerals);
        tally.record(&CanonicalMatchType::NormalizedWithGap { gap_chars: 12 });
        tally.record(&CanonicalMatchType::NotFound);
        tally.record(&CanonicalMatchType::EmptyAfterStripping);

        assert_eq!(tally.exact, 1);
        assert_eq!(tally.normalized, 3);
        assert_eq!(tally.not_found, 2, "both refusals count as ungrounded");
    }
}
