//! Admin endpoint for bulk evidence import from reviewed JSON.
//!
//! ## Rust Learning: Neo4j Transactions
//!
//! `graph.start_txn()` opens an explicit transaction. All Cypher run inside
//! it is atomic — either all succeed or all roll back. If the `Txn` is
//! dropped without `.commit()`, it auto-rolls back. This is critical for
//! evidence import: we don't want orphaned nodes if one item fails.

use axum::{extract::State, http::StatusCode, Json};
use neo4rs::query;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::document_repository::DocumentRepository;
use crate::state::AppState;

use super::admin_evidence_helpers::{create_relationship, create_relationship_labelless_target};

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// A single evidence item to import.
#[derive(Debug, Deserialize)]
pub struct EvidenceImportItem {
    pub id: String,
    pub title: String,
    pub content: String,
    pub verbatim_quote: Option<String>,
    pub page_number: Option<i32>,
    pub date: Option<String>,
    pub topic: Option<String>,

    /// Person or org ID who made the statement (e.g., "george-phillips").
    pub stated_by: String,
    #[serde(default)]
    pub about: Vec<String>,
    #[serde(default)]
    pub supports_counts: Vec<String>,
    #[serde(default)]
    pub contradicts: Vec<ContradictionRef>,
    #[serde(default)]
    pub rebuts: Vec<String>,
    #[serde(default)]
    pub proves_allegations: Vec<String>,
}

/// Reference to an evidence node that the new evidence contradicts.
#[derive(Debug, Deserialize)]
pub struct ContradictionRef {
    pub evidence_id: String,
    /// Brief description of what's contradicted.
    pub topic: Option<String>,
    /// The conflicting values (e.g., "none vs three").
    pub value: Option<String>,
}

/// Full import request — a document ID and array of evidence items.
#[derive(Debug, Deserialize)]
pub struct ImportEvidenceRequest {
    pub document_id: String,
    pub evidence: Vec<EvidenceImportItem>,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Response showing what was created.
#[derive(Debug, Serialize)]
pub struct ImportEvidenceResponse {
    pub created: usize,
    pub relationships: RelationshipCounts,
}

#[derive(Debug, Serialize, Default)]
pub struct RelationshipCounts {
    pub contained_in: usize,
    pub stated_by: usize,
    pub about: usize,
    pub supports: usize,
    pub contradicts: usize,
    pub rebuts: usize,
    pub proves: usize,
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// POST /api/admin/evidence — Import reviewed evidence into Neo4j.
///
/// Creates Evidence nodes and all relationships in a single transaction.
/// If any validation fails, the entire import rolls back.
pub async fn import_evidence(
    user: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ImportEvidenceRequest>,
) -> Result<(StatusCode, Json<ImportEvidenceResponse>), AppError> {
    require_admin(&user)?;
    tracing::info!(
        user = %user.username,
        doc_id = %req.document_id,
        count = req.evidence.len(),
        "POST /api/admin/evidence"
    );

    if req.evidence.is_empty() {
        return Err(AppError::BadRequest {
            message: "evidence array must not be empty".to_string(),
            details: json!({ "field": "evidence" }),
        });
    }

    // 1. Validate document exists
    let repo = DocumentRepository::new(state.graph.clone());
    repo.get_document_by_id(&req.document_id).await.map_err(|_| {
        AppError::NotFound {
            message: format!("Document '{}' not found", req.document_id),
        }
    })?;

    // 2. Check for duplicate evidence IDs (batch check before transaction)
    for item in &req.evidence {
        let mut result = state
            .graph
            .execute(
                query("MATCH (e:Evidence {id: $id}) RETURN e.id")
                    .param("id", item.id.as_str()),
            )
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Neo4j query failed: {e}"),
            })?;

        if result.next().await.map_err(|e| AppError::Internal {
            message: format!("Neo4j row fetch failed: {e}"),
        })?.is_some() {
            return Err(AppError::Conflict {
                message: format!("Evidence '{}' already exists", item.id),
                details: json!({ "existing_id": item.id }),
            });
        }
    }

    // 3. Open transaction — all-or-nothing
    let mut txn = state.graph.start_txn().await.map_err(|e| AppError::Internal {
        message: format!("Failed to start transaction: {e}"),
    })?;

    let mut counts = RelationshipCounts::default();

