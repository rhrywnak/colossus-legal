//! `GET /api/cases/:slug/proof-matrix/export.docx?count=<n>` — one Count as a
//! Word document (PROOF_MATRIX_v2 §4).
//!
//! ## Why this reads the Elements one at a time
//!
//! It could be one wide Cypher. It is instead a loop over
//! [`fetch_element_with_allegations`] — the SAME read the drill-down uses,
//! followed by the SAME ordering overlay — so the document cannot disagree with
//! the page it was exported from. A Count holds four to six Elements, so the loop
//! is a handful of queries on a control a reader presses occasionally; a second
//! query shaped differently would be a second definition of "what is under this
//! accusation", free to drift from the first.
//!
//! ## What is NOT in the document
//!
//! Hidden items, and anything past the stored `matrix_visible_items`. Both
//! decisions live in `services::matrix_export`, which is where they are tested;
//! this handler gathers and responds.

use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use serde::Deserialize;
use tracing::{error, info, instrument};

use crate::auth::AuthUser;
use crate::domain::settings::Settings;
use crate::repositories::causes_of_action_repository::{fetch_counts, fetch_elements};
use crate::repositories::element_detail_repository::{
    fetch_element_with_allegations, AllegationSummary,
};
use crate::repositories::pipeline_repository::evidence_allegation_rulings::list_rulings_for_allegations;
use crate::services::matrix_detail::apply_rulings_and_order;
use crate::services::matrix_export::build_document;
use crate::services::matrix_export_docx::render;
use crate::state::AppState;

/// The MIME type Word registers for `.docx`.
// CONST: an IANA media type — a protocol fact, not a deployment value.
const DOCX_MEDIA_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document";

/// `?count=<n>`.
///
/// ## serde: `deny_unknown_fields` — a mistyped parameter is refused, not ignored
///
/// The house posture for every request shape (`UpdateNotesRequest`,
/// `DocumentSearchQuery`, `RulingRequest`). Without it, `?Count=1` decodes as a
/// MISSING `count` and the request fails with a bland parse error naming nothing;
/// with it, the offending key is named. `count` itself has no `#[serde(default)]`
/// and is deliberately required — a defaulted 0 would export a Count that does
/// not exist as a document with a title and nothing under it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportQuery {
    pub count: i64,
}

/// Failure classes, mapped to statuses the caller can act on.
pub enum ExportError {
    /// No Count with that number, or it has no Elements loaded. → 404.
    NotFound { count: i64 },
    /// Graph, Postgres or packing failure. → 500, logged, never echoed.
    Internal,
}

impl IntoResponse for ExportError {
    fn into_response(self) -> Response {
        // Bland bodies, matching the sibling proof-matrix endpoints: an export
        // failure must not put Cypher or a file-format error in front of a
        // reader. The operator log carries the detail.
        match self {
            ExportError::NotFound { count } => (
                StatusCode::NOT_FOUND,
                axum::Json(serde_json::json!({
                    "error": "count not found or has no elements loaded",
                    "count": count
                })),
            )
                .into_response(),
            ExportError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({ "error": "internal server error" })),
            )
                .into_response(),
        }
    }
}

/// `GET …/proof-matrix/export.docx?count=<n>`.
///
/// Open read (`Option<AuthUser>`), matching every other read on this page: the
/// Matrix is a reference Chuck opens, and reading it is not an edit. The user is
/// logged when present so an operator can see who exported what.
#[instrument(skip(state, user), fields(slug = %slug, count = query.count))]
pub async fn get_count_export(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<ExportQuery>,
) -> Result<Response, ExportError> {
    let username = user
        .as_ref()
        .map(|u| u.username.as_str())
        .unwrap_or("<anonymous>");
    info!(
        username,
        "GET /api/cases/{slug}/proof-matrix/export.docx?count={}", query.count
    );

    let settings = state.settings.current();
    let count_name = count_name(&state, query.count).await?;
    let elements = gather_elements(&state, query.count, &settings).await?;

    let document = build_document(
        query.count,
        &count_name,
        &elements,
        settings.matrix_visible_items,
        &Utc::now().format("%Y-%m-%d").to_string(),
        &settings.matrix_wording,
    );
    let bytes = render(&document).map_err(|e| {
        error!(
            error = %e,
            count = query.count,
            "could not pack the Proof Matrix export"
        );
        ExportError::Internal
    })?;

    info!(
        count = query.count,
        elements = document.elements.len(),
        bytes = bytes.len(),
        "exported a Count as a Word document"
    );
    Ok(docx_response(query.count, bytes))
}

