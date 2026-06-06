//! Pure shaping for `GET /api/cases/:slug/proof-review`: turns the two raw row
//! sets from [`super::proof_review_repository`] into the wire
//! [`ProofReviewResponse`].
//!
//! ## Why a builder, and why the summary is derived here (not in Cypher)
//!
//! The Summary sub-view's category counts are computed in Rust from the exact
//! `proof_edges` and `excluded` rows the page renders — not from a second
//! database `GROUP BY`. This is deliberate: deriving the headline numbers from
//! the same rows makes it *impossible* for the summary to disagree with the
//! detail tables (a class of bug separate count queries invite). The data is
//! tiny (tens of rows), so the in-memory tally is free.
//!
//! This is distinct from — and does not touch — the proof-matrix per-Count
//! *allegation* rollup, which remains the sole source of truth for that
//! different number (`proof_matrix_repository::fetch_rollup`). Nothing here
//! recomputes a per-Count total.
//!
//! Keeping the shaping in a pure function (no `Graph`, no I/O) is what lets the
//! grouping invariants be unit-tested without a live database — see the tests.

use std::collections::HashMap;

use crate::dto::proof_review::{
    CategoryCount, CorroboratingSummary, ExcludedEvidence, ExcludedSummary, ProofEdge,
    ProofReviewResponse, ProofReviewSummary, StatementTypeCount,
};
use crate::models::document_status::STMT_PARTIAL_ADMISSION;
use crate::repositories::proof_review_repository::{EvidenceProofRow, ExcludedEvidenceRow};

/// Tally a flat list of `statement_type` values into per-type counts, ordered by
/// count descending then type ascending (deterministic — the `ORDER BY n DESC`
/// convention George's source queries used, with a stable tie-break).
///
/// ## Rust Learning: `HashMap` accumulate then sort for a deterministic `Vec`
///
/// A `HashMap` has no defined iteration order, so we count into it, then collect
/// to a `Vec` and `sort_by` with an explicit total order. Without the explicit
/// sort the wire output (and any test asserting it) would be non-deterministic.
fn tally_statement_types(types: &[&str]) -> Vec<StatementTypeCount> {
    let mut counts: HashMap<&str, i64> = HashMap::new();
    for t in types {
        *counts.entry(t).or_insert(0) += 1;
    }
    let mut out: Vec<StatementTypeCount> = counts
        .into_iter()
        .map(|(statement_type, count)| StatementTypeCount {
            statement_type: statement_type.to_string(),
            count,
        })
        .collect();
    out.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.statement_type.cmp(&b.statement_type))
    });
    out
}

/// Tally proof edges by (`statement_type`, `evidence_strength`), ordered by
/// count descending then the two keys ascending (deterministic).
fn tally_categories(edges: &[EvidenceProofRow]) -> Vec<CategoryCount> {
    let mut counts: HashMap<(&str, &str), i64> = HashMap::new();
    for e in edges {
        *counts
            .entry((e.statement_type.as_str(), e.evidence_strength.as_str()))
            .or_insert(0) += 1;
    }
    let mut out: Vec<CategoryCount> = counts
        .into_iter()
        .map(
            |((statement_type, evidence_strength), count)| CategoryCount {
                statement_type: statement_type.to_string(),
                evidence_strength: evidence_strength.to_string(),
                count,
            },
        )
        .collect();
    out.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.statement_type.cmp(&b.statement_type))
            .then_with(|| a.evidence_strength.cmp(&b.evidence_strength))
    });
    out
}

/// Move one repository proof-edge row into its wire DTO row (field-for-field).
fn proof_edge_dto(r: EvidenceProofRow) -> ProofEdge {
    ProofEdge {
        answer: r.answer,
        question: r.question,
        evidence_verbatim_quote: r.evidence_verbatim_quote,
        statement_type: r.statement_type,
        evidence_strength: r.evidence_strength,
        paragraph: r.paragraph,
        page_number: r.page_number,
        source_document: r.source_document,
        allegation_summary: r.allegation_summary,
        allegation_title: r.allegation_title,
        allegation_paragraph_number: r.allegation_paragraph_number,
        allegation_id: r.allegation_id,
    }
}

