//! `quote_gap` — the verifier's SECOND-CHANCE matcher.
//!
//! [`super::quote_match`] answers "is this quote on this page, contiguously?".
//! This module answers the two questions that remain when the answer is no and
//! the quote is nonetheless real:
//!
//! 1. Is it there once the page's stray numeral tokens — footnote markers,
//!    gutter numerals — are taken out? ([`normalize_without_foreign_numerals`])
//! 2. Is it there in two pieces around ONE interruption, a footnote body the
//!    PDF's text layer spliced into the sentence? ([`locate_with_gap`])
//!
//! ## Why a separate module
//!
//! The contiguous matcher is load-bearing for every document in the case; this
//! is a fallback that runs only after it fails. Keeping them apart means a
//! reader can satisfy themselves about the strict path without reading the
//! lenient one, and it keeps each file inside the 300-line limit (CLAUDE.md §17).

use std::collections::BTreeSet;
use std::ops::Range;

use super::quote_match::{normalize_text, pipeline, Ch, Normalized};

//
// Measured on `doc-awad-v-catholic-family-motion-for-default-and-default-judgment-as-to-phillips`
// (2026-08-17): six Evidence quotes were flagged `not_found` while being real
// text — 67 of 71 words present verbatim, 54 of 76, 50 of 88. The PDF's text
// layer splices footnote markers AND footnote bodies into the sentence:
//
//     …not two months as Mr. Phillips' response seems to indicate 24 .
//     Second, …
//     …billed for the title 12 Exhibit 11 - Transcript from December 15, 2009,
//     Page 4 and 5 13 search 14 , he is either negligent…
//
// One contiguous match cannot survive that. Two more tiers can, and neither
// lowers the bar on what counts as present:
//
//   * strip the foreign numerals (the markers), then match contiguously;
//   * allow ONE gap (the footnote body) between two halves that, together, are
//     the WHOLE quote.
//
// ## The safety argument, because it is not the obvious one
//
// A gap match splits the quote at one word boundary and requires BOTH halves to
// be found, in order, on the same page. Head plus tail is the entire quote by
// construction — nothing partial is ever accepted, and no word is skipped. What
// makes it safe is therefore not how the split falls but that the gap is small
// and the quote is long: 67 matched words followed by a 44-character gap and 4
// more matched words is not a coincidence.

/// A quote found in two contiguous pieces around one interruption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GapMatch {
    /// Byte span in the searched (normalized) text, from the first matched
    /// character to the last.
    pub span: Range<usize>,
    /// Characters between the two halves — the interruption's size.
    pub gap_chars: usize,
    /// Words in the first half, and in the second. They sum to the whole quote.
    pub head_words: usize,
    pub tail_words: usize,
}

/// What a one-gap match is allowed to accept.
///
/// Config-driven end to end (`VERIFY_*` env vars) rather than literals here:
/// these are thresholds a human tunes against real documents, which is Rule 2's
/// definition of configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GapPolicy {
    /// Longest interruption tolerated, in characters of normalized text.
    pub max_gap_chars: usize,
    /// Smallest either half may be, as a fraction of the quote's words.
    pub min_half_fraction: f64,
    /// Smallest either half may be in absolute words, whatever the fraction
    /// says. A 22-word quote whose "head" is the single word "For" is a
    /// coincidence, not a match, and this is what refuses it.
    pub min_half_words: usize,
}

impl Default for GapPolicy {
    /// The shipped thresholds — the ONE place they are written down.
    ///
    /// Both startup paths (`AppConfig` and `AppContext`) and the offline proof
    /// bin start from this and let the environment override it. Holding the
    /// numbers in two `unwrap_or(...)` calls instead was the earlier shape, and
    /// it meant tuning one of them would silently leave the other verify path on
    /// the old value — a divergence with no log and no compile error.
    ///
    /// `max_gap_chars: 240` — measured: the Phillips default motion's
    /// footnote-interrupted quotes needed 44–169 characters.
    ///
    /// `min_half_fraction: 0.05` — ruled 2026-08-17. The obvious value is 0.40,
    /// and it was the first proposal; against the six real failures it recovers
    /// 2, because their gaps fall near an end. 0.05 recovers 4, and 0.04 would
    /// recover 5. Nothing is weakened by the lower value: the two halves ARE the
    /// whole quote, so a match still means every word is present, in order,
    /// separated by one gap no larger than the maximum above.
    ///
    /// `min_half_words: 3` — what refuses item 9402, whose "head" is the single
    /// word "For" matching by coincidence 99 characters before the rest of the
    /// quote. A fraction alone cannot express that: 1 word of 22 is 4.5%, and so
    /// is 3 words of 66.
    fn default() -> Self {
        Self {
            max_gap_chars: 240,
            min_half_fraction: 0.05,
            min_half_words: 3,
        }
    }
}

/// Is this token a bare numeral — digits, optionally with a trailing `.`?
///
/// `24` and `24.` yes; `2009,` no (the comma is part of the token as split on
/// whitespace, and a date is prose); `$50,000.00` no. Deliberately strict: the
/// only thing being removed is a footnote or gutter marker standing alone.
fn is_bare_numeral(token: &str) -> bool {
    let core = token.strip_suffix('.').unwrap_or(token);
    !core.is_empty() && core.chars().all(|c| c.is_ascii_digit())
}

