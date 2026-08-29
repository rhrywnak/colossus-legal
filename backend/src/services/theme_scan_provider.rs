//! Per-run Theme Scan provider + parameter resolution (LLM Config Chunk B).
//!
//! The pre-Chunk-B scan used ONE boot-time Anthropic provider
//! (`AppState.theme_scan_provider`). This module replaces that: it resolves a
//! model id (request → `THEME_SCAN_MODEL` → chat default) to an `llm_models`
//! row, resolves+constrains the LLM parameters through Chunk A's resolver, and
//! builds the provider via the unified `provider_for_model` seam. The scan then
//! judges through that per-run provider — which can be Anthropic OR a local vLLM
//! model, the whole point of the benchmark (driver D1).

use std::sync::Arc;

use colossus_extract::LlmProvider;

use crate::domain::llm_params::{
    constrain, resolve, LlmConfigError, LlmParamsSpec, ModelConstraints, ParamValue,
    ResolvedLlmParams,
};
use crate::pipeline::providers::provider_for_model;
use crate::repositories::pipeline_repository::models::{get_active_model_by_id, LlmModelRecord};
use crate::services::theme_scan::ThemeScanError;
use crate::state::AppState;

/// Everything the scan needs about its chosen model: the built provider, the
/// resolved parameters (for the wire AND the `scan_runs` snapshot), the resolved
/// model id, the per-token costs (for `computed_cost`), and — for a vLLM model
/// only — the endpoint the L2 hard gate must poll.
pub(crate) struct ResolvedScanProvider {
    pub provider: Arc<dyn LlmProvider>,
    pub params: ResolvedLlmParams,
    pub model_id: String,
    pub cost_per_input_token: Option<f64>,
    pub cost_per_output_token: Option<f64>,
    /// The per-run fan-out cap (A5): the model row's `max_concurrency`, else the
    /// `THEME_SCAN_CONCURRENCY` env default. Resolved here where the row is in hand.
    pub concurrency: usize,
    /// `Some(endpoint)` iff the resolved model's provider is `"vllm"` — the
    /// signal the caller uses to run the `/v1/models` hard gate. `None` for
    /// Anthropic (which skips the gate).
    pub vllm_endpoint: Option<String>,
}

/// Resolve the per-run fan-out concurrency (A5).
///
/// The model row's `max_concurrency` wins when set to a positive value (e.g. cap
/// 4 for the 14B); a NULL column (or a non-positive stored value) falls back to
/// the `THEME_SCAN_CONCURRENCY` env default. NULL-vs-set is a real distinction
/// (Standing Rule 1): NULL means "this model states no cap", not "cap of 0".
fn effective_concurrency(record_max: Option<i32>, config_default: usize) -> usize {
    match record_max {
        Some(n) if n > 0 => usize::try_from(n).unwrap_or(config_default),
        _ => config_default,
    }
}

/// The TASK layer of the parameter resolver for a Theme Scan.
///
/// Determinism (driver D4): the judge runs at temperature 0. Note the wire
/// temperature actually comes from provider CONSTRUCTION today (the Anthropic
/// bridge pins `Some(0.0)`; vLLM hardcodes `0.0`) — this `Set(0.0)` makes the
/// resolved SNAPSHOT reflect that reality, since the params seam does not yet
/// thread temperature to the wire (the Chunk B ceiling). Timeout defers to the
/// model-default / system layer.
///
/// ## Why `max_tokens` is an ARGUMENT and not a constant read in here
///
/// It was `ParamValue::Set(THEME_SCAN_MAX_TOKENS)` — a compiled-in 512 that
/// truncated 7 of 104 verdicts on 2026-08-09 once the judge became a model that
/// thinks inside the same budget (the full account is in `services::theme_scan`,
/// where the constant used to live).
///
/// Taking it as a parameter is the structural half of that fix: this function
/// cannot reach the settings store, so it cannot read the value itself, and the
/// only way a number gets into the task layer is for a caller to have resolved it
/// from somewhere. A future compiled-in default cannot be reintroduced here
/// without changing this signature — which is exactly what
/// `the_scan_judge_cap_is_read_from_settings_at_scan_start` pins.
fn scan_task_spec(max_tokens: u32) -> LlmParamsSpec {
    LlmParamsSpec {
        temperature: ParamValue::Set(0.0),
        timeout_secs: ParamValue::Unset,
        max_tokens: ParamValue::Set(max_tokens),
    }
}

