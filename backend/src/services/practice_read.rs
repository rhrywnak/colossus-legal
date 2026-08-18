//! The one-sentence read: resolving the model, making the call, logging it.
//!
//! The impure half of the read. Its rules — what is sent, what is accepted back —
//! live in [`super::practice_read_parse`], where they are unit-tested without a
//! provider.
//!
//! ## The contract this module keeps with the screen
//!
//! There is exactly one way for a read to reach Marie: a call that returned, on
//! one line, inside its word cap. Every other outcome — an unknown model, a
//! missing prompt file, a timeout, a rate limit, a paragraph — produces the SAME
//! observable for her ("no system read this time") and a DIFFERENT one in the log
//! (`read_error` names which). That split is Standing Rule 1 exactly: the failure
//! is observable, and the reader of the logs can tell what failed and why,
//! without a witness being shown a stack trace mid-session.
//!
//! ## Why a failed read never fails the request
//!
//! Her answer is already worth recording, and the four boxes, the points, the
//! pair and the watch-for do not depend on a model. Returning a 502 here would
//! discard her typed answer because a vendor was slow.

use std::sync::Arc;
use std::time::Instant;

use colossus_extract::{LlmProvider, LlmResponse};

use crate::domain::llm_params::{
    constrain, resolve, LlmParamsSpec, ModelConstraints, ParamValue, ResolvedLlmParams,
};
use crate::llm_retry::call_with_rate_limit_retry_params;
use crate::pipeline::providers::provider_for_model;
use crate::repositories::pipeline_repository::models::get_active_model_by_id;
use crate::services::practice_read_parse::{
    build_user_message, parse_read, ReadInputs, ReadRejection, ReadRules,
};
use crate::state::AppState;

/// What one read attempt produced, in the shape the answer row stores.
///
/// ## Rust Learning: one struct instead of `Result`
///
/// A `Result` would push the caller into `match`ing two shapes for something the
/// database stores as one row either way. Every field here maps to a column, and
/// the invariant the type carries is: `text` is `Some` exactly when `error` is
/// `None`. Making that a `Result` would then need the token counts duplicated in
/// both arms, since a call that SUCCEEDED and was then refused for length still
/// cost money and still must be logged.
#[derive(Debug, Clone, Default)]
pub struct ReadOutcome {
    /// The one sentence, or `None` for every failure.
    pub text: Option<String>,
    /// `Some(true)` = "Fine.", `Some(false)` = it named a tactic, `None` = no read.
    pub ok: Option<bool>,
    /// Why there is no read. `None` when there is one.
    pub error: Option<String>,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    /// Wall-clock milliseconds for the call, whether it succeeded or not.
    pub ms: Option<i32>,
    /// The model that was asked. Recorded even on failure — "which model was
    /// down" is the first question of any morning after.
    pub model: Option<String>,
    /// What the model said, when this build REFUSED to show it.
    ///
    /// `None` on an accepted read (`text` is the model's own line then) and on a
    /// call that never returned. Kept because a wave of refusals is a prompt
    /// problem or a model change, and diagnosing one means reading what the model
    /// actually wrote — after the log window has rolled over.
    pub raw_reply: Option<String>,
}

impl ReadOutcome {
    /// A read that never happened, with the reason.
    fn failed(model: Option<String>, ms: Option<i32>, reason: String) -> Self {
        ReadOutcome {
            error: Some(reason),
            model,
            ms,
            ..Default::default()
        }
    }
}

/// The parameter spec for a read: deterministic, capped by the settings row.
///
/// Temperature 0 because the same answer must earn the same sentence — a coach
/// who says something different each rep is not a coach. `timeout_secs` defers to
/// the model's own default rather than inventing one here.
fn read_task_spec(max_tokens: u32) -> LlmParamsSpec {
    LlmParamsSpec {
        temperature: ParamValue::Set(0.0),
        timeout_secs: ParamValue::Unset,
        max_tokens: ParamValue::Set(max_tokens),
    }
}

