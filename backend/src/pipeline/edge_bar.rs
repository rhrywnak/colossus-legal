//! The pre-ingest edge bar — the mechanical half of the 2026-08-25 rulings.
//!
//! Pass 2 asks an LLM for relationships and, until this module existed, every
//! edge it returned was stored if (and only if) both endpoint keys resolved.
//! There was no type allowlist, no duplicate check and no way for an operator to
//! see what a run had produced beyond a single total. The census of
//! `doc-penzien-coa-brief-03-14-2011` measured what that costs: 457 published
//! edges for 62 assertions, of which 66 CHARACTERIZES sat on a pair that already
//! carried an ABOUT — a 100% duplication rate that no code could have caught.
//!
//! This module is the gate. It is **pure**: no database, no graph, no clock. It
//! takes the candidate edges and the schema's rules and returns a verdict per
//! edge, which is what makes every rule here testable without a fixture run.
//!
//! ## The three rules, in the order they are applied
//!
//! 1. **Exact duplicate** — a second edge with the same
//!    `(from, type, to)` inside one document's output is a no-op. The first
//!    wins; later ones are counted and dropped at `debug`. Two identical edges
//!    were never two facts.
//! 2. **Bar B — CHARACTERIZES replaces ABOUT on a pair.** If one document emits
//!    both `CHARACTERIZES(s → t)` and `ABOUT(s → t)`, the ABOUT is dropped.
//!    CHARACTERIZES already establishes that `s` concerns `t` and adds an
//!    evaluative claim on top, so the pair keeps the stronger edge alone.
//!    Order-independent: the ABOUT loses whether it arrived first or second.
//! 3. **Pattern allowlist** — each surviving edge is checked against the
//!    schema's `valid_patterns` for the document type.
//!
//! ## Rust Learning: why this returns verdicts instead of a filtered `Vec`
//!
//! The obvious signature is `fn apply(edges) -> Vec<Edge>`. It is the wrong one,
//! because the caller must be able to say *what it removed and why* — Standing
//! Rule 1's "every operationally distinct state produces a different
//! observable". Returning [`EdgeVerdict`] per input index keeps the rejection
//! reason attached to the edge that earned it, so the caller can log each one
//! with its endpoints and count them by class. A filtered `Vec` would make
//! "rejected 40 edges" and "the model only sent 10" indistinguishable.

use std::collections::{HashMap, HashSet};

/// A supersession rule: `(weaker_type, stronger_type)`.
///
/// When both edges exist on one `(source, target)` pair inside one document, the
/// weaker one is dropped. `("ABOUT", "CHARACTERIZES")` is this case's instance
/// of it — but those names are the CASE's ontology, not this module's, so they
/// arrive as a parameter from the call site rather than as constants here. That
/// is what lets another Colossus project use this filter with zero code changes:
/// the mechanic ("one pair, one edge, the stronger wins") is generic; the
/// vocabulary is not (Standing Rule 2's reusability checkpoint).
pub type SupersedeRule = (String, String);

/// How hard the pattern allowlist bites.
///
/// ## Why this is a mode and not simply "on"
///
/// Measured 2026-08-25: seven of the eleven schema files declare **two**
/// `valid_patterns` while declaring **six** `relationship_types`. The lists were
/// authored for `ExtractionSchema::validate`'s boot-time self-check, never as a
/// complete statement of which edges are legal — the appellate schema, for
/// instance, lists `Evidence ABOUT Party` and `Evidence STATED_BY Party` and
/// nothing else, while its own template mandates `ABOUT → Allegation`,
/// `CHARACTERIZES` and `REBUTS`. Rejecting against that list today would have
/// discarded 155 of the Penzien brief's 457 edges, including every one of the
/// 111 that reach an Allegation — the only part of that document the census
/// found substantive.
///
/// So the check ships in [`Self::ReportOnly`]: it runs, it counts, it logs, and
/// it stores the edge anyway. Completing the schemas is a data-authoring job
/// with its own review; [`Self::Enforce`] is what that job turns on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternMode {
    /// Count and log allowlist misses; store the edge regardless.
    ReportOnly,
    /// Reject allowlist misses.
    Enforce,
}