    for item in &req.evidence {
        // 4a. Create the Evidence node
        txn.run(
            query(
                "CREATE (e:Evidence {
                    id: $id, title: $title, content: $content,
                    verbatim_quote: $quote, page_number: $page,
                    date: $date, topic: $topic
                })",
            )
            .param("id", item.id.as_str())
            .param("title", item.title.as_str())
            .param("content", item.content.as_str())
            .param("quote", item.verbatim_quote.clone())
            .param("page", item.page_number)
            .param("date", item.date.clone())
            .param("topic", item.topic.clone()),
        )
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to create Evidence '{}': {e}", item.id),
        })?;

        // 4b. CONTAINED_IN → Document
        create_relationship(
            &mut txn, &item.id, &req.document_id,
            "Evidence", "Document",
            "CONTAINED_IN", None,
        ).await.map_err(|msg| AppError::Internal { message: msg })?;
        counts.contained_in += 1;

        // 4c. STATED_BY → Person or Organization (label-free match)
        create_relationship_labelless_target(
            &mut txn, &item.id, &item.stated_by,
            "STATED_BY",
        ).await.map_err(|_| AppError::BadRequest {
            message: format!(
                "Person/Org '{}' not found — check stated_by for evidence '{}'",
                item.stated_by, item.id
            ),
            details: json!({ "field": "stated_by", "evidence_id": item.id, "target_id": item.stated_by }),
        })?;
        counts.stated_by += 1;

        // 4d. ABOUT → Person(s)
        for person_id in &item.about {
            create_relationship_labelless_target(
                &mut txn, &item.id, person_id,
                "ABOUT",
            ).await.map_err(|_| AppError::BadRequest {
                message: format!(
                    "Person/Org '{person_id}' not found — check about[] for evidence '{}'",
                    item.id
                ),
                details: json!({ "field": "about", "evidence_id": item.id, "target_id": person_id }),
            })?;
            counts.about += 1;
        }

        // 4e. SUPPORTS → LegalCount(s)
        for count_id in &item.supports_counts {
            create_relationship(
                &mut txn, &item.id, count_id,
                "Evidence", "LegalCount",
                "SUPPORTS", None,
            ).await.map_err(|_| AppError::BadRequest {
                message: format!(
                    "LegalCount '{count_id}' not found — check supports_counts[] for evidence '{}'",
                    item.id
                ),
                details: json!({ "field": "supports_counts", "evidence_id": item.id, "target_id": count_id }),
            })?;
            counts.supports += 1;
        }

        // 4f. CONTRADICTS → Evidence (with optional properties)
        for cref in &item.contradicts {
            let props = Some(json!({
                "topic": cref.topic,
                "value": cref.value,
            }));
            create_relationship(
                &mut txn, &item.id, &cref.evidence_id,
                "Evidence", "Evidence",
                "CONTRADICTS", props.as_ref(),
            ).await.map_err(|_| AppError::BadRequest {
                message: format!(
                    "Evidence '{}' not found — check contradicts[] for evidence '{}'",
                    cref.evidence_id, item.id
                ),
                details: json!({ "field": "contradicts", "evidence_id": item.id, "target_id": cref.evidence_id }),
            })?;
            counts.contradicts += 1;
        }

        // 4g. REBUTS → Evidence
        for target_id in &item.rebuts {
            create_relationship(
                &mut txn, &item.id, target_id,
                "Evidence", "Evidence",
                "REBUTS", None,
            ).await.map_err(|_| AppError::BadRequest {
                message: format!(
                    "Evidence '{target_id}' not found — check rebuts[] for evidence '{}'",
                    item.id
                ),
                details: json!({ "field": "rebuts", "evidence_id": item.id, "target_id": target_id }),
            })?;
            counts.rebuts += 1;
        }

        // 4h. PROVES → ComplaintAllegation
        for allegation_id in &item.proves_allegations {
            create_relationship(
                &mut txn, &item.id, allegation_id,
                "Evidence", "ComplaintAllegation",
                "PROVES", None,
            ).await.map_err(|_| AppError::BadRequest {
                message: format!(
                    "ComplaintAllegation '{allegation_id}' not found — check proves_allegations[] for evidence '{}'",
                    item.id
                ),
                details: json!({ "field": "proves_allegations", "evidence_id": item.id, "target_id": allegation_id }),
            })?;
            counts.proves += 1;
        }
    }

    // 5. Commit — if we got here, everything validated
    txn.commit().await.map_err(|e| AppError::Internal {
        message: format!("Transaction commit failed: {e}"),
    })?;

    let created = req.evidence.len();
    tracing::info!(
        user = %user.username,
        created,
        rels = counts.contained_in + counts.stated_by + counts.about
            + counts.supports + counts.contradicts + counts.rebuts + counts.proves,
        "Evidence import complete"
    );

    Ok((StatusCode::CREATED, Json(ImportEvidenceResponse { created, relationships: counts })))
}

