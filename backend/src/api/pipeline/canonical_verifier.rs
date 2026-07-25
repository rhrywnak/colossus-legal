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

/// Strip standalone gutter line-number lines (transcript margin numerals).
///
/// ## Domain note: line-numbered hearing transcripts
/// A court reporter prints a line number (1–25) in the left margin of
/// every line of a hearing transcript. OCR (Surya) and PDF text
/// extraction interleave those numerals into the text stream as
/// standalone lines, e.g.:
///
/// ```text
/// George Phillips on behalf of
/// 14
/// Catholic Family Service, the
/// 15
/// personal representative.
/// ```
///
/// The verbatim quote the LLM emits carries no such numerals. Once
/// `normalize_text` collapses whitespace, the stored page reads
/// `"...behalf of 14 catholic family service..."` while the quote reads
/// `"...behalf of catholic family service..."`, so the quote is no longer
/// a contiguous substring and grounding returns `not_found`. A short,
/// single-line quote never spans a gutter numeral and still matches —
/// which is exactly the observed "short quotes verify, long quotes fail"
/// pattern on the first court_transcript run.
///
/// This removes any line that, after trimming, is composed ONLY of a 1–2
/// digit number (the gutter range 1–25). It must run BEFORE whitespace
/// collapse, because the collapse erases the line structure it keys on.
///
/// ## Why scoped to 1–2 digits
/// Bounding the strip to 1–2 digit lines avoids removing a legitimately
/// standalone longer number a quote might genuinely span — a year
/// (`2009`), a dollar figure, a 3-digit page number. It mirrors the
/// existing `strip_*_page_number` precedent but applies per-line across
/// the whole page rather than only at a page boundary.
///
/// ## Why this is safe for already-verified documents
/// A document with no standalone 1–2 digit line has no line to remove, so
/// this returns its text unchanged (see the identity test). Because
/// `normalize_text` runs it on both sides of every comparison, the change
/// is a no-op for every non-line-numbered document type.
fn strip_gutter_line_numbers(text: &str) -> String {
    text.lines()
        .filter(|line| {
            let trimmed = line.trim();
            // Drop the line only when it is ENTIRELY a 1–2 digit numeral.
            // ASCII digits are one byte each, so byte length equals digit
            // count here.
            //
            // CONST: the 1–2 digit bound is a fixed legal-formatting
            // invariant, NOT a configurable threshold — deliberately not a
            // ProcessingProfile setting. U.S. court reporters number 1–25
            // lines per page, so a gutter numeral is always 1–2 digits. A
            // larger bound would strip legitimately standalone years (4
            // digits), page numbers (3 digits), and dollar figures a quote
            // may span — the exact false positives this scoping exists to
            // avoid. It cannot vary by document type or environment.
            !(matches!(trimmed.len(), 1 | 2) && trimmed.bytes().all(|b| b.is_ascii_digit()))
        })
        .collect::<Vec<_>>()
        .join("\n")
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
pub fn find_in_canonical_text(
    snippet: &str,
    document_pages: &[(u32, String)],
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
        return CanonicalGroundingResult {
            match_type: CanonicalMatchType::NotFound,
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

    // 5. Not found
    CanonicalGroundingResult {
        match_type: CanonicalMatchType::NotFound,
        page_number: None,
    }
}

/// Normalize text for fuzzy matching.
///
/// Mirrors the comprehensive normalization in
/// `colossus-pdf::page_grounder::normalize_text` so canonical-text matching
/// and PDF grounding behave identically. Without the character substitutions,
/// LLM-emitted straight quotes never match stored text containing smart
/// quotes, and em-dashes / ligatures cause systematic `not_found` results.
///
/// Order matters — hyphenated line breaks must be rejoined, and gutter
/// line-numbers stripped, BEFORE whitespace collapsing: both operate on
/// line structure that the collapse destroys. The trailing `-\n` would be
/// flattened to `- ` (leaving the word split), and a standalone `\n14\n`
/// gutter numeral would be smeared into `" 14 "` mid-sentence (where it
/// can no longer be told apart from a spoken number) if the collapse ran
/// first.
///
/// Handles:
/// * invisible characters (soft hyphen U+00AD, zero-width space U+200B, BOM U+FEFF)
/// * hyphenated line breaks (word-split across lines with a trailing `-`)
/// * transcript gutter line-numbers (standalone 1–2 digit lines — see
///   `strip_gutter_line_numbers`)
/// * paragraph markers (`¶`)
/// * smart quotes — both single (U+2018/U+2019) and double (U+201C/U+201D)
/// * em dash (U+2014) and en dash (U+2013) → plain hyphen
/// * ellipsis (U+2026) → `...`
/// * ligatures `fi` (U+FB01), `fl` (U+FB02)
/// * whitespace collapse + lowercase
///
/// Note: character-level OCR errors like split words ("M ilton" → two
/// tokens) will NOT match "Milton" via normalization alone. A future
/// enhancement could add Levenshtein as a third tier.
pub fn normalize_text(text: &str) -> String {
    let mut s = text.to_string();

    // 1. Remove invisible characters (soft hyphen, zero-width space, BOM)
    s = s.replace(['\u{00AD}', '\u{200B}', '\u{FEFF}'], "");

    // 2. Rejoin hyphenated line breaks BEFORE whitespace collapsing
    s = rejoin_hyphenated_breaks(&s);

    // 2b. Strip standalone gutter line-numbers (transcript margin numerals)
    //     BEFORE whitespace collapse — see `strip_gutter_line_numbers`.
    //     Applied here (inside normalize_text) so BOTH the snippet and the
    //     stored page pass through it: the operation is symmetric, and a
    //     document without gutter numerals has no such lines, so its text —
    //     and therefore its grounding — is byte-for-byte unchanged.
    s = strip_gutter_line_numbers(&s);

    // 3. Replace paragraph markers
    s = s.replace('¶', " ");

    // 4. Normalize quote characters
    s = s.replace(['\u{201C}', '\u{201D}'], "\""); // smart double quotes
    s = s.replace(['\u{2018}', '\u{2019}'], "'"); // smart single quotes

    // 5. Normalize dashes to plain hyphen
    s = s.replace(['\u{2014}', '\u{2013}'], "-"); // em dash, en dash

    // 6. Normalize ellipsis and expand ligatures
    s = s.replace('\u{2026}', "...");
    s = s.replace('\u{FB01}', "fi");
    s = s.replace('\u{FB02}', "fl");

    // 7. Collapse whitespace and lowercase
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Rejoin words split across lines by a hyphen.
///
/// Scans for pattern: word char + `-` + `\n` (with optional `\r` and
/// trailing whitespace) + word char. Removes the hyphen and line break,
/// joining the word fragments. Copied verbatim from
/// `colossus-pdf::page_grounder` so canonical-text and PDF grounding
/// stay byte-identical without introducing a cross-crate dependency.
///
/// Uses a simple char-by-char scan — no regex dependency needed.
fn rejoin_hyphenated_breaks(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut result = String::with_capacity(text.len());
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '-' && i > 0 && chars[i - 1].is_alphanumeric() {
            // Look ahead: skip optional \r, require \n, skip optional whitespace
            let mut j = i + 1;
            if j < chars.len() && chars[j] == '\r' {
                j += 1;
            }
            if j < chars.len() && chars[j] == '\n' {
                j += 1;
                while j < chars.len() && (chars[j] == ' ' || chars[j] == '\t') {
                    j += 1;
                }
                if j < chars.len() && chars[j].is_alphanumeric() {
                    // Skip the hyphen and line break — rejoin the word
                    i = j;
                    continue;
                }
            }
        }
        result.push(chars[i]);
        i += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