/// Every BARE numeral appearing in `text` — exactly what [`is_bare_numeral`]
/// would remove.
///
/// This is the KEEP set: a bare numeral the quote itself contains is never
/// stripped from the page, so a quote reading "on 2009 the estate paid" cannot
/// match a page reading "on 2011 the estate paid", and a quote that swallowed
/// its own footnote marker still matches the page that carries it.
///
/// ## Why the two functions must agree, and why a wider keep set is not "safer"
///
/// Stripping only ever removes BARE tokens. A punctuated numeral in the quote —
/// the `23,` of a date, the `$50,000.00` of a sum — is never a bare token, so it
/// is never stripped from the quote OR from the page, and it needs no
/// protection. Adding such a numeral to the keep set therefore protects nothing,
/// while making the page retain footnote markers that merely share digits with a
/// date in the quote. Keep exactly what strip removes, and no more.
///
/// ## What this runs on
///
/// The text has already been through [`normalize_text`], which removes gutter
/// LINES. A footnote marker sitting alone on its own line in the quote is
/// therefore gone before this function sees it — measured on item 9441 of the
/// Phillips default motion, whose stored quote carries markers `24` and `25` on
/// their own lines and whose keep set comes back empty. That is the intended
/// outcome: the marker is dropped from the quote and from the page alike.
pub fn bare_numerals(text: &str) -> BTreeSet<String> {
    normalize_text(text)
        .split_whitespace()
        .filter(|t| is_bare_numeral(t))
        .map(|t| t.trim_end_matches('.').to_string())
        .collect()
}

/// Normalize, keeping the origin map, with foreign bare numerals removed.
///
/// "Foreign" means: a bare numeral token that does NOT appear in `keep`. The
/// space that followed it goes too, so `"indicate 24 ."` becomes `"indicate ."`
/// rather than `"indicate  ."` — and the caller's own whitespace rules then see
/// the same shape the quote has.
pub fn normalize_without_foreign_numerals(text: &str, keep: &BTreeSet<String>) -> Normalized {
    let chars = pipeline(text);

    // Walk the collapsed stream token by token. Tokens are single-space
    // separated by this point, which is what makes a token boundary findable
    // without re-splitting the string and losing the origins.
    let mut out: Vec<Ch> = Vec::with_capacity(chars.len());
    let mut token: Vec<Ch> = Vec::new();

    let flush = |token: &mut Vec<Ch>, out: &mut Vec<Ch>| {
        if token.is_empty() {
            return;
        }
        let text: String = token.iter().map(|c| c.c).collect();
        let core = text.trim_end_matches('.');
        let foreign = is_bare_numeral(&text) && !keep.contains(core);
        if !foreign {
            out.append(token);
        } else {
            token.clear();
            // Drop the separator this token would otherwise have left behind.
            if out.last().is_some_and(|c| c.c == ' ') {
                out.pop();
            }
        }
    };

    for ch in chars {
        if ch.c == ' ' {
            flush(&mut token, &mut out);
            out.push(ch);
        } else {
            token.push(ch);
        }
    }
    flush(&mut token, &mut out);

    // A dropped leading token can leave the stream starting with a space.
    while out.first().is_some_and(|c| c.c == ' ') {
        out.remove(0);
    }

    Normalized::from_chars(out)
}

/// Find `needle` in `haystack` allowing exactly ONE interruption.
///
/// Both sides must already be normalized the same way. Returns the tightest
/// match found — smallest gap — or `None` when no split satisfies `policy`.
///
/// ## Rust Learning: why the search is O(words × occurrences) and that is fine
///
/// It runs only after both contiguous tiers have failed, on one document at a
/// time, for a quote of at most a few hundred words. The naive scan is a few
/// thousand `str::find` calls on a page of kilobytes — microseconds — and the
/// alternative (a suffix automaton) would be a great deal of code standing
/// between a reader and a decision about whether a quote is real.
pub fn locate_with_gap(haystack: &str, needle: &str, policy: GapPolicy) -> Option<GapMatch> {
    let words: Vec<&str> = needle.split_whitespace().collect();
    let n = words.len();
    if n < 2 {
        return None;
    }

    let mut best: Option<GapMatch> = None;
    for split in 1..n {
        let (head_words, tail_words) = (split, n - split);
        let shorter = head_words.min(tail_words);
        if shorter < policy.min_half_words {
            continue;
        }
        if (shorter as f64) / (n as f64) < policy.min_half_fraction {
            continue;
        }

        let head = words[..split].join(" ");
        let tail = words[split..].join(" ");

        let mut from = 0usize;
        while let Some(i) = haystack[from..].find(&head).map(|at| at + from) {
            let after = i + head.len();
            if let Some(j) = haystack[after..].find(&tail).map(|at| at + after) {
                let gap = j - after;
                if gap <= policy.max_gap_chars {
                    let candidate = GapMatch {
                        span: i..j + tail.len(),
                        gap_chars: gap,
                        head_words,
                        tail_words,
                    };
                    if best.as_ref().is_none_or(|b| gap < b.gap_chars) {
                        best = Some(candidate);
                    }
                }
            }
            from = i + 1;
            if from >= haystack.len() {
                break;
            }
        }
    }
    best
}

#[cfg(test)]
#[path = "quote_gap_tests.rs"]
mod tests;