/// Resolve the configured model into a provider and its parameters.
///
/// Returns the reason as a `String` rather than a typed error: every caller does
/// the same thing with it — stores it in `read_error` and shows the fixed line —
/// so a taxonomy here would be a type nobody branches on.
async fn resolve_provider(
    state: &AppState,
    model_id: &str,
    max_tokens: u32,
) -> Result<(Arc<dyn LlmProvider>, ResolvedLlmParams), String> {
    let record = get_active_model_by_id(&state.pipeline_pool, model_id)
        .await
        .map_err(|e| format!("model lookup failed for {model_id}: {e}"))?
        .ok_or_else(|| {
            format!("practice_read_model names {model_id}, which is not an active llm_models row")
        })?;

    let constraints = ModelConstraints::from_record(&record)
        .map_err(|e| format!("model {model_id} has unusable parameter columns: {e}"))?;
    // Model-default layer stays silent: the read's temperature and cap are the
    // TASK's, and `constrain` refuses (never clamps) a cap above the model's
    // ceiling — which is why the seeded 1024 sits under every active row's
    // max_output_tokens, including the two 2048-token vLLM ones.
    let params = resolve(
        &LlmParamsSpec::SILENT,
        &read_task_spec(max_tokens),
        &LlmParamsSpec::SILENT,
    )
    .and_then(|r| constrain(r, &constraints))
    .map_err(|e| format!("model {model_id} refused the read's parameters: {e}"))?;

    let provider: Arc<dyn LlmProvider> = Arc::from(
        provider_for_model(&state.extraction_engine, &record)
            .map_err(|detail| format!("could not build a provider for {model_id}: {detail}"))?,
    );
    Ok((provider, params))
}

/// Read the system prompt off disk.
///
/// Read PER CALL rather than cached at boot, for the freshness reason the theme
/// scan's pointer already follows: the row naming the file can change on the
/// Settings page, and a cached prompt would keep judging by the old one until
/// somebody restarted the service. Boot has already proved the file resolves, so
/// this failing means it was removed while the service ran — which the log says.
fn read_prompt(state: &AppState) -> Result<String, String> {
    let settings = state.settings.current();
    let path = std::path::Path::new(state.registry.template_dir())
        .join(settings.practice_read.prompt_file.trim());
    std::fs::read_to_string(&path)
        .map_err(|e| format!("the read prompt at {} is unreadable: {e}", path.display()))
}

/// Judge one typed answer, and never propagate a failure.
///
/// # Panics
/// None. Every path returns a [`ReadOutcome`]; the failure arms carry the reason.
/// Everything one call needs, once the store and the registry have been read.
///
/// Exists so [`read_answer`] can be a straight line: prepare, call, judge. The
/// two ways preparation fails — no prompt on disk, no usable model — are handled
/// where they happen, and neither reaches the call.
struct ReadSetup {
    system: String,
    provider: Arc<dyn LlmProvider>,
    params: ResolvedLlmParams,
    rules_max_words: usize,
    rules_max_words_after_fine: usize,
    fine_token: String,
}

/// Read the prompt, resolve the model, and settle the rules — or say why not.
///
/// Every value comes from ONE settings snapshot, so a single read is judged by a
/// single consistent configuration even if the store is edited mid-call.
async fn prepare(state: &AppState, model_id: &str) -> Result<ReadSetup, String> {
    let settings = state.settings.current();
    let system = read_prompt(state)?;
    let (provider, params) =
        resolve_provider(state, model_id, settings.practice_read.max_tokens).await?;

    Ok(ReadSetup {
        system,
        provider,
        params,
        rules_max_words: usize::try_from(settings.practice_read.max_words).unwrap_or(usize::MAX),
        rules_max_words_after_fine: usize::try_from(settings.practice_read.max_words_after_fine)
            .unwrap_or(usize::MAX),
        fine_token: settings.practice_read.fine_token.clone(),
    })
}

