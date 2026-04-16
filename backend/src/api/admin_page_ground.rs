//! Admin endpoint: ground document items to PDF page numbers.
//!
//! Uses colossus-pdf to search the document's PDF text layer for each
//! item's verbatim quote (or title fallback), returning the page where
//! each was found. Optionally persists discovered page numbers to Neo4j.
//!
//! ## Rust Learning: spawn_blocking for sync libraries
//!
//! colossus-pdf is synchronous (built on pdf_oxide). Axum handlers are
//! async. Calling sync I/O in an async context blocks the tokio runtime.
//! `tokio::task::spawn_blocking` moves the work to a dedicated thread
//! pool, returning a Future that resolves when the blocking work completes.

use axum::{extract::Path, extract::State, Json};
use neo4rs::query;
use serde::{Deserialize, Serialize};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::document_repository::DocumentRepository;
use crate::state::AppState;

use super::admin_document_evidence_queries::{fetch_content_for_document, fetch_document_meta};

// ── Request / Response DTOs ──────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct GroundPagesRequest {
    #[serde(default)]
    pub persist: bool,
}

#[derive(Debug, Serialize)]
pub struct GroundPagesResponse {
    pub document_id: String,
    pub document_title: String,
    pub pdf_pages: u32,
    pub items_processed: usize,
    pub items_grounded: usize,
    pub items_not_found: usize,
    pub persisted: bool,
    pub results: Vec<GroundingItemResult>,
}

#[derive(Debug, Serialize)]
pub struct GroundingItemResult {
    pub node_id: String,
    pub node_type: String,
    pub title: Option<String>,
    pub snippet_used: Option<String>,
    pub page_found: Option<u32>,
    pub match_type: String,
    pub previous_page: Option<String>,
}

// ── Handler ──────────────────────────────────────────────────────

/// POST /admin/documents/:id/ground-pages
///
/// Reads the document's PDF, searches for each item's text snippet,
/// and returns the page number where each was found. If `persist: true`,
/// updates Neo4j page_number properties.
pub async fn ground_pages(
    user: AuthUser,
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
    body: Option<Json<GroundPagesRequest>>,
) -> Result<Json<GroundPagesResponse>, AppError> {
    require_admin(&user)?;
    let persist = body.map(|b| b.persist).unwrap_or(false);

    // 1. Fetch document metadata (title) and file_path
    let (doc_title, _source_type) = fetch_document_meta(&state.graph, &doc_id).await?;

    let repo = DocumentRepository::new(state.graph.clone());
    let document = repo
        .get_document_by_id(&doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to fetch document: {e:?}"),
        })?;

    let file_path = document.file_path.ok_or_else(|| AppError::BadRequest {
        message: "Document has no associated PDF file".to_string(),
        details: serde_json::json!({}),
    })?;

    // Security: prevent path traversal
    if file_path.contains("..") || file_path.contains('/') || file_path.contains('\\') {
        return Err(AppError::BadRequest {
            message: "Invalid file path".to_string(),
            details: serde_json::json!({}),
        });
    }

    let full_path = format!("{}/{}", state.config.document_storage_path, file_path);

    // Verify the PDF exists before doing more work
    if !tokio::fs::try_exists(&full_path).await.unwrap_or(false) {
        return Err(AppError::NotFound {
            message: format!("PDF file not found: {full_path}"),
        });
    }

    // 2. Fetch all items for this document
    let items = fetch_content_for_document(&state.graph, &doc_id).await?;

    // 3. Build snippet list: (index, snippet_text) for items with usable text
    let mut snippet_map: Vec<(usize, String)> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        if let Some(text) = get_snippet_text(item) {
            snippet_map.push((i, text));
        }
    }

    let snippet_texts: Vec<String> = snippet_map.iter().map(|(_, s)| s.clone()).collect();

    // 4. Run colossus-pdf grounding in a blocking thread (it's sync)
    let pdf_path = full_path.clone();
    let grounding_results =
        tokio::task::spawn_blocking(move || run_grounding(&pdf_path, &snippet_texts))
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Grounding task failed: {e}"),
            })??;

    let pdf_pages = grounding_results.0;
    let pdf_results = grounding_results.1;

    // 5. Build response, mapping grounding results back to items
    let mut results = Vec::with_capacity(items.len());
    let mut grounded_count = 0usize;
    let mut not_found_count = 0usize;

    // Track which items got grounding results
    let mut grounding_by_index: Vec<Option<(Option<u32>, String)>> = vec![None; items.len()];
    for (map_idx, gr) in pdf_results.iter().enumerate() {
        let item_idx = snippet_map[map_idx].0;
        grounding_by_index[item_idx] = Some((gr.page_number, format!("{:?}", gr.match_type)));
    }

    for (i, item) in items.iter().enumerate() {
        let snippet_used = get_snippet_text(item);
        let (page_found, match_type) = match &grounding_by_index[i] {
            Some((page, mt)) => (*page, mt.clone()),
            None => (None, "NoSnippet".to_string()),
        };

        if page_found.is_some() {
            grounded_count += 1;
        } else {
            not_found_count += 1;
        }

        results.push(GroundingItemResult {
            node_id: item.id.clone(),
            node_type: item.node_type.clone(),
            title: item.title.clone(),
            snippet_used,
            page_found,
            match_type,
            previous_page: item.page_number.clone(),
        });
    }

    // 6. Persist to Neo4j if requested
    if persist {
        persist_page_numbers(&state.graph, &results).await?;
    }

    Ok(Json(GroundPagesResponse {
        document_id: doc_id,
        document_title: doc_title,
        pdf_pages,
        items_processed: items.len(),
        items_grounded: grounded_count,
        items_not_found: not_found_count,
        persisted: persist,
        results,
    }))
}

