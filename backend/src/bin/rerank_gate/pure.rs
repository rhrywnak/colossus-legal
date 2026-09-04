//! Every decision the gate makes, as pure functions.
//!
//! Nothing in this module opens a socket, reads a file or reads the clock. That
//! is deliberate: the gate's verdict is arithmetic over a score vector, and
//! arithmetic that can only be exercised by calling a GPU is arithmetic nobody
//! checks. The tests at the bottom run the whole chain — compose, build the
//! three surfaces, rank with a deliberate tie, measure recall — on a
//! hand-written five-card fixture.

use crate::fixture::{Allegation, Card};

/// Which text of a candidate is handed to the reranker.
///
/// ## Domain note: why three, and why the verdict is read on S2
///
/// The plan's literal instruction was to score `(query, quote)`. The design's
/// 09-01 correction observed that 22 of S-11's 292 quotes are bare "Admitted."
/// or "Denied as untrue.", whose substance lives in `title` and `significance` —
/// so quote-alone is structurally blind to exactly the admissions the cascade
/// exists to surface. All three surfaces are scored in one run because the run
/// costs nothing, and a ruling made on three columns of numbers is worth more
/// than a ruling made on one plus a second run to get the other two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    /// Quote only — the plan's literal `(query, quote)`.
    S1Quote,
    /// Quote ⏎ title ⏎ significance — L1's `probe_text` order. **The verdict
    /// surface.**
    S2Probe,
    /// Title ⏎ quote ⏎ significance.
    S3Titled,
}

impl Surface {
    /// All three, in printing order.
    pub const ALL: [Surface; 3] = [Surface::S1Quote, Surface::S2Probe, Surface::S3Titled];

    /// The short label used in every printed block and CSV column.
    pub fn label(self) -> &'static str {
        match self {
            Surface::S1Quote => "S1 quote",
            Surface::S2Probe => "S2 probe",
            Surface::S3Titled => "S3 titled",
        }
    }
}

/// Join the non-empty, trimmed pieces with newlines.
///
/// The single rule behind both the query composer and all three surface
/// builders, written once. "Skip empty pieces" rather than "render blank lines"
/// matters: a blank line is a token the model has to spend attention on, and a
/// card missing its significance would otherwise be scored against a different
/// shape of text than its neighbours.
fn join_pieces(pieces: &[&str]) -> String {
    pieces
        .iter()
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect::<Vec<&str>>()
        .join("\n")
}

/// Compose the gather query — L2a's rule, reproduced.
///
/// Theme, then each allegation's verbatim text in the order given, then the
/// talking points in the order given; each piece trimmed, empty pieces skipped,
/// newline-joined.
///
/// ## Why this is a copy and not a call
///
/// `compose_gather_query` lives in `services::gather_query` on
/// `feat/gather-query-composer-l2a` and is NOT on main, which this branch was
/// cut from. Rather than merge an unrelated feature branch to run a gate, the
/// rule is reproduced here and was read against the L2a source before writing:
/// L2a builds `pieces` in exactly this order, pushes `theme.trim()` only when
/// non-empty, pushes `allegation.text.trim()` only when non-empty, pushes each
/// talking point on the same condition, and returns `pieces.join("\n")`. The
/// halves of L2a this gate does NOT reproduce — `reachable_parties` and
/// `QueryBasis` — are the party-widening, which is not on trial here.
pub fn compose_query(theme: &str, allegations: &[Allegation], talking_points: &[String]) -> String {
    let mut pieces: Vec<&str> = Vec::with_capacity(1 + allegations.len() + talking_points.len());
    pieces.push(theme);
    for allegation in allegations {
        pieces.push(&allegation.text);
    }
    for point in talking_points {
        pieces.push(point);
    }
    join_pieces(&pieces)
}

/// Build one candidate's text for the given surface.
pub fn surface_text(card: &Card, surface: Surface) -> String {
    match surface {
        Surface::S1Quote => join_pieces(&[&card.quote]),
        Surface::S2Probe => join_pieces(&[&card.quote, &card.title, &card.significance]),
        Surface::S3Titled => join_pieces(&[&card.title, &card.quote, &card.significance]),
    }
}

