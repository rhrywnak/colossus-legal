//! The printed block and the CSV — the two artefacts the architect reads.
//!
//! Printing is separated from measuring so that the numbers cannot be shaped by
//! the thing that formats them. Everything here takes finished score vectors and
//! renders them; it computes no verdict it was not handed.

use crate::fixture::{Card, Fixture};
use crate::pure::{recall_at, Surface};
use std::fmt::Write as _;

/// The two gate thresholds, from the ruling (design §0.4).
///
/// Parameters rather than constants for the same reason the fixture counts are:
/// 60/40/20 is this ruling's bar, and a bin that could only ever test this bar
/// would have to be edited to test the next one.
#[derive(Debug, Clone, Copy)]
pub struct GateBars {
    /// Gate A's cut-off — the reranker's top `a_k`.
    pub a_k: usize,
    /// Gate A's floor — at least this many of `opus_relevant` inside `a_k`.
    pub a_min: usize,
    /// Gate B's cut-off — the top `b_k` must contain EVERY Included card.
    pub b_k: usize,
}

/// One surface's finished measurement.
pub struct SurfaceRun {
    pub surface: Surface,
    /// Score per candidate, in fixture order.
    pub candidate_scores: Vec<f64>,
    /// Rank per candidate, in fixture order. 1 = best.
    pub candidate_ranks: Vec<usize>,
    /// Score per `outside_pool` card, in fixture order.
    pub pool_scores: Vec<f64>,
    /// The rank each `outside_pool` card WOULD take if inserted.
    pub pool_would_be_ranks: Vec<usize>,
    pub elapsed_secs: f64,
}

impl SurfaceRun {
    /// Pairs scored per second — candidates plus outside-pool cards.
    pub fn pairs_per_second(&self) -> f64 {
        let pairs = (self.candidate_scores.len() + self.pool_scores.len()) as f64;
        if self.elapsed_secs > 0.0 {
            pairs / self.elapsed_secs
        } else {
            f64::INFINITY
        }
    }
}

/// PASS/FAIL for one surface.
pub struct Verdict {
    pub gate_a: bool,
    pub gate_b: bool,
}

impl Verdict {
    pub fn overall(&self) -> bool {
        self.gate_a && self.gate_b
    }

    fn word(flag: bool) -> &'static str {
        if flag {
            "PASS"
        } else {
            "FAIL"
        }
    }
}

/// Measure one surface against the two bars.
pub fn judge(run: &SurfaceRun, relevant: &[usize], included: &[usize], bars: GateBars) -> Verdict {
    Verdict {
        gate_a: recall_at(&run.candidate_ranks, relevant, bars.a_k) >= bars.a_min,
        gate_b: recall_at(&run.candidate_ranks, included, bars.b_k) == included.len(),
    }
}

/// The header: what was loaded, what was composed, what it was sent to.
pub fn header(
    fixture: &Fixture,
    path: &std::path::Path,
    query: &str,
    base_url: &str,
    model: &str,
    batch: usize,
) -> String {
    let mut out = String::new();
    let rule = "=".repeat(96);
    let _ = writeln!(out, "{rule}");
    let _ = writeln!(
        out,
        "{} · scenario_id {} · run_id {}",
        fixture.scenario, fixture.scenario_id, fixture.run_id
    );
    let _ = writeln!(out, "{rule}");
    let _ = writeln!(out, "  fixture       : {}", path.display());
    let _ = writeln!(
        out,
        "  run_started_at: {}   extracted_at: {}",
        fixture.run_started_at, fixture.extracted_at
    );
    let _ = writeln!(
        out,
        "  counts        : {} candidates · {} opus_relevant · {} included · {} outside_pool   [STOP 3 PASS]",
        fixture.candidates.len(),
        fixture.opus_relevant_ids.len(),
        fixture.included_ids.len(),
        fixture.outside_pool.len()
    );
    let handles: Vec<&str> = fixture
        .query
        .allegations
        .iter()
        .map(|a| a.id.as_str())
        .collect();
    let _ = writeln!(
        out,
        "  query         : {} chars — L2a rule: theme + {} allegations [{}] + {} talking points",
        query.chars().count(),
        fixture.query.allegations.len(),
        handles.join(" "),
        fixture.query.talking_points.len()
    );
    let first: String = query.chars().take(120).collect();
    let _ = writeln!(out, "  query[..120]  : {}", first.replace('\n', " ⏎ "));
    let _ = writeln!(out, "  subject       : {}", fixture.query.subject);
    let _ = writeln!(
        out,
        "  model         : {model} @ {base_url} · batch {batch} (hand-off §3 cap)"
    );
    out
}

