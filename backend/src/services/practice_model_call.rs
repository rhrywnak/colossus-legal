//! ONE plumbing for every practice model call: resolve the model, make the call,
//! count what it cost.
//!
//! CC_TASK_QUESTION_CHAT_v1 §2: "Reuse the read engine's call/cost/log path — ONE
//! plumbing, no second engine." Extracted from `practice_read_setup` (model
//! resolution) and `practice_read` (the rate-limit-retrying call and the token
//! arithmetic). The answer read and the "Discuss with AI" dock both call through
//! here, so a change to how a practice call is made — a timeout, a retry policy, a
//! parameter the wire ignores — lands on both at once.
//!
//! ## What stays with each caller
//!
//! Everything that decides what a reply MEANS: the read's JSON parse, citation
//! check and re-request loop stay in `practice_read`; the dock stores the reply as
//! a turn. This module never interprets a reply.

use std::sync::Arc;
use std::time::Instant;

use colossus_extract::{LlmProvider, LlmResponse};

use crate::domain::llm_effort::Effort;
use crate::domain::llm_params::{
    constrain, resolve, LlmParamsSpec, ModelConstraints, ResolvedLlmParams,
};
use crate::llm_retry::call_with_rate_limit_retry_params;
use crate::pipeline::providers::provider_for_model;
use crate::repositories::pipeline_repository::models::{get_active_model_by_id, LlmModelRecord};
use crate::state::AppState;

/// A resolved model, ready to call.
pub(crate) struct ResolvedModel {
    pub(crate) provider: Arc<dyn LlmProvider>,
    pub(crate) params: ResolvedLlmParams,
    /// The catalogue row — its `display_name` is what a screen prints.
    pub(crate) record: LlmModelRecord,
}

/// Resolve `model_id` into a provider and parameters, or say why not.
///
/// `setting_key` names the settings row that chose this model, so the failure
/// sentence tells an operator which row to fix.
///
/// ## Domain note: the CALLER decides how much the model may think
///
/// `effort` is passed in rather than read from a policy here, because the two
/// callers are different jobs: the answer read is a strict-JSON verdict whose
/// thinking competes with its own budget (`practice_read_effort`), and the dock
/// is a conversation (`practice_discuss_effort`). `None` sends no `effort` key at
/// all — the API's own default — which is a real state and not a synonym for
/// `high` (`domain::llm_effort`).
///
/// # Errors
/// An operator's sentence when the model is not an active catalogue row, its
/// parameter columns are unusable, it refuses `spec`, or no provider can be built.
pub(crate) async fn resolve_model(
    state: &AppState,
    model_id: &str,
    spec: &LlmParamsSpec,
    setting_key: &str,
    effort: Option<Effort>,
) -> Result<ResolvedModel, String> {
    let record = get_active_model_by_id(&state.pipeline_pool, model_id)
        .await
        .map_err(|e| format!("model lookup failed for {model_id}: {e}"))?
        .ok_or_else(|| {
            format!(
                "{setting_key} names {model_id}, which is not an active llm_models row \
                 — set is_active on that row, or point {setting_key} at a model that is active"
            )
        })?;

    let constraints = ModelConstraints::from_record(&record)
        .map_err(|e| format!("model {model_id} has unusable parameter columns: {e}"))?;
    let params = resolve(&LlmParamsSpec::SILENT, spec, &LlmParamsSpec::SILENT)
        .and_then(|r| constrain(r, &constraints))
        .map_err(|e| format!("model {model_id} refused the call's parameters: {e}"))?;

    let provider: Arc<dyn LlmProvider> = Arc::from(
        provider_for_model(&state.extraction_engine, &record, effort)
            .map_err(|detail| format!("could not build a provider for {model_id}: {detail}"))?,
    );
    Ok(ResolvedModel {
        provider,
        params,
        record,
    })
}

/// One call, with the deployment's rate-limit retry policy.
///
/// # Errors
/// The provider's own error when the call never returned a reply.
pub(crate) async fn call_model(
    state: &AppState,
    provider: &dyn LlmProvider,
    system: &str,
    user: &str,
    params: &ResolvedLlmParams,
) -> Result<LlmResponse, colossus_extract::PipelineError> {
    // `0, 1`: the retry helper was written for chunked extraction; one practice
    // call is chunk 0 of 1. The policy is the one read at startup (2026-08-28).
    call_with_rate_limit_retry_params(
        provider,
        Some(system),
        user,
        params,
        0,
        1,
        state.config.llm_retry_policy,
    )
    .await
}

/// A reply's token counts as the store's `INTEGER` columns hold them.
///
/// best-effort: a count above 2^31 is not a real reply; it becomes `None`, which
/// the row stores as "not reported" rather than a wrapped negative.
pub(crate) fn response_tokens(response: &LlmResponse) -> (Option<i32>, Option<i32>) {
    (
        // best-effort: a count that does not fit INTEGER is stored as "not reported".
        response.input_tokens.and_then(|n| i32::try_from(n).ok()),
        // best-effort: same — the call still happened, and `ms` and `model` say so.
        response.output_tokens.and_then(|n| i32::try_from(n).ok()),
    )
}

/// Milliseconds since `started`, saturating rather than wrapping.
pub(crate) fn elapsed_ms(started: Instant) -> i32 {
    i32::try_from(started.elapsed().as_millis()).unwrap_or(i32::MAX)
}
