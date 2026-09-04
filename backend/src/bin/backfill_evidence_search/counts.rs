//! The three counts L1b asserts, and the pure verdict over them.
//!
//! Pure: no database, no graph, no vector store. That is the point — the
//! interesting cases (a short mirror, a Qdrant that disagrees, a Qdrant that
//! cannot be asked) are all failure states, and a failure state that can only be
//! reached by breaking a real system is a failure state nobody tests.

/// The three numbers the task asserts, and what they mean together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Counts {
    pub(crate) graph_nodes: i64,
    pub(crate) mirror_rows: i64,
    /// `None` when Qdrant could not be reached. Distinct from `Some(0)`, which
    /// would mean the collection genuinely holds no Evidence points — a far more
    /// alarming fact than "we could not ask".
    pub(crate) qdrant_points: Option<i64>,
}

/// What the counts add up to, as a verdict plus the lines to print.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CountVerdict {
    pub(crate) lines: Vec<String>,
    /// The graph and the mirror agree. The only condition that gates the exit code.
    pub(crate) mirror_complete: bool,
    /// Qdrant disagrees with the graph — reported, never fatal here.
    pub(crate) vector_index_finding: Option<String>,
}

/// Judge the three counts. Pure: no I/O, so the mismatch cases are testable
/// without a database, a graph or a vector store.
///
/// ## Domain note: why only ONE of the two comparisons is fatal
///
/// The graph-vs-mirror comparison is a statement about work this tool just did:
/// if they differ, the backfill did not finish and re-running is the fix. The
/// graph-vs-Qdrant comparison is a statement about work something ELSE did, on
/// another day — if they differ, the vector half of the gather is already
/// missing evidence, which is worth knowing before L2 relies on it but is not
/// this task's bug and must not block this task's output.
pub(crate) fn judge_counts(counts: Counts) -> CountVerdict {
    let qdrant_line = match counts.qdrant_points {
        Some(n) => n.to_string(),
        None => "unreachable (see the log)".to_string(),
    };
    let lines = vec![
        format!("graph Evidence nodes      : {}", counts.graph_nodes),
        format!("rows in evidence_search   : {}", counts.mirror_rows),
        format!("Qdrant colossus_evidence  : {qdrant_line}"),
    ];

    let vector_index_finding = match counts.qdrant_points {
        Some(points) if points != counts.graph_nodes => Some(format!(
            "Qdrant holds {points} Evidence points against {} nodes in the graph — a difference \
             of {}. The VECTOR half of the gather is missing evidence; that is a finding about \
             the index step, not about this backfill.",
            counts.graph_nodes,
            (points - counts.graph_nodes).abs()
        )),
        _ => None,
    };

    CountVerdict {
        lines,
        mirror_complete: counts.graph_nodes == counts.mirror_rows,
        vector_index_finding,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counts(graph: i64, mirror: i64, qdrant: Option<i64>) -> Counts {
        Counts {
            graph_nodes: graph,
            mirror_rows: mirror,
            qdrant_points: qdrant,
        }
    }

    /// All three agreeing is the good case, and it says nothing extra.
    #[test]
    fn three_matching_counts_pass_with_no_finding() {
        let verdict = judge_counts(counts(1209, 1209, Some(1209)));
        assert!(verdict.mirror_complete);
        assert_eq!(verdict.vector_index_finding, None);
        assert_eq!(verdict.lines[0], "graph Evidence nodes      : 1209");
        assert_eq!(verdict.lines[1], "rows in evidence_search   : 1209");
        assert_eq!(verdict.lines[2], "Qdrant colossus_evidence  : 1209");
    }

    /// A short mirror is fatal, and both numbers are printed rather than one.
    #[test]
    fn a_short_mirror_fails_and_prints_both_numbers() {
        let verdict = judge_counts(counts(1209, 1180, Some(1209)));
        assert!(!verdict.mirror_complete);
        assert!(verdict.lines[0].contains("1209"));
        assert!(verdict.lines[1].contains("1180"));
    }

    /// A mirror with MORE rows than the graph is also a mismatch — a node
    /// deleted from the graph leaves its row behind, and equality catches that
    /// where a `<` comparison would not.
    #[test]
    fn a_mirror_holding_more_than_the_graph_is_also_a_mismatch() {
        assert!(!judge_counts(counts(1209, 1210, Some(1209))).mirror_complete);
    }

    /// Qdrant disagreeing is reported and is NOT fatal: it is a fact about the
    /// vector index, which this task does not touch.
    #[test]
    fn a_qdrant_mismatch_is_reported_but_never_fatal() {
        let verdict = judge_counts(counts(1209, 1209, Some(1100)));
        assert!(
            verdict.mirror_complete,
            "the lexical backfill succeeded; the vector store's state is not its verdict"
        );
        let finding = verdict
            .vector_index_finding
            .expect("a difference must produce a finding");
        assert!(finding.contains("1100"));
        assert!(finding.contains("1209"));
        assert!(
            finding.contains("109"),
            "the size of the gap must be stated"
        );
    }

    /// Unreachable Qdrant is rendered as unreachable, never as zero — the two
    /// are completely different facts and only one of them is alarming.
    #[test]
    fn an_unreachable_qdrant_is_not_reported_as_zero() {
        let verdict = judge_counts(counts(1209, 1209, None));
        assert!(verdict.lines[2].contains("unreachable"));
        assert!(!verdict.lines[2].contains('0'));
        assert_eq!(
            verdict.vector_index_finding, None,
            "not asking is not the same as asking and being told zero"
        );
        assert!(verdict.mirror_complete);
    }

    /// A genuine zero DOES produce a finding — the collection being empty is
    /// exactly the state the third count exists to surface.
    #[test]
    fn a_genuinely_empty_qdrant_produces_a_finding() {
        let verdict = judge_counts(counts(1209, 1209, Some(0)));
        assert!(verdict.vector_index_finding.is_some());
        assert!(verdict.mirror_complete);
    }
}
