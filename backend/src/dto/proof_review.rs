//! Response DTOs for `GET /api/cases/:slug/proof-review` — the read-only
//! payload behind the Proof-Review page's four sub-views (Summary, Proof edges,
//! Excluded, Borderline) over the `Evidence -[:CORROBORATES]-> Allegation`
//! proof edges in Neo4j.
//!
//! Serialize-only response types: the backend produces these and sends them to
//! the client; nothing here is decoded from stored JSON, so (like the
//! proof-matrix DTOs) no `Deserialize` is derived — see that module's note.
//!
//! ## Why every display field is `Option<_>` and serialized as `null`, not skipped
//!
//! Standing Rule 1 requires a *missing* value, an *empty* value, and a
//! *present* value to stay distinguishable. So display/locator fields decode as
//! `Option<String>` / `Option<i64>` and we deliberately do **not** add
//! `#[serde(skip_serializing_if = "Option::is_none")]`: a missing field
//! serializes as JSON `null` (key present, value null), which is distinct from
//! an empty string `""`. Skipping would collapse "field absent in the graph"
//! and "field never queried" into the same wire shape. The categorization
//! fields (`statement_type`, `evidence_strength`) are required `String`s
//! instead — they are the grouping backbone, and a null there is a genuine data
//! problem the repository surfaces as a logged 500, never a silent default.

use serde::Serialize;

/// Top-level Proof-Review payload: the echoed request context plus the four
/// sub-view sections.
///
/// ## Rust Learning: `#[derive(Serialize)]` only
///
/// `serde::Serialize` is the half of the serde pair axum's `Json(...)` needs to
/// turn this struct into the HTTP response body. `Deserialize` is intentionally
/// omitted because the backend never parses this shape back in — deriving only
/// what is used keeps the type honest about its direction of flow.
#[derive(Debug, Clone, Serialize)]
pub struct ProofReviewResponse {
    /// Echoed from the request path. The Neo4j graph is single-case and not
    /// slug-namespaced, so the slug does not filter the reads — it is returned
    /// for the caller's correlation, matching the proof-matrix contract.
    pub case_slug: String,
    /// Echoed from the optional `?document_id=` filter: `Some(id)` when the
    /// payload is scoped to one source document, `None` when it spans all
    /// documents. Echoed so the caller can confirm which scope it received.
    pub document_id: Option<String>,
    /// Sub-view 1: corroboration category breakdown + excluded non-answer
    /// counts, derived from the same rows shown in the sections below.
    pub summary: ProofReviewSummary,
    /// Sub-view 2: one row per `CORROBORATES` edge (answer → allegation).
    pub proof_edges: Vec<ProofEdge>,
    /// Sub-view 3: preserved-but-unlinked non-answer Evidence (no edge).
    pub excluded: Vec<ExcludedEvidence>,
    /// Sub-view 4: the hedged-partial queue — the `proof_edges` subset whose
    /// `statement_type` is `partial_admission`. Surfacing only; no verdict.
    pub borderline: Vec<ProofEdge>,
}

/// Sub-view 1. Two independent breakdowns: corroborations (the `CORROBORATES`
/// edges) and exclusions (non-answers with no edge).
///
/// Domain note: these counts are derived in Rust from the exact `proof_edges`
/// and `excluded` rows returned below — they are NOT a second database count
/// path. This guarantees the headline numbers can never drift from the detail
/// rows the page renders. (Per-Count *allegation* totals are a different
/// number and remain the sole job of the proof-matrix `fetch_rollup`; they are
/// out of scope here.)
#[derive(Debug, Clone, Serialize)]
pub struct ProofReviewSummary {
    pub corroborating: CorroboratingSummary,
    pub excluded: ExcludedSummary,
}

/// Corroboration counts at two granularities, both over the same edge set.
#[derive(Debug, Clone, Serialize)]
pub struct CorroboratingSummary {
    /// Total `CORROBORATES` edges in scope (the headline corroboration count).
    pub total: i64,
    /// Per-`statement_type` totals (e.g. `admission`, `partial_admission`) —
    /// the granularity the page's headline numbers use.
    pub by_statement_type: Vec<StatementTypeCount>,
    /// Finer breakdown by (`statement_type`, `evidence_strength`) — the
    /// grouping the design specifies for the category table.
    pub by_category: Vec<CategoryCount>,
}

