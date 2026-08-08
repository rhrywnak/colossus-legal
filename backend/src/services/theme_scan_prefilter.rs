//! What reaches the Theme Scan judge, and what is set aside before it — task
//! 2.15 Tier 2 (SCAN_QUALITY_DESIGN_v1, items 1a/1b/1c).
//!
//! The gathering read is deliberately ungated (100% recall by construction), and
//! until this module every one of its rows cost an LLM call. Two classes of row
//! never earned one:
//!
//! * **Byte-identical twins.** The 2026-07-29 S-4 scan admitted three
//!   byte-duplicate PAIRS (C-45/C-46, C-42/C-43, C-9/C-11) — the human was asked
//!   to rule the same sentence twice, and paid twice to be asked.
//! * **Statements structurally incapable of bearing on anything** — an empty
//!   quote, a cross-reference with no assertion ("See the responses to the
//!   previous interrogatories."), a fragment too short to carry a claim and with
//!   no question to give it one.
//!
//! ## Nothing is dropped. Everything is ACCOUNTED FOR (Standing Rule 1)
//!
//! This module never discards a candidate silently. Every row of the pool leaves
//! here in exactly one of three places — a judge group, an exclusion with a named
//! reason, or a duplicate member folded into a group — and
//! [`crate::dto::theme_scan::ScanConservation`] carries the arithmetic that
//! proves it:
//!
//! ```text
//! pool = excluded_empty + excluded_statement_type + excluded_too_short
//!      + duplicates_collapsed + judged
//! ```
//!
//! That identity is asserted in the tests, reported in the log at scan start, and
//! rendered on screen under the run's results. A candidate that went missing would
//! show up as a sum that does not add up, rather than as an absence nobody counted.
//!
//! ## Pure by design
//!
//! No database, no graph, no clock, no settings lookup — the caller resolves the
//! two stored parameters into a [`PrefilterConfig`] and hands them in. Every branch
//! below is therefore unit-testable without a scenario, a pool, or a model, which
//! is what lets the conservation identity be a test rather than a promise.

use std::collections::HashMap;

use crate::bias::dto::BiasInstance;
use crate::dto::theme_scan::ScanConservation;

/// The two stored parameters that decide what is set aside.
///
/// ## Rust Learning: a borrowed slice in a config struct, and the `'a` it needs
///
/// `dropped_statement_types` is `&'a [String]` rather than `Vec<String>` because
/// the caller already owns the list (it lives in the settings snapshot, behind an
/// `Arc`) and this module only READS it. Borrowing avoids cloning a vector per
/// scan; the lifetime `'a` is the compiler's way of being told "this struct may
/// not outlive the settings snapshot it points into", which is exactly true — it
/// is built and consumed inside one call to [`prepare_pool`].
pub(crate) struct PrefilterConfig<'a> {
    /// A quote shorter than this, WITH NO PAIRED QUESTION, never reaches the
    /// judge. `0` disables the rule (no quote is shorter than zero characters).
    ///
    /// Domain note: the no-question clause is the whole safety of this rule. A
    /// bare "That would be correct." is four words and decisive when the
    /// interrogatory above it is in evidence — the judge is shown both (see
    /// `theme_scan_judge::build_user_message`), so a short answer WITH a question
    /// is a full candidate and survives.
    pub min_chars: usize,
    /// Statement kinds that carry no assertion, lower-cased by the caller. Empty
    /// disables the rule.
    pub dropped_statement_types: &'a [String],
}

/// Why one candidate never reached the judge.
///
/// A closed enum rather than a free string: these are the reasons this build
/// implements, each one is counted separately in the conservation block, and a new
/// reason must therefore be added HERE — where the counter and the on-screen line
/// are forced to grow with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExclusionReason {
    /// No quote text at all, or only whitespace.
    Empty,
    /// The statement's kind is one the settings row names as content-free.
    StatementType(String),
    /// Shorter than the configured minimum AND carrying no question.
    TooShortNoQuestion { chars: usize },
}

impl ExclusionReason {
    /// The stable token used in logs. Not shown to a human — the on-screen line
    /// is composed from a stored template (see `services::scan_conservation`).
    pub(crate) fn code(&self) -> &'static str {
        match self {
            ExclusionReason::Empty => "empty",
            ExclusionReason::StatementType(_) => "statement_type",
            ExclusionReason::TooShortNoQuestion { .. } => "too_short_no_question",
        }
    }
}