/// One candidate edge as pass 2 emitted it, before any endpoint resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeCandidate {
    /// The LLM's `from_entity` key (e.g. `"evidence-014"`).
    pub from_key: String,
    /// The LLM's `to_entity` key.
    pub to_key: String,
    /// The relationship type name as emitted.
    pub rel_type: String,
}

/// Why an edge did not survive the bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectReason {
    /// An identical `(from, type, to)` already appeared in this output.
    ExactDuplicate,
    /// A supersession rule fired: a stronger edge holds the same `(from, to)`
    /// pair.
    SupersededBy {
        /// The stronger relationship type that kept the pair. Carried so the log
        /// names the winner instead of assuming the reader knows which rule was
        /// configured.
        stronger: String,
    },
    /// The `(from_type, rel_type, to_type)` triple is absent from the
    /// document type's `valid_patterns`. Carries the triple it looked for, so
    /// the log names the pattern an author would have to add.
    PatternNotAllowed {
        /// Resolved entity type of the source, or `"?"` when unknown.
        from_type: String,
        /// Resolved entity type of the target, or `"?"` when unknown.
        to_type: String,
    },
}

impl RejectReason {
    /// Whether an operator needs to SEE this removal, or merely be able to find
    /// it.
    ///
    /// [`Self::ExactDuplicate`] is `false`: two identical edges were never two
    /// facts, so collapsing them is a declared no-op and belongs at `debug`. The
    /// other two removed something the model actually asked for — a Bar-B
    /// casualty or an allowlist rejection — and belong at `warn`, because
    /// "nothing was removed" and "an edge you may have wanted was removed" are
    /// different states an operator must be able to tell apart from the log
    /// alone (Standing Rule 1).
    ///
    /// Extracted as its own function so the level choice is a decision that can
    /// be asserted, rather than a `match` arm buried in a logging call that only
    /// a subscriber could observe.
    pub fn is_operator_visible(&self) -> bool {
        !matches!(self, RejectReason::ExactDuplicate)
    }
}

/// The bar's decision for one input edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeVerdict {
    /// Store it.
    Accept,
    /// Store it, but an allowlist miss was recorded ([`PatternMode::ReportOnly`]).
    AcceptWithPatternWarning {
        /// Resolved entity type of the source, or `"?"` when unknown.
        from_type: String,
        /// Resolved entity type of the target, or `"?"` when unknown.
        to_type: String,
    },
    /// Do not store it.
    Reject(RejectReason),
}

/// Per-class tallies, for the run record and the operator log.
///
/// Every field is a count of edges the model produced that did not reach the
/// database, except [`pattern_warnings`](Self::pattern_warnings), which counts
/// edges that DID reach it while failing the allowlist. Keeping those two apart
/// is the whole point of [`PatternMode`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EdgeBarCounts {
    /// Edges that survived.
    pub accepted: usize,
    /// Dropped by rule 1.
    pub exact_duplicates: usize,
    /// Dropped by rule 2 (Bar B).
    pub deduped: usize,
    /// Dropped by rule 3 under [`PatternMode::Enforce`].
    pub rejected_by_pattern: usize,
    /// Allowlist misses that were stored anyway under [`PatternMode::ReportOnly`].
    pub pattern_warnings: usize,
}

/// The bar's output: one verdict per input edge, in input order, plus tallies.
#[derive(Debug, Clone)]
pub struct EdgeBarOutcome {
    /// Same length as the input slice; index `i` is the verdict for edge `i`.
    pub verdicts: Vec<EdgeVerdict>,
    /// Roll-up of `verdicts`, for logging and the run record.
    pub counts: EdgeBarCounts,
}

impl EdgeBarOutcome {
    /// True when nothing was removed and nothing was flagged — the shape a
    /// caller can log as "the bar had no effect" rather than staying silent.
    pub fn is_clean(&self) -> bool {
        self.counts.exact_duplicates == 0
            && self.counts.deduped == 0
            && self.counts.rejected_by_pattern == 0
            && self.counts.pattern_warnings == 0
    }
}

/// An allowlist entry: `(from_type, rel_type, to_type)`.
pub type PatternTriple = (String, String, String);

