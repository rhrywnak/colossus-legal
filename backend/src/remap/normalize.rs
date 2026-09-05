//! Loose text normalization and the two near-match measures, for the remap's
//! tier-2 and tier-3 matching.
//!
//! ## Why this is NOT `evidence_key::normalize`
//!
//! `api::pipeline::evidence_key::normalize` (NFC + collapse whitespace) is the
//! material of the stable Evidence **id**. Every live id was hashed over its
//! output, so touching it would move all 525 ids at once — it is a value that can
//! never be loosened. This module is the opposite kind of thing: it is used only
//! to *propose* that two nodes are the same statement, its output is never
//! hashed, never stored and never becomes an id, and a human reads every match it
//! suggests before anything is written.
//!
//! Keeping the two functions apart is therefore deliberate, not duplication. They
//! answer different questions: "what is this node's identity?" and "might these
//! two nodes be the same statement?"
//!
//! ## What the loose form throws away, and why each is safe here
//!
//! Measured on `doc-transcript-post-appeal-restructure-02-28-2012-clean`, where a
//! re-extraction from corrected page text left **0 of 14** movable nodes matchable
//! byte-for-byte:
//!
//! | dropped | why it is layout, not content |
//! |---|---|
//! | case | the OCR pass re-cased nothing, but a template change could |
//! | `word-` + newline + `rest` | justified-text hyphenation at a line break |
//! | whitespace runs | the new quotes carry hard line breaks the old ones did not |
//! | a trailing `.` or `,` | an extraction that stops one character earlier is the same statement |

use strsim::normalized_levenshtein;
use unicode_normalization::UnicodeNormalization;

/// How close two quotes must be before tier 3 will propose them as one statement.
///
/// ## Rust Learning: a `Default` impl instead of `const` thresholds
///
/// Standing Rule 2 asks that a threshold be configuration rather than a compiled
/// literal. A `Default` impl is the smallest honest way to do that in a binary
/// with no config file: the numbers live in exactly one place, the CLI overrides
/// them (`--near-similarity`, `--near-word-coverage`), and no caller can silently
/// disagree with the documented default because there is nothing else to read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NearMatchSettings {
    /// Minimum normalized Levenshtein ratio between the two loose quotes.
    pub similarity: f64,
    /// Minimum fraction of the OLD quote's words that appear, in order, in the
    /// new quote.
    pub word_coverage: f64,
}

impl Default for NearMatchSettings {
    fn default() -> Self {
        // The task's numbers. They are the DEFAULT and not the law: both are
        // dialable from the command line precisely so a document can be measured
        // before anything is applied.
        Self {
            similarity: 0.90,
            word_coverage: 0.80,
        }
    }
}

/// Why a threshold could not be used.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum SettingsError {
    #[error("{name} must be a ratio between 0.0 and 1.0 inclusive, got {value}")]
    OutOfRange { name: &'static str, value: f64 },
}

impl NearMatchSettings {
    /// Build the settings from optional CLI overrides, refusing nonsense.
    ///
    /// A threshold of 5.0 would match nothing and a threshold of -1.0 would match
    /// everything, and both would look like a working run — so this is a startup
    /// error with the flag named in it, not a runtime surprise (Rule 15).
    ///
    /// ## Rust Learning: `RangeInclusive::contains` and NaN
    ///
    /// `(0.0..=1.0).contains(&f64::NAN)` is `false`, because every comparison
    /// against NaN is false. That is exactly the answer wanted here, so NaN needs
    /// no separate arm — but it is worth knowing it is handled rather than
    /// assuming it slipped through.
    pub fn new(similarity: Option<f64>, word_coverage: Option<f64>) -> Result<Self, SettingsError> {
        let fallback = Self::default();
        let settings = Self {
            similarity: similarity.unwrap_or(fallback.similarity),
            word_coverage: word_coverage.unwrap_or(fallback.word_coverage),
        };
        for (name, value) in [
            ("--near-similarity", settings.similarity),
            ("--near-word-coverage", settings.word_coverage),
        ] {
            if !(0.0..=1.0).contains(&value) {
                return Err(SettingsError::OutOfRange { name, value });
            }
        }
        Ok(settings)
    }
}

/// The loose form of one piece of text: NFC, lowercase, hyphen line-breaks
/// joined, whitespace collapsed, trailing `.`/`,` removed.
///
/// The order matters and is not interchangeable: the hyphen-join pass has to run
/// while the newlines are still there, because a line break is the only thing
/// that distinguishes an OCR word-split from a real hyphenated compound
/// (`two-fold`). Collapsing whitespace first would turn every `two-\nfold` into
/// `two- fold` and lose the evidence.
pub fn loose_normalize(text: &str) -> String {
    let composed: String = text.nfc().collect();
    let joined = join_hyphen_line_breaks(&composed.to_lowercase());
    let collapsed = joined.split_whitespace().collect::<Vec<_>>().join(" ");
    // `trim_end_matches` takes a closure so the run is stripped in one pass:
    // `settlement. ,` and `settlement..` both end up at `settlement`.
    collapsed
        .trim_end_matches(|c: char| c == '.' || c == ',' || c.is_whitespace())
        .to_string()
}

