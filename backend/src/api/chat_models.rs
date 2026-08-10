//! GET /api/chat/models — catalog of chat-selectable LLMs.
//!
//! Backs the frontend model dropdown (Part 3/3). Reads every active row
//! from `llm_models`, marks the server's configured default, and returns
//! the list verbatim — the handler does NOT filter against
//! `chat_providers`, because the catalog is a DB-level truth and the map
//! only exists when `ANTHROPIC_API_KEY` is configured.

use axum::{extract::State, Json};
use serde::Serialize;

use crate::api::embed::ErrorResponse;
use crate::auth::{require_ai, require_edit, AuthUser};
use crate::domain::billing_class::BillingClass;
use crate::repositories::pipeline_repository::models;
use crate::state::AppState;

/// A single entry in the chat-models response.
#[derive(Debug, Serialize)]
pub struct ChatModelEntry {
    pub model_id: String,
    pub display_name: String,
    /// True when this row's id equals `AppState::default_chat_model`.
    pub is_default: bool,
    /// `local` | `billed` — who pays for a call (task 1.7B, migration
    /// `20260802134438`). Carried as the raw token so a client can branch on the
    /// STATE rather than parse the label, the same state+label discipline the
    /// scenario card payload uses throughout.
    pub billing_class: String,
    /// The name as the picker should show it, with the billing warning already
    /// attached — "claude-opus-4-8 (API — billed)".
    ///
    /// ## Why the label is composed HERE
    ///
    /// The alternative is a browser that maps `billing_class` → English, which
    /// puts a statement about what this deployment COSTS in the one place that
    /// cannot be tested without a browser, and duplicates it per client. The
    /// vocabulary belongs to `domain::billing_class`; this field is what it says.
    pub display_label: String,
}

/// Compose one catalog entry, with its billing label attached.
///
/// A model whose stored `billing_class` this build cannot read is REFUSED —
/// serving it unlabelled would be the one failure this whole mechanism exists to
/// prevent, a metered model presented as though a scan across it were free.
///
/// # Errors
/// Returns the sentence to show the human, naming the model and the token that
/// could not be read. The caller drops the model, logs it, and carries the
/// sentence back on `ChatModelsResponse::warnings` — one misclassified row must
/// not fail the whole catalog, and must not vanish quietly either.
fn entry_for(model: models::LlmModelRecord, default_model: &str) -> Result<ChatModelEntry, String> {
    let class = BillingClass::try_from(model.billing_class.as_str()).map_err(|e| {
        tracing::error!(
            model = %model.id,
            stored = %model.billing_class,
            error = %e,
            "a model is not offered because its billing class cannot be read"
        );
        format!("{} is not listed: {e}", model.id)
    })?;

    Ok(ChatModelEntry {
        is_default: model.id == default_model,
        display_label: match class.suffix() {
            Some(suffix) => format!("{} {suffix}", model.display_name),
            None => model.display_name.clone(),
        },
        model_id: model.id,
        display_name: model.display_name,
        billing_class: class.code().to_string(),
    })
}

/// Split a catalog read into the entries that can be offered and the sentences
/// explaining the rows that cannot.
fn classify(
    rows: Vec<models::LlmModelRecord>,
    default_model: &str,
) -> (Vec<ChatModelEntry>, Vec<String>) {
    let mut entries = Vec::with_capacity(rows.len());
    let mut warnings = Vec::new();
    for row in rows {
        match entry_for(row, default_model) {
            Ok(entry) => entries.push(entry),
            Err(warning) => warnings.push(warning),
        }
    }
    (entries, warnings)
}

/// Response body for `GET /api/chat/models`.
#[derive(Debug, Serialize)]
pub struct ChatModelsResponse {
    pub models: Vec<ChatModelEntry>,
    pub default_model: String,
    /// Rows this build could not offer, one operator-readable sentence each.
    ///
    /// ## Why a shorter list is not allowed to be silent
    ///
    /// A model whose stored `billing_class` cannot be read is DROPPED from the
    /// catalog (see `entry_for`) — the alternative, showing it unlabelled, could
    /// present a metered model as though scanning across it were free. But a
    /// picker that is simply one row shorter than the database looks exactly like
    /// a picker that is complete. The only person who can repair the row is the
    /// one reading this screen, and a server log they are not tailing does not
    /// reach them. Empty on every healthy deployment.
    pub warnings: Vec<String>,
}