/// Build the MODEL-DEFAULT layer spec from an `llm_models` row — the lowest of
/// the three resolution layers (model-default < task < user).
///
/// A NULL column means "this layer is silent for this knob" → [`ParamValue::Unset`]
/// (defer to a higher layer or the system default), NOT a value — the distinction
/// Standing Rule 1 requires. `timeout_secs` (`i32`) narrows to `u64` via `try_from`
/// at this boundary; a negative row value is a loud
/// [`LlmConfigError::NegativeTokenValue`], never an `as`-cast (R3). `max_tokens` is
/// ALWAYS `Unset`: there is no per-model *default* max-tokens column — the model's
/// `max_output_tokens` is a CEILING enforced by [`constrain`], a different role.
///
/// Lives here (not in `domain::llm_params`) so the pure resolver module stays
/// under the 300-line limit and does not grow record-shaped helpers; it is used
/// only by this per-run resolution path.
fn model_default_spec(record: &LlmModelRecord) -> Result<LlmParamsSpec, LlmConfigError> {
    let temperature = match record.default_temperature {
        Some(t) => ParamValue::Set(t),
        None => ParamValue::Unset,
    };
    let timeout_secs =
        match record.timeout_secs {
            None => ParamValue::Unset,
            Some(v) => ParamValue::Set(u64::try_from(v).map_err(|_| {
                LlmConfigError::NegativeTokenValue {
                    model_id: record.id.clone(),
                    column: "timeout_secs",
                    value: v,
                }
            })?),
        };
    Ok(LlmParamsSpec {
        temperature,
        timeout_secs,
        max_tokens: ParamValue::Unset,
    })
}

/// Resolve the scan's model id, load its active row, resolve+constrain its
/// parameters, and build the provider.
///
/// Model-id precedence: the per-run request override, else `THEME_SCAN_MODEL`
/// (`config.theme_scan_model`), else the chat default (`state.default_chat_model`
/// — the library-side source of the same value `main.rs` uses).
///
/// Every failure is a typed [`ThemeScanError`] the route maps to an HTTP status
/// (Standing Rule 1): an unknown/inactive model, a corrupt-row parameter fault,
/// or a provider-construction failure are each distinct and named.
pub(crate) async fn resolve_scan_provider(
    state: &AppState,
    requested_model_id: Option<&str>,
) -> Result<ResolvedScanProvider, ThemeScanError> {
    let model_id = requested_model_id
        .map(str::to_string)
        .or_else(|| state.config.theme_scan_model.clone())
        .unwrap_or_else(|| state.default_chat_model.clone());

    let record = get_active_model_by_id(&state.pipeline_pool, &model_id)
        .await
        .map_err(|source| ThemeScanError::ModelLookupFailed {
            model_id: model_id.clone(),
            source,
        })?
        .ok_or_else(|| ThemeScanError::ModelNotAvailable {
            model_id: model_id.clone(),
        })?;

    // Resolve THEN constrain (A3): merge the three layers, then clamp to the
    // model's capabilities. Both stages fold their LlmConfigError into one typed
    // scan error naming the model.
    let constraints =
        ModelConstraints::from_record(&record).map_err(|source| ThemeScanError::ParamsInvalid {
            model_id: model_id.clone(),
            source,
        })?;
    let model_default =
        model_default_spec(&record).map_err(|source| ThemeScanError::ParamsInvalid {
            model_id: model_id.clone(),
            source,
        })?;
    // The judge's token budget, READ PER SCAN rather than cached (the freshness
    // law the prompt pointer already follows): the settings write path swaps the
    // whole snapshot before its response returns, so a cap raised on the Settings
    // page is in force for the NEXT scan with no restart.
    let max_tokens = state.settings.current().theme_scan_max_tokens;
    let resolved = resolve(
        &model_default,
        &scan_task_spec(max_tokens),
        &LlmParamsSpec::SILENT,
    )
    .and_then(|r| constrain(r, &constraints))
    .map_err(|source| ThemeScanError::ParamsInvalid {
        model_id: model_id.clone(),
        source,
    })?;

    // Build via the unified seam. `provider_for_model` returns a `Box`; the scan
    // shares its provider across the concurrent fan-out, so it becomes an `Arc`.
    let provider: Arc<dyn LlmProvider> = Arc::from(
        // The SCAN effort — `None` unless `LLM_SCAN_EFFORT` is set. The scan
        // JUDGES candidates, which is the kind of work thinking helps, so it is
        // deliberately not defaulted down with extraction (2026-08-28 ruling).
        provider_for_model(
            &state.extraction_engine,
            &record,
            state.config.llm_effort_policy.scan,
        )
        .map_err(|detail| ThemeScanError::ProviderBuildFailed {
            model_id: model_id.clone(),
            detail,
        })?,
    );

    let vllm_endpoint = if record.provider == "vllm" {
        record.api_endpoint.clone()
    } else {
        None
    };

    Ok(ResolvedScanProvider {
        cost_per_input_token: provider.cost_per_input_token(),
        cost_per_output_token: provider.cost_per_output_token(),
        concurrency: effective_concurrency(
            record.max_concurrency,
            state.config.theme_scan_concurrency,
        ),
        provider,
        params: resolved,
        model_id,
        vllm_endpoint,
    })
}

