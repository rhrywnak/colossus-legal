//! Recording a document's own date (task P4b, B2 §3).
//!
//! ## One endpoint, two callers
//!
//! The ruling asks for two things: a date field on the intake screen, and a
//! post-hoc edit path for the nine documents already ingested. They are the same
//! write with the same rule, so they are the same endpoint —
//! `PUT /documents/:id/date` — called once by the upload dialog after the file
//! lands, and again by the document page whenever Roman corrects it.
//!
//! Two callers, one validation. A second write path would eventually validate
//! differently, and the difference would be invisible until a date was wrong.
//!
//! ## Mandatory-with-override, at the boundary
//!
//! `domain::date_precision::validate` owns the rule; this handler owns the HTTP
//! surface for it. A missing date with a real precision is a 400 whose message
//! names the override, so the operator is told what to send instead of being told
//! no. See that module for why "unknown" is an answer and not an absence.
//!
//! ## The Neo4j mirror
//!
//! Postgres is the record; the `Document` node gets a mirrored property so graph
//! queries can reach a document's date without a cross-store join. The mirror is
//! written only when the document has already been ingested (there is no node
//! before that), and a document dated before ingest gets its property from
//! `create_document_node`, which reads the row.
//!
//! ## No consumer wiring here (P4c)
//!
//! Nothing in this batch READS these values — not the timeline, not the .394 P4a
//! count-line fallback. Those ride later, named separately. This is the write
//! path and the storage, finished, and nothing else.

use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::domain::date_precision::{
    validate, DatePrecision, DatePrecisionError, ALL_DATE_PRECISIONS,
};
use crate::error::AppError;
use crate::repositories::pipeline_repository;
use crate::state::AppState;

/// What the intake dialog and the document page send.
///
/// `deny_unknown_fields` because this is a request body from a browser: a field
/// name the backend does not know is a client that has drifted, and reading it as
/// "no date supplied" would silently record the wrong thing. Better a 422 naming
/// the stray field.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetDocumentDateRequest {
    /// `YYYY-MM-DD`. For month precision this is the first of the month; for
    /// year precision, 1 January. Absent when `date_precision` is `unknown`.
    #[serde(default)]
    pub document_date: Option<String>,
    /// `day` | `month` | `year` | `unknown`.
    pub date_precision: String,
}

/// What either caller gets back — the stored state, read back rather than echoed.
#[derive(Debug, Clone, Serialize)]
pub struct DocumentDateResponse {
    pub document_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_date: Option<String>,
    pub date_precision: String,
    /// The label for the stored precision, so the page does not re-derive it.
    pub date_precision_label: String,
}

/// One precision the intake control can offer.
#[derive(Debug, Clone, Serialize)]
pub struct DatePrecisionOption {
    pub value: String,
    pub label: String,
    /// Whether choosing this precision requires a date alongside it.
    pub requires_date: bool,
}

/// `GET /documents/date-precisions` — the vocabulary, for the intake control.
///
/// ## Why the frontend does not hardcode this list
///
/// Standing Rule 12: no business logic in the frontend. Which precisions exist,
/// and which of them require a date, is a backend decision — and a hardcoded
/// TypeScript copy would drift from `DATE_PRECISION_LOOKUP_V` the first time the
/// vocabulary changed.
pub async fn list_date_precisions() -> Json<Vec<DatePrecisionOption>> {
    Json(
        ALL_DATE_PRECISIONS
            .iter()
            .map(|p| DatePrecisionOption {
                value: p.as_str().to_string(),
                label: p.label().to_string(),
                requires_date: p.expects_a_date(),
            })
            .collect(),
    )
}

/// `GET /documents/:id/date` — what is recorded now.
///
/// Three distinguishable answers, which is the whole reason this is its own
/// endpoint rather than a field folded into a wider document read:
///
/// - `404` — no such document;
/// - `200` with no `date_precision` — the document exists and NOBODY HAS BEEN
///   ASKED yet. This is the state all nine ingested documents are in, and it is
///   what the edit control shows as an unanswered question;
/// - `200` with a precision — someone answered, and the answer may be "unknown".
pub async fn get_document_date(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
) -> Result<Json<StoredDocumentDate>, AppError> {
    let record = pipeline_repository::get_document_date(&state.pipeline_pool, &document_id)
        .await
        .map_err(|source| {
            tracing::error!(
                document_id = %document_id,
                error = %source,
                "could not read the document date"
            );
            AppError::Internal {
                message: format!("Failed to read the date for document '{document_id}': {source}"),
            }
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{document_id}' not found"),
        })?;

    // A precision the database holds but this build does not define is a real
    // disagreement, not something to render as "unknown" — so the label says so
    // and the operator can see which value is unrecognised.
    let label =
        record
            .date_precision
            .as_deref()
            .map(|token| match DatePrecision::from_token(token) {
                Some(p) => p.label().to_string(),
                None => format!("Unrecognised precision '{token}'"),
            });

    Ok(Json(StoredDocumentDate {
        document_id,
        document_date: record.document_date.map(|d| d.to_string()),
        date_precision: record.date_precision,
        date_precision_label: label,
    }))
}