type ApiError = (axum::http::StatusCode, Json<ErrorResponse>);

/// `GET /api/chat/models` handler. Requires AI-role auth so it matches
/// `/ask`'s access rules — the catalog exposes which models the user can
/// actually select for synthesis.
pub async fn list_chat_models(
    user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ChatModelsResponse>, ApiError> {
    require_ai(&user).map_err(|e| {
        (
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse { error: e.message }),
        )
    })?;

    let rows = models::list_active_models(&state.pipeline_pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list active llm_models");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("DB error: {e}"),
                }),
            )
        })?;

    // Chat's ordering and default are UNCHANGED by task 1.7B: the billing label
    // rides along because the two catalogs share one entry type, but local-first
    // ordering is the SCAN control's rule (a scan is 148 metered calls; one chat
    // turn is one), and quietly reordering the chat picker would be this task
    // editing a surface it was not asked about.
    let default_model = state.default_chat_model.clone();
    let (models, warnings) = classify(rows, &default_model);

    Ok(Json(ChatModelsResponse {
        models,
        default_model,
        warnings,
    }))
}

/// `GET /api/scan/models` handler — the model catalog for the SCAN/benchmark
/// picker. Identical shape to [`list_chat_models`] but sourced from
/// `list_scan_eligible_models` (`is_active = true AND scan_eligible = true`), so
/// retired-but-extraction-active models stay out of the scan picker without being
/// deactivated (ruling A). Chat's `/api/chat/models` is deliberately untouched.
///
/// ## Why this is EDIT-gated while `/api/chat/models` stays AI-gated (task 2.13c)
///
/// It used to require the AI role, by analogy with the chat catalogue. That put
/// the gate in the wrong place: **the expensive action was the more permissive
/// one.** Running a theme scan spends real LLM calls and is `require_edit`
/// (`api::scenario_theme_scan`), so a legal editor could START a billed scan but
/// could not LIST the models to choose which one to spend money on — the picker
/// read "Could not load models" for exactly the people entitled to use it.
///
/// So this read now matches the write it serves. It widens nobody's spending
/// power, because that was already `require_edit`.
///
/// The payload carries `model_id`, `display_name`, `is_default`, `billing_class`,
/// the composed label and operator warnings — no credentials, no keys, nothing
/// admin-only. `billing_class` and its "(API — billed)" label make cost MORE
/// visible to the person about to incur it, which is the opposite of a leak.
///
/// `/api/chat/models` and `/api/ask` deliberately stay on `require_ai`: those are
/// AI CONSUMPTION surfaces, and the AI role is exactly the right fence for them.
///
/// `default_model` / `is_default` mark the scan default: `THEME_SCAN_MODEL` when
/// configured, else the chat default (which, being scan-ineligible, simply yields
/// `is_default = false` for every listed model — the frontend then selects the
/// first).
pub async fn list_scan_models(
    user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ChatModelsResponse>, ApiError> {
    require_edit(&user).map_err(|e| {
        (
            axum::http::StatusCode::FORBIDDEN,
            Json(ErrorResponse { error: e.message }),
        )
    })?;

    let rows = models::list_scan_eligible_models(&state.pipeline_pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list scan-eligible llm_models");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("DB error: {e}"),
                }),
            )
        })?;

    let configured_default = scan_default_model(
        state.config.theme_scan_model.as_deref(),
        &state.settings.current().theme_scan_default_model,
        &state.default_chat_model,
    );
    tracing::info!(
        default_source = scan_default_source(
            state.config.theme_scan_model.as_deref(),
            &state.settings.current().theme_scan_default_model,
        ),
        default_model = %configured_default,
        "scan model catalogue: default resolved"
    );

    let (entries, warnings) = classify(rows, &configured_default);
    let (models, default_model) = local_first(entries, configured_default);

    Ok(Json(ChatModelsResponse {
        models,
        default_model,
        warnings,
    }))
}