#[cfg(test)]
mod tests {
    use super::{effective_concurrency, model_default_spec, scan_task_spec};
    use crate::domain::llm_params::ParamValue;
    use crate::repositories::pipeline_repository::models::LlmModelRecord;
    use chrono::Utc;

    /// The judge's cap comes from the SETTINGS ROW, not from this file.
    ///
    /// ## What this pins, and what it deliberately does not
    ///
    /// `scan_task_spec` cannot reach the settings store — it takes no state — so
    /// the only way a number arrives in the task layer is for `resolve_scan_provider`
    /// to have read one. That is the structural half of ruling R3: a compiled-in
    /// default cannot be reintroduced here without changing this signature, and
    /// changing it breaks this test.
    ///
    /// The value under test is `Settings::for_test().theme_scan_max_tokens`, which
    /// `settings_store_tests` separately pins to the migration's seed. What that
    /// chain covers, stated precisely: the row on disk equals the fixture, and the
    /// fixture's value survives the task layer unchanged. The one link it does NOT
    /// cover is `resolve_scan_provider` actually reading the snapshot — that
    /// function needs a database, a graph and a registry before it can be called,
    /// so the six lines that read `state.settings.current()` rest on review rather
    /// than on this test. Said plainly because "end to end" would be a claim this
    /// file cannot back.
    #[test]
    fn the_scan_judge_cap_is_read_from_settings_at_scan_start() {
        use crate::domain::settings::Settings;

        let from_the_store = Settings::for_test().theme_scan_max_tokens;
        assert_eq!(
            scan_task_spec(from_the_store).max_tokens,
            ParamValue::Set(from_the_store),
            "the task layer must carry the stored cap verbatim"
        );

        // ANTI-VACUITY. A `scan_task_spec` that ignored its argument and set some
        // fixed number would pass the assertion above whenever that number
        // happened to equal the seed. Two different caps must produce two
        // different specs — which is what makes the first assertion mean "reads
        // the row" rather than "happens to match today".
        assert_ne!(
            scan_task_spec(512).max_tokens,
            scan_task_spec(8192).max_tokens,
            "the cap must be the argument, not a constant this file still owns"
        );
        assert_eq!(
            scan_task_spec(512).max_tokens,
            ParamValue::Set(512),
            "…including the 512 that used to be compiled in — it is now merely a \
             number a caller may pass, with no special standing"
        );
    }

