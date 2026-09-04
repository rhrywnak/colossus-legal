//! The input file, the guard, and the two pure predicates the guard leans on.
//!
//! Nothing in this module opens a socket. That is deliberate: the whole point of
//! a one-off write bin is that the DECISION to write is testable without a
//! database, so `guard` takes the node it found as a plain struct and every STOP
//! it can produce has a test beside it.

use serde::Deserialize;

/// One corrected card, straight out of `OCR_REPAIR_v1.json`'s `apply` array.
///
/// ## Rust Learning: `#[derive(Deserialize)]` and field names
///
/// serde maps JSON keys to field names by exact string match unless told
/// otherwise. The five keys below are named exactly as the audit file writes
/// them, so no `#[serde(rename)]` is needed — and an audit file that grows a
/// sixth key deserialises fine, because serde ignores unknown fields by default.
/// That is the forward-compatibility CLAUDE.md §7 asks for.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Repair {
    pub id: String,
    pub document: String,
    pub page: i64,
    pub how: String,
    pub old_quote: String,
    pub new_quote: String,
}

/// A card the audit named but did NOT correct. These two arrays carry only the
/// locating keys — no `how`, no quotes — so they get their own, narrower struct
/// rather than a `Repair` with three fields that would always be absent.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Untouched {
    pub id: String,
    pub document: String,
    pub page: i64,
}

/// The audit file. Only `apply` is written from; the other two arrays are read
/// so the run can PROVE it left them alone — `false_alarm_dash_only`'s ids are
/// what the B8 re-count is checked against afterwards — and so a reader of this
/// struct can see they were deliberately skipped rather than forgotten.
#[derive(Debug, Deserialize)]
pub struct RepairFile {
    pub apply: Vec<Repair>,
    #[serde(default)]
    pub false_alarm_dash_only: Vec<Untouched>,
    #[serde(default)]
    pub pending_missing_pdf: Vec<Untouched>,
}

/// What the graph currently holds for one card. Built from a Cypher row, or by
/// hand in a test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeState {
    pub source_document: String,
    pub page_number: Option<i64>,
    pub quote: String,
}

/// Every way one card can refuse to be repaired.
///
/// ## Rust Learning: `thiserror` on an enum whose variants carry the evidence
///
/// Each variant holds the values that made it fire, so the `Display` text names
/// them. A `Stop::QuoteChanged` that said only "quote changed" would satisfy the
/// compiler and fail standing Rule 1 — the operator needs to see BOTH strings to
/// judge whether the audit is stale or the node was tampered with.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Stop {
    #[error("no Evidence node has id {id}")]
    NotFound { id: String },

    #[error("id {id} matched {found} Evidence nodes; exactly one was required")]
    NotUnique { id: String, found: usize },

    #[error("id {id} sits on source_document {actual:?}, but the audit says {expected:?}")]
    WrongDocument {
        id: String,
        expected: String,
        actual: String,
    },

    #[error("id {id} is on page {actual:?}, but the audit says page {expected}")]
    WrongPage {
        id: String,
        expected: i64,
        actual: Option<i64>,
    },

    #[error(
        "id {id} no longer holds the quote the audit read — somebody changed it since 2026-09-04.\n\
         audit old_quote (normalised): {expected}\n\
         node  verbatim_quote (normalised): {actual}"
    )]
    QuoteChanged {
        id: String,
        expected: String,
        actual: String,
    },
}

/// Collapse every run of whitespace to one space and trim.
///
/// ## Why this does NOT casefold
///
/// `evidence_corpus_read::norm::normalise_quote` lowercases as well, because it
/// is answering "do two cards say the same thing" for a census. This is a WRITE
/// guard: it is asking "is the node still exactly what the audit read", and a
/// node whose text differs only in case IS a node somebody edited. Casefolding
/// here would wave that through.
///
/// ## Rust Learning: `split_whitespace` is trim-and-collapse in one pass
///
/// It splits on any run of Unicode whitespace and never yields an empty piece,
/// so joining its pieces with a single space does both jobs with no regex and no
/// manual state machine. Newlines, tabs and the double spaces a court reporter
/// leaves after a colon all fold to the same thing.
pub fn normalise(quote: &str) -> String {
    quote.split_whitespace().collect::<Vec<&str>>().join(" ")
}

/// The read-verify half of the write. Returns `Ok(())` only when the node found
/// is, beyond doubt, the node the audit read.
///
/// `found` is the full set of rows the `MATCH` returned, so "zero nodes" and
/// "two nodes" are different STOPs rather than one collapsed "not exactly one" —
/// standing Rule 1's "operationally distinct states produce different
/// observables", applied to a query result.
pub fn guard(repair: &Repair, found: &[NodeState]) -> Result<(), Stop> {
    let node = match found {
        [] => {
            return Err(Stop::NotFound {
                id: repair.id.clone(),
            })
        }
        [one] => one,
        many => {
            return Err(Stop::NotUnique {
                id: repair.id.clone(),
                found: many.len(),
            })
        }
    };
    if node.source_document != repair.document {
        return Err(Stop::WrongDocument {
            id: repair.id.clone(),
            expected: repair.document.clone(),
            actual: node.source_document.clone(),
        });
    }
    if node.page_number != Some(repair.page) {
        return Err(Stop::WrongPage {
            id: repair.id.clone(),
            expected: repair.page,
            actual: node.page_number,
        });
    }
    let (expected, actual) = (normalise(&repair.old_quote), normalise(&node.quote));
    if expected != actual {
        return Err(Stop::QuoteChanged {
            id: repair.id.clone(),
            expected,
            actual,
        });
    }
    Ok(())
}

/// **B8**, replicated from `evidence_corpus_read::norm::ocr_damage`.
///
/// ## Why this is a copy and not a call
///
/// `evidence_corpus_read` is a BINARY, not a library: Cargo gives one bin no way
/// to import another's modules. The instruction anticipated this and allowed
/// either — "call its function or replicate its three regexes". The three
/// signatures are reproduced verbatim, and `the_b8_replica_matches_the_audit`
/// below pins each one to the worked example from
/// `CC-REPORTS/transcript_grounding_classification.md` §C1 that the original
/// tests use, so a silent drift between the two copies fails a test rather than
/// changing a number in a report.
///
/// The three signatures: a `-` immediately before a newline (a mid-word line
/// break swept into the quote); `--` anywhere in the text (a transposed line
/// landing inside a hyphenated split); and a line that is nothing but digits (a
/// swallowed gutter numeral).
pub fn has_ocr_damage(quote: &str) -> bool {
    let mid_word_break = quote
        .chars()
        .zip(quote.chars().skip(1))
        .any(|(a, b)| a == '-' && b == '\n');
    let double_hyphen_join = quote.contains("--");
    let stray_gutter_numeral = quote
        .lines()
        .any(|line| !line.trim().is_empty() && line.trim().chars().all(|c| c.is_ascii_digit()));
    mid_word_break || double_hyphen_join || stray_gutter_numeral
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
