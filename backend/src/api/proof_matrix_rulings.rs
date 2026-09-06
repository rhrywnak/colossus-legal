//! `PUT` / `DELETE /api/cases/:slug/proof-matrix/rulings` — one human's verdict
//! on one machine-ranked item (PROOF_MATRIX_v2 §2).
//!
//! Two handlers over
//! [`crate::repositories::pipeline_repository::evidence_allegation_rulings`].
//! Both write the state row and its ledger entry in one transaction; neither
//! touches Neo4j — the machine's own claim on the edge is left exactly as the
//! linking pass wrote it, and this is a layer OVER it.
//!
//! ## Why the pair travels in the BODY, on both verbs
//!
//! An Evidence id is a content hash of the shape
//! `doc-george-phillips-admissions-response:evidence:01d3e125` — colons and all.
//! Putting two of those in a path would make every call depend on the client and
//! the router agreeing about percent-encoding, on a control a reader clicks
//! dozens of times a sitting. A body has no such hazard, and the two verbs then
//! read as what they are: the same address, one setting a verdict and one taking
//! it back.
//!
//! ## Why `:slug` is unused
//!
//! The Neo4j graph is single-case and the ids are globally unique, exactly as on
//! the neighbouring element-detail routes. The segment keeps the public URL
//! shaped like its siblings and leaves room for per-case scoping later.

use axum::extract::{Path, State};
use axum::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use crate::auth::{require_edit, AuthUser};
use crate::domain::matrix_ruling::MatrixRuling;
use crate::error::AppError;
use crate::repositories::pipeline_repository::evidence_allegation_rulings::{
    save_ruling, withdraw_ruling, RulingWrite,
};
use crate::state::AppState;

/// Request body for both verbs.
///
/// `#[serde(deny_unknown_fields)]` rejects a camelCase typo at the boundary
/// rather than writing a row with a defaulted field — the same posture as
/// `UpdateNotesRequest`. `ruling` is a typed enum, so serde refuses anything
/// outside `keep`/`remove` before the handler runs, and the DELETE ignores it
/// (there is no verdict in force after a withdrawal).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RulingRequest {
    pub evidence_id: String,
    pub allegation_id: String,
    /// The verdict. Required on `PUT`; `#[serde(default)]` lets the `DELETE`
    /// body omit it, because a withdrawal asserts nothing.
    #[serde(default)]
    pub ruling: Option<MatrixRuling>,
    /// An optional sentence from the human.
    #[serde(default)]
    pub note: Option<String>,
}

/// What a write did, so the browser can say something true about it.
#[derive(Debug, Serialize)]
pub struct RulingResponse {
    pub evidence_id: String,
    pub allegation_id: String,
    /// `rule` / `rerule` / `withdraw`, or `none` when a withdrawal found nothing.
    pub action: &'static str,
    /// Whether anything actually changed. A withdrawal of an unruled item
    /// reports `false` rather than claiming work it did not do — the caller's
    /// view was stale, and only one of those two outcomes means that.
    pub changed: bool,
}

/// `PUT …/proof-matrix/rulings` — keep or remove one item.
#[instrument(skip(state, user, payload), fields(slug = %slug))]
pub async fn put_ruling(
    user: AuthUser,
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(payload): Json<RulingRequest>,
) -> Result<Json<RulingResponse>, AppError> {
    require_edit(&user)?;
    let (evidence_id, allegation_id) = validated_pair(&payload)?;

    // Required on this verb, and refused by name rather than defaulted: guessing
    // `keep` would confirm an item nobody read, and guessing `remove` would strike
    // one nobody objected to.
    let ruling = payload.ruling.ok_or_else(|| AppError::BadRequest {
        message: "A ruling must say which way it goes: keep or remove.".to_string(),
        details: serde_json::json!({ "field": "ruling" }),
    })?;

    let action = save_ruling(
        &state.pipeline_pool,
        &RulingWrite {
            evidence_id,
            allegation_id,
            ruling,
            ruled_by: &user.username,
            note: payload.note.as_deref(),
            written_at: Utc::now(),
        },
    )
    .await
    .map_err(write_failed(
        Pair {
            evidence_id,
            allegation_id,
            author: &user.username,
        },
        "failed to save a proof-matrix ruling; nothing was stored",
        "failed to save the ruling",
    ))?;

    info!(
        author = %user.username,
        %evidence_id,
        %allegation_id,
        ruling = ruling.code(),
        action = action.code(),
        "ruled a proof-matrix item"
    );

    Ok(Json(RulingResponse {
        evidence_id: evidence_id.to_string(),
        allegation_id: allegation_id.to_string(),
        action: action.code(),
        changed: true,
    }))
}

