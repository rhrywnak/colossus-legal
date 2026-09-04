//! Normalisation and the pure defect predicates.
//!
//! Nothing here opens a socket or reads a file. Every rule the audit counts on
//! is a function in this module with a test beside it, because a bucket count is
//! only as trustworthy as the predicate that produced it — and "1,209 cards, 412
//! of them broken" is a number somebody will act on by deleting rows.

/// Normalise a quote for equality comparison: trim, collapse internal
/// whitespace to single spaces, casefold.
///
/// ## Why this is stricter than the shipped prefilter
///
/// `theme_scan_prefilter::collapse_exact_duplicates` groups on BYTE-identical
/// text, deliberately — it is a pre-LLM cost saver and wanted zero false
/// positives. This audit is a census, not a spend gate, so it asks the broader
/// question: how many cards say the same thing modulo whitespace and case? Both
/// numbers are reported, and the byte-identical one is the subset.
///
/// ## Rust Learning: `split_whitespace` does the collapsing for free
///
/// It splits on ANY run of Unicode whitespace and never yields an empty piece,
/// so `split_whitespace().collect::<Vec<_>>().join(" ")` is trim and
/// collapse-runs in one pass — no regex, no manual state machine.
pub fn normalise_quote(quote: &str) -> String {
    quote
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .to_lowercase()
}

/// Character length, counted in `char`s rather than bytes.
///
/// The shipped prefilter makes the same choice for the same reason: `len()` is
/// bytes, so a quote full of curly quotation marks and accented names measures
/// longer than it reads and slips past a threshold a human set by eye.
pub fn char_len(text: &str) -> usize {
    text.trim().chars().count()
}

/// **B1** — is this quote a bare discovery-response answer token?
///
/// The token set is passed in rather than compiled in: the instruction requires
/// the list to be DERIVED from the corpus (every distinct quote under 25
/// characters, with its count) and only then fixed. A hardcoded set would be the
/// audit deciding its own answer.
///
/// Comparison is on the normalised text with any trailing period removed, so
/// `Admitted.` and `admitted` are the same token.
pub fn is_answer_token(quote: &str, tokens: &[String]) -> bool {
    let key = answer_token_key(quote);
    !key.is_empty() && tokens.iter().any(|t| answer_token_key(t) == key)
}

/// The comparison key for an answer token: normalised, trailing `.` stripped.
fn answer_token_key(quote: &str) -> String {
    normalise_quote(quote).trim_end_matches('.').to_string()
}

/// **B8** — does this quote carry OCR damage of the class recorded in
/// `CC-REPORTS/transcript_grounding_classification.md` §C1?
///
/// Three signatures, all of them structural rather than statistical:
///
/// 1. **Mid-word line break** — a `-` immediately before a newline. §C1's
///    worked example is `busi-` / `ness.` four lines apart; when the fragment is
///    swept into a quote the hyphen-newline comes with it.
/// 2. **Double-hyphen join** — `--` inside the text, §C1's `for--as--ashe's`,
///    where a transposed line landed inside a hyphenated split.
/// 3. **Stray gutter numeral** — a line inside the quote consisting only of
///    digits. §C1 measured 962 standalone gutter numerals across 37 pages; a
///    quote that swallowed one is a quote whose line stream was mis-ordered.
///
/// Returns which signatures fired, so the report can say WHICH kind of damage
/// rather than only that there was some.
pub fn ocr_damage(quote: &str) -> OcrSignatures {
    let mut sig = OcrSignatures::default();
    let mut previous: Option<char> = None;
    for ch in quote.chars() {
        if ch == '\n' && previous == Some('-') {
            sig.mid_word_break = true;
        }
        previous = Some(ch);
    }
    sig.double_hyphen_join = quote.contains("--");
    sig.stray_gutter_numeral = quote
        .lines()
        .any(|line| !line.trim().is_empty() && line.trim().chars().all(|c| c.is_ascii_digit()));
    sig
}

/// Which §C1 signatures a quote carries.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct OcrSignatures {
    pub mid_word_break: bool,
    pub double_hyphen_join: bool,
    pub stray_gutter_numeral: bool,
}

impl OcrSignatures {
    pub fn any(self) -> bool {
        self.mid_word_break || self.double_hyphen_join || self.stray_gutter_numeral
    }
}

