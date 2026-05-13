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
    /// hyphenated line breaks). See `normalize_text` for the full list.
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
/// Order matters — hyphenated line breaks must be rejoined before
/// whitespace collapsing, otherwise the trailing `-\n` would be flattened
/// to `- ` and the word would stay split.
///
/// Handles:
/// * invisible characters (soft hyphen U+00AD, zero-width space U+200B, BOM U+FEFF)
/// * hyphenated line breaks (word-split across lines with a trailing `-`)
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
}
