//! Admin endpoint: per-document evidence listing with audit status.
//!
//! Queries Neo4j for evidence linked to a document, then joins with
//! PostgreSQL verification and flag data.
//!
//! ## Rust Learning: Joining Neo4j and PostgreSQL Data
//!
//! The primary data (evidence nodes, relationships) lives in Neo4j,
//! while audit metadata (who verified what, when) lives in PostgreSQL.
//! We query both and merge using a HashMap for O(1) lookup — efficient
//! and idiomatic Rust (O(n) instead of O(n²) nested loops).

use axum::{extract::Path, extract::State, Json};
use neo4rs::query;
use serde::Serialize;
use std::collections::HashMap;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::state::AppState;

// ── Response DTOs ────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct DocumentEvidenceResponse {
    pub document_id: String,
    pub document_title: String,
    pub evidence_count: usize,
    pub verified_count: usize,
    pub flagged_count: usize,
    pub evidence: Vec<EvidenceWithAudit>,
}

#[derive(Debug, Serialize)]
pub struct EvidenceWithAudit {
    pub id: String,
    pub title: Option<String>,
    pub verbatim_quote: Option<String>,
    pub page_number: Option<i64>,
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

    // 1. Get document title from Neo4j
    let doc_title = fetch_document_title(&state.graph, &doc_id).await?;

    // 2. Query Neo4j for evidence linked to this document
    let evidence_nodes = fetch_evidence_for_document(&state.graph, &doc_id).await?;

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
        evidence_count,
        verified_count,
        flagged_count,
        evidence,
    }))
}

// ── Neo4j helpers ────────────────────────────────────────────────

/// Simple struct to hold evidence data from Neo4j before merging.
struct EvidenceNode {
    id: String,
    title: Option<String>,
    verbatim_quote: Option<String>,
    page_number: Option<i64>,
    kind: Option<String>,
    weight: Option<String>,
    speaker: Option<String>,
}

async fn fetch_document_title(graph: &neo4rs::Graph, doc_id: &str) -> Result<String, AppError> {
    let mut result = graph
        .execute(
            query("MATCH (d:Document {id: $doc_id}) RETURN d.title AS title")
                .param("doc_id", doc_id),
        )
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Neo4j query failed: {e}"),
        })?;

    if let Some(row) = result.next().await.map_err(|e| AppError::Internal {
        message: format!("Neo4j row fetch failed: {e}"),
    })? {
        Ok(row.get::<String>("title").unwrap_or_else(|_| doc_id.to_string()))
    } else {
        Err(AppError::NotFound {
            message: format!("Document not found: {doc_id}"),
        })
    }
}

async fn fetch_evidence_for_document(
    graph: &neo4rs::Graph,
    doc_id: &str,
) -> Result<Vec<EvidenceNode>, AppError> {
    let mut result = graph
        .execute(
            query(
                "MATCH (e:Evidence)-[:CONTAINED_IN]->(d:Document {id: $doc_id})
                 OPTIONAL MATCH (e)-[:STATED_BY]->(p:Person)
                 RETURN e.id AS id, e.title AS title, e.verbatim_quote AS verbatim_quote,
                        e.page_number AS page_number, e.kind AS kind, e.weight AS weight,
                        p.name AS speaker
                 ORDER BY e.page_number, e.title",
            )
            .param("doc_id", doc_id),
        )
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Neo4j query failed: {e}"),
        })?;

    let mut nodes = Vec::new();
    while let Some(row) = result.next().await.map_err(|e| AppError::Internal {
        message: format!("Neo4j row fetch failed: {e}"),
    })? {
        nodes.push(EvidenceNode {
            id: row.get("id").unwrap_or_default(),
            title: row.get("title").ok(),
            verbatim_quote: row.get("verbatim_quote").ok(),
            page_number: row.get("page_number").ok(),
            kind: row.get("kind").ok(),
            weight: row.get("weight").ok(),
            speaker: row.get("speaker").ok(),
        });
    }

    Ok(nodes)
}