/// What [`filter_pass2_payload`] hands back.
///
/// ## Rust Learning: a named struct instead of a 4-tuple
///
/// The obvious return is `(Value, EdgeBarOutcome, Vec<_>, Vec<_>)`. Clippy's
/// `type_complexity` lint would object, but the real reason is the caller: `.2`
/// and `.3` say nothing about which list is the removals and which is the
/// stored-but-flagged, and logging one at the other's level is exactly the
/// mistake this module exists to prevent. Naming them makes that mix-up hard.
pub struct FilteredPayload {
    /// The payload carrying only the surviving relationships — what storage gets.
    pub payload: serde_json::Value,
    /// Verdicts and tallies for the whole document.
    pub outcome: EdgeBarOutcome,
    /// `(index into the ORIGINAL array, why)` for every REMOVED edge.
    pub rejections: Vec<(usize, RejectReason)>,
    /// `(index, from_type, to_type)` for every edge STORED despite failing the
    /// allowlist. Empty under [`PatternMode::Enforce`], where those edges become
    /// rejections instead. Under the shipped [`PatternMode::ReportOnly`] this is
    /// the list an operator acts on, so it names each edge rather than only
    /// totalling them.
    pub pattern_warnings: Vec<(usize, String, String)>,
}

/// Apply the three rules to one document's pass-2 relationship output.
///
/// * `edges` — the candidates, in the order the model returned them.
/// * `entity_type_of` — key → resolved entity type, for the pattern check. A key
///   the map does not know yields `"?"` and, under [`PatternMode::Enforce`], a
///   rejection: an edge whose endpoint type cannot be established has not been
///   shown to be legal. (Endpoints that do not resolve at all are dropped later,
///   by the storage layer, for a different reason.)
/// * `valid_patterns` — the document type's allowlist, already normalised.
/// * `mode` — see [`PatternMode`].
///
/// ## Rust Learning: two passes, and why the first one cannot be folded in
///
/// Bar B needs to know whether a CHARACTERIZES exists on a pair *before* it can
/// judge an ABOUT that arrives earlier in the list. A single pass would make the
/// outcome depend on emission order — the same document, re-run, could keep the
/// ABOUT or drop it. So pass one collects the characterized pairs into a
/// `HashSet`, and pass two judges every edge against a set that is already
/// complete. Determinism here is not tidiness: it is what makes the mutation
/// proofs meaningful.
pub fn apply_edge_bar(
    edges: &[EdgeCandidate],
    entity_type_of: &HashMap<String, String>,
    valid_patterns: &[PatternTriple],
    supersede: Option<&SupersedeRule>,
    mode: PatternMode,
) -> EdgeBarOutcome {
    // Pass 1 — every (from, to) pair that carries a CHARACTERIZES anywhere in
    // this output. Built first so rule 2 is order-independent.
    let superseding_pairs: HashSet<(&str, &str)> = match supersede {
        Some((_, stronger)) => edges
            .iter()
            .filter(|e| &e.rel_type == stronger)
            .map(|e| (e.from_key.as_str(), e.to_key.as_str()))
            .collect(),
        // No rule configured: rule 2 is inert and the other two still apply.
        None => HashSet::new(),
    };

    let allow: HashSet<(&str, &str, &str)> = valid_patterns
        .iter()
        .map(|(f, r, t)| (f.as_str(), r.as_str(), t.as_str()))
        .collect();

    let mut seen: HashSet<(&str, &str, &str)> = HashSet::new();
    let mut verdicts = Vec::with_capacity(edges.len());
    let mut counts = EdgeBarCounts::default();

    for edge in edges {
        let triple = (
            edge.from_key.as_str(),
            edge.rel_type.as_str(),
            edge.to_key.as_str(),
        );

        // Rule 1 — exact duplicate. `insert` returns false when already present,
        // which is the check and the record in one call.
        if !seen.insert(triple) {
            counts.exact_duplicates += 1;
            verdicts.push(EdgeVerdict::Reject(RejectReason::ExactDuplicate));
            continue;
        }

        // Rule 2 — Bar B. Only ABOUT ever loses, and only to a CHARACTERIZES on
        // the identical pair.
        if let Some((weaker, stronger)) = supersede {
            if &edge.rel_type == weaker
                && superseding_pairs.contains(&(edge.from_key.as_str(), edge.to_key.as_str()))
            {
                counts.deduped += 1;
                verdicts.push(EdgeVerdict::Reject(RejectReason::SupersededBy {
                    stronger: stronger.clone(),
                }));
                continue;
            }
        }

        // Rule 3 — the allowlist. `"?"` for an endpoint whose type is unknown,
        // so the log says which side could not be established rather than
        // reporting a mismatch against an empty string.
        let from_type = entity_type_of
            .get(&edge.from_key)
            .map(String::as_str)
            .unwrap_or("?");
        let to_type = entity_type_of
            .get(&edge.to_key)
            .map(String::as_str)
            .unwrap_or("?");

        if allow.contains(&(from_type, edge.rel_type.as_str(), to_type)) {
            counts.accepted += 1;
            verdicts.push(EdgeVerdict::Accept);
            continue;
        }

        match mode {
            PatternMode::Enforce => {
                counts.rejected_by_pattern += 1;
                verdicts.push(EdgeVerdict::Reject(RejectReason::PatternNotAllowed {
                    from_type: from_type.to_string(),
                    to_type: to_type.to_string(),
                }));
            }
            PatternMode::ReportOnly => {
                counts.accepted += 1;
                counts.pattern_warnings += 1;
                verdicts.push(EdgeVerdict::AcceptWithPatternWarning {
                    from_type: from_type.to_string(),
                    to_type: to_type.to_string(),
                });
            }
        }
    }

    EdgeBarOutcome { verdicts, counts }
}