/// The v1 borderline (hedged-partial) queue: the `partial_admission` subset of
/// the proof edges. A *view* of the rows already built — filtered and cloned,
/// never re-queried or re-shaped — so it can never contain a row the
/// `proof_edges` section does not.
fn borderline_edges(proof_edges: &[ProofEdge]) -> Vec<ProofEdge> {
    proof_edges
        .iter()
        .filter(|e| e.statement_type == STMT_PARTIAL_ADMISSION)
        .cloned()
        .collect()
}

/// Move one repository excluded row into its wire DTO row.
fn excluded_dto(r: ExcludedEvidenceRow) -> ExcludedEvidence {
    ExcludedEvidence {
        answer: r.answer,
        question: r.question,
        evidence_verbatim_quote: r.evidence_verbatim_quote,
        statement_type: r.statement_type,
        paragraph: r.paragraph,
        page_number: r.page_number,
        source_document: r.source_document,
    }
}

/// Assemble the full Proof-Review payload from the two raw row sets.
///
/// `case_slug` and `document_id` are echoed for the caller's correlation. The
/// summary is tallied from `edge_rows`/`excluded_rows` *before* they are moved
/// into the DTO sections, so it is provably over the same rows. Borderline is
/// the `partial_admission` subset of the proof edges (v1 definition — the
/// design's `evidence_strength = 'sworn_party_evasion'` value does not exist in
/// the graph, so it is not used).
pub(crate) fn build_proof_review(
    case_slug: String,
    document_id: Option<String>,
    edge_rows: Vec<EvidenceProofRow>,
    excluded_rows: Vec<ExcludedEvidenceRow>,
) -> ProofReviewResponse {
    // Derive both summary breakdowns from the raw rows (borrowed, not consumed).
    let corroborating = CorroboratingSummary {
        total: edge_rows.len() as i64,
        by_statement_type: tally_statement_types(
            &edge_rows
                .iter()
                .map(|e| e.statement_type.as_str())
                .collect::<Vec<_>>(),
        ),
        by_category: tally_categories(&edge_rows),
    };
    let excluded_summary = ExcludedSummary {
        total: excluded_rows.len() as i64,
        by_statement_type: tally_statement_types(
            &excluded_rows
                .iter()
                .map(|e| e.statement_type.as_str())
                .collect::<Vec<_>>(),
        ),
    };

    // Now move the rows into their DTO sections.
    let proof_edges: Vec<ProofEdge> = edge_rows.into_iter().map(proof_edge_dto).collect();
    let excluded: Vec<ExcludedEvidence> = excluded_rows.into_iter().map(excluded_dto).collect();

    let borderline = borderline_edges(&proof_edges);

    ProofReviewResponse {
        case_slug,
        document_id,
        summary: ProofReviewSummary {
            corroborating,
            excluded: excluded_summary,
        },
        proof_edges,
        excluded,
        borderline,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::document_status::{
        EVIDENCE_STRENGTH_SWORN_PARTY_ADMISSION, STMT_ADMISSION, STMT_DENIAL, STMT_EVASIVE,
    };

    /// Build a proof-edge fixture row with a given statement_type/strength; the
    /// display fields are filled with traceable placeholders.
    fn edge(statement_type: &str, evidence_strength: &str, id: &str) -> EvidenceProofRow {
        EvidenceProofRow {
            statement_type: statement_type.to_string(),
            evidence_strength: evidence_strength.to_string(),
            answer: Some(format!("answer-{id}")),
            question: Some(format!("question-{id}")),
            evidence_verbatim_quote: Some(format!("quote-{id}")),
            paragraph: Some(format!("Q{id}")),
            page_number: Some(1),
            source_document: Some("doc-george".to_string()),
            allegation_summary: Some(format!("summary-{id}")),
            allegation_title: Some(format!("title-{id}")),
            allegation_paragraph_number: Some(id.to_string()),
            allegation_id: Some(format!("alleg-{id}")),
        }
    }

    fn excluded_row(statement_type: &str, id: &str) -> ExcludedEvidenceRow {
        ExcludedEvidenceRow {
            statement_type: statement_type.to_string(),
            answer: Some(format!("answer-{id}")),
            question: Some(format!("question-{id}")),
            evidence_verbatim_quote: Some(format!("quote-{id}")),
            paragraph: Some(format!("Q{id}")),
            page_number: Some(2),
            source_document: Some("doc-george".to_string()),
        }
    }

    /// A representative fixture: 3 admissions + 2 partial_admissions (5 edges),
    /// and 4 evasive + 1 denial excluded (5 non-answers). This mirrors the
    /// *structure* of the live distribution (admission/partial_admission
    /// corroborations; evasive/denial/… exclusions) at small scale so the
    /// invariants are checkable without a live graph. (On current DEV data the
    /// same invariants hold at 43 corroborations / 79 excluded — see the
    /// endpoint's DEV verification curl.)
    fn fixture() -> ProofReviewResponse {
        let edges = vec![
            edge(STMT_ADMISSION, EVIDENCE_STRENGTH_SWORN_PARTY_ADMISSION, "1"),
            edge(STMT_ADMISSION, EVIDENCE_STRENGTH_SWORN_PARTY_ADMISSION, "2"),
            edge(STMT_ADMISSION, EVIDENCE_STRENGTH_SWORN_PARTY_ADMISSION, "3"),
            edge(
                STMT_PARTIAL_ADMISSION,
                EVIDENCE_STRENGTH_SWORN_PARTY_ADMISSION,
                "4",
            ),
            edge(
                STMT_PARTIAL_ADMISSION,
                EVIDENCE_STRENGTH_SWORN_PARTY_ADMISSION,
                "5",
            ),
        ];
        let excluded = vec![
            excluded_row(STMT_EVASIVE, "6"),
            excluded_row(STMT_EVASIVE, "7"),
            excluded_row(STMT_EVASIVE, "8"),
            excluded_row(STMT_EVASIVE, "9"),
            excluded_row(STMT_DENIAL, "10"),
        ];
        build_proof_review(
            "awad_v_catholic_family_service".to_string(),
            None,
            edges,
            excluded,
        )
    }

    /// The corroboration summary total equals the number of proof edges, and
    /// BOTH breakdowns sum back to that total. This is the core "no miscount"
    /// guarantee: a grouping bug that dropped or double-counted a row would
    /// break one of these sums. (The production failure this catches: a summary
    /// header that disagrees with the rows the page lists.)
    #[test]
    fn corroborating_summary_sums_to_proof_edge_count() {
        let r = fixture();
        assert_eq!(r.proof_edges.len(), 5);
        assert_eq!(r.summary.corroborating.total, 5);
        let by_type: i64 = r
            .summary
            .corroborating
            .by_statement_type
            .iter()
            .map(|c| c.count)
            .sum();
        let by_cat: i64 = r
            .summary
            .corroborating
            .by_category
            .iter()
            .map(|c| c.count)
            .sum();
        assert_eq!(by_type, 5);
        assert_eq!(by_cat, 5);
    }

    /// Per-statement_type corroboration counts are exact and ordered by count
    /// desc (admission 3 before partial_admission 2).
    #[test]
    fn corroborating_by_statement_type_is_exact_and_ordered() {
        let r = fixture();
        let by_type = &r.summary.corroborating.by_statement_type;
        assert_eq!(by_type.len(), 2);
        assert_eq!(by_type[0].statement_type, STMT_ADMISSION);
        assert_eq!(by_type[0].count, 3);
        assert_eq!(by_type[1].statement_type, STMT_PARTIAL_ADMISSION);
        assert_eq!(by_type[1].count, 2);
    }

    /// The (`statement_type`, `evidence_strength`) breakdown is exact and
    /// ordered by count desc. The fixture's edges all carry
    /// `sworn_party_admission`, so the two category rows are admission/3 then
    /// partial_admission/2. Guards `tally_categories` against a grouping bug
    /// (wrong key, wrong sort, off-by-one) that the sum-only test would miss —
    /// the parallel guarantee to `corroborating_by_statement_type_is_exact_and_ordered`.
    #[test]
    fn corroborating_by_category_is_exact_and_ordered() {
        let r = fixture();
        let by_cat = &r.summary.corroborating.by_category;
        assert_eq!(by_cat.len(), 2);
        assert_eq!(by_cat[0].statement_type, STMT_ADMISSION);
        assert_eq!(
            by_cat[0].evidence_strength,
            EVIDENCE_STRENGTH_SWORN_PARTY_ADMISSION
        );
        assert_eq!(by_cat[0].count, 3);
        assert_eq!(by_cat[1].statement_type, STMT_PARTIAL_ADMISSION);
        assert_eq!(
            by_cat[1].evidence_strength,
            EVIDENCE_STRENGTH_SWORN_PARTY_ADMISSION
        );
        assert_eq!(by_cat[1].count, 2);
    }

    /// The excluded summary total equals the number of excluded rows and its
    /// breakdown sums back to it; counts are exact (evasive 4, denial 1).
    #[test]
    fn excluded_summary_sums_and_counts_are_exact() {
        let r = fixture();
        assert_eq!(r.excluded.len(), 5);
        assert_eq!(r.summary.excluded.total, 5);
        let sum: i64 = r
            .summary
            .excluded
            .by_statement_type
            .iter()
            .map(|c| c.count)
            .sum();
        assert_eq!(sum, 5);
        let evasive = r
            .summary
            .excluded
            .by_statement_type
            .iter()
            .find(|c| c.statement_type == STMT_EVASIVE)
            .expect("evasive bucket present");
        assert_eq!(evasive.count, 4);
        let denial = r
            .summary
            .excluded
            .by_statement_type
            .iter()
            .find(|c| c.statement_type == STMT_DENIAL)
            .expect("denial bucket present");
        assert_eq!(denial.count, 1);
    }

    /// Borderline is exactly the `partial_admission` subset of the proof edges:
    /// same count as the partial_admission tally, and every borderline row has
    /// that statement_type. Guards the v1 borderline definition against drift
    /// (e.g. accidentally keying off the non-existent `sworn_party_evasion`).
    #[test]
    fn borderline_is_the_partial_admission_subset() {
        let r = fixture();
        assert_eq!(r.borderline.len(), 2);
        assert!(r
            .borderline
            .iter()
            .all(|e| e.statement_type == STMT_PARTIAL_ADMISSION));
        // It is a subset of proof_edges, not extra rows.
        let partials_in_edges = r
            .proof_edges
            .iter()
            .filter(|e| e.statement_type == STMT_PARTIAL_ADMISSION)
            .count();
        assert_eq!(r.borderline.len(), partials_in_edges);
    }

    /// Empty inputs produce zero totals, empty breakdowns, and empty sections —
    /// never a panic or a phantom bucket. The handler turns the all-empty case
    /// into a 404 before serialization, but the builder must still be correct.
    #[test]
    fn empty_inputs_produce_empty_payload() {
        let r = build_proof_review("c".to_string(), Some("doc-x".to_string()), vec![], vec![]);
        assert_eq!(r.summary.corroborating.total, 0);
        assert!(r.summary.corroborating.by_statement_type.is_empty());
        assert!(r.summary.corroborating.by_category.is_empty());
        assert_eq!(r.summary.excluded.total, 0);
        assert!(r.proof_edges.is_empty());
        assert!(r.excluded.is_empty());
        assert!(r.borderline.is_empty());
        assert_eq!(r.document_id.as_deref(), Some("doc-x"));
    }
}
