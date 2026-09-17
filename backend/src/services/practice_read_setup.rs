//! Getting ready to ask: the prompt, the model, and the ceilings.
//!
//! Split from [`super::practice_read`] in T1 under Rule 17 — and split HERE
//! rather than anywhere else because this is where the next task lands. T2 owns
//! `temperature` reaching the wire (the Chunk-B seam), the 90-versus-600 timeout
//! reconciliation, and a prompt-caching breakpoint at the request-build seam. All
//! three are changes to how a call is SET UP, none of them changes what a reply
//! MEANS, and putting the seam here means T2 edits one small module instead of
//! growing the one that decides what happened.
//!
//! Everything here comes from ONE settings snapshot, so a single read is judged by
//! a single consistent configuration even if the store is edited mid-call.

use std::sync::Arc;

use colossus_extract::LlmProvider;

use crate::domain::llm_params::{LlmParamsSpec, ParamValue, ResolvedLlmParams};
use crate::services::practice_model_call::resolve_model;
use crate::services::practice_read_parse::ReadRules;
use crate::state::AppState;

/// The parameter spec for a read: deterministic, capped by the settings row.
///
/// Temperature 0 because the same answer should earn the same read. Domain note,
/// carried from the audit so nobody reads this as a guarantee: this temperature
/// does NOT reach the wire — only `max_tokens` crosses the Chunk-B seam, and the
/// model row's `temperature_mode` is `omit`. Fixing that is T2's, explicitly.
fn read_task_spec(max_tokens: u32) -> LlmParamsSpec {
    LlmParamsSpec {
        temperature: ParamValue::Set(0.0),
        timeout_secs: ParamValue::Unset,
        max_tokens: ParamValue::Set(max_tokens),
    }
}

/// Resolve the configured model into a provider and its parameters — through the
/// one plumbing every practice call shares (`practice_model_call`).
async fn resolve_provider(
    state: &AppState,
    model_id: &str,
    max_tokens: u32,
) -> Result<(Arc<dyn LlmProvider>, ResolvedLlmParams), String> {
    let resolved = resolve_model(
        state,
        model_id,
        &read_task_spec(max_tokens),
        "practice_read_model",
    )
    .await?;
    Ok((resolved.provider, resolved.params))
}

/// Read the system prompt off disk.
///
/// Read PER CALL rather than cached at boot: the row naming the file can change
/// on the Settings page, and a cached prompt would keep judging by the old one
/// until somebody restarted the service. Boot has already proved the file
/// resolves, so this failing means it was removed while the service ran.
fn read_prompt(state: &AppState) -> Result<String, String> {
    let settings = state.settings.current();
    let path = std::path::Path::new(state.registry.template_dir())
        .join(settings.practice_read.prompt_file.trim());
    std::fs::read_to_string(&path).map_err(|e| {
        format!(
            "the read prompt at {} is unreadable: {e} — deploy the file to the template \
                 directory, or point practice_read_prompt_file at one that is there",
            path.display()
        )
    })
}

/// Everything one call needs, once the store and the registry have been read.
pub(crate) struct ReadSetup {
    pub(crate) system: String,
    pub(crate) provider: Arc<dyn LlmProvider>,
    pub(crate) params: ResolvedLlmParams,
    pub(crate) rules_max_words_call: usize,
    pub(crate) rules_max_words_why: usize,
    pub(crate) rules_max_words_pointer: usize,
    pub(crate) rules_max_pointers: usize,
    pub(crate) rules_max_words_after_fine: usize,
    pub(crate) fine_token: String,
    /// The prompt file that produced this read — stamped on the row.
    pub(crate) version: String,
}

impl ReadSetup {
    /// The ceilings, borrowed from this setup's own snapshot.
    pub(crate) fn rules(&self) -> ReadRules<'_> {
        ReadRules {
            max_words_call: self.rules_max_words_call,
            max_words_why: self.rules_max_words_why,
            max_words_pointer: self.rules_max_words_pointer,
            max_pointers: self.rules_max_pointers,
            max_words_after_fine: self.rules_max_words_after_fine,
            fine_token: &self.fine_token,
        }
    }
}

/// Read the prompt, resolve the model, and settle the ceilings — or say why not.
///
/// Every value comes from ONE settings snapshot, so a single read is judged by a
/// single consistent configuration even if the store is edited mid-call.
pub(crate) async fn prepare(state: &AppState, model_id: &str) -> Result<ReadSetup, String> {
    let settings = state.settings.current();
    let system = read_prompt(state)?;
    let (provider, params) =
        resolve_provider(state, model_id, settings.practice_read.max_tokens).await?;

    // `usize::try_from` on a u32 cannot fail on any platform this ships to; the
    // saturating fallback is there so a hypothetical 16-bit target degrades to
    // "no ceiling" rather than panicking on a witness's answer.
    let read = &settings.practice_read;
    Ok(ReadSetup {
        system,
        provider,
        params,
        rules_max_words_call: usize::try_from(read.max_words_call).unwrap_or(usize::MAX),
        rules_max_words_why: usize::try_from(read.max_words_why).unwrap_or(usize::MAX),
        rules_max_words_pointer: usize::try_from(read.max_words_pointer).unwrap_or(usize::MAX),
        rules_max_pointers: usize::try_from(read.max_pointers).unwrap_or(usize::MAX),
        rules_max_words_after_fine: usize::try_from(read.max_words_after_fine)
            .unwrap_or(usize::MAX),
        fine_token: read.fine_token.clone(),
        version: read.prompt_file.trim().to_string(),
    })
}