// ── Helpers ──────────────────────────────────────────────────────

/// Extract the best snippet text from an item for PDF search.
/// Prefers verbatim_quote, falls back to title.
fn get_snippet_text(item: &super::admin_document_evidence_queries::ContentNode) -> Option<String> {
    item.verbatim_quote
        .as_ref()
        .filter(|q| !q.is_empty())
        .or(item.title.as_ref())
        .filter(|t| !t.is_empty())
        .cloned()
}

/// Run PDF text extraction and snippet grounding (sync, runs in spawn_blocking).
/// Returns (total_pages, Vec<GroundingResult>).
fn run_grounding(
    pdf_path: &str,
    snippets: &[String],
) -> Result<(u32, Vec<colossus_pdf::GroundingResult>), AppError> {
    let mut extractor =
        colossus_pdf::PdfTextExtractor::open(pdf_path).map_err(|e| AppError::Internal {
            message: format!("Failed to open PDF: {e}"),
        })?;

    let pages = extractor
        .extract_all_pages()
        .map_err(|e| AppError::Internal {
            message: format!("Failed to extract PDF pages: {e}"),
        })?;
    let total_pages = pages.len() as u32;

    let snippet_refs: Vec<&str> = snippets.iter().map(|s| s.as_str()).collect();
    let mut grounder = colossus_pdf::PageGrounder::new(&mut extractor);
    let results = grounder
        .ground_snippets(&snippet_refs)
        .map_err(|e| AppError::Internal {
            message: format!("Grounding failed: {e}"),
        })?;

    Ok((total_pages, results))
}

/// Update Neo4j page_number properties for grounded items.
async fn persist_page_numbers(
    graph: &neo4rs::Graph,
    results: &[GroundingItemResult],
) -> Result<(), AppError> {
    let cypher = "MATCH (n {id: $node_id}) SET n.page_number = $page_number RETURN n.id AS id";

    for item in results {
        if let Some(page) = item.page_found {
            let mut result = graph
                .execute(
                    query(cypher)
                        .param("node_id", item.node_id.as_str())
                        .param("page_number", page as i64),
                )
                .await
                .map_err(|e| AppError::Internal {
                    message: format!("Failed to update page for {}: {e}", item.node_id),
                })?;
            // Consume the result stream to complete the query
            while result
                .next()
                .await
                .map_err(|e| AppError::Internal {
                    message: format!("Neo4j result error: {e}"),
                })?
                .is_some()
            {}
        }
    }

    Ok(())
}