/// Judge one typed answer, and never propagate a failure.
///
/// # Panics
/// None. Every path returns a [`ReadOutcome`]; the failure arms carry the reason.
pub async fn read_answer(state: &AppState, inputs: &ReadInputs<'_>) -> ReadOutcome {
    let model_id = state.settings.current().practice_read.model.clone();

    let setup = match prepare(state, &model_id).await {
        Ok(setup) => setup,
        Err(reason) => {
            tracing::error!(model = %model_id, reason = %reason, "practice read: not attempted");
            return ReadOutcome::failed(Some(model_id), None, reason);
        }
    };

    let user = build_user_message(inputs);
    let started = Instant::now();
    let result = call_with_rate_limit_retry_params(
        setup.provider.as_ref(),
        Some(&setup.system),
        &user,
        &setup.params,
        0,
        1,
    )
    .await;
    let ms = i32::try_from(started.elapsed().as_millis()).unwrap_or(i32::MAX);

    let response = match result {
        Ok(response) => response,
        Err(e) => {
            let reason = format!("the call failed: {e}");
            tracing::warn!(model = %model_id, ms, reason = %reason, "practice read: call failed");
            return ReadOutcome::failed(Some(model_id), Some(ms), reason);
        }
    };

    judge(&setup, response, model_id, ms)
}

/// Turn one returned reply into the row that will be stored.
///
/// Split from [`read_answer`] because it is the part with a DECISION in it — the
/// three-way outcome the answer row records — and because keeping it separate
/// left that function short enough to read as the four steps it is.
fn judge(setup: &ReadSetup, response: LlmResponse, model_id: String, ms: i32) -> ReadOutcome {
    // best-effort: the columns are INTEGER and a token count above 2^31 is not a
    // number any model in the registry can produce — the largest ceiling here is
    // 128,000. A value that somehow did not fit is recorded as "not reported"
    // rather than failing an answer over a metric, and the same `ms`/`model`
    // fields still say the call happened. The read itself is unaffected.
    let input_tokens = response.input_tokens.and_then(|n| i32::try_from(n).ok());
    let output_tokens = response.output_tokens.and_then(|n| i32::try_from(n).ok());

    let rules = ReadRules {
        max_words: setup.rules_max_words,
        max_words_after_fine: setup.rules_max_words_after_fine,
        fine_token: &setup.fine_token,
    };

    match parse_read(&response.text, rules) {
        Ok(line) => {
            tracing::info!(
                model = %model_id, ms, input_tokens, output_tokens, ok = line.ok,
                "practice read"
            );
            ReadOutcome {
                text: Some(line.text),
                ok: Some(line.ok),
                error: None,
                input_tokens,
                output_tokens,
                ms: Some(ms),
                model: Some(model_id),
                // Nothing to keep: `text` IS the model's own line here.
                raw_reply: None,
            }
        }
        Err(rejection) => reject(
            rejection,
            response.text,
            model_id,
            ms,
            input_tokens,
            output_tokens,
        ),
    }
}

/// A reply that arrived and was REFUSED.
///
/// The call SUCCEEDED and cost money; the reply was unusable. Three things follow
/// from that, and they are the reason this arm is worth its own function:
///
/// - the tokens are still recorded on the row — a refused read that looked free
///   would hide a model quietly spending on paragraphs nobody sees;
/// - the reply ITSELF is kept, because diagnosing a wave of refusals means
///   reading what the model actually wrote, after the log window has rolled over;
/// - the log line says "refused", never "failed" — an operator seeing a hundred
///   of these has a prompt problem, not a network one.
#[allow(clippy::too_many_arguments)]
fn reject(
    rejection: ReadRejection,
    reply: String,
    model_id: String,
    ms: i32,
    input_tokens: Option<i32>,
    output_tokens: Option<i32>,
) -> ReadOutcome {
    let reason = format!("{rejection}");
    tracing::warn!(
        model = %model_id, ms, input_tokens, output_tokens, reason = %reason,
        reply = %reply.chars().take(300).collect::<String>(),
        "practice read: reply refused"
    );
    ReadOutcome {
        error: Some(reason),
        input_tokens,
        output_tokens,
        ms: Some(ms),
        model: Some(model_id),
        raw_reply: Some(reply),
        ..Default::default()
    }
}
