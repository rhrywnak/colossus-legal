// =============================================================================
// backend/src/domain/quote_match.rs — finding a quote in the text it came from
// =============================================================================
//
// Task 1.7A, defect D3. Two jobs, one implementation:
//
//   1. `normalize_text` — the fuzzy-matching normalizer the grounding path has
//      always used (moved here from `api::pipeline::canonical_verifier`, which
//      re-exports it; no behaviour change, no caller changed).
//   2. `normalize_with_map` / `locate` — the same normalization, but carrying
//      the byte span each normalized character came FROM, so a match found in
//      normalized space can be pointed back at the original text.
//
// ## The defect this module exists for
//
// A candidate card shows the surrounding page text on each side of its quote
// (v2 §7.1) so a ruling is made in context rather than on a fragment. The
// assembler located that context with `page.find(quote)` — an exact,
// case-sensitive substring search. The ingest path, meanwhile, grounds quotes in
// TWO tiers: exact, then normalized (smart quotes, dashes, ligatures, collapsed
// whitespace, hyphenated line breaks, transcript gutter numerals).
//
// So every quote grounded `normalized` — 340 of 849 items in the corpus, and
// measured 320 of 320 that have stored page text — failed that `find` and was
// served with EMPTY context. Silently. The card that most needs its surroundings
// is the one whose wording does not match the page character-for-character.
//
// ## Why the position is recomputed here and not read from the record
//
// Grounding does not persist a position. `CanonicalGroundingResult` carries a
// match type and a page number and nothing else, and the normalized tier asks
// `contains` — a bool — so there is no offset even in flight. Recomputing at
// assembly needs no migration, no backfill, and no re-verification of the
// corpus, and it repairs every card already in the database the moment it ships.
//
// Persisting `(offset, len)` at grounding time is the better long-term answer
// and is task 2.5's (§12.1) design default; 2.5 inherits `normalize_with_map`
// as its primitive, because you cannot know a match's ORIGINAL offset without
// exactly this mapping.
//
// ## Why the context is sliced from the ORIGINAL text
//
// The cheap version of this fix slices the normalized text instead, and needs no
// map at all. It was rejected: normalization lowercases, collapses whitespace and
// rewrites punctuation, so the context under a quote would render as
// de-punctuated lowercase prose. On a surface a lawyer reads to decide whether a
// quote means what it appears to mean, that is a different kind of lie from an
// empty box.

use std::ops::Range;

/// One character of the working text, with the byte span it came from.
///
/// ## Rust Learning: why a struct and not a `(char, usize)` tuple
///
/// The normalization is a pipeline of stages, and every stage moves characters
/// around. A named `start` / `end` survives that; `.0` and `.1` do not survive
/// the third stage of anyone's reading. `Copy` because it is three words — the
/// stages pass them by value and never alias.
#[derive(Debug, Clone, Copy)]
struct Ch {
    c: char,
    /// Byte offset of the source character in the ORIGINAL text.
    start: usize,
    /// Byte offset just past that source character.
    end: usize,
}

/// Normalized text, plus the map back to where each byte came from.
///
/// ## Rust Learning: why the map is per BYTE and not per char
///
/// The consumer of this type holds byte indices — `str::find` returns one, and
/// slicing a `str` needs one. Storing one entry per byte makes the lookup an
/// index rather than a scan, at the cost of a vector a few times the size of the
/// page. Pages are kilobytes and this runs on the coldest path in the system (a
/// human reading a card), so the trade is not close.
pub struct Normalized {
    text: String,
    /// One entry per byte of `text`: the `(start, end)` byte span in the
    /// ORIGINAL text that this byte's character came from. Every byte of a
    /// multi-byte character carries the same span, as do all the characters a
    /// one-to-many substitution produced (`…` → `...`).
    origin: Vec<(usize, usize)>,
}

impl Normalized {
    /// The normalized text — byte-identical to [`normalize_text`]'s output.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Translate a byte range in normalized space back to the original text.
    ///
    /// The returned range is tight: it starts where the first matched
    /// character started in the original and ends where the last matched
    /// character ended. It deliberately does NOT run to the start of whatever
    /// follows — text dropped by normalization (a stripped gutter numeral, a
    /// collapsed run of whitespace) sits between them, and swallowing it would
    /// silently widen the span.
    ///
    /// # Errors
    /// Returns `None` for an empty range or one that runs past the normalized
    /// text — both mean the caller is holding an index from a different string.
    pub fn to_original(&self, span: Range<usize>) -> Option<Range<usize>> {
        let last = span.end.checked_sub(1)?;
        let start = self.origin.get(span.start)?.0;
        let end = self.origin.get(last)?.1;
        Some(start..end)
    }
}