    /// The two files that used to own the cap no longer decide it (ruling R3).
    ///
    /// Targeted rather than a source scan: `services::theme_scan` no longer
    /// exports `THEME_SCAN_MAX_TOKENS` at all, so a reintroduction would have to
    /// re-export it and re-import it here — and the import list above is what
    /// would have to change. The other half is the signature test above.
    ///
    /// Written as a compile-time claim in a doc comment rather than a runtime
    /// assertion because that is what it honestly is: `use
    /// crate::services::theme_scan::THEME_SCAN_MAX_TOKENS;` does not compile, and
    /// there is no way to assert the absence of a constant at runtime without
    /// reading source text, which ruling R3 declined.
    #[test]
    fn no_compiled_in_cap_remains_on_the_judge_call_path() {
        // What CAN be asserted at runtime: every cap the task layer produces is
        // one somebody passed in. Sweeping a range proves the spec is a pure
        // function of its argument over the row's whole legal span (256..=64000),
        // which a smuggled `max(512, n)` or a clamp would fail.
        for cap in [256_u32, 512, 4096, 8192, 64000] {
            assert_eq!(
                scan_task_spec(cap).max_tokens,
                ParamValue::Set(cap),
                "cap {cap} was altered between the argument and the task layer"
            );
        }

        // And the two knobs ruling R4 froze are untouched by any of it.
        let spec = scan_task_spec(8192);
        assert_eq!(spec.temperature, ParamValue::Set(0.0));
        assert_eq!(spec.timeout_secs, ParamValue::Unset);
    }

    #[test]
    fn concurrency_prefers_positive_row_value() {
        // The 14B row caps at 4; the env default is 8 — the row wins.
        assert_eq!(effective_concurrency(Some(4), 8), 4);
    }

    #[test]
    fn concurrency_falls_back_on_null_or_nonpositive() {
        assert_eq!(effective_concurrency(None, 8), 8, "NULL → env default");
        assert_eq!(effective_concurrency(Some(0), 8), 8, "0 is not a cap → env");
        assert_eq!(effective_concurrency(Some(-1), 8), 8, "negative → env");
    }

    /// A minimal record varying only the two columns `model_default_spec` reads.
    fn record(default_temperature: Option<f64>, timeout_secs: Option<i32>) -> LlmModelRecord {
        LlmModelRecord {
            id: "m".to_string(),
            display_name: "m".to_string(),
            provider: "anthropic".to_string(),
            api_endpoint: None,
            max_context_tokens: None,
            max_output_tokens: None,
            cost_per_input_token: None,
            cost_per_output_token: None,
            is_active: true,
            created_at: Utc::now(),
            notes: None,
            default_temperature,
            temperature_mode: None,
            timeout_secs,
            structured_output_mode: None,
            max_concurrency: None,
            // Not read by this path; the fixture asserts nothing about it.
            billing_class: "local".to_string(),
        }
    }

    #[test]
    fn model_default_spec_null_columns_are_unset() {
        let spec = model_default_spec(&record(None, None)).expect("null columns are valid");
        assert_eq!(spec.temperature, ParamValue::Unset);
        assert_eq!(spec.timeout_secs, ParamValue::Unset);
        assert_eq!(
            spec.max_tokens,
            ParamValue::Unset,
            "no per-model default max_tokens"
        );
    }

    #[test]
    fn model_default_spec_maps_present_columns() {
        let spec =
            model_default_spec(&record(Some(0.5), Some(30))).expect("present columns are valid");
        assert_eq!(spec.temperature, ParamValue::Set(0.5));
        assert_eq!(spec.timeout_secs, ParamValue::Set(30));
    }

    #[test]
    fn model_default_spec_negative_timeout_is_a_loud_error() {
        // A corrupt negative timeout must not `as`-cast into a huge u64 — it is a
        // named error naming the column and value (Standing Rule 1 / R3).
        let err = model_default_spec(&record(None, Some(-1)))
            .expect_err("negative timeout must be rejected");
        let msg = err.to_string();
        assert!(msg.contains("timeout_secs"), "names the column: {msg}");
        assert!(msg.contains("-1"), "names the bad value: {msg}");
    }
}