/// One unit of judging: the quote that is sent, plus every node id it speaks for.
///
/// `members` always contains `representative.evidence_id` first, so a group of
/// one is not a special case anywhere downstream. For a collapsed set the extra
/// ids are the byte-identical twins, and they matter twice: each gets its own
/// `scan_run_verdicts` row (so the scorecard's `graph_node_id` join finds it), and
/// a merge of this pick rules the whole set at once (Roman's ruling R2).
pub(crate) struct CandidateGroup {
    pub representative: BiasInstance,
    pub members: Vec<String>,
}

impl CandidateGroup {
    /// How many pool rows this group speaks for (1 when nothing was collapsed).
    pub(crate) fn size(&self) -> usize {
        self.members.len()
    }
}

/// One candidate set aside, with the reason it was.
pub(crate) struct Excluded {
    pub candidate: BiasInstance,
    pub reason: ExclusionReason,
}

/// The pool, sorted into what is judged and what is not.
pub(crate) struct PreparedPool {
    pub groups: Vec<CandidateGroup>,
    pub excluded: Vec<Excluded>,
    pub conservation: ScanConservation,
}

/// Sort one gathered pool into judge groups and named exclusions.
///
/// Input order is PRESERVED (the gather query returns `ORDER BY e.id`, and the
/// results list shows picks in candidate order), so two scans of an unchanged pool
/// judge the same quotes in the same sequence.
///
/// ## Why exclusions run before de-duplication
///
/// A content-free row is content-free whether or not it has a twin, and counting
/// it once as "excluded" is more useful than counting it once as "collapsed" and
/// once as "excluded". Running exclusions first also means the dedup key is never
/// an empty string — every survivor has real text to group on.
pub(crate) fn prepare_pool(
    candidates: Vec<BiasInstance>,
    config: PrefilterConfig<'_>,
) -> PreparedPool {
    let pool = candidates.len();
    let mut excluded: Vec<Excluded> = Vec::new();
    let mut survivors: Vec<BiasInstance> = Vec::with_capacity(pool);

    for candidate in candidates {
        match exclusion_for(&candidate, &config) {
            Some(reason) => excluded.push(Excluded { candidate, reason }),
            None => survivors.push(candidate),
        }
    }

    let groups = collapse_exact_duplicates(survivors);

    let conservation = ScanConservation {
        pool,
        excluded_empty: count_reason(&excluded, "empty"),
        excluded_statement_type: count_reason(&excluded, "statement_type"),
        excluded_too_short: count_reason(&excluded, "too_short_no_question"),
        // Every member beyond each group's representative is a pool row that will
        // not cost an LLM call — and WILL still get a verdict row (ruling R2).
        duplicates_collapsed: groups.iter().map(|g| g.size() - 1).sum(),
        judged: groups.len(),
    };

    PreparedPool {
        groups,
        excluded,
        conservation,
    }
}

/// Count the exclusions carrying one reason code.
fn count_reason(excluded: &[Excluded], code: &str) -> usize {
    excluded.iter().filter(|e| e.reason.code() == code).count()
}

/// Decide whether one candidate is set aside, and why. `None` = it is judged.
///
/// The order of the three tests is the order of specificity, and it decides which
/// reason a row is COUNTED under when more than one applies (a referral is usually
/// also short). Most specific first: no text at all, then a kind that cannot carry
/// an assertion, then the length heuristic — which is the only one of the three
/// that is a judgement call rather than a structural fact.
fn exclusion_for(
    candidate: &BiasInstance,
    config: &PrefilterConfig<'_>,
) -> Option<ExclusionReason> {
    let quote = candidate.verbatim_quote.as_deref().unwrap_or("").trim();
    if quote.is_empty() {
        return Some(ExclusionReason::Empty);
    }

    if let Some(kind) = dropped_kind(candidate, config) {
        return Some(ExclusionReason::StatementType(kind));
    }

    // `chars().count()`, not `len()`: `len()` is BYTES, so a quote of curly
    // quotation marks and accented names would measure longer than it reads and
    // slip past a threshold a human set by eye.
    let chars = quote.chars().count();
    if config.min_chars > 0 && chars < config.min_chars && !has_question(candidate) {
        return Some(ExclusionReason::TooShortNoQuestion { chars });
    }

    None
}

