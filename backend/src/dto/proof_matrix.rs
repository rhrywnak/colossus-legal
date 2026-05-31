//! Response DTOs for `GET /api/cases/:slug/proof-matrix/rollup` — the first
//! piece of the Proof Matrix compute layer.
//!
//! This endpoint is the single source of truth for Count-level *deduped*
//! allegation totals: per `LegalCount`, the number of DISTINCT `Allegation`s
//! that bear on any of that Count's `Element`s. (Home will consume this later;
//! it is not wired here.)
//!
//! Serialize-only response types — these are produced by the backend and sent
//! to the client; nothing here is decoded from stored JSON, so (unlike the
//! causes-of-action DTOs) no `Deserialize` is needed.
//!
//! Field names are snake_case and align 1:1 with the Cypher result aliases
//! (`count_number`, `count_id`, `deduped_allegations`) so the wire shape is
//! traceable straight back to the query.

use serde::Serialize;

/// Top-level payload: the requested case slug (echoed) and the per-Count
/// rollup rows.
///
/// ## Rust Learning: `#[derive(Serialize)]` without `Deserialize`
///
/// `serde::Serialize` is the only half of the serde pair we need here: axum's
/// `Json(...)` wrapper turns this struct into the HTTP response body. We do not
/// derive `Deserialize` because the backend never parses this shape back in —
/// deriving only what is used keeps the type honest about its direction of flow
/// and avoids an unused trait impl.
#[derive(Debug, Clone, Serialize)]
pub struct ProofMatrixRollupResponse {
    /// Echoed from the request path. The Neo4j graph is single-case and not
    /// slug-namespaced, so the slug does not filter the query — it is returned
    /// for the caller's correlation, matching the causes-of-action contract.
    pub case_slug: String,
    /// One entry per `LegalCount` that has at least one bearing Allegation,
    /// ordered by `count_number` ascending (the query's `ORDER BY`).
    pub counts: Vec<CountRollup>,
}

/// One Count's deduped allegation total.
///
/// Domain note: `deduped_allegations` is `count(DISTINCT a)` *per Count*. An
/// Allegation that bears on several Elements of the same Count is counted once
/// here. This is deliberately NOT the same as summing the per-Element
/// allegation counts exposed by the causes-of-action endpoint, where that same
/// Allegation is counted once per Element it touches. See
/// `proof_matrix_repository` for the full reconciliation note.
#[derive(Debug, Clone, Serialize)]
pub struct CountRollup {
    /// The Count's ordinal (1-4 in the current case), from `LegalCount.count_number`.
    pub count_number: i64,
    /// The Count's stable identifier, from `LegalCount.id` (e.g. `"count-1"`).
    pub count_id: String,
    /// Number of DISTINCT Allegations bearing on any Element of this Count.
    pub deduped_allegations: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Pins the wire contract: top-level `case_slug` + `counts`, and each row's
    /// exact snake_case field names and value types. A rename of any field (or
    /// an accidental `camelCase` serde attribute) would break Home's later
    /// wiring silently — this catches it at `cargo test` time.
    #[test]
    fn rollup_response_serializes_with_expected_field_names() {
        let response = ProofMatrixRollupResponse {
            case_slug: "awad_v_catholic_family_service".to_string(),
            counts: vec![
                CountRollup {
                    count_number: 1,
                    count_id: "count-1".to_string(),
                    deduped_allegations: 51,
                },
                CountRollup {
                    count_number: 2,
                    count_id: "count-2".to_string(),
                    deduped_allegations: 41,
                },
            ],
        };

        let value = serde_json::to_value(&response).expect("response serializes");
        assert_eq!(
            value,
            json!({
                "case_slug": "awad_v_catholic_family_service",
                "counts": [
                    {"count_number": 1, "count_id": "count-1", "deduped_allegations": 51},
                    {"count_number": 2, "count_id": "count-2", "deduped_allegations": 41}
                ]
            })
        );
    }

    /// An empty graph (no bearing Allegations on any Count) must serialize as an
    /// empty array, never `null` — "no rows" stays distinguishable from "field
    /// absent" (Standing Rule 1). The handler turns this case into a 404 before
    /// it reaches serialization, but the DTO must still encode it correctly.
    #[test]
    fn empty_counts_serialize_as_array_not_null() {
        let response = ProofMatrixRollupResponse {
            case_slug: "awad_v_catholic_family_service".to_string(),
            counts: vec![],
        };
        let value = serde_json::to_value(&response).expect("response serializes");
        assert_eq!(
            value,
            json!({"case_slug": "awad_v_catholic_family_service", "counts": []})
        );
    }
}
