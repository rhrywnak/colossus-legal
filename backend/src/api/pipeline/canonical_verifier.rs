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
    /// Whitespace-collapsed, case-insensitive match.
    /// Handles OCR artifacts like extra spaces and line breaks.
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
/// The LLM produces "clean" quotes, but OCR text may have artifacts like
/// extra spaces ("M ilton  Higgs") or line breaks mid-word ("Nadia\nAwad").
/// Normalized matching bridges this gap by collapsing whitespace and
/// lowercasing before comparison.
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
/// Collapses all whitespace (spaces, newlines, tabs) to single spaces,
/// lowercases, and trims. This handles the most common OCR artifacts
/// without going full fuzzy (Levenshtein).
///
/// Note: character-level OCR errors like split words ("M ilton" → two
/// tokens) will NOT match "Milton" via normalization alone. Normalized
/// matching handles whitespace between words, line breaks, and case
/// differences. A future enhancement could add Levenshtein as a third tier.
pub fn normalize_text(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
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
}