/// Build candidates from one pass-2 `relationships` array and return the same
/// JSON with only the accepted edges, alongside the outcome.
///
/// ## Why the filter returns JSON rather than editing in place
///
/// The storage function this feeds takes the parsed payload, and the SAME
/// payload is read a second time by the cross-tier writer, which resolves
/// authored (`ctx:element-*`) endpoints the bar cannot type. Handing storage a
/// filtered copy and leaving the original intact keeps one concern from silently
/// truncating the other's input — the class of bug this whole module exists to
/// stop.
///
/// `relationships` missing or not an array yields an empty outcome and a payload
/// echoing the input unchanged: "the model returned no relationships" and "the
/// bar removed them all" stay distinguishable in the counts (Standing Rule 1).
pub fn filter_pass2_payload(
    parsed: &serde_json::Value,
    resolve: impl Fn(&serde_json::Value) -> (String, String, String),
    entity_type_of: &HashMap<String, String>,
    valid_patterns: &[PatternTriple],
    supersede: Option<&SupersedeRule>,
    mode: PatternMode,
) -> FilteredPayload {
    let Some(rels) = parsed.get("relationships").and_then(|v| v.as_array()) else {
        return FilteredPayload {
            payload: parsed.clone(),
            outcome: EdgeBarOutcome {
                verdicts: Vec::new(),
                counts: EdgeBarCounts::default(),
            },
            rejections: Vec::new(),
            pattern_warnings: Vec::new(),
        };
    };

    let candidates: Vec<EdgeCandidate> = rels
        .iter()
        .map(|r| {
            let (from_key, to_key, rel_type) = resolve(r);
            EdgeCandidate {
                from_key,
                to_key,
                rel_type,
            }
        })
        .collect();

    let outcome = apply_edge_bar(&candidates, entity_type_of, valid_patterns, supersede, mode);

    let mut kept = Vec::with_capacity(rels.len());
    let mut rejections = Vec::new();
    // Collected separately from `rejections` because these edges WERE stored.
    // In `ReportOnly` — the shipped default — this is the list an operator acts
    // on, so it must name each edge rather than only totalling them.
    let mut pattern_warnings = Vec::new();
    for (i, verdict) in outcome.verdicts.iter().enumerate() {
        match verdict {
            EdgeVerdict::Accept => kept.push(rels[i].clone()),
            EdgeVerdict::AcceptWithPatternWarning { from_type, to_type } => {
                kept.push(rels[i].clone());
                pattern_warnings.push((i, from_type.clone(), to_type.clone()));
            }
            EdgeVerdict::Reject(reason) => rejections.push((i, reason.clone())),
        }
    }

    let mut filtered = parsed.clone();
    if let Some(obj) = filtered.as_object_mut() {
        obj.insert("relationships".to_string(), serde_json::Value::Array(kept));
    }
    FilteredPayload {
        payload: filtered,
        outcome,
        rejections,
        pattern_warnings,
    }
}

#[cfg(test)]
#[path = "edge_bar_tests.rs"]
mod tests;