/// The scan picker's default, in the order a deployment can actually control it.
///
///   1. `THEME_SCAN_MODEL` — the env var, unchanged and still winning wherever it
///      is set, so no deploy or Ansible edit rides with the settings row below.
///   2. `theme_scan_default_model` — a row Roman edits with no build. This is the
///      step that did not exist before .391: beneath the env var the fallback was
///      the CHAT default, which is scan-ineligible by design, so in practice the
///      picker fell through to `catalog.models[0]` in the browser — a default
///      decided by however the registry happened to sort.
///   3. the chat default, kept as the last resort so this function's contract is
///      unchanged for a deployment that has neither of the above.
///
/// Pure, and split out of the handler for the same reason `classify` and
/// `local_first` are: it is the rule, and a rule nobody has exercised is a rule
/// nobody can trust. A slip in the emptiness guard here would silently restore the
/// list-order default with every other test still green.
fn scan_default_model(env_var: Option<&str>, from_settings: &str, chat_default: &str) -> String {
    if let Some(configured) = env_var {
        return configured.to_string();
    }
    if from_settings.trim().is_empty() {
        return chat_default.to_string();
    }
    from_settings.to_string()
}

/// Which of the three steps answered, for the log line.
///
/// Cheap, and it is the difference between "the picker opened on the wrong model"
/// being a five-minute question and an afternoon of guessing which layer is in
/// force on this deployment.
fn scan_default_source(env_var: Option<&str>, from_settings: &str) -> &'static str {
    if env_var.is_some() {
        "THEME_SCAN_MODEL env var"
    } else if from_settings.trim().is_empty() {
        "chat default (last resort)"
    } else {
        "theme_scan_default_model settings row"
    }
}

/// Order the scan catalog local-first and choose the model the page opens on.
///
/// Pure, and split out of the handler so both rules are testable without a
/// database — they are the whole point of task 1.7B's scan control, and a rule
/// nobody has exercised is a rule nobody can trust.
///
/// **Ordering:** local before billed. A scan is one metered call per candidate —
/// 148 of them on S-1 — so the picker must not put a billed model under the
/// cursor. Ordered by the stored class, never by name: which models cost money is
/// a deployment fact, and a name list in code would be wrong the first time a
/// self-hosted model arrives with an unfamiliar id (Rule 13).
///
/// **Default:** the configured default if it is local, else the first local
/// model, else the configured one unchanged. Free-by-default is the point — a
/// human who wants to spend money should have to choose to. The last case is the
/// honest floor: with no local model available there is nothing free to fall back
/// to, and silently selecting nothing would leave the Run button dead with no
/// explanation.
fn local_first(
    mut entries: Vec<ChatModelEntry>,
    configured_default: String,
) -> (Vec<ChatModelEntry>, String) {
    // `sort_by_key` is STABLE, so within a class the repository's own ordering
    // (display_name) survives — the result is "local first, then as listed",
    // not an arbitrary shuffle.
    entries.sort_by_key(|e| {
        BillingClass::try_from(e.billing_class.as_str())
            .map(BillingClass::sort_key)
            // Unreachable: `entry_for` already dropped anything unparseable.
            // Sorted last rather than panicking — a picker that refuses to render
            // is worse than one whose order is imperfect.
            .unwrap_or(u8::MAX)
    });

    let is_local = |e: &&ChatModelEntry| e.billing_class == BillingClass::Local.code();
    let default_model = entries
        .iter()
        .find(|e| e.is_default && is_local(e))
        .or_else(|| entries.iter().find(is_local))
        .map(|e| e.model_id.clone())
        .unwrap_or(configured_default);

    // `is_default` must agree with the answer above, or the picker highlights one
    // row and selects another.
    for entry in &mut entries {
        entry.is_default = entry.model_id == default_model;
    }
    (entries, default_model)
}

#[cfg(test)]
#[path = "chat_models_tests.rs"]
mod tests;
