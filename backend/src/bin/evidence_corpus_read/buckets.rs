//! Assigning the twelve buckets, and the arithmetic over them.
//!
//! Every rule here is either a call into `norm` (which is tested) or a count of
//! edges the graph reported. Nothing in this module decides what to DELETE — the
//! instruction is explicit that the disposal ruling is Roman's and the
//! architect's, on these numbers.

use std::collections::HashMap;

use crate::model::{Card, Flags, BUCKETS};
use crate::norm::{
    is_answer_token, is_near_duplicate, normalise_quote, ocr_damage, page_unresolvable,
};

/// The four relationship types that connect an `Evidence` card to an
/// `Allegation`, measured by STOP 0.
///
/// ## Domain note: `BEARS_ON` does not exist, and `ABOUT` is overloaded
///
/// The instruction names `BEARS_ON` for the Evidence→Allegation link. STOP 0
/// found no such type in the graph. What exists is `ABOUT` (426 edges to
/// Allegation), `CORROBORATES` (127), `REBUTS` (118) and `CHARACTERIZES` (80) —
/// and `ABOUT` ALSO reaches Person (1,525) and Organization (539), so the same
/// type carries both the party sense and the allegation sense. B5 and B6 are
/// therefore separated by the LABEL at the far end, never by the type alone.
// STRUCTURAL: Neo4j relationship type names are graph SCHEMA vocabulary, not a
// deployment setting. Renaming one is a data migration — every existing edge has
// to be rewritten — so a config knob here could only ever make this query
// disagree with the graph it reads. Observed by STOP 0 and pinned by
// `the_card_query_matches_the_documented_allegation_relationships`.
pub const ALLEGATION_RELS: [&str; 4] = ["ABOUT", "CORROBORATES", "REBUTS", "CHARACTERIZES"];

/// Grounding statuses the instruction names as the suspect classes.
// STRUCTURAL: the literal strings stored in the `grounding_status` property, so
// the same argument as ALLEGATION_RELS applies — these are what the graph holds,
// not a threshold somebody might want to tune per environment. A deployment that
// wanted different words would need the graph rewritten first.
pub const SUSPECT_GROUNDING: [&str; 2] = ["unverified", "derived"];

/// Everything the bucket rules need that is not on the card itself.
pub struct Rules<'a> {
    /// The answer-token set, DERIVED from the corpus (every distinct quote under
    /// the survey length) rather than compiled in.
    pub answer_tokens: &'a [String],
    /// Statement kinds the shipped prefilter drops. Read from `app_settings`,
    /// lower-cased, exactly as `theme_scan_prefilter::dropped_kind` compares them.
    pub dropped_statement_types: &'a [String],
    /// B3's length ratio. Printed beside the count.
    pub near_duplicate_min_ratio: f64,
    /// The ids the `evidence_search` mirror holds with a non-blank `probe_text`.
    ///
    /// `None` means the table does not exist on this database — which is a
    /// different state from "it exists and is empty", and B12 must not collapse
    /// the two (standing Rule 1). When it is `None` no card is flagged, and the
    /// summary says the table is absent in one line.
    pub mirror_ok_ids: Option<&'a std::collections::HashSet<String>>,
}

/// Assign all twelve buckets to every card.
///
/// Returns the flags in card order, plus the duplicate-cluster index B2 built,
/// which the report prints separately.
pub fn assign(cards: &[Card], rules: &Rules<'_>) -> (Vec<Flags>, DuplicateIndex) {
    let duplicates = DuplicateIndex::build(cards);
    let near = near_duplicate_flags(cards, rules.near_duplicate_min_ratio);

    let flags = cards
        .iter()
        .enumerate()
        .map(|(i, card)| {
            let mut f = Flags::default();
            f.0[0] = b1_no_text(card, rules.answer_tokens);
            f.0[1] = duplicates.is_duplicate(&card.normalised());
            f.0[2] = near.get(i).copied().unwrap_or(false);
            f.0[3] = SUSPECT_GROUNDING.contains(&card.grounding_status.trim());
            f.0[4] = card.party_count == 0 || card.unnamed_party_count > 0 || card.party_count > 4;
            f.0[5] = card.allegation_count == 0;
            f.0[6] = b7_cross_reference(card, rules.dropped_statement_types);
            f.0[7] = ocr_damage(&card.quote).any();
            f.0[8] = page_unresolvable(card.page_number, card.doc_page_count);
            f.0[9] = !card.doc_row_exists || card.document_node_count == 0;
            f.0[10] = card.template_name.is_none() || card.model_name.is_none();
            // B12: only meaningful when the mirror exists. With no table there is
            // nothing a card could be missing FROM, so flagging all 1,209 would
            // be true and useless; the absence is reported as a one-line fact.
            f.0[11] = rules.mirror_ok_ids.is_some_and(|ok| !ok.contains(&card.id));
            f
        })
        .collect();

    (flags, duplicates)
}

