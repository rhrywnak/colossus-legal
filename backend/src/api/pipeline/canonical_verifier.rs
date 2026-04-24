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

    // 3. Not found
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

    #[test]
    fn test_normalize_lowercases() {
        assert_eq!(normalize_text("Hello World"), "hello world");
    }

    #[test]
    fn test_normalize_trims() {
        assert_eq!(normalize_text("  hello  "), "hello");
    }

    #[test]
    fn test_normalize_empty_string() {
        assert_eq!(normalize_text(""), "");
    }

    #[test]
    fn test_normalize_whitespace_only() {
        assert_eq!(normalize_text("   \n\t  "), "");
    }

    // ── find_in_canonical_text tests ─────────────────────────────

    fn sample_pages() -> Vec<(u32, String)> {
        vec![
            (1, "This is page one with Milton Higgs as plaintiff.".to_string()),
            (2, "Page two discusses the defendant George Phillips.".to_string()),
            (3, "Page three contains\nmulti-line\ntext about damages.".to_string()),
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
        let pages = vec![
            (1, "Milton Higgs is here.".to_string()),
        ];
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
        let s = format!("foo\u{00AD}bar\u{200B}baz\u{FEFF}qux");
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
}