/// Find `needle` in `haystack`, returning its byte span in `haystack`.
///
/// Two tiers, deliberately the same two the grounding path uses
/// (`find_in_canonical_text`): an exact substring match first, then a match
/// after normalizing both sides. A quote that grounded `normalized` at ingest is
/// found by the second tier here, which is the whole point.
///
/// Returns `None` when the quote is genuinely not on this text — the cross-page
/// case (grounding records the LEFT page of a pair, and the assembler loads that
/// page alone) and the OCR-transposition case both land here. The caller must
/// treat `None` as "no context", never as "context from somewhere nearby": a
/// window drawn around the wrong words would be worse than an empty one, because
/// it would read as evidence.
pub fn locate(haystack: &str, needle: &str) -> Option<Range<usize>> {
    if needle.is_empty() {
        return None;
    }

    // Tier 1 — exact. Covers every `exact`-grounded quote, and costs one scan.
    if let Some(at) = haystack.find(needle) {
        return Some(at..at + needle.len());
    }

    // Tier 2 — normalized. Only reached on a miss, so the mapping work is paid
    // for exactly by the cards that were broken.
    let hay = normalize_with_map(haystack);
    let normalized_needle = normalize_text(needle);
    if normalized_needle.is_empty() {
        return None;
    }
    let at = hay.text().find(&normalized_needle)?;
    hay.to_original(at..at + normalized_needle.len())
}

/// Normalize text for fuzzy matching.
///
/// THE definition of normalization for this codebase, and the single
/// implementation: `api::pipeline::canonical_verifier` re-exports this function,
/// so grounding and context assembly cannot drift apart. Handles:
///
/// * invisible characters (soft hyphen U+00AD, zero-width space U+200B, BOM U+FEFF)
/// * hyphenated line breaks (a word split across lines with a trailing `-`)
/// * transcript gutter line-numbers (standalone 1–2 digit lines)
/// * paragraph markers (`¶`)
/// * smart quotes — single (U+2018/U+2019) and double (U+201C/U+201D)
/// * em dash (U+2014) and en dash (U+2013) → plain hyphen
/// * ellipsis (U+2026) → `...`
/// * ligatures `ﬁ` (U+FB01), `ﬂ` (U+FB02)
/// * whitespace collapse + lowercase
///
/// Order matters — hyphenated breaks must be rejoined and gutter numerals
/// stripped BEFORE the whitespace collapse, because both key on line structure
/// that the collapse destroys.
///
/// Note: character-level OCR errors like split words ("M ilton" → two tokens)
/// will NOT match "Milton" via normalization alone.
pub fn normalize_text(text: &str) -> String {
    render(pipeline(text))
}

/// Normalize, and keep the map back to the original text.
///
/// Same output as [`normalize_text`] — literally the same pipeline — plus the
/// per-byte origin span that [`Normalized::to_original`] reads.
pub fn normalize_with_map(text: &str) -> Normalized {
    let chars = pipeline(text);

    let mut out = String::with_capacity(chars.len());
    let mut origin = Vec::with_capacity(chars.len());
    for ch in chars {
        out.push(ch.c);
        // One entry per byte of the encoded character, so a byte index into
        // `out` indexes straight into `origin`.
        for _ in 0..ch.c.len_utf8() {
            origin.push((ch.start, ch.end));
        }
    }
    Normalized { text: out, origin }
}

/// The normalization pipeline, as characters carrying their origins.
///
/// Every stage is a `Vec<Ch>` → `Vec<Ch>` transform, so the map is maintained by
/// construction rather than by a second pass that could disagree with the first.
fn pipeline(text: &str) -> Vec<Ch> {
    let chars = to_chars(text);
    let chars = drop_invisibles(chars);
    let chars = rejoin_breaks(chars);
    let chars = strip_gutter_lines(chars);
    let chars = substitute(chars);
    let chars = collapse_whitespace(chars);
    lowercase(chars)
}

/// The characters of `text`, each tagged with its own byte span.
fn to_chars(text: &str) -> Vec<Ch> {
    text.char_indices()
        .map(|(start, c)| Ch {
            c,
            start,
            end: start + c.len_utf8(),
        })
        .collect()
}

/// Just the characters, as a `String` — the half of the result callers who do
/// not need the map are asking for.
fn render(chars: Vec<Ch>) -> String {
    chars.into_iter().map(|ch| ch.c).collect()
}

/// Soft hyphen, zero-width space, byte-order mark: present in extracted PDF
/// text, invisible on screen, and fatal to a substring match.
fn drop_invisibles(chars: Vec<Ch>) -> Vec<Ch> {
    chars
        .into_iter()
        .filter(|ch| !matches!(ch.c, '\u{00AD}' | '\u{200B}' | '\u{FEFF}'))
        .collect()
}