/// One surface's numbers, the Included ranks, and its three verdict words.
pub fn surface_block(
    fixture: &Fixture,
    run: &SurfaceRun,
    relevant: &[usize],
    included: &[usize],
    bars: GateBars,
) -> String {
    let verdict = judge(run, relevant, included, bars);
    let n_relevant = fixture.opus_relevant_ids.len();
    let n_included = fixture.included_ids.len();
    let at_a = recall_at(&run.candidate_ranks, relevant, bars.a_k);
    let at_b = recall_at(&run.candidate_ranks, relevant, bars.b_k);
    let inc_at_b = recall_at(&run.candidate_ranks, included, bars.b_k);

    let mut out = String::new();
    let _ = writeln!(out, "{}", "-".repeat(96));
    let tag = if run.surface == Surface::S2Probe {
        "  ← VERDICT SURFACE"
    } else {
        "  (information)"
    };
    let _ = writeln!(out, "  {}{}", run.surface.label(), tag);
    let _ = writeln!(
        out,
        "    recall@{} of opus_relevant = {}/{} (need ≥ {})   ·   recall@{} = {}/{}",
        bars.a_k, at_a, n_relevant, bars.a_min, bars.b_k, at_b, n_relevant
    );
    let _ = writeln!(
        out,
        "    included_in_top{} = {}/{} (need {})",
        bars.b_k, inc_at_b, n_included, n_included
    );
    out.push_str(&included_lines(fixture, run, included, bars));
    let _ = writeln!(
        out,
        "    GATE A {} · GATE B {} · VERDICT {}{}",
        Verdict::word(verdict.gate_a),
        Verdict::word(verdict.gate_b),
        Verdict::word(verdict.overall()),
        if run.surface == Surface::S2Probe {
            ""
        } else {
            "   (information — the gate is read on S2)"
        }
    );
    let _ = writeln!(
        out,
        "    timing: {:.2}s wall-clock · {:.1} pairs/second",
        run.elapsed_secs,
        run.pairs_per_second()
    );
    out
}

/// Every Included card's rank, one per line, best rank first.
///
/// A card whose rank is outside Gate B's cut-off is marked `✗` — that card is
/// the one the architect reads first when Gate B fails.
fn included_lines(
    fixture: &Fixture,
    run: &SurfaceRun,
    included: &[usize],
    bars: GateBars,
) -> String {
    let mut rows: Vec<(usize, f64, &Card)> = included
        .iter()
        .filter_map(|&p| {
            Some((
                *run.candidate_ranks.get(p)?,
                *run.candidate_scores.get(p)?,
                fixture.candidates.get(p)?,
            ))
        })
        .collect();
    rows.sort_by_key(|(rank, _, _)| *rank);
    let mut out = String::new();
    for (rank, score, card) in rows {
        let flag = if rank <= bars.b_k { ' ' } else { '✗' };
        let _ = writeln!(
            out,
            "      {flag} rank {rank:>3}  {:<6} score {score:>7.4}  {}",
            card.c_number.as_deref().unwrap_or("—"),
            truncate(&card.title, 60)
        );
    }
    out
}