/// What `GET /documents/:id/date` returns.
///
/// Every field but the id is optional, because "nobody has been asked yet" is a
/// state the caller must be able to see — collapsing it into `unknown` would
/// make the intake question look already answered on all nine ingested
/// documents.
#[derive(Debug, Clone, Serialize)]
pub struct StoredDocumentDate {
    pub document_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_precision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_precision_label: Option<String>,
}

/// `PUT /documents/:id/date` — record or correct a document's own date.
pub async fn set_document_date(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
    Json(body): Json<SetDocumentDateRequest>,
) -> Result<Json<DocumentDateResponse>, AppError> {
    let (date, precision) = validate(body.document_date.as_deref(), &body.date_precision)
        .map_err(|e| to_app_error(&document_id, e))?;

    let rows = pipeline_repository::set_document_date(
        &state.pipeline_pool,
        &document_id,
        date.as_deref(),
        precision.as_str(),
    )
    .await
    .map_err(|source| {
        tracing::error!(
            document_id = %document_id,
            date = ?date,
            precision = precision.as_str(),
            error = %source,
            "could not store the document date"
        );
        AppError::Internal {
            message: format!("Failed to store the date for document '{document_id}': {source}"),
        }
    })?;

    // An UPDATE that matched nothing is not an error to Postgres and would not
    // be one to `?` either — it simply changed zero rows. Discarding the count
    // here would mean a date typed against a document id that does not exist
    // returned 200 with the values echoed back, logged "recorded", and stored
    // nothing. Standing Rule 1: "no such document" and "stored" are two
    // operationally distinct states, so they get two observables.
    if rows == 0 {
        tracing::warn!(
            document_id = %document_id,
            "refused a document date: no such document. Nothing was stored"
        );
        return Err(AppError::NotFound {
            message: format!("Document '{document_id}' not found — the date was not stored"),
        });
    }

    // Only after a confirmed write. Mirroring a date onto a graph node whose
    // Postgres row does not exist would put the two stores into a disagreement
    // that nothing else would ever notice.
    mirror_to_graph(&state, &document_id, date.as_deref(), precision).await;

    tracing::info!(
        document_id = %document_id,
        date = ?date,
        precision = precision.as_str(),
        "document date recorded"
    );
    Ok(Json(DocumentDateResponse {
        document_id,
        document_date: date,
        date_precision: precision.as_str().to_string(),
        date_precision_label: precision.label().to_string(),
    }))
}

/// Mirror the date onto the `Document` node, if there is one yet.
///
/// ## Why a failure here is a WARN and not a 400
///
/// Postgres is the record of a document's date; the graph property is a
/// convenience for graph queries. Refusing the whole write because the mirror
/// failed would lose the date the operator just typed in order to protect a
/// derived copy — the wrong trade. So the mirror failure is loud in the log with
/// the document and the cause, the write stands, and re-saving the date re-runs
/// the mirror.
///
/// This is NOT the best-effort carve-out for cosmetic preferences: the failure IS
/// surfaced, at WARN with full context, and the operation it belongs to has
/// already succeeded in the store that owns the value.
async fn mirror_to_graph(
    state: &AppState,
    document_id: &str,
    date: Option<&str>,
    precision: DatePrecision,
) {
    let cypher = "MATCH (d:Document) WHERE d.id = $id OR d.source_document_id = $id \
         SET d.document_date = $date, d.date_precision = $precision";
    let result = state
        .graph
        .run(
            neo4rs::query(cypher)
                .param("id", document_id)
                .param("date", date.unwrap_or_default())
                .param("precision", precision.as_str()),
        )
        .await;

    if let Err(source) = result {
        tracing::warn!(
            document_id = %document_id,
            error = %source,
            "the document date was stored in Postgres but the Neo4j Document node \
             could not be updated. Postgres is the record; re-save the date to \
             retry the mirror"
        );
    }
}

/// Turn a validation failure into the 400 the caller can act on.
fn to_app_error(document_id: &str, error: DatePrecisionError) -> AppError {
    tracing::warn!(
        document_id = %document_id,
        error = %error,
        "refused a document date"
    );
    // `details` carries the accepted vocabulary, so a caller that got the
    // precision wrong is handed the valid set rather than having to read the
    // prose message and guess at spelling.
    AppError::BadRequest {
        message: error.to_string(),
        details: serde_json::json!({
            "accepted_precisions": ALL_DATE_PRECISIONS
                .iter()
                .map(|p| p.as_str())
                .collect::<Vec<_>>(),
        }),
    }
}

#[cfg(test)]
#[path = "document_date_tests.rs"]
mod tests;