/// Rejoin words split across lines by a hyphen.
///
/// Pattern: word character + `-` + optional `\r` + `\n` + optional spaces/tabs +
/// word character. The hyphen and the break are dropped, joining the fragments.
/// Ported character-for-character from the implementation this module replaced,
/// so grounding results are unchanged.
fn rejoin_breaks(chars: Vec<Ch>) -> Vec<Ch> {
    let mut result = Vec::with_capacity(chars.len());
    let mut i = 0;

    while i < chars.len() {
        if chars[i].c == '-' && i > 0 && chars[i - 1].c.is_alphanumeric() {
            let mut j = i + 1;
            if chars.get(j).is_some_and(|ch| ch.c == '\r') {
                j += 1;
            }
            if chars.get(j).is_some_and(|ch| ch.c == '\n') {
                j += 1;
                while chars.get(j).is_some_and(|ch| ch.c == ' ' || ch.c == '\t') {
                    j += 1;
                }
                if chars.get(j).is_some_and(|ch| ch.c.is_alphanumeric()) {
                    // Skip the hyphen and the break; resume at the word's tail.
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

/// Strip standalone gutter line-number lines (transcript margin numerals).
///
/// ## Domain note: line-numbered hearing transcripts
///
/// A court reporter prints a line number (1–25) in the left margin of every line
/// of a hearing transcript. OCR (Surya) and PDF text extraction interleave those
/// numerals into the text stream as standalone lines, e.g.:
///
/// ```text
/// George Phillips on behalf of
/// 14
/// Catholic Family Service, the
/// 15
/// personal representative.
/// ```
///
/// The verbatim quote the LLM emits carries no such numerals. Once whitespace is
/// collapsed the stored page reads `"...behalf of 14 catholic family service..."`
/// while the quote reads `"...behalf of catholic family service..."`, so the
/// quote is no longer a contiguous substring and grounding returns `not_found`.
/// A short, single-line quote never spans a gutter numeral and still matches —
/// which is exactly the observed "short quotes verify, long quotes fail" pattern
/// on the first court_transcript run.
///
/// ## Why scoped to 1–2 digits
///
/// Bounding the strip to 1–2 digit lines avoids removing a legitimately
/// standalone longer number a quote might genuinely span — a year (`2009`), a
/// dollar figure, a 3-digit page number.
///
/// ## Why this is safe for already-verified documents
///
/// A document with no standalone 1–2 digit line has no line to remove, so this
/// returns its text unchanged (see the identity test in `canonical_verifier`).
/// Because normalization runs it on both sides of every comparison, it is a
/// no-op for every non-line-numbered document type.
///
/// Mirrors `str::lines()` + `join("\n")` exactly, including its two quiet
/// behaviours: a `\r\n` becomes `\n`, and a trailing newline is dropped.
fn strip_gutter_lines(chars: Vec<Ch>) -> Vec<Ch> {
    let ends_with_newline = chars.last().is_some_and(|ch| ch.c == '\n');
    let mut lines: Vec<Vec<Ch>> = Vec::new();
    let mut current: Vec<Ch> = Vec::new();

    for ch in chars {
        if ch.c == '\n' {
            // `lines()` treats `\r\n` as one ending, so the carriage return is
            // part of the terminator rather than of the line.
            if current.last().is_some_and(|c| c.c == '\r') {
                current.pop();
            }
            lines.push(std::mem::take(&mut current));
        } else {
            current.push(ch);
        }
    }
    if !ends_with_newline {
        lines.push(current);
    }

    let kept: Vec<Vec<Ch>> = lines.into_iter().filter(|line| !is_gutter(line)).collect();
    join_lines(kept)
}

/// A line that is nothing but a 1–2 digit number, ignoring surrounding space.
///
// CONST: the 1–2 digit bound is a fixed legal-formatting invariant, NOT a
// configurable threshold — deliberately not a ProcessingProfile setting. U.S.
// court reporters number 1–25 lines per page, so a gutter numeral is always 1–2
// digits. A larger bound would strip legitimately standalone years (4 digits),
// page numbers (3 digits), and dollar figures a quote may span — the exact false
// positives this scoping exists to avoid. It cannot vary by document type or
// environment.
fn is_gutter(line: &[Ch]) -> bool {
    let trimmed: Vec<char> = {
        let mut chars: Vec<char> = line.iter().map(|ch| ch.c).collect();
        while chars.first().is_some_and(|c| c.is_whitespace()) {
            chars.remove(0);
        }
        while chars.last().is_some_and(|c| c.is_whitespace()) {
            chars.pop();
        }
        chars
    };
    matches!(trimmed.len(), 1 | 2) && trimmed.iter().all(|c| c.is_ascii_digit())
}

/// Rejoin kept lines with `\n`, giving each inserted newline the span of the
/// character it follows.
///
/// The separator is synthetic — the original newline was consumed by the split —
/// so it is mapped to a zero-width point at the end of the preceding line. It
/// cannot survive to the output anyway: the whitespace collapse two stages later
/// turns it into a space or removes it.
fn join_lines(lines: Vec<Vec<Ch>>) -> Vec<Ch> {
    let mut result: Vec<Ch> = Vec::new();
    for (index, line) in lines.into_iter().enumerate() {
        if index > 0 {
            let at = result.last().map(|ch| ch.end).unwrap_or(0);
            result.push(Ch {
                c: '\n',
                start: at,
                end: at,
            });
        }
        result.extend(line);
    }
    result
}

/// Character substitutions: paragraph marks, smart quotes, dashes, ellipsis,
/// ligatures. One character in, one or more out — every character produced keeps
/// the span of the character it came from, which is what lets `…` → `...` stay
/// mappable.
fn substitute(chars: Vec<Ch>) -> Vec<Ch> {
    let mut result = Vec::with_capacity(chars.len());
    for ch in chars {
        let replacement: &str = match ch.c {
            '¶' => " ",
            '\u{201C}' | '\u{201D}' => "\"",
            '\u{2018}' | '\u{2019}' => "'",
            '\u{2014}' | '\u{2013}' => "-",
            '\u{2026}' => "...",
            '\u{FB01}' => "fi",
            '\u{FB02}' => "fl",
            _ => {
                result.push(ch);
                continue;
            }
        };
        result.extend(replacement.chars().map(|c| Ch { c, ..ch }));
    }
    result
}

/// Collapse every run of whitespace to a single space, and trim both ends —
/// `split_whitespace().join(" ")`, with the origins kept.
fn collapse_whitespace(chars: Vec<Ch>) -> Vec<Ch> {
    let mut result: Vec<Ch> = Vec::with_capacity(chars.len());
    let mut pending: Option<Ch> = None;

    for ch in chars {
        if ch.c.is_whitespace() {
            // A run maps to its FIRST character, and a run before any output is
            // leading whitespace — dropped, as `split_whitespace` drops it.
            if !result.is_empty() && pending.is_none() {
                pending = Some(Ch { c: ' ', ..ch });
            }
        } else {
            if let Some(space) = pending.take() {
                result.push(space);
            }
            result.push(ch);
        }
    }
    // `pending` is deliberately dropped here: a trailing run is trailing
    // whitespace, and it is trimmed.
    result
}

/// Lowercase, one character at a time.
///
/// ## Why per-character and not `str::to_lowercase`
///
/// The origins have to survive, and `str::to_lowercase` returns a fresh `String`
/// with no way to ask which input character produced which output. The one known
/// difference is Greek: `str::to_lowercase` renders a word-final `Σ` as `ς`,
/// where this renders `σ`. It cannot affect a grounding decision — both the
/// quote and the page pass through this same function, so the two sides agree —
/// and no document in this corpus is Greek. Recorded rather than hidden.
fn lowercase(chars: Vec<Ch>) -> Vec<Ch> {
    let mut result = Vec::with_capacity(chars.len());
    for ch in chars {
        result.extend(ch.c.to_lowercase().map(|c| Ch { c, ..ch }));
    }
    result
}

/// [`strip_gutter_lines`] as a string-in, string-out transform.
///
/// ## Two callers, two very different purposes
///
/// 1. **Matching** (since 1.7A): the normalization pipeline runs this on BOTH
///    sides of every comparison, so a quote carrying no gutter numerals still
///    grounds against a page riddled with them.
/// 2. **Display** (task 1.7C, defect D6): the card's quote-in-context must not
///    render the court reporter's margin numerals. Measured on DEV, 87 of the 499
///    locatable cards showed at least one — on the hearing transcript it was
///    every card, because that document carries 26 gutter lines on all 37 pages.
///
/// ## Why this is safe to use for display, stated plainly
///
/// It is a DISPLAY-TIME transform over a string the caller already sliced. It
/// does not touch stored text, it does not run before [`locate`] (which consumes
/// the raw page), and it is never applied to the anchor quote — design v2 §3.4/§3.5
/// and §12: anchors are sacred, only the surrounding context is groomed. The
/// caller ([`crate::services::scenario_card_context`]) enforces that by
/// construction: it strips only the two slices that lie OUTSIDE the located span.
///
/// The gutter tests in `api::pipeline::canonical_verifier` address this function
/// by name, including the disk-scanning identity test over the real fixture pages.
pub fn strip_gutter_line_numbers(text: &str) -> String {
    render(strip_gutter_lines(to_chars(text)))
}

#[cfg(test)]
#[path = "quote_match_tests.rs"]
mod tests;