/// Join `word-` + line break + `rest` into `wordrest`.
///
/// Covers both shapes the transcripts produce: `[a-z]-\s*\n\s*[a-z]` (the hyphen
/// hard against the word) and `[a-z] -\n[a-z]` (a space before the hyphen, which
/// the OCR emits when the gutter numeral was stripped). A hyphen NOT followed by
/// a line break is left alone — that is a real compound word, and joining
/// `two-fold` into `twofold` would make two genuinely different quotes look the
/// same.
///
/// ## Rust Learning: why `Vec<char>` and an index, not an iterator
///
/// The decision needs to look BACKWARD (is the previous non-space character a
/// letter?) and FORWARD past an unbounded whitespace run. `Chars` is a forward
/// iterator with one item of lookahead via `Peekable`, which is not enough, and
/// `char_indices` on a `&str` would need byte-offset arithmetic that is easy to
/// get wrong on multi-byte input. Collecting to `Vec<char>` costs one allocation
/// over a quote-length string and makes the scan obviously correct.
fn join_hyphen_line_breaks(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == '-' {
            if let Some(resume) = word_resumes_after(&chars, index, &out) {
                // Drop the hyphen, any spaces that preceded it, and the whole
                // line break — the two halves become one word.
                while out.ends_with(' ') || out.ends_with('\t') {
                    out.pop();
                }
                index = resume;
                continue;
            }
        }
        out.push(chars[index]);
        index += 1;
    }
    out
}

/// If the hyphen at `hyphen` is a line-break split, where the word resumes.
///
/// Returns `None` — leaving the hyphen intact — unless all three hold: the text
/// already emitted ends in a letter (ignoring spaces), the run after the hyphen
/// contains a newline, and a letter follows it.
fn word_resumes_after(chars: &[char], hyphen: usize, out: &str) -> Option<usize> {
    let ends_in_letter = out
        .trim_end_matches([' ', '\t'])
        .ends_with(|c: char| c.is_ascii_alphabetic());
    if !ends_in_letter {
        return None;
    }

    let mut index = hyphen + 1;
    let mut saw_newline = false;
    while index < chars.len() && chars[index].is_whitespace() {
        saw_newline |= chars[index] == '\n';
        index += 1;
    }
    if !saw_newline {
        return None;
    }
    chars
        .get(index)
        .filter(|c| c.is_ascii_alphabetic())
        .map(|_| index)
}

/// How alike two already-loose quotes are, as a 0.0–1.0 ratio.
///
/// ## Why normalized Levenshtein and not Jaro-Winkler
///
/// Jaro-Winkler deliberately rewards a shared prefix, which is the wrong bias
/// here: two different answers by the same speaker on the same page routinely
/// open with the same clause, and a prefix bonus would push exactly the pairs
/// that need separating over the line. `strsim::normalized_levenshtein` is
/// `1 - distance / longest`, which is symmetric, has no positional bias, and is a
/// true ratio rather than a heuristic score.
///
/// Its consequence is documented rather than hidden: because it divides by the
/// LONGER string, a new quote that keeps the old one whole but adds a clause is
/// penalised in proportion to the clause, not to the disagreement. That is why
/// [`word_coverage`] exists as a second, independent measure.
pub fn similarity(left: &str, right: &str) -> f64 {
    normalized_levenshtein(left, right)
}

/// The fraction of the OLD quote's words that appear, in order, in the new one.
///
/// This is the containment measure: 1.0 means the new quote holds every word of
/// the old, in sequence, whatever it adds around them. An empty old quote scores
/// 0.0 rather than 1.0 — a node with no words must never match everything.
pub fn word_coverage(old: &str, new: &str) -> f64 {
    let old_words: Vec<&str> = old.split_whitespace().collect();
    let new_words: Vec<&str> = new.split_whitespace().collect();
    if old_words.is_empty() {
        return 0.0;
    }
    common_subsequence_len(&old_words, &new_words) as f64 / old_words.len() as f64
}

/// Length of the longest common subsequence of two word lists.
///
/// ## Why a full LCS rather than a greedy forward scan
///
/// A greedy scan ("walk the new words, tick off old words as they appear")
/// under-counts whenever a word repeats: it can spend the old quote's only `the`
/// on the wrong occurrence and then fail to match the rest. LCS is exact, and at
/// quote length — tens of words, not thousands — the quadratic cost is nothing.
///
/// ## Rust Learning: the rolling row
///
/// The classic LCS table is `(n+1) x (m+1)`, but each row depends only on the one
/// above it, so only two rows are ever live. `prev` is the row above; `current` is
/// built left to right and then becomes `prev`. Memory is O(m) instead of O(n*m),
/// and the recurrence reads exactly as it does in the textbook.
fn common_subsequence_len(left: &[&str], right: &[&str]) -> usize {
    let mut prev = vec![0usize; right.len() + 1];
    for word_left in left {
        let mut current = vec![0usize; right.len() + 1];
        for (index, word_right) in right.iter().enumerate() {
            current[index + 1] = if word_left == word_right {
                prev[index] + 1
            } else {
                current[index].max(prev[index + 1])
            };
        }
        prev = current;
    }
    prev[right.len()]
}

#[cfg(test)]
#[path = "normalize_tests.rs"]
mod tests;
