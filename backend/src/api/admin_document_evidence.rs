//! Admin endpoint: per-document content listing with audit status.
//!
//! Queries Neo4j for ALL extracted content linked to a document — not just
//! Evidence nodes, but also ComplaintAllegation, LegalCount, Harm, and
//! MotionClaim nodes. Each result includes a `node_type` field so the
//! frontend can render type-specific cards.
//!
//! The query uses UNION ALL across five node types, with `toString()`
//! coercion on integer fields to satisfy Neo4j's strict type-matching
//! requirement for UNION columns.
//!
//! ## Rust Learning: Joining Neo4j and PostgreSQL Data
//!
//! The primary data (content nodes, relationships) lives in Neo4j,
//! while audit metadata (who verified what, when) lives in PostgreSQL.
//! We query both and merge using a HashMap for O(1) lookup — efficient
//! and idiomatic Rust (O(n) instead of O(n²) nested loops).

use axum::{extract::Path, extract::State, Json};
use serde::Serialize;
use std::collections::HashMap;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::state::AppState;

use super::admin_document_evidence_queries::{fetch_content_for_document, fetch_document_meta};

// ── Response DTOs ────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct DocumentEvidenceResponse {
    pub document_id: String,
    pub document_title: String,
    /// Document source type — tells the frontend whether text highlighting
    /// is available (e.g. "native_pdf", "docx_converted", "scanned").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,
    pub evidence_count: usize,
    pub verified_count: usize,
    pub flagged_count: usize,
    pub evidence: Vec<EvidenceWithAudit>,
}

#[derive(Debug, Serialize)]
pub struct EvidenceWithAudit {
    pub id: String,
    /// Node label: "Evidence", "ComplaintAllegation", "LegalCount", "Harm", "MotionClaim".
    pub node_type: String,
    pub title: Option<String>,
    pub verbatim_quote: Option<String>,
    /// Page number or paragraph number as a string (coerced via `toString()`
    /// in the UNION ALL query because different branches have different types).
    pub page_number: Option<String>,
    pub kind: Option<String>,
    pub weight: Option<String>,
    pub speaker: Option<String>,
    pub verification: Option<VerificationStatus>,
    pub flags: Vec<FlagEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerificationStatus {
    pub status: String,
    pub notes: Option<String>,
    pub verified_by: String,
    pub verified_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FlagEntry {
    pub severity: String,
    pub description: Option<String>,
    pub flagged_by: String,
    pub flagged_at: String,
}

// ── PostgreSQL row types ─────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
struct VerificationRow {
    evidence_id: String,
    status: String,
    notes: Option<String>,
    verified_by: String,
    verified_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct FlagRow {
    evidence_id: Option<String>,
    severity: String,
    description: Option<String>,
    found_by: String,
    found_at: chrono::DateTime<chrono::Utc>,
}

// ── Handler ──────────────────────────────────────────────────────

/// GET /admin/documents/:id/evidence
///
/// Returns all evidence nodes linked to a document, with verification
/// status and flags from PostgreSQL.
pub async fn get_document_evidence(
    user: AuthUser,
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
) -> Result<Json<DocumentEvidenceResponse>, AppError> {
    require_admin(&user)?;

    tracing::info!(
        user = %user.username,
        doc_id = %doc_id,
        "GET /admin/documents/{}/evidence", doc_id
    );

    // 1. Get document metadata from Neo4j
    let (doc_title, source_type) = fetch_document_meta(&state.graph, &doc_id).await?;

    // 2. Query Neo4j for ALL content linked to this document
    let mut evidence_nodes = fetch_content_for_document(&state.graph, &doc_id).await?;

    // Sort in Rust — Neo4j UNION ALL does not preserve per-branch ORDER BY.
    // Sort by page_number (numerically where possible), then by title.
    evidence_nodes.sort_by(|a, b| {
        let pa = a.page_number.as_deref().and_then(|s| s.parse::<i64>().ok());
        let pb = b.page_number.as_deref().and_then(|s| s.parse::<i64>().ok());
        pa.cmp(&pb).then_with(|| a.title.cmp(&b.title))
    });

    // 3. Query PostgreSQL for verifications
    let verifications = sqlx::query_as::<_, VerificationRow>(
        "SELECT evidence_id, status, notes, verified_by, verified_at
         FROM audit_verifications
         WHERE document_id = $1
         ORDER BY verified_at DESC",
    )
    .bind(&doc_id)
    .fetch_all(&state.pg_pool)
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Failed to query verifications: {e}"),
    })?;

    // 4. Query PostgreSQL for flags
    let flags = sqlx::query_as::<_, FlagRow>(
        "SELECT evidence_id, severity, description, found_by, found_at
         FROM audit_findings
         WHERE document_id = $1 AND status = 'open'
         ORDER BY found_at DESC",
    )
    .bind(&doc_id)
    .fetch_all(&state.pg_pool)
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Failed to query findings: {e}"),
    })?;

    // 5. Build lookup maps — most recent verification per evidence
    //
    // ## Rust Learning: HashMap::entry for deduplication
    // We only keep the most recent verification per evidence_id.
    // Since results are ORDER BY verified_at DESC, the first one
    // we see for each evidence_id is the most recent.
    let mut verification_map: HashMap<String, VerificationStatus> = HashMap::new();
    for v in &verifications {
        verification_map.entry(v.evidence_id.clone()).or_insert_with(|| {
            VerificationStatus {
                status: v.status.clone(),
                notes: v.notes.clone(),
                verified_by: v.verified_by.clone(),
                verified_at: v.verified_at.to_rfc3339(),
            }
        });
    }

    let mut flag_map: HashMap<String, Vec<FlagEntry>> = HashMap::new();
    for f in &flags {
        if let Some(ref eid) = f.evidence_id {
            flag_map.entry(eid.clone()).or_default().push(FlagEntry {
                severity: f.severity.clone(),
                description: f.description.clone(),
                flagged_by: f.found_by.clone(),
                flagged_at: f.found_at.to_rfc3339(),
            });
        }
    }

    // 6. Merge evidence nodes with audit data
    let mut verified_count = 0usize;
    let mut flagged_count = 0usize;

    let evidence: Vec<EvidenceWithAudit> = evidence_nodes
        .into_iter()
        .map(|e| {
            let verification = verification_map.get(&e.id).cloned();
            let flags = flag_map.get(&e.id).cloned().unwrap_or_default();

            if verification.as_ref().is_some_and(|v| v.status == "verified") {
                verified_count += 1;
            }
            if !flags.is_empty() {
                flagged_count += 1;
            }

            EvidenceWithAudit {
                id: e.id,
                node_type: e.node_type,
                title: e.title,
                verbatim_quote: e.verbatim_quote,
                page_number: e.page_number,
                kind: e.kind,
                weight: e.weight,
                speaker: e.speaker,
                verification,
                flags,
            }
        })
        .collect();

    let evidence_count = evidence.len();

    Ok(Json(DocumentEvidenceResponse {
        document_id: doc_id,
        document_title: doc_title,
        source_type,
        evidence_count,
        verified_count,
        flagged_count,
        evidence,
    }))
}

// Neo4j query helpers live in admin_document_evidence_queries.rs
// (extracted to keep this module under 300 lines).