/// The AT-1 / AT-2 preview: where each outside-pool card would land on S2.
pub fn outside_pool_block(fixture: &Fixture, s2: &SurfaceRun, bars: GateBars) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{}", "-".repeat(96));
    let _ = writeln!(
        out,
        "  outside-pool preview on S2 (AT-1 / AT-2 · information only)"
    );
    let mut rows: Vec<(usize, f64, &Card)> = fixture
        .outside_pool
        .iter()
        .enumerate()
        .filter_map(|(i, card)| {
            Some((
                *s2.pool_would_be_ranks.get(i)?,
                *s2.pool_scores.get(i)?,
                card,
            ))
        })
        .collect();
    rows.sort_by_key(|(rank, _, _)| *rank);
    let in_b = rows.iter().filter(|(r, _, _)| *r <= bars.b_k).count();
    let in_a = rows.iter().filter(|(r, _, _)| *r <= bars.a_k).count();
    for (rank, score, card) in &rows {
        let _ = writeln!(
            out,
            "      would-be rank {rank:>3}  score {score:>7.4}  {}",
            truncate(&card.title, 66)
        );
    }
    let _ = writeln!(
        out,
        "    {}/{} inside top {} · {}/{} inside top {}",
        in_b,
        rows.len(),
        bars.b_k,
        in_a,
        rows.len(),
        bars.a_k
    );
    out
}

/// Shorten for the fixed-width block without cutting a char in half.
fn truncate(text: &str, max: usize) -> String {
    let flat = text.replace('\n', " ");
    if flat.chars().count() <= max {
        return flat;
    }
    let kept: String = flat.chars().take(max.saturating_sub(1)).collect();
    format!("{kept}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A run whose candidate at fixture position `i` took rank `ranks[i]`.
    fn run(ranks: &[usize]) -> SurfaceRun {
        SurfaceRun {
            surface: Surface::S1Quote,
            candidate_scores: vec![0.0; ranks.len()],
            candidate_ranks: ranks.to_vec(),
            pool_scores: Vec::new(),
            pool_would_be_ranks: Vec::new(),
            elapsed_secs: 0.0,
        }
    }

    const BARS: GateBars = GateBars {
        a_k: 20,
        a_min: 2,
        b_k: 10,
    };

    /// Both gates pass: two relevant cards inside the top 20, and every
    /// included card inside the top 10.
    #[test]
    fn both_gates_pass_when_the_bars_are_met() {
        // positions 0,1,2 took ranks 1, 5, 9.
        let v = judge(&run(&[1, 5, 9]), &[0, 1], &[0, 2], BARS);
        assert!(v.gate_a);
        assert!(v.gate_b);
    }

    /// Gate A is a FLOOR, so exactly `a_min` passes and one fewer does not.
    /// The boundary is the whole point: `>=` versus `>` is a silent one-card
    /// difference in a published verdict.
    #[test]
    fn gate_a_is_inclusive_at_the_floor() {
        // Two relevant inside k=20 — exactly a_min.
        assert!(judge(&run(&[1, 20, 99]), &[0, 1], &[], BARS).gate_a);
        // One inside, one out — below the floor.
        assert!(!judge(&run(&[1, 21, 99]), &[0, 1], &[], BARS).gate_a);
    }

    /// Gate A's cut-off is also inclusive: rank == a_k counts as inside.
    #[test]
    fn gate_a_counts_a_card_sitting_exactly_on_the_cut_off() {
        assert!(judge(&run(&[20, 20]), &[0, 1], &[], BARS).gate_a);
        assert!(!judge(&run(&[21, 21]), &[0, 1], &[], BARS).gate_a);
    }

    /// Gate B is ALL-OR-NOTHING, not a floor: every included card must be
    /// inside `b_k`. One outside fails it however good the rest are.
    #[test]
    fn gate_b_fails_when_a_single_included_card_falls_outside() {
        assert!(judge(&run(&[1, 2, 10]), &[], &[0, 1, 2], BARS).gate_b);
        // The third slips one rank past b_k = 10.
        assert!(!judge(&run(&[1, 2, 11]), &[], &[0, 1, 2], BARS).gate_b);
    }

    /// The degenerate case, stated so nobody has to guess: a fixture with no
    /// Included cards passes Gate B vacuously (0 == 0). That is correct — there
    /// was nothing to miss — and it is exactly the sort of thing a reader of a
    /// green verdict deserves to have been told is intentional.
    #[test]
    fn gate_b_passes_vacuously_when_there_are_no_included_cards() {
        assert!(judge(&run(&[99, 99]), &[], &[], BARS).gate_b);
    }
}