/// Exclusion counts: how many preserved non-answers each category kept out of
/// the corroboration set.
#[derive(Debug, Clone, Serialize)]
pub struct ExcludedSummary {
    /// Total preserved non-answer Evidence with no `CORROBORATES` edge.
    pub total: i64,
    /// Per-`statement_type` totals (`evasive`, `objection`, `referral`,
    /// `denial`).
    pub by_statement_type: Vec<StatementTypeCount>,
}

/// One per-`statement_type` tally.
#[derive(Debug, Clone, Serialize)]
pub struct StatementTypeCount {
    pub statement_type: String,
    pub count: i64,
}

/// One per-(`statement_type`, `evidence_strength`) tally.
#[derive(Debug, Clone, Serialize)]
pub struct CategoryCount {
    pub statement_type: String,
    pub evidence_strength: String,
    pub count: i64,
}

/// Sub-views 2 & 4: a single readable proof edge — the discovery *answer* on
/// one side and the complaint *allegation* it corroborates on the other. Carries
/// enough for the frontend to render "answer → allegation" with the Q-number and
/// page, and to deep-link both sides later.
///
/// `statement_type` / `evidence_strength` are the required categorization
/// backbone (`String`); everything else is a display/locator field decoded as
/// `Option` (see the module note on null-vs-skip).
#[derive(Debug, Clone, Serialize)]
pub struct ProofEdge {
    // ── answer side (Evidence) ──────────────────────────────────
    /// The discovery answer text (`Evidence.answer`).
    pub answer: Option<String>,
    /// The question the answer responds to (`Evidence.question`).
    pub question: Option<String>,
    /// The verbatim quote backing the answer (`Evidence.verbatim_quote`).
    pub evidence_verbatim_quote: Option<String>,
    /// Answer classification — required grouping key (`Evidence.statement_type`).
    pub statement_type: String,
    /// Corroboration strength tier — required grouping key
    /// (`Evidence.evidence_strength`).
    pub evidence_strength: String,
    /// The answer's Q-number locator, e.g. `"Q4"` (`Evidence.paragraph`).
    pub paragraph: Option<String>,
    /// The answer's page in the source document (`Evidence.page_number`, an
    /// integer in the graph).
    pub page_number: Option<i64>,
    /// The source document the answer came from (`Evidence.source_document`).
    pub source_document: Option<String>,
    // ── allegation side (Allegation) ────────────────────────────
    /// The corroborated complaint allegation's summary (`Allegation.summary`).
    pub allegation_summary: Option<String>,
    /// The allegation's short title (`Allegation.title`).
    pub allegation_title: Option<String>,
    /// The allegation's complaint paragraph number, e.g. `"54"`
    /// (`Allegation.paragraph_number`, a string in the graph).
    pub allegation_paragraph_number: Option<String>,
    /// The allegation's stable id, for deep-linking (`Allegation.id`).
    pub allegation_id: Option<String>,
}