/// Rank a score vector: 1 = best. Returns rank BY ORIGINAL POSITION.
///
/// ## Rust Learning: `sort_by` is stable, and that is the tie-break
///
/// The hand-off says scores are 0–1 and several cards can land on the same
/// value. Rust's `slice::sort_by` is a stable sort — equal elements keep their
/// input order — so sorting the index vector by score descending gives
/// "ties broken by fixture order" for free, with no secondary comparator. A
/// gate whose ranks shifted between runs on tied scores would be unreproducible,
/// which is why this is spelled out rather than left to `sort_unstable_by`.
///
/// ## Rust Learning: `partial_cmp` on floats
///
/// `f64` is `PartialOrd`, not `Ord`, because NaN compares false against
/// everything. `unwrap_or(Ordering::Equal)` cannot be reached here — the client
/// rejects a non-finite score before this is called — and choosing `Equal`
/// rather than panicking keeps a hypothetical NaN from taking the process down
/// mid-run; it would sort as a tie and be visible in the CSV.
pub fn rank_desc(scores: &[f64]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..scores.len()).collect();
    order.sort_by(|&a, &b| {
        scores[b]
            .partial_cmp(&scores[a])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut ranks = vec![0usize; scores.len()];
    for (position, &index) in order.iter().enumerate() {
        ranks[index] = position + 1;
    }
    ranks
}

/// How many of `positions` have a rank of `k` or better.
///
/// Used for both gate numbers: Gate A is `recall_at(relevant, 60)` and Gate B is
/// `recall_at(included, 20)`. They are the same arithmetic over different id
/// sets, so they are one function.
pub fn recall_at(ranks: &[usize], positions: &[usize], k: usize) -> usize {
    positions
        .iter()
        .filter(|&&p| ranks.get(p).is_some_and(|&r| r <= k))
        .count()
}

/// The rank an outside-pool card WOULD take if inserted into the ranked list.
///
/// `1 + the number of candidates scoring strictly higher`, exactly as specified.
/// Strictly higher means a card tied with the best candidate previews at rank 1,
/// which is the generous reading — and the AT bars are information only, so the
/// generous reading is the honest one to print.
pub fn would_be_rank(score: f64, candidate_scores: &[f64]) -> usize {
    1 + candidate_scores.iter().filter(|&&s| s > score).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(quote: &str, title: &str, significance: &str) -> Card {
        Card {
            id: format!("id-{title}"),
            c_number: None,
            title: title.to_string(),
            document: "DOC".to_string(),
            page: Some(1),
            pinpoint: Some("p. 1".to_string()),
            quote: quote.to_string(),
            significance: significance.to_string(),
            about: vec![],
        }
    }

    fn allegation(id: &str, text: &str) -> Allegation {
        Allegation {
            id: id.to_string(),
            text: text.to_string(),
        }
    }

    #[test]
    fn compose_query_orders_theme_then_allegations_then_points() {
        let text = compose_query(
            "  THEME  ",
            &[allegation("A-1", "first"), allegation("A-2", "second")],
            &["point one".to_string()],
        );
        assert_eq!(text, "THEME\nfirst\nsecond\npoint one");
    }

    #[test]
    fn compose_query_skips_empty_pieces_without_leaving_blank_lines() {
        let text = compose_query(
            "",
            &[allegation("A-1", "   "), allegation("A-2", " kept ")],
            &["".to_string(), "tail".to_string()],
        );
        assert_eq!(text, "kept\ntail");
    }

    #[test]
    fn compose_query_of_nothing_is_empty_not_newlines() {
        assert_eq!(compose_query("  ", &[], &[]), "");
    }

    /// The "Admitted." card is the reason three surfaces exist: S1 carries four
    /// words and no subject matter, while S2 and S3 carry the request that was
    /// admitted.
    #[test]
    fn surfaces_of_an_admitted_card_differ_in_the_way_that_matters() {
        let c = card(
            "Admitted.",
            "Phillips admits the $50,000 check was turned over to CFS",
            "Key admission on authority.",
        );
        assert_eq!(surface_text(&c, Surface::S1Quote), "Admitted.");
        assert_eq!(
            surface_text(&c, Surface::S2Probe),
            "Admitted.\nPhillips admits the $50,000 check was turned over to CFS\nKey admission on authority."
        );
        assert_eq!(
            surface_text(&c, Surface::S3Titled),
            "Phillips admits the $50,000 check was turned over to CFS\nAdmitted.\nKey admission on authority."
        );
    }

    #[test]
    fn surface_builders_trim_and_skip_empty_pieces() {
        let c = card("  quoted  ", "titled", "   ");
        assert_eq!(surface_text(&c, Surface::S2Probe), "quoted\ntitled");
        assert_eq!(surface_text(&c, Surface::S3Titled), "titled\nquoted");
        assert_eq!(surface_text(&c, Surface::S1Quote), "quoted");
    }

    #[test]
    fn rank_desc_puts_the_highest_score_first() {
        let ranks = rank_desc(&[0.1, 0.9, 0.5]);
        assert_eq!(ranks, vec![3, 1, 2]);
    }

    /// Two cards tied at 0.5: the earlier fixture position must take the better
    /// rank, on every run.
    #[test]
    fn rank_desc_breaks_ties_by_fixture_order() {
        let ranks = rank_desc(&[0.5, 0.9, 0.5, 0.5]);
        assert_eq!(ranks, vec![2, 1, 3, 4]);
    }

    #[test]
    fn rank_desc_of_an_empty_list_is_empty() {
        assert!(rank_desc(&[]).is_empty());
    }

    #[test]
    fn recall_at_counts_only_positions_within_k() {
        // ranks by position: card 0 is 3rd, card 1 is 1st, card 2 is 2nd.
        let ranks = vec![3, 1, 2];
        assert_eq!(recall_at(&ranks, &[0, 1, 2], 2), 2);
        assert_eq!(recall_at(&ranks, &[0, 1, 2], 3), 3);
        assert_eq!(recall_at(&ranks, &[0], 2), 0);
        assert_eq!(recall_at(&ranks, &[], 60), 0);
    }

    #[test]
    fn recall_at_ignores_a_position_outside_the_list() {
        let ranks = vec![1, 2];
        assert_eq!(recall_at(&ranks, &[0, 99], 60), 1);
    }

    #[test]
    fn would_be_rank_counts_strictly_higher_scores() {
        let candidates = vec![0.9, 0.7, 0.5, 0.5];
        assert_eq!(would_be_rank(0.95, &candidates), 1);
        assert_eq!(would_be_rank(0.8, &candidates), 2);
        assert_eq!(would_be_rank(0.5, &candidates), 3);
        assert_eq!(would_be_rank(0.1, &candidates), 5);
    }

    #[test]
    fn would_be_rank_against_an_empty_pool_is_one() {
        assert_eq!(would_be_rank(0.5, &[]), 1);
    }
}