/// `DELETE …/proof-matrix/rulings` — take a verdict back (the row's Undo).
///
/// Withdrawing a verdict that was not there is not an error: the caller wanted
/// the item unruled and unruled is what it is. It IS reported differently
/// (`changed: false`), because that outcome means the browser's view was stale
/// and the page should say so rather than pretend it undid something.
#[instrument(skip(state, user, payload), fields(slug = %slug))]
pub async fn delete_ruling(
    user: AuthUser,
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(payload): Json<RulingRequest>,
) -> Result<Json<RulingResponse>, AppError> {
    require_edit(&user)?;
    let (evidence_id, allegation_id) = validated_pair(&payload)?;

    let removed = withdraw_ruling(
        &state.pipeline_pool,
        evidence_id,
        allegation_id,
        &user.username,
        Utc::now(),
    )
    .await
    .map_err(write_failed(
        Pair {
            evidence_id,
            allegation_id,
            author: &user.username,
        },
        "failed to withdraw a proof-matrix ruling; nothing was changed",
        "failed to withdraw the ruling",
    ))?;

    info!(
        author = %user.username,
        %evidence_id,
        %allegation_id,
        removed,
        "withdrew a proof-matrix ruling"
    );

    Ok(Json(RulingResponse {
        evidence_id: evidence_id.to_string(),
        allegation_id: allegation_id.to_string(),
        // "none" and "withdraw" are different outcomes and are named
        // differently: one changed the record, one found nothing to change.
        action: if removed { "withdraw" } else { "none" },
        changed: removed,
    }))
}

/// Who was ruling on what, for the failure log.
///
/// A struct rather than three `&str` arguments, for the same reason
/// [`RulingWrite`] is one: they are all `&str`, so a call site could swap two
/// with no compile error and file the failure against the wrong item.
struct Pair<'a> {
    evidence_id: &'a str,
    allegation_id: &'a str,
    author: &'a str,
}

/// Log a failed write with its operator sentence, and return the bland client one.
///
/// ## Why the ids are passed in rather than read off the span
///
/// Both handlers are `#[instrument]`ed, but they `skip(payload)` and bind only
/// `slug` — the pair lives inside the skipped body, so it never reaches the span.
/// A log line naming only the case would leave an operator, reading it after a
/// report of a lost decision, unable to say WHICH item was being ruled or by
/// whom. The success paths already log all three; the failure path is the one
/// that has to.
///
/// ## Why the two sentences are different, and both are here
///
/// The log line says what did NOT happen — "nothing was stored" — because an
/// operator needs to know whether half of it landed. The client line says only
/// that the write failed; the browser renders it through the stored
/// `matrix_ruling_failed_template`, which is the sentence a reader actually sees.
///
/// ## Rust Learning: returning `impl Fn(E) -> AppError + '_` for `.map_err`
///
/// The closure borrows the ids in `pair`, so the returned value cannot outlive
/// them — the `+ '_` says so, tying its lifetime to the argument. Same shape as
/// `proof_matrix::internal`, which does this for the rollup read without a
/// borrow.
fn write_failed<'a>(
    pair: Pair<'a>,
    operator_message: &'static str,
    client_message: &'static str,
) -> impl Fn(crate::repositories::pipeline_repository::PipelineRepoError) -> AppError + 'a {
    move |e| {
        tracing::error!(
            error = %e,
            evidence_id = %pair.evidence_id,
            allegation_id = %pair.allegation_id,
            author = %pair.author,
            "{operator_message}"
        );
        AppError::Internal {
            message: client_message.to_string(),
        }
    }
}

/// The two ids, trimmed and refused if either is blank.
///
/// ## Why blank is refused rather than stored
///
/// Both columns are `NOT NULL`, which does not stop `''`. A row keyed on an empty
/// id can never be matched by a read, so it would be an invisible verdict: the
/// write reports success, the ledger records a human decision, and the page never
/// shows it. Refusing names the field instead.
///
/// ## Rust Learning: returning borrowed `&str` from a `&T` argument
///
/// The slices point into the payload the caller still owns, so nothing is cloned
/// on the happy path. Rust ties the returned lifetime to `payload` automatically,
/// because that is the only reference in the signature.
fn validated_pair(payload: &RulingRequest) -> Result<(&str, &str), AppError> {
    for (field, value) in [
        ("evidence_id", &payload.evidence_id),
        ("allegation_id", &payload.allegation_id),
    ] {
        if value.trim().is_empty() {
            return Err(AppError::BadRequest {
                message: format!("A ruling needs a {field}."),
                details: serde_json::json!({ "field": field }),
            });
        }
    }
    Ok((payload.evidence_id.trim(), payload.allegation_id.trim()))
}

#[cfg(test)]
#[path = "proof_matrix_rulings_tests.rs"]
mod tests;