/// The candidate's statement kind, when the settings row names it as droppable.
///
/// Compared case-insensitively against the caller's already-lower-cased list: the
/// extractor writes `referral`, but a human editing the settings row may well type
/// `Referral`, and a setting that silently does nothing because of a capital
/// letter is the quiet failure this comparison exists to prevent.
fn dropped_kind(candidate: &BiasInstance, config: &PrefilterConfig<'_>) -> Option<String> {
    let kind = candidate.statement_type.as_deref()?.trim().to_lowercase();
    if kind.is_empty() {
        return None;
    }
    config
        .dropped_statement_types
        .contains(&kind)
        .then_some(kind)
}

/// Whether a paired interrogatory question gives a short answer its meaning.
///
/// An empty-string question counts as ABSENT, matching how the judge's user
/// message treats it — a `question` property holding `""` carries no interpretive
/// value, and the two must not disagree about whether a bare "Yes" is rulable.
fn has_question(candidate: &BiasInstance) -> bool {
    candidate
        .question
        .as_deref()
        .map(str::trim)
        .is_some_and(|q| !q.is_empty())
}

/// Fold byte-identical quotes into one group each, preserving first-seen order.
///
/// ## Why EXACT text and nothing cleverer
///
/// Near-duplicate detection (embeddings, normalised whitespace, OCR-repair) is
/// Tier 3 and needs coverage this pool has not been measured for. Exact equality
/// is free, has no false positives, and the measurement found 12 texts / 26 rows /
/// 14 redundant rows in the S-2 pool — the whole of the observed harm.
///
/// ## Rust Learning: `HashMap<&str, usize>` borrowing from the vector being built
///
/// The index map would like to borrow its keys from the groups vector, but that
/// vector is being pushed to in the same loop — two live borrows of the same data,
/// one mutable. The lifetime that resolves it is the candidate's own text: the key
/// is an owned `String` cloned once per DISTINCT quote (not per row), which is
/// cheap and lets the borrow checker see two independent structures.
fn collapse_exact_duplicates(survivors: Vec<BiasInstance>) -> Vec<CandidateGroup> {
    let mut groups: Vec<CandidateGroup> = Vec::with_capacity(survivors.len());
    let mut index: HashMap<String, usize> = HashMap::new();

    for candidate in survivors {
        // Exclusions ran first, so every survivor has non-empty text here.
        let key = candidate.verbatim_quote.clone().unwrap_or_default();
        match index.get(&key) {
            Some(&position) => groups[position].members.push(candidate.evidence_id),
            None => {
                index.insert(key, groups.len());
                groups.push(CandidateGroup {
                    members: vec![candidate.evidence_id.clone()],
                    representative: candidate,
                });
            }
        }
    }
    groups
}

/// Log what the pre-filter did, by reason, before a single LLM call is made.
///
/// Separate from [`prepare_pool`] so that function stays pure and testable. The
/// counts are also persisted (in the run summary) and rendered — this line is for
/// the operator watching a scan start, who otherwise sees only the judged count
/// and no way to tell a small pool from a heavily filtered one.
pub(crate) fn log_prefilter(scenario_id: uuid::Uuid, prepared: &PreparedPool) {
    // Per-item, at debug: the counts say HOW MANY were set aside, and only this
    // says WHICH. Without it, a human asking "why was C-51 never scored?" has a
    // number and no way to answer (Standing Rule 1).
    for excluded in &prepared.excluded {
        tracing::debug!(
            %scenario_id,
            evidence_id = %excluded.candidate.evidence_id,
            reason = excluded.reason.code(),
            "theme scan: candidate set aside before judging"
        );
    }

    let c = &prepared.conservation;
    tracing::info!(
        %scenario_id,
        pool = c.pool,
        excluded_empty = c.excluded_empty,
        excluded_statement_type = c.excluded_statement_type,
        excluded_too_short = c.excluded_too_short,
        duplicates_collapsed = c.duplicates_collapsed,
        judged = c.judged,
        "theme scan: pool prepared (pool = excluded + collapsed + judged)"
    );
}

#[cfg(test)]
#[path = "theme_scan_prefilter_tests.rs"]
mod tests;