/// **B3** — is `a` a near-duplicate of `b`?
///
/// ## The rule, and why it is containment and not edit distance
///
/// Two normalised quotes are near-duplicates when one is a **prefix or suffix**
/// of the other AND the shorter is at least `min_ratio` of the longer's length.
/// The instruction allows an edit-distance threshold; containment was chosen
/// instead for three reasons:
///
/// - It is the defect actually observed — an extractor that took a sentence and
///   the same sentence plus its trailing clause, which is a prefix relation, not
///   a scatter of substitutions.
/// - Levenshtein over 1,209 × 1,209 quotes averaging hundreds of characters is
///   ~730k comparisons of quadratic cost each; containment is a byte compare.
/// - An edit-distance threshold is a number nobody can defend. "One quote starts
///   with the other" needs no threshold at all — only the length ratio, which is
///   there to stop a four-word fragment matching every quote that begins with
///   the same four words.
///
/// `min_ratio` is a parameter, printed in the report beside the count.
pub fn is_near_duplicate(a: &str, b: &str, min_ratio: f64) -> bool {
    if a == b {
        return false; // that is B2's business, not B3's.
    }
    let (short, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    if short.is_empty() {
        return false;
    }
    if !(long.starts_with(short) || long.ends_with(short)) {
        return false;
    }
    (short.chars().count() as f64) / (long.chars().count() as f64) >= min_ratio
}

/// **B9** — is this page number unusable?
///
/// Null, zero, negative, or past the end of the document it claims to be in.
/// A page beyond the document's `page_count` is the interesting case: it means
/// the card cites a location that does not exist, which no amount of re-reading
/// the PDF will fix.
pub fn page_unresolvable(page: Option<i64>, doc_page_count: Option<i64>) -> bool {
    match page {
        None => true,
        Some(p) if p <= 0 => true,
        // An unknown page_count cannot condemn the page — absence of evidence.
        // Those cards are caught by B10 instead, if the document is missing.
        Some(p) => doc_page_count.is_some_and(|count| p > count),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalise_collapses_whitespace_trims_and_casefolds() {
        assert_eq!(normalise_quote("  The   Court \n SAID  "), "the court said");
    }

    #[test]
    fn normalise_of_blank_is_empty() {
        assert_eq!(normalise_quote("   \n\t "), "");
    }

    #[test]
    fn char_len_counts_characters_not_bytes() {
        // Six characters, but more than six bytes: curly quotes and an accent.
        assert_eq!(char_len(" “Awad” "), 6);
    }

    #[test]
    fn answer_token_matches_regardless_of_case_and_trailing_period() {
        let tokens = vec!["Admitted.".to_string(), "Denied as untrue.".to_string()];
        assert!(is_answer_token("Admitted.", &tokens));
        assert!(is_answer_token("  admitted  ", &tokens));
        assert!(is_answer_token("DENIED AS UNTRUE", &tokens));
        assert!(!is_answer_token("Admitted in part.", &tokens));
    }

    #[test]
    fn answer_token_never_matches_an_empty_quote() {
        let tokens = vec!["Admitted.".to_string()];
        assert!(!is_answer_token("", &tokens));
        assert!(!is_answer_token("   ", &tokens));
    }

    #[test]
    fn ocr_detects_the_mid_word_line_break_from_c1() {
        let sig = ocr_damage("that's not our busi-\nness.");
        assert!(sig.mid_word_break);
        assert!(sig.any());
    }

    #[test]
    fn ocr_detects_the_double_hyphen_join_from_c1() {
        let sig = ocr_damage("for--as--ashe's");
        assert!(sig.double_hyphen_join);
    }

    #[test]
    fn ocr_detects_a_swallowed_gutter_numeral() {
        let sig = ocr_damage("let Mr. Phillips finish his\n9\nexplanation");
        assert!(sig.stray_gutter_numeral);
    }

    #[test]
    fn ocr_leaves_clean_prose_alone() {
        let sig = ocr_damage("The court ordered the $50,000 returned to the estate.");
        assert!(!sig.any());
        // A hyphenated word mid-line is not damage.
        assert!(!ocr_damage("a well-known conservator-ship question").any());
    }

    #[test]
    fn near_duplicate_catches_a_prefix_extension() {
        assert!(is_near_duplicate(
            "the court ordered the money returned",
            "the court ordered the money returned to the estate",
            0.5
        ));
    }

    #[test]
    fn near_duplicate_catches_a_suffix_extension() {
        assert!(is_near_duplicate(
            "returned to the estate",
            "the court ordered the money returned to the estate",
            0.4
        ));
    }

    #[test]
    fn near_duplicate_rejects_a_short_fragment_and_identical_text() {
        // Too short relative to the long one — a common opening, not a duplicate.
        assert!(!is_near_duplicate(
            "the court",
            "the court ordered the money returned to the estate",
            0.5
        ));
        // Identical text is B2's, not B3's.
        assert!(!is_near_duplicate("same text", "same text", 0.5));
        // Unrelated text shares no boundary.
        assert!(!is_near_duplicate("alpha beta", "gamma delta", 0.1));
    }

    #[test]
    fn page_unresolvable_covers_null_zero_and_overrun() {
        assert!(page_unresolvable(None, Some(10)));
        assert!(page_unresolvable(Some(0), Some(10)));
        assert!(page_unresolvable(Some(-3), Some(10)));
        assert!(page_unresolvable(Some(11), Some(10)));
        assert!(!page_unresolvable(Some(10), Some(10)));
        // Unknown page count cannot condemn a plausible page.
        assert!(!page_unresolvable(Some(999), None));
    }
}