/// **B1** — blank, or a bare discovery-response answer token.
fn b1_no_text(card: &Card, tokens: &[String]) -> bool {
    card.quote.trim().is_empty() || is_answer_token(&card.quote, tokens)
}

/// **B7** — the shipped prefilter's own rule, not a new one.
///
/// `theme_scan_prefilter::dropped_kind` takes `statement_type`, trims it,
/// lower-cases it, and asks whether the settings list contains it. This is that
/// function's logic applied to the same stored list, so B7 counts exactly the
/// cards the scan already sets aside — no invented text heuristic.
fn b7_cross_reference(card: &Card, dropped: &[String]) -> bool {
    let kind = match card.statement_type.as_deref() {
        Some(k) => k.trim().to_lowercase(),
        None => return false,
    };
    !kind.is_empty() && dropped.contains(&kind)
}

/// **B3** — pairwise containment, bucketed by first word to keep it tractable.
///
/// ## Rust Learning: bucketing to avoid an O(n²) scan of long strings
///
/// A prefix/suffix relation forces the two quotes to share either their first or
/// their last word. Grouping by those two keys turns 1,209² full-string compares
/// into a few hundred small groups; the answer is identical because a pair that
/// shares neither boundary word could never have satisfied the rule.
fn near_duplicate_flags(cards: &[Card], min_ratio: f64) -> Vec<bool> {
    let normalised: Vec<String> = cards.iter().map(Card::normalised).collect();
    let mut by_edge: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, text) in normalised.iter().enumerate() {
        if text.is_empty() {
            continue;
        }
        let mut words = text.split(' ');
        if let Some(first) = words.next() {
            by_edge.entry(format!("s:{first}")).or_default().push(i);
        }
        if let Some(last) = text.rsplit(' ').next() {
            by_edge.entry(format!("e:{last}")).or_default().push(i);
        }
    }

    let mut flags = vec![false; cards.len()];
    for group in by_edge.values() {
        for (position, &i) in group.iter().enumerate() {
            for &j in &group[position + 1..] {
                if flags[i] && flags[j] {
                    continue;
                }
                if is_near_duplicate(&normalised[i], &normalised[j], min_ratio) {
                    flags[i] = true;
                    flags[j] = true;
                }
            }
        }
    }
    flags
}

/// **B2** — clusters of cards sharing one normalised quote.
pub struct DuplicateIndex {
    /// normalised quote -> the card indices carrying it.
    pub clusters: HashMap<String, Vec<usize>>,
}

impl DuplicateIndex {
    /// Group every non-empty normalised quote; keep only the groups of 2+.
    pub fn build(cards: &[Card]) -> Self {
        let mut all: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, card) in cards.iter().enumerate() {
            let key = normalise_quote(&card.quote);
            if !key.is_empty() {
                all.entry(key).or_default().push(i);
            }
        }
        all.retain(|_, members| members.len() > 1);
        Self { clusters: all }
    }

    pub fn is_duplicate(&self, normalised: &str) -> bool {
        self.clusters.contains_key(normalised)
    }

    /// How many cards sit in some cluster.
    pub fn card_count(&self) -> usize {
        self.clusters.values().map(Vec::len).sum()
    }

    /// Clusters sorted largest first, as `(quote, members)`.
    pub fn largest(&self, take: usize) -> Vec<(&String, &Vec<usize>)> {
        let mut all: Vec<(&String, &Vec<usize>)> = self.clusters.iter().collect();
        all.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));
        all.into_iter().take(take).collect()
    }

    /// Split each cluster into true twins (same document AND page) and
    /// cross-references (the same sentence quoted in two different documents).
    ///
    /// Domain note: the instruction is explicit that the second class is NOT
    /// crap — a sentence from a ruling quoted back in a brief is a real
    /// cross-reference, and deleting one of the pair would lose the citation.
    pub fn twin_split(&self, cards: &[Card]) -> (usize, usize) {
        let mut twins = 0usize;
        let mut cross = 0usize;
        for members in self.clusters.values() {
            let mut seen: HashMap<(String, Option<i64>), usize> = HashMap::new();
            for &i in members {
                let card = &cards[i];
                *seen
                    .entry((card.source_document.clone(), card.page_number))
                    .or_insert(0) += 1;
            }
            for count in seen.values() {
                if *count > 1 {
                    twins += count;
                }
            }
            if seen.len() > 1 {
                cross += members.len();
            }
        }
        (twins, cross)
    }
}

/// The overlap matrix: `m[i][j]` = cards in both bucket i and bucket j.
pub fn overlap_matrix(flags: &[Flags]) -> Vec<Vec<usize>> {
    let n = BUCKETS.len();
    let mut matrix = vec![vec![0usize; n]; n];
    for f in flags {
        for (i, row) in matrix.iter_mut().enumerate().take(n) {
            if !f.0[i] {
                continue;
            }
            for (j, cell) in row.iter_mut().enumerate() {
                if f.0[j] {
                    *cell += 1;
                }
            }
        }
    }
    matrix
}

#[cfg(test)]
#[path = "buckets_tests.rs"]
mod tests;