/// The Count's title, or a 404 if there is no such Count.
///
/// ## Why a Count with no title is still exported
///
/// `lc.title` is `Option` in the graph. A Count that exists but is untitled gets
/// an empty name in the heading rather than a 404: the accusations under it are
/// real, and refusing to export them because a label is missing would withhold
/// the evidence over the caption.
async fn count_name(state: &AppState, count: i64) -> Result<String, ExportError> {
    let counts = fetch_counts(&state.graph).await.map_err(|e| {
        error!(error = %e, count, "could not read the Counts for the export");
        ExportError::Internal
    })?;
    counts
        .into_iter()
        .find(|c| c.count_number == count)
        .map(|c| c.count_name.unwrap_or_default())
        .ok_or(ExportError::NotFound { count })
}

/// Every Element of the Count, in order, with its accusations ordered and ruled.
///
/// Returns `NotFound` when the Count has no Elements: a document with a title and
/// nothing under it would look like an export that worked.
async fn gather_elements(
    state: &AppState,
    count: i64,
    settings: &Settings,
) -> Result<Vec<(String, Vec<AllegationSummary>)>, ExportError> {
    let mut rows: Vec<_> = fetch_elements(&state.graph)
        .await
        .map_err(|e| {
            error!(error = %e, count, "could not read the Elements for the export");
            ExportError::Internal
        })?
        .into_iter()
        .filter(|row| row.count_number == count)
        .collect();
    if rows.is_empty() {
        return Err(ExportError::NotFound { count });
    }
    // Same ordering the page uses: the authored position, with anything unordered
    // last, then by name so the document is stable between exports.
    rows.sort_by(|a, b| {
        a.order_in_count
            .unwrap_or(i64::MAX)
            .cmp(&b.order_in_count.unwrap_or(i64::MAX))
            .then_with(|| a.element_name.cmp(&b.element_name))
    });

    let mut elements = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        let heading = format!(
            "{count}.{} {}",
            row.order_in_count.unwrap_or(index as i64 + 1),
            row.element_name
        );
        elements.push((
            heading,
            allegations_for(state, &row.element_id, settings).await?,
        ));
    }
    Ok(elements)
}

/// One Element's accusations, read and ordered exactly as the drill-down does.
async fn allegations_for(
    state: &AppState,
    element_id: &str,
    settings: &Settings,
) -> Result<Vec<AllegationSummary>, ExportError> {
    let mut detail = fetch_element_with_allegations(&state.graph, &state.pipeline_pool, element_id)
        .await
        .map_err(|e| {
            error!(error = ?e, element_id, "could not read an Element for the export");
            ExportError::Internal
        })?;

    let allegation_ids: Vec<String> = detail
        .allegations
        .iter()
        .map(|a| a.allegation_id.clone())
        .collect();
    let rulings = list_rulings_for_allegations(&state.pipeline_pool, &allegation_ids)
        .await
        .map_err(|e| {
            // Same refusal the drill-down makes, for the same reason and with more
            // force: a document printed with every ruling missing would carry no ✓
            // at all, and the footer would then claim confirmation of nothing.
            error!(
                error = %e,
                element_id,
                "could not read the human rulings for the export — the document is \
                 refused rather than printed without its confirmations"
            );
            ExportError::Internal
        })?;

    apply_rulings_and_order(&mut detail, &rulings, &settings.matrix_wording);
    Ok(detail.allegations)
}

/// The bytes, with the headers that make a browser save them as a file.
///
/// ## Why the filename is built here and not stored
///
/// It is not a sentence anybody reads — it is an identifier that lands in a
/// downloads folder, in the same category as a URL path segment. Keeping it ASCII
/// and predictable is what makes it safe in a `Content-Disposition` header, which
/// is a place a translated string with a quote in it would break.
fn docx_response(count: i64, bytes: Vec<u8>) -> Response {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, DOCX_MEDIA_TYPE.to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"proof-matrix-count-{count}.docx\""),
            ),
        ],
        Body::from(bytes),
    )
        .into_response()
}

#[cfg(test)]
#[path = "proof_matrix_export_tests.rs"]
mod tests;