/// Sub-view 3: a preserved non-answer that produced no `CORROBORATES` edge.
/// Same answer-side fields as [`ProofEdge`], with no allegation side (there is
/// no edge to an allegation — that is the whole point of this list).
#[derive(Debug, Clone, Serialize)]
pub struct ExcludedEvidence {
    pub answer: Option<String>,
    pub question: Option<String>,
    pub evidence_verbatim_quote: Option<String>,
    /// Required: which non-answer category excluded this (`evasive` /
    /// `objection` / `referral` / `denial`).
    pub statement_type: String,
    pub paragraph: Option<String>,
    pub page_number: Option<i64>,
    pub source_document: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Pins the top-level wire contract: the four sections plus the echoed
    /// `case_slug` / `document_id`, with exact snake_case field names. A rename
    /// or an accidental `camelCase` serde attribute would break the frontend
    /// silently — this catches it at `cargo test` time.
    #[test]
    fn response_serializes_with_expected_shape() {
        let response = ProofReviewResponse {
            case_slug: "awad_v_catholic_family_service".to_string(),
            document_id: None,
            summary: ProofReviewSummary {
                corroborating: CorroboratingSummary {
                    total: 1,
                    by_statement_type: vec![StatementTypeCount {
                        statement_type: "admission".to_string(),
                        count: 1,
                    }],
                    by_category: vec![CategoryCount {
                        statement_type: "admission".to_string(),
                        evidence_strength: "sworn_party_admission".to_string(),
                        count: 1,
                    }],
                },
                excluded: ExcludedSummary {
                    total: 0,
                    by_statement_type: vec![],
                },
            },
            proof_edges: vec![ProofEdge {
                answer: Some("Yes.".to_string()),
                question: Some("Did you sign it?".to_string()),
                evidence_verbatim_quote: Some("Yes, I signed it.".to_string()),
                statement_type: "admission".to_string(),
                evidence_strength: "sworn_party_admission".to_string(),
                paragraph: Some("Q4".to_string()),
                page_number: Some(12),
                source_document: Some("doc-george".to_string()),
                allegation_summary: Some("The agreement was signed.".to_string()),
                allegation_title: Some("Signature".to_string()),
                allegation_paragraph_number: Some("54".to_string()),
                allegation_id: Some("alleg-1".to_string()),
            }],
            excluded: vec![],
            borderline: vec![],
        };

        let value = serde_json::to_value(&response).expect("response serializes");
        assert_eq!(
            value,
            json!({
                "case_slug": "awad_v_catholic_family_service",
                "document_id": null,
                "summary": {
                    "corroborating": {
                        "total": 1,
                        "by_statement_type": [{"statement_type": "admission", "count": 1}],
                        "by_category": [{
                            "statement_type": "admission",
                            "evidence_strength": "sworn_party_admission",
                            "count": 1
                        }]
                    },
                    "excluded": {"total": 0, "by_statement_type": []}
                },
                "proof_edges": [{
                    "answer": "Yes.",
                    "question": "Did you sign it?",
                    "evidence_verbatim_quote": "Yes, I signed it.",
                    "statement_type": "admission",
                    "evidence_strength": "sworn_party_admission",
                    "paragraph": "Q4",
                    "page_number": 12,
                    "source_document": "doc-george",
                    "allegation_summary": "The agreement was signed.",
                    "allegation_title": "Signature",
                    "allegation_paragraph_number": "54",
                    "allegation_id": "alleg-1"
                }],
                "excluded": [],
                "borderline": []
            })
        );
    }

    /// A missing display field must serialize as `null`, never be omitted, so
    /// "absent in the graph" stays distinguishable from "empty string"
    /// (Standing Rule 1). This guards against anyone adding
    /// `skip_serializing_if` to the `Option` fields later.
    #[test]
    fn absent_optional_fields_serialize_as_null_not_omitted() {
        let edge = ProofEdge {
            answer: None,
            question: None,
            evidence_verbatim_quote: None,
            statement_type: "partial_admission".to_string(),
            evidence_strength: "sworn_party_admission".to_string(),
            paragraph: None,
            page_number: None,
            source_document: None,
            allegation_summary: None,
            allegation_title: None,
            allegation_paragraph_number: None,
            allegation_id: None,
        };
        let value = serde_json::to_value(&edge).expect("edge serializes");
        let obj = value.as_object().expect("object body");
        // Every field is present as a key (12 fields), and the optional ones
        // are JSON null rather than absent.
        assert_eq!(obj.len(), 12);
        assert!(obj.get("answer").expect("answer key present").is_null());
        assert!(obj
            .get("page_number")
            .expect("page_number key present")
            .is_null());
        assert_eq!(
            obj.get("statement_type").and_then(|v| v.as_str()),
            Some("partial_admission")
        );
    }

    /// Empty sections must serialize as `[]`, never `null` — "no rows" stays
    /// distinguishable from "field absent" at the wire boundary.
    #[test]
    fn empty_sections_serialize_as_arrays_not_null() {
        let response = ProofReviewResponse {
            case_slug: "c".to_string(),
            document_id: Some("doc-x".to_string()),
            summary: ProofReviewSummary {
                corroborating: CorroboratingSummary {
                    total: 0,
                    by_statement_type: vec![],
                    by_category: vec![],
                },
                excluded: ExcludedSummary {
                    total: 0,
                    by_statement_type: vec![],
                },
            },
            proof_edges: vec![],
            excluded: vec![],
            borderline: vec![],
        };
        let value = serde_json::to_value(&response).expect("serializes");
        assert!(
            value["proof_edges"].is_array() && value["proof_edges"].as_array().unwrap().is_empty()
        );
        assert!(value["excluded"].is_array());
        assert!(value["borderline"].is_array());
        assert_eq!(value["document_id"], json!("doc-x"));
    }
}
