// =============================================================================
// backend/src/domain/llm_params.rs — LLM parameter resolution & constraint pass
// (LLM Configuration Method, Chunk A)
// =============================================================================
//
// An LLM call needs three runtime knobs settled before it is dispatched:
// TEMPERATURE, TIMEOUT, and MAX_TOKENS. Each can be set at three layers — a
// per-USER override, a per-TASK default, or the MODEL's own default — and then
// the settled values must be checked against what the MODEL can actually do
// (some models reject an explicit temperature; every model has an output-token
// ceiling; not every model can emit structured output). This module owns that
// two-stage computation, and NOTHING else: it is pure — no `sqlx`, no I/O, no
// `async`. The database record enters only at a narrow boundary
// ([`ModelConstraints::from_record`]) that copies the capability facts out; the
// record itself never crosses into the resolver.
//
// ## Why two stages (resolve THEN constrain), not one
//
// The design (A3) splits the work deliberately:
//
//   1. `resolve()`  — WHAT the layered configuration ASKS for (user > task >
//                     model-default). Pure precedence arithmetic. Knows nothing
//                     about the model's limits.
//   2. `constrain()`— WHAT the model will ACCEPT. Clamps the resolved request to
//                     the model's capabilities: an omit-required model forces
//                     temperature off; an over-ceiling max_tokens is a LOUD error
//                     (clamp-by-error, never silent-truncate).
//
// Keeping them separate means the precedence rules and the capability rules can
// each be read, tested, and changed without touching the other — and the order
// (resolve first, constrain second) is a fact the type signatures make explicit:
// `constrain` consumes a `ResolvedLlmParams`, so it CANNOT run first.
//
// ## Rust Learning: parse-don't-validate, the config twin of `FactStatus`
//
// `LlmParamsSpec` is the UNVALIDATED request (three `ParamValue`s that may defer,
// omit, or set). `ResolvedLlmParams` is the VALIDATED, settled result — every
// field is a concrete decision. Like `FactStatus` turns a raw token into a typed
// state at a loud boundary, `resolve`/`constrain` turn a layered request into a
// settled result at a loud boundary: an impossible combination becomes an `Err`,
// never a silently-patched value. Downstream code that holds a `ResolvedLlmParams`
// never has to re-ask "is this temperature legal for this model?" — the type is
// the proof that the question was already answered.
//
// Domain note: TEMPERATURE resolving to "nothing" is legal and meaningful — it
// means "omit the parameter entirely" (an omit-required model, or nobody asked
// for one). TIMEOUT and MAX_TOKENS resolving to "nothing" is NOT legal: an HTTP
// call must have a timeout and a token budget, so those fall back to a documented
// system default rather than ever being absent.

use crate::repositories::pipeline_repository::models::LlmModelRecord;

/// The system-default HTTP timeout, in seconds, used when no layer sets one.
///
/// ## Rust Learning: a documented floor constant, not a magic number
///
/// Standing Rule 2 (no hardcoded values) exempts a value that is genuinely
/// code-level and cannot vary per deployment — the resolver's LAST-RESORT floor
/// is such a value: it exists so the resolver can never produce an absent timeout.
/// Its numeric magnitude mirrors `rig_provider::DEFAULT_TIMEOUT_SECS` (600s) so a
/// silent all-layers-unset path behaves like the existing provider default.
///
/// Future reconciliation: Chunk B owns the provider-side timeout plumbing; when
/// it lands, this floor and the provider default should be sourced from ONE place.
/// Defined here independently so Chunk A does not reach into Chunk B's module.
// CONST: a last-resort resolver FLOOR, not a per-deployment tunable. It exists so
// resolution can never yield an absent timeout; a floor of zero is not a safe
// fallback (a zero timeout aborts every call), so this is NOT env-configurable
// down to arbitrary values. (Future: Chunk B may source it and the provider
// default from one place — flagged, not done here.)
pub const DEFAULT_TIMEOUT_SECS: u64 = 600;

/// The system-default max-output-tokens floor, used when no layer sets one.
///
/// Mirrors `pipeline::providers::FALLBACK_MAX_TOKENS` (8000) in magnitude but is
/// deliberately a SEPARATE constant: `FALLBACK_MAX_TOKENS` lives in Chunk B's
/// module, and Chunk A must not import it. When Chunk B wires the resolver into
/// the provider call sites, the two should be reconciled to a single owner.
// CONST: a last-resort resolver FLOOR, not a per-deployment tunable. It exists so
// resolution can never yield an absent token budget; a floor of zero is not a
// safe fallback (a zero budget produces no output), so this is NOT env-configurable
// down to arbitrary values. (Future: reconcile with FALLBACK_MAX_TOKENS in Chunk
// B — flagged, not done here.)
pub const DEFAULT_MAX_TOKENS: u32 = 8000;

/// A single configuration knob's state at ONE layer.
///
/// ## Rust Learning: a three-state value vs `Option<T>`
///
/// `Option<T>` has two states (`Some`/`None`) and cannot express the difference
/// between "I did not touch this — defer to a lower layer" and "I explicitly want
/// this OFF". A layered override system needs BOTH, so this is a three-state type:
///
/// - `Unset` — this layer is silent; DEFER to the next-lower layer.
/// - `Clear` — this layer explicitly OMITS the parameter (temperature only).
/// - `Set(v)`— this layer pins a concrete value.
///
/// Collapsing `Unset` and `Clear` into a single `None` would make "the user left
/// it alone" indistinguishable from "the user demanded no temperature" — exactly
/// the silent-collapse Standing Rule 1 forbids.
///
/// `Copy` is derived because every `T` we instantiate it with (`f64`, `u64`,
/// `u32`) is itself `Copy`, so a `ParamValue` is a small plain value passed by
/// value without a `.clone()`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParamValue<T> {
    /// This layer is silent — defer to the next-lower layer.
    Unset,
    /// This layer explicitly omits the parameter (meaningful for temperature).
    Clear,
    /// This layer pins a concrete value.
    Set(T),
}

/// One layer's requested LLM parameters (the UNVALIDATED request).
///
/// Three of these — model-default, task, user — are the input to [`resolve`].
/// Each field is a [`ParamValue`] so a layer can defer, omit, or set per knob.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LlmParamsSpec {
    /// Sampling temperature. `Clear` here means "omit temperature entirely".
    pub temperature: ParamValue<f64>,
    /// HTTP timeout in seconds. `Clear` is meaningless (an HTTP call must have a
    /// timeout) and is rejected by [`resolve`].
    pub timeout_secs: ParamValue<u64>,
    /// Max output tokens. `Clear` is meaningless (a call must have a token
    /// budget) and is rejected by [`resolve`].
    pub max_tokens: ParamValue<u32>,
}

impl LlmParamsSpec {
    /// An all-`Unset` spec — the neutral "this layer requests nothing" value.
    ///
    /// Handy for tests and for a caller that has no overrides at some layer.
    pub const SILENT: LlmParamsSpec = LlmParamsSpec {
        temperature: ParamValue::Unset,
        timeout_secs: ParamValue::Unset,
        max_tokens: ParamValue::Unset,
    };
}

/// The settled, VALIDATED parameters — every field is a concrete decision.
///
/// Produced by [`resolve`] and refined by [`constrain`]. The presence of this
/// type is the proof that resolution succeeded.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedLlmParams {
    /// `Some(v)` = send temperature `v`; `None` = UNAMBIGUOUSLY omit the
    /// parameter (either nobody set one, or the model is omit-required).
    pub temperature: Option<f64>,
    /// Concrete HTTP timeout in seconds — never absent.
    pub timeout_secs: u64,
    /// Concrete max output tokens — never absent.
    pub max_tokens: u32,
}

/// A model's temperature capability, owned by CODE (not a DB CHECK).
///
/// Domain note: some models accept an explicit `temperature = 0.0`
/// ([`ZeroOk`](TemperatureMode::ZeroOk)); some (e.g. certain reasoning models)
/// REJECT any explicit temperature and require it be omitted
/// ([`Omit`](TemperatureMode::Omit)). The distinction changes the wire request,
/// so it is first-class, not a note.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemperatureMode {
    /// The model accepts an explicit temperature, including `0.0`.
    ZeroOk,
    /// The model rejects an explicit temperature — it MUST be omitted.
    Omit,
    /// Capability not recorded (the DB column was NULL). Treated conservatively.
    Unknown,
}

impl TemperatureMode {
    /// Map the raw `Option<String>` DB column to the typed mode.
    ///
    /// ## Rust Learning: loud-on-bad-input, matching `FactStatus`
    ///
    /// `None` (the column was NULL) is a LEGITIMATE state → [`Unknown`], not an
    /// error: an un-marked model simply has an unrecorded capability. But a
    /// NON-empty string that is not a known token is a genuine data fault → a
    /// loud [`LlmConfigError::UnknownTemperatureMode`] carrying the offending
    /// token, exactly as `FactStatus::try_from` rejects an unknown status. The
    /// two cases are DISTINGUISHABLE (Standing Rule 1): missing ≠ malformed.
    ///
    /// `model_id` names the row this token came from so a malformed value points
    /// the operator at the exact `llm_models` row to fix (the WHERE of the error).
    fn from_optional_token(model_id: &str, token: Option<&str>) -> Result<Self, LlmConfigError> {
        match token {
            None => Ok(TemperatureMode::Unknown),
            Some("zero-ok") => Ok(TemperatureMode::ZeroOk),
            Some("omit") => Ok(TemperatureMode::Omit),
            Some(other) => Err(LlmConfigError::UnknownTemperatureMode {
                model_id: model_id.to_string(),
                token: other.to_string(),
            }),
        }
    }
}

/// The deterministic temperature for a `zero-ok`/`unknown` model whose row sets no
/// explicit `default_temperature`.
///
/// Named (not a bare `0.0` literal) per Standing Rule 2: it is the value the
/// extraction path has always pinned for byte-identical reruns, now sourced from
/// ONE place instead of a per-call-site constant.
// CONST: not directly env-configurable — a temperature IS configurable PER MODEL
// via the `llm_models.default_temperature` column (the operator sets it there).
// This constant is only the ABSOLUTE FALLBACK when that column is NULL, anchoring
// the extraction determinism contract (byte-identical reruns) for rows that carry
// no explicit default. A value of 0.0 is the determinism anchor, not a tunable.
const ZERO_OK_DEFAULT_TEMPERATURE: f64 = 0.0;

/// Derive the CONSTRUCTION-time temperature for a model from its `llm_models` row,
/// honoring the per-model temperature capability.
///
/// This is the SINGLE SOURCE OF TRUTH for "row → construction temperature". It
/// replaces the hardcoded `EXTRACTION_TEMPERATURE` in `pipeline::providers`, which
/// ignored `temperature_mode` and so sent `temperature = 0` to EVERY Claude model
/// — 400-ing temperature-deprecated ones (e.g. `claude-opus-4-7`).
///
/// - `temperature_mode = 'omit'`      → `None` (send NO temperature key; the model
///   rejects any explicit temperature).
/// - `'zero-ok'` / NULL (`Unknown`)   → the row's `default_temperature` if set,
///   else [`ZERO_OK_DEFAULT_TEMPERATURE`] (extraction's long-standing pin).
///
/// ## Rust Learning: reuse the token→enum mapping, don't re-match strings
///
/// The `'omit'`/`'zero-ok'` vocabulary is owned by
/// [`TemperatureMode::from_optional_token`] — the SAME mapping the constraint pass
/// uses. Calling it here (rather than a second `match` on the raw column) means
/// this construction path cannot drift from the resolver's understanding of the
/// column, and a malformed token is the same loud [`LlmConfigError`] in both.
///
/// # Errors
///
/// [`LlmConfigError::UnknownTemperatureMode`] if the row's `temperature_mode`
/// holds a non-empty, unrecognized token.
pub fn construction_temperature(record: &LlmModelRecord) -> Result<Option<f64>, LlmConfigError> {
    let mode =
        TemperatureMode::from_optional_token(&record.id, record.temperature_mode.as_deref())?;
    Ok(match mode {
        TemperatureMode::Omit => None,
        TemperatureMode::ZeroOk | TemperatureMode::Unknown => record
            .default_temperature
            .or(Some(ZERO_OK_DEFAULT_TEMPERATURE)),
    })
}

/// A model's structured-output capability, owned by CODE (not a DB CHECK).
///
/// Domain note: `None_` (a KNOWN "cannot") is different from `Unknown` (not
/// recorded). An un-onboarded local model is `Unknown`, and the constraint pass
/// treats it conservatively — it is NOT assumed capable just because nobody said
/// otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuredOutputMode {
    /// Native structured output (e.g. Anthropic tool-use).
    Native,
    /// Guided/constrained decoding (e.g. a vLLM grammar).
    Guided,
    /// Known to NOT support structured output.
    ///
    /// ## Rust Learning: the trailing underscore on `None_`
    ///
    /// `None` is not a Rust keyword, but it is the ubiquitous `Option::None`
    /// variant brought into every scope by the prelude. Naming a variant plain
    /// `None` would shadow it confusingly at match sites. The trailing underscore
    /// (`None_`) is the idiomatic escape — a distinct name that still reads as
    /// "none".
    None_,
    /// Capability not recorded (the DB column was NULL). Treated conservatively.
    Unknown,
}

impl StructuredOutputMode {
    /// Map the raw `Option<String>` DB column to the typed mode. Same
    /// loud-on-bad-input contract as [`TemperatureMode::from_optional_token`];
    /// `model_id` names the offending row for the operator.
    fn from_optional_token(model_id: &str, token: Option<&str>) -> Result<Self, LlmConfigError> {
        match token {
            None => Ok(StructuredOutputMode::Unknown),
            Some("native") => Ok(StructuredOutputMode::Native),
            Some("guided") => Ok(StructuredOutputMode::Guided),
            Some("none") => Ok(StructuredOutputMode::None_),
            Some(other) => Err(LlmConfigError::UnknownStructuredOutputMode {
                model_id: model_id.to_string(),
                token: other.to_string(),
            }),
        }
    }

    /// The wire/display token for this mode (`Unknown` has no DB token).
    fn label(self) -> &'static str {
        match self {
            StructuredOutputMode::Native => "native",
            StructuredOutputMode::Guided => "guided",
            StructuredOutputMode::None_ => "none",
            StructuredOutputMode::Unknown => "unknown",
        }
    }
}

/// The per-model capability facts the constraint pass checks against.
///
/// This small pure struct is filled FROM an [`LlmModelRecord`] at the boundary
/// ([`ModelConstraints::from_record`]) so the record — a database shape with
/// `Option<i32>` columns and raw mode strings — never enters the pure resolver.
/// The boundary does the `i32 → u32` narrowing and the string → enum mapping.
///
/// ## Rust Learning: why this dropped its `Copy` derive
///
/// Adding the owned `model_id: String` means the struct now holds heap data, so
/// it can no longer be `Copy` (a bitwise copy would double-own the `String`'s
/// allocation). It keeps `Clone` — an explicit, visible copy — and the constraint
/// pass takes it by shared reference (`&ModelConstraints`), so no clone is needed
/// on the hot path anyway. Carrying the id is worth the trade: every capability
/// error can now name the exact model row (the WHERE of Standing Rule 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelConstraints {
    /// The model this capability set describes. Carried so a capability failure
    /// (over-ceiling tokens, unsupported structured output) names the exact row.
    pub model_id: String,
    /// How this model handles an explicit temperature.
    pub temperature_mode: TemperatureMode,
    /// Output-token ceiling. `None` = unknown/unbounded (no clamp applied).
    pub max_output_tokens: Option<u32>,
    /// Context-window ceiling. `None` = unknown/unbounded. Used by Chunk B's
    /// prompt-token check ([`LlmConfigError::PromptExceedsContext`]); Chunk A has
    /// no prompt to count, so this field is carried but not yet consulted.
    pub max_context_tokens: Option<u32>,
    /// Whether/how this model can emit structured output.
    pub structured_output_mode: StructuredOutputMode,
}

impl ModelConstraints {
    /// Build the capability facts from a DB record at the pure/impure boundary.
    ///
    /// ## Rust Learning: `i32 → u32` via `u32::try_from`, never `as`
    ///
    /// Postgres `INTEGER` decodes to `i32`. The pure model wants `u32`. A `value
    /// as u32` cast would turn a negative (a corrupt row) into a huge positive —
    /// a silent garbage value (Standing Rule 1). `u32::try_from` returns `Err`
    /// for anything that does not fit, which we convert into a loud, named
    /// [`LlmConfigError::NegativeTokenValue`] carrying the column and the bad
    /// value. Narrowing happens HERE, at the boundary, not scattered downstream.
    pub fn from_record(record: &LlmModelRecord) -> Result<Self, LlmConfigError> {
        let model_id = record.id.as_str();
        Ok(ModelConstraints {
            model_id: record.id.clone(),
            temperature_mode: TemperatureMode::from_optional_token(
                model_id,
                record.temperature_mode.as_deref(),
            )?,
            max_output_tokens: narrow_ceiling(
                model_id,
                "max_output_tokens",
                record.max_output_tokens,
            )?,
            max_context_tokens: narrow_ceiling(
                model_id,
                "max_context_tokens",
                record.max_context_tokens,
            )?,
            structured_output_mode: StructuredOutputMode::from_optional_token(
                model_id,
                record.structured_output_mode.as_deref(),
            )?,
        })
    }
}

/// Narrow a nullable Postgres `INTEGER` ceiling to `Option<u32>`, loudly.
///
/// `None` stays `None` (unknown/unbounded). A present value is `u32::try_from`-ed
/// so a negative row value fails with a named error instead of `as`-casting to a
/// nonsense ceiling. `model_id` names the offending row for the operator.
fn narrow_ceiling(
    model_id: &str,
    column: &'static str,
    value: Option<i32>,
) -> Result<Option<u32>, LlmConfigError> {
    match value {
        None => Ok(None),
        Some(v) => u32::try_from(v)
            .map(Some)
            .map_err(|_| LlmConfigError::NegativeTokenValue {
                model_id: model_id.to_string(),
                column,
                value: v,
            }),
    }
}

/// A typed, loggable failure of parameter resolution or constraint checking.
///
/// ## Rust Learning: one variant per distinct, loggable case
///
/// Every operationally-distinct failure is its OWN variant carrying the offending
/// values (Standing Rule 1 / observability): a log or a `?`-propagated error names
/// exactly what went wrong and with which numbers. No `anyhow` in the domain
/// layer — a `thiserror` enum keeps each case a first-class, matchable value, the
/// same style as `FactStatusParseError` and `ThemeScanError`.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum LlmConfigError {
    /// A `Clear` was requested for a parameter that cannot be omitted. `Clear`
    /// is meaningful only for temperature; timeout and max_tokens MUST resolve to
    /// a concrete value, so clearing them is a configuration error, not a no-op.
    #[error(
        "parameter '{param}' cannot be cleared — Clear (omit) is only meaningful \
         for temperature; use Unset to defer or Set to pin a value"
    )]
    ClearNotAllowed {
        /// The offending parameter name (`timeout_secs` / `max_tokens`).
        param: &'static str,
    },

    /// The resolved max_tokens exceeds the model's output-token ceiling. A
    /// NET-NEW guard: clamp-by-error, never silent-truncate a token budget. The
    /// message names the model and the recovery action (WHERE + WHAT-TO-DO).
    #[error(
        "requested max_tokens {requested} exceeds output ceiling {ceiling} for model \
         '{model_id}' — lower max_tokens in the task spec or the model-default configuration"
    )]
    MaxTokensExceedsCeiling {
        /// The model whose ceiling was exceeded.
        model_id: String,
        /// What the layered configuration asked for.
        requested: u32,
        /// The model's hard output-token ceiling.
        ceiling: u32,
    },

    /// A task requires structured output but the model cannot provide it. The
    /// message names the model and points at the fix (choose a capable model).
    #[error(
        "model '{model_id}' cannot satisfy required structured output (capability: {}) — \
         choose a model with native or guided structured-output support",
        .mode.label()
    )]
    StructuredOutputUnsupported {
        /// The model that failed the requirement.
        model_id: String,
        /// The model's (in)capability that failed the requirement.
        mode: StructuredOutputMode,
    },

    /// The `temperature_mode` DB column held a non-empty, unrecognized token.
    #[error(
        "model '{model_id}': unknown temperature_mode token '{token}' — set the \
         llm_models.temperature_mode column to zero-ok or omit"
    )]
    UnknownTemperatureMode {
        /// The model whose row carried the bad token.
        model_id: String,
        /// The offending token from the row.
        token: String,
    },

    /// The `structured_output_mode` DB column held a non-empty, unrecognized
    /// token.
    #[error(
        "model '{model_id}': unknown structured_output_mode token '{token}' — set the \
         llm_models.structured_output_mode column to native, guided, or none"
    )]
    UnknownStructuredOutputMode {
        /// The model whose row carried the bad token.
        model_id: String,
        /// The offending token from the row.
        token: String,
    },

    /// A nullable token-ceiling column held a value that does not fit `u32`
    /// (e.g. a negative). Caught at the narrowing boundary, not `as`-cast away.
    #[error(
        "model '{model_id}': column '{column}' holds an out-of-range value {value} \
         (must be a non-negative u32) — correct the llm_models row"
    )]
    NegativeTokenValue {
        /// The model whose row carried the bad value.
        model_id: String,
        /// Which column carried the bad value.
        column: &'static str,
        /// The raw `i32` that failed to narrow.
        value: i32,
    },

    /// Reserved for Chunk B: the assembled prompt exceeds the model's context
    /// window. Chunk A has no prompt to count tokens for, so this variant is
    /// DEFINED (so Chunk B's wiring has a named error ready) but never
    /// constructed here. The check itself is a Chunk B wiring point.
    #[error(
        "model '{model_id}': prompt of {prompt_tokens} tokens exceeds context window \
         {max_context_tokens} — shorten the prompt or choose a model with a larger context window"
    )]
    PromptExceedsContext {
        /// The model whose context window was exceeded (Chunk B).
        model_id: String,
        /// Counted prompt tokens (Chunk B).
        prompt_tokens: u32,
        /// The model's context ceiling.
        max_context_tokens: u32,
    },
}

/// Reduce the three layers of one knob to its effective request.
///
/// ## Rust Learning: precedence as "first non-`Unset` wins"
///
/// The layer order is fixed — user beats task beats model-default. Walking the
/// layers highest-priority-first and returning the FIRST that is not `Unset`
/// encodes exactly that: an `Unset` layer is transparent (defer), a `Set` or
/// `Clear` layer is opaque (it decides). If every layer is `Unset`, the knob is
/// `Unset` overall and the caller applies its own default. `T: Copy` lets us pull
/// the value out of the borrowed spec without cloning.
fn effective<T: Copy>(
    user: ParamValue<T>,
    task: ParamValue<T>,
    model_default: ParamValue<T>,
) -> ParamValue<T> {
    for layer in [user, task, model_default] {
        if !matches!(layer, ParamValue::Unset) {
            return layer;
        }
    }
    ParamValue::Unset
}

/// Resolve the three layers into settled parameters (stage 1 of 2).
///
/// Precedence per knob: `user` > `task` > `model_default`. `Unset` defers to the
/// next-lower layer; `Set(v)` pins `v`; `Clear` omits (temperature only).
///
/// - `temperature`: effective `Set(v)` → `Some(v)`; `Clear` OR all-`Unset` →
///   `None` (omit — a legal, meaningful outcome).
/// - `timeout_secs` / `max_tokens`: effective `Set(v)` → `v`; all-`Unset` → the
///   documented system default; `Clear` → a loud [`LlmConfigError::ClearNotAllowed`]
///   (these knobs cannot be omitted).
///
/// Returns [`ResolvedLlmParams`], or an error if a non-omittable knob was cleared.
/// This stage knows nothing about model capabilities — that is [`constrain`].
pub fn resolve(
    model_default: &LlmParamsSpec,
    task: &LlmParamsSpec,
    user: &LlmParamsSpec,
) -> Result<ResolvedLlmParams, LlmConfigError> {
    let temperature = match effective(
        user.temperature,
        task.temperature,
        model_default.temperature,
    ) {
        ParamValue::Set(v) => Some(v),
        // Both an explicit Clear and an all-silent chain mean "omit temperature".
        ParamValue::Clear | ParamValue::Unset => None,
    };

    let timeout_secs = resolve_required(
        effective(
            user.timeout_secs,
            task.timeout_secs,
            model_default.timeout_secs,
        ),
        DEFAULT_TIMEOUT_SECS,
        "timeout_secs",
    )?;

    let max_tokens = resolve_required(
        effective(user.max_tokens, task.max_tokens, model_default.max_tokens),
        DEFAULT_MAX_TOKENS,
        "max_tokens",
    )?;

    Ok(ResolvedLlmParams {
        temperature,
        timeout_secs,
        max_tokens,
    })
}

/// Resolve one NON-omittable knob to a concrete value.
///
/// `Set(v)` → `v`; `Unset` → `system_default`; `Clear` → error (you cannot omit a
/// knob that must have a value). Shared by `timeout_secs` and `max_tokens` so the
/// "must be concrete" rule lives in one place.
fn resolve_required<T: Copy>(
    effective: ParamValue<T>,
    system_default: T,
    param: &'static str,
) -> Result<T, LlmConfigError> {
    match effective {
        ParamValue::Set(v) => Ok(v),
        ParamValue::Unset => Ok(system_default),
        ParamValue::Clear => Err(LlmConfigError::ClearNotAllowed { param }),
    }
}

/// Constrain resolved parameters to the model's capabilities (stage 2 of 2).
///
/// - `temperature_mode == Omit` FORCES `temperature = None`, overriding whatever
///   was resolved — even an explicit user `Set(0.0)` (A3: the model's hard
///   requirement wins over any request).
/// - `max_output_tokens == Some(ceiling)` and `max_tokens > ceiling` →
///   [`LlmConfigError::MaxTokensExceedsCeiling`] (clamp-by-error). `None` ceiling
///   = unknown/unbounded, no clamp.
///
/// Structured-output gating is NOT here — it needs a task-declared requirement
/// flag that Chunk A has no caller for; see [`check_structured_output`].
pub fn constrain(
    resolved: ResolvedLlmParams,
    c: &ModelConstraints,
) -> Result<ResolvedLlmParams, LlmConfigError> {
    let temperature = match c.temperature_mode {
        // The model rejects any explicit temperature: force omission regardless
        // of what resolution produced.
        TemperatureMode::Omit => None,
        TemperatureMode::ZeroOk | TemperatureMode::Unknown => resolved.temperature,
    };

    if let Some(ceiling) = c.max_output_tokens {
        if resolved.max_tokens > ceiling {
            return Err(LlmConfigError::MaxTokensExceedsCeiling {
                model_id: c.model_id.clone(),
                requested: resolved.max_tokens,
                ceiling,
            });
        }
    }

    Ok(ResolvedLlmParams {
        temperature,
        ..resolved
    })
}

/// Check whether a model can satisfy a task's structured-output REQUIREMENT.
///
/// Separated from [`constrain`] because the `requires_structured` flag is a TASK
/// property that Chunk A has no caller supplying yet (the Theme Scan wires it in
/// Chunk B). Written now, with the flag as an explicit parameter, so the rule is
/// unit-testable immediately even though nothing calls it.
///
/// - `requires_structured == false` → always `Ok` (nothing required).
/// - required against `Native`/`Guided` → `Ok`.
/// - required against `None_` (known-incapable) or `Unknown` (not recorded — not
///   assumed capable) → [`LlmConfigError::StructuredOutputUnsupported`].
///
/// `model_id` names the model in the failure so the error is operator-actionable
/// once Chunk B wires this to a call site.
pub fn check_structured_output(
    model_id: &str,
    mode: StructuredOutputMode,
    requires_structured: bool,
) -> Result<(), LlmConfigError> {
    if !requires_structured {
        return Ok(());
    }
    match mode {
        StructuredOutputMode::Native | StructuredOutputMode::Guided => Ok(()),
        StructuredOutputMode::None_ | StructuredOutputMode::Unknown => {
            Err(LlmConfigError::StructuredOutputUnsupported {
                model_id: model_id.to_string(),
                mode,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stable model id for constraint/error tests (case-agnostic label).
    const TEST_MODEL: &str = "test-model";

    /// Build a spec with only temperature set, the rest silent — test sugar.
    fn temp_only(t: ParamValue<f64>) -> LlmParamsSpec {
        LlmParamsSpec {
            temperature: t,
            ..LlmParamsSpec::SILENT
        }
    }

    /// Build `ModelConstraints` with a fixed test model id and no context
    /// ceiling — test sugar so each constraint test names only what it exercises.
    fn constraints(
        temperature_mode: TemperatureMode,
        max_output_tokens: Option<u32>,
        structured_output_mode: StructuredOutputMode,
    ) -> ModelConstraints {
        ModelConstraints {
            model_id: TEST_MODEL.to_string(),
            temperature_mode,
            max_output_tokens,
            max_context_tokens: None,
            structured_output_mode,
        }
    }

    /// Build an `LlmModelRecord` carrying only the capability columns the
    /// `from_record` boundary reads; the rest are neutral. Lets the boundary test
    /// exercise the PUBLIC entry point rather than the private helpers.
    fn record(
        temperature_mode: Option<&str>,
        max_output_tokens: Option<i32>,
        max_context_tokens: Option<i32>,
        structured_output_mode: Option<&str>,
    ) -> LlmModelRecord {
        LlmModelRecord {
            id: TEST_MODEL.to_string(),
            display_name: TEST_MODEL.to_string(),
            provider: "anthropic".to_string(),
            api_endpoint: None,
            max_context_tokens,
            max_output_tokens,
            cost_per_input_token: None,
            cost_per_output_token: None,
            is_active: true,
            // A fixed epoch timestamp — deterministic, no wall-clock in a test.
            created_at: chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0)
                .expect("epoch 0 is a valid timestamp"),
            notes: None,
            default_temperature: None,
            temperature_mode: temperature_mode.map(String::from),
            timeout_secs: None,
            structured_output_mode: structured_output_mode.map(String::from),
            max_concurrency: None,
        }
    }

    // ── construction_temperature (row → provider-construction temperature) ──

    /// A record varying only the two columns `construction_temperature` reads.
    fn ctemp_record(mode: Option<&str>, default_temperature: Option<f64>) -> LlmModelRecord {
        let mut r = record(mode, None, None, None);
        r.default_temperature = default_temperature;
        r
    }

    #[test]
    fn construction_temperature_omit_sends_none_even_with_a_default() {
        // A temperature-deprecated model (e.g. claude-opus-4-7) omits the key —
        // and `omit` wins even if a default_temperature is set (the model rejects it).
        assert_eq!(
            construction_temperature(&ctemp_record(Some("omit"), None)).expect("valid row"),
            None
        );
        assert_eq!(
            construction_temperature(&ctemp_record(Some("omit"), Some(0.7))).expect("valid row"),
            None
        );
    }

    #[test]
    fn construction_temperature_zero_ok_uses_default_else_the_pin() {
        assert_eq!(
            construction_temperature(&ctemp_record(Some("zero-ok"), Some(0.5))).expect("valid row"),
            Some(0.5)
        );
        // zero-ok + NULL default → the deterministic pin (extraction's Some(0.0)).
        assert_eq!(
            construction_temperature(&ctemp_record(Some("zero-ok"), None)).expect("valid row"),
            Some(ZERO_OK_DEFAULT_TEMPERATURE)
        );
    }

    #[test]
    fn construction_temperature_null_mode_behaves_like_zero_ok() {
        // NULL temperature_mode (Unknown) defaults like zero-ok for construction —
        // this preserves the extraction models' Some(0.0) if they were ever unmarked.
        assert_eq!(
            construction_temperature(&ctemp_record(None, None)).expect("valid row"),
            Some(ZERO_OK_DEFAULT_TEMPERATURE)
        );
        assert_eq!(
            construction_temperature(&ctemp_record(None, Some(0.3))).expect("valid row"),
            Some(0.3)
        );
    }

    #[test]
    fn construction_temperature_rejects_an_unknown_token() {
        let err =
            construction_temperature(&ctemp_record(Some("hot"), None)).expect_err("bad token");
        assert!(matches!(err, LlmConfigError::UnknownTemperatureMode { .. }));
    }

    // ── Resolution precedence ────────────────────────────────────────────

    #[test]
    fn user_beats_task_beats_model_default_per_field() {
        // Each knob set at a different layer; the highest-priority setter wins.
        let model_default = LlmParamsSpec {
            temperature: ParamValue::Set(0.1),
            timeout_secs: ParamValue::Set(100),
            max_tokens: ParamValue::Set(1000),
        };
        let task = LlmParamsSpec {
            temperature: ParamValue::Set(0.2),
            timeout_secs: ParamValue::Set(200),
            max_tokens: ParamValue::Unset,
        };
        let user = LlmParamsSpec {
            temperature: ParamValue::Set(0.3),
            timeout_secs: ParamValue::Unset,
            max_tokens: ParamValue::Unset,
        };

        let r = resolve(&model_default, &task, &user).expect("valid");
        assert_eq!(r.temperature, Some(0.3)); // user wins
        assert_eq!(r.timeout_secs, 200); // task wins (user Unset)
        assert_eq!(r.max_tokens, 1000); // model-default wins (user+task Unset)
    }

    #[test]
    fn unset_at_a_layer_defers_to_next_lower() {
        // User silent everywhere → task decides; where task is also silent →
        // model-default decides. Confirms the deferral chain, not just the ends.
        let model_default = LlmParamsSpec {
            temperature: ParamValue::Set(0.9),
            timeout_secs: ParamValue::Set(30),
            max_tokens: ParamValue::Set(4096),
        };
        let task = temp_only(ParamValue::Set(0.5));
        let user = LlmParamsSpec::SILENT;

        let r = resolve(&model_default, &task, &user).expect("valid");
        assert_eq!(r.temperature, Some(0.5)); // task set, model-default deferred
        assert_eq!(r.timeout_secs, 30); // only model-default set
        assert_eq!(r.max_tokens, 4096);
    }

    // ── Clear semantics ──────────────────────────────────────────────────

    #[test]
    fn clear_on_temperature_resolves_to_none() {
        // A Clear at the winning layer omits temperature — distinct from Unset,
        // though both land on None here; the point is Clear is ACCEPTED (no err).
        let user = temp_only(ParamValue::Clear);
        let r = resolve(&LlmParamsSpec::SILENT, &LlmParamsSpec::SILENT, &user).expect("valid");
        assert_eq!(r.temperature, None);
    }

    #[test]
    fn clear_on_temperature_beats_lower_set() {
        // Precedence still applies to Clear: a user Clear overrides a model
        // default that set a temperature → omitted, not the lower value.
        let model_default = temp_only(ParamValue::Set(0.7));
        let user = temp_only(ParamValue::Clear);
        let r = resolve(&model_default, &LlmParamsSpec::SILENT, &user).expect("valid");
        assert_eq!(r.temperature, None);
    }

    #[test]
    fn clear_on_timeout_is_an_error() {
        // DECISION (instruction "decide and test"): Clear on a non-omittable knob
        // is a loud error, not a silent no-op — timeout MUST be concrete.
        let user = LlmParamsSpec {
            timeout_secs: ParamValue::Clear,
            ..LlmParamsSpec::SILENT
        };
        let err = resolve(&LlmParamsSpec::SILENT, &LlmParamsSpec::SILENT, &user).unwrap_err();
        assert_eq!(
            err,
            LlmConfigError::ClearNotAllowed {
                param: "timeout_secs"
            }
        );
    }

    #[test]
    fn clear_on_max_tokens_is_an_error() {
        let task = LlmParamsSpec {
            max_tokens: ParamValue::Clear,
            ..LlmParamsSpec::SILENT
        };
        let err = resolve(&LlmParamsSpec::SILENT, &task, &LlmParamsSpec::SILENT).unwrap_err();
        assert_eq!(
            err,
            LlmConfigError::ClearNotAllowed {
                param: "max_tokens"
            }
        );
    }

    // ── All-silent falls back to system defaults ─────────────────────────

    #[test]
    fn all_silent_uses_system_defaults() {
        // No layer touches anything: timeout/max_tokens take the documented
        // floors; temperature is omitted (None) — the legal "nobody asked" case.
        let r = resolve(
            &LlmParamsSpec::SILENT,
            &LlmParamsSpec::SILENT,
            &LlmParamsSpec::SILENT,
        )
        .expect("valid");
        assert_eq!(r.temperature, None);
        assert_eq!(r.timeout_secs, DEFAULT_TIMEOUT_SECS);
        assert_eq!(r.max_tokens, DEFAULT_MAX_TOKENS);
    }

    // ── Constraint: temperature Omit mode ────────────────────────────────

    #[test]
    fn omit_mode_forces_temperature_none_even_when_set_zero() {
        // The A3 headline: a user who explicitly Set(0.0) is still overridden to
        // None when the model is omit-required. The model's capability wins.
        let resolved = ResolvedLlmParams {
            temperature: Some(0.0),
            timeout_secs: 30,
            max_tokens: 1000,
        };
        let c = constraints(TemperatureMode::Omit, None, StructuredOutputMode::Native);
        let out = constrain(resolved, &c).expect("valid");
        assert_eq!(out.temperature, None);
        // Other fields pass through untouched.
        assert_eq!(out.timeout_secs, 30);
        assert_eq!(out.max_tokens, 1000);
    }

    #[test]
    fn zero_ok_mode_preserves_resolved_temperature() {
        let resolved = ResolvedLlmParams {
            temperature: Some(0.0),
            timeout_secs: 30,
            max_tokens: 1000,
        };
        let c = constraints(TemperatureMode::ZeroOk, None, StructuredOutputMode::Native);
        let out = constrain(resolved, &c).expect("valid");
        assert_eq!(out.temperature, Some(0.0));
    }

    #[test]
    fn unknown_temperature_mode_preserves_resolved_temperature() {
        // The Unknown arm of constrain is "treated conservatively" = pass-through:
        // an unrecorded temperature capability does NOT force omission. Asserts the
        // ZeroOk | Unknown branch for the Unknown side specifically.
        let resolved = ResolvedLlmParams {
            temperature: Some(0.4),
            timeout_secs: 30,
            max_tokens: 1000,
        };
        let c = constraints(TemperatureMode::Unknown, None, StructuredOutputMode::Native);
        let out = constrain(resolved, &c).expect("valid");
        assert_eq!(out.temperature, Some(0.4));
    }

    // ── Constraint: max_tokens ceiling ───────────────────────────────────

    #[test]
    fn max_tokens_above_ceiling_errors() {
        let resolved = ResolvedLlmParams {
            temperature: None,
            timeout_secs: 30,
            max_tokens: 9000,
        };
        let c = constraints(
            TemperatureMode::ZeroOk,
            Some(8000),
            StructuredOutputMode::Native,
        );
        let err = constrain(resolved, &c).unwrap_err();
        assert_eq!(
            err,
            LlmConfigError::MaxTokensExceedsCeiling {
                model_id: TEST_MODEL.to_string(),
                requested: 9000,
                ceiling: 8000
            }
        );
    }

    #[test]
    fn max_tokens_at_or_below_ceiling_passes() {
        let c = constraints(
            TemperatureMode::ZeroOk,
            Some(8000),
            StructuredOutputMode::Native,
        );
        // Exactly at the ceiling is allowed (`>` not `>=`).
        let at = ResolvedLlmParams {
            temperature: None,
            timeout_secs: 30,
            max_tokens: 8000,
        };
        assert_eq!(constrain(at, &c).expect("at ceiling ok").max_tokens, 8000);
        // Below the ceiling is allowed.
        let below = ResolvedLlmParams {
            max_tokens: 4000,
            ..at
        };
        assert_eq!(
            constrain(below, &c).expect("below ceiling ok").max_tokens,
            4000
        );
    }

    #[test]
    fn none_ceiling_never_clamps() {
        // Unknown/unbounded ceiling: even a large budget passes.
        let resolved = ResolvedLlmParams {
            temperature: None,
            timeout_secs: 30,
            max_tokens: 1_000_000,
        };
        let c = constraints(TemperatureMode::ZeroOk, None, StructuredOutputMode::Native);
        assert!(constrain(resolved, &c).is_ok());
    }

    // ── Structured-output gating ─────────────────────────────────────────

    #[test]
    fn requires_structured_against_capable_modes_passes() {
        assert!(check_structured_output(TEST_MODEL, StructuredOutputMode::Native, true).is_ok());
        assert!(check_structured_output(TEST_MODEL, StructuredOutputMode::Guided, true).is_ok());
    }

    #[test]
    fn requires_structured_against_incapable_modes_errors() {
        // Both a KNOWN-incapable model and an UNKNOWN one fail a hard requirement
        // — unknown is not optimistically assumed capable.
        for mode in [StructuredOutputMode::None_, StructuredOutputMode::Unknown] {
            let err = check_structured_output(TEST_MODEL, mode, true).unwrap_err();
            assert_eq!(
                err,
                LlmConfigError::StructuredOutputUnsupported {
                    model_id: TEST_MODEL.to_string(),
                    mode
                }
            );
        }
    }

    #[test]
    fn not_requiring_structured_passes_for_any_mode() {
        // When the task does not require structured output, even an incapable
        // model is fine.
        for mode in [
            StructuredOutputMode::Native,
            StructuredOutputMode::Guided,
            StructuredOutputMode::None_,
            StructuredOutputMode::Unknown,
        ] {
            assert!(check_structured_output(TEST_MODEL, mode, false).is_ok());
        }
    }

    // ── Mode-token mapping (loud on malformed) ───────────────────────────

    #[test]
    fn temperature_mode_maps_known_tokens_and_null() {
        assert_eq!(
            TemperatureMode::from_optional_token(TEST_MODEL, Some("zero-ok")).unwrap(),
            TemperatureMode::ZeroOk
        );
        assert_eq!(
            TemperatureMode::from_optional_token(TEST_MODEL, Some("omit")).unwrap(),
            TemperatureMode::Omit
        );
        // NULL column → Unknown (legitimate), NOT an error.
        assert_eq!(
            TemperatureMode::from_optional_token(TEST_MODEL, None).unwrap(),
            TemperatureMode::Unknown
        );
    }

    #[test]
    fn temperature_mode_rejects_malformed_token() {
        // DECISION (instruction "decide and test"): a non-empty unknown token is
        // a LOUD error carrying the token, matching FactStatus's character.
        let err = TemperatureMode::from_optional_token(TEST_MODEL, Some("scorching")).unwrap_err();
        assert_eq!(
            err,
            LlmConfigError::UnknownTemperatureMode {
                model_id: TEST_MODEL.to_string(),
                token: "scorching".to_string()
            }
        );
    }

    #[test]
    fn structured_output_mode_maps_known_tokens_and_null() {
        assert_eq!(
            StructuredOutputMode::from_optional_token(TEST_MODEL, Some("native")).unwrap(),
            StructuredOutputMode::Native
        );
        assert_eq!(
            StructuredOutputMode::from_optional_token(TEST_MODEL, Some("guided")).unwrap(),
            StructuredOutputMode::Guided
        );
        assert_eq!(
            StructuredOutputMode::from_optional_token(TEST_MODEL, Some("none")).unwrap(),
            StructuredOutputMode::None_
        );
        assert_eq!(
            StructuredOutputMode::from_optional_token(TEST_MODEL, None).unwrap(),
            StructuredOutputMode::Unknown
        );
    }

    #[test]
    fn structured_output_mode_rejects_malformed_token() {
        let err = StructuredOutputMode::from_optional_token(TEST_MODEL, Some("magic")).unwrap_err();
        assert_eq!(
            err,
            LlmConfigError::UnknownStructuredOutputMode {
                model_id: TEST_MODEL.to_string(),
                token: "magic".to_string()
            }
        );
    }

    // ── Boundary narrowing (i32 → u32) ───────────────────────────────────

    #[test]
    fn negative_ceiling_errors_not_wraps() {
        // The whole point of try_from over `as`: a negative row value is caught
        // loudly, not wrapped into a giant u32 ceiling.
        let err = narrow_ceiling(TEST_MODEL, "max_output_tokens", Some(-1)).unwrap_err();
        assert_eq!(
            err,
            LlmConfigError::NegativeTokenValue {
                model_id: TEST_MODEL.to_string(),
                column: "max_output_tokens",
                value: -1
            }
        );
    }

    #[test]
    fn null_and_valid_ceilings_narrow_cleanly() {
        assert_eq!(
            narrow_ceiling(TEST_MODEL, "max_output_tokens", None).unwrap(),
            None
        );
        assert_eq!(
            narrow_ceiling(TEST_MODEL, "max_output_tokens", Some(64000)).unwrap(),
            Some(64000)
        );
    }

    // ── `from_record` public boundary (exercises the whole record → constraints step) ──

    #[test]
    fn from_record_maps_all_capability_columns() {
        // Drive the PUBLIC boundary, not the private helpers: an all-valid record
        // produces the expected ModelConstraints, model_id included.
        let rec = record(Some("omit"), Some(64000), Some(200000), Some("guided"));
        let c = ModelConstraints::from_record(&rec).expect("valid record");
        assert_eq!(
            c,
            ModelConstraints {
                model_id: TEST_MODEL.to_string(),
                temperature_mode: TemperatureMode::Omit,
                max_output_tokens: Some(64000),
                max_context_tokens: Some(200000),
                structured_output_mode: StructuredOutputMode::Guided,
            }
        );
    }

    #[test]
    fn from_record_null_modes_become_unknown() {
        // NULL mode columns are the legitimate un-onboarded case → Unknown, no err.
        let rec = record(None, None, None, None);
        let c = ModelConstraints::from_record(&rec).expect("valid record");
        assert_eq!(c.temperature_mode, TemperatureMode::Unknown);
        assert_eq!(c.structured_output_mode, StructuredOutputMode::Unknown);
        assert_eq!(c.max_output_tokens, None);
        assert_eq!(c.max_context_tokens, None);
    }

    #[test]
    fn from_record_propagates_negative_context_ceiling_error() {
        // A corrupt negative context ceiling must fail loudly THROUGH the public
        // boundary (not only via narrow_ceiling directly), naming the column+row.
        let rec = record(Some("zero-ok"), Some(8000), Some(-5), Some("native"));
        let err = ModelConstraints::from_record(&rec).unwrap_err();
        assert_eq!(
            err,
            LlmConfigError::NegativeTokenValue {
                model_id: TEST_MODEL.to_string(),
                column: "max_context_tokens",
                value: -5
            }
        );
    }

    // ── Error Display strings (operator-facing text is pinned, not just fields) ──

    #[test]
    fn error_display_strings_name_the_offending_values() {
        // thiserror derives Display from the #[error("…")] templates; assert the
        // rendered text names WHAT failed and WHICH values, so a template edit that
        // drops a field is caught. One assertion per variant.
        assert!(LlmConfigError::ClearNotAllowed {
            param: "timeout_secs"
        }
        .to_string()
        .contains("timeout_secs"));

        let over = LlmConfigError::MaxTokensExceedsCeiling {
            model_id: TEST_MODEL.to_string(),
            requested: 9000,
            ceiling: 8000,
        }
        .to_string();
        assert!(over.contains("9000") && over.contains("8000") && over.contains(TEST_MODEL));

        // Each StructuredOutputMode label reaches the message — covers Guided and
        // Unknown labels that no other test renders.
        for (mode, label) in [
            (StructuredOutputMode::None_, "none"),
            (StructuredOutputMode::Guided, "guided"),
            (StructuredOutputMode::Unknown, "unknown"),
        ] {
            let msg = LlmConfigError::StructuredOutputUnsupported {
                model_id: TEST_MODEL.to_string(),
                mode,
            }
            .to_string();
            assert!(msg.contains(label) && msg.contains(TEST_MODEL));
        }

        assert!(LlmConfigError::UnknownTemperatureMode {
            model_id: TEST_MODEL.to_string(),
            token: "scorching".to_string(),
        }
        .to_string()
        .contains("scorching"));

        assert!(LlmConfigError::UnknownStructuredOutputMode {
            model_id: TEST_MODEL.to_string(),
            token: "magic".to_string(),
        }
        .to_string()
        .contains("magic"));

        let neg = LlmConfigError::NegativeTokenValue {
            model_id: TEST_MODEL.to_string(),
            column: "max_output_tokens",
            value: -1,
        }
        .to_string();
        assert!(neg.contains("max_output_tokens") && neg.contains("-1"));

        // Chunk B's reserved variant: constructed here purely to pin its Display,
        // since Chunk A never raises it.
        let ctx = LlmConfigError::PromptExceedsContext {
            model_id: TEST_MODEL.to_string(),
            prompt_tokens: 250_000,
            max_context_tokens: 200_000,
        }
        .to_string();
        assert!(ctx.contains("250000") && ctx.contains("200000") && ctx.contains(TEST_MODEL));
    }

    #[test]
    fn structured_output_label_is_stable() {
        // The StructuredOutputUnsupported error message embeds these labels; pin
        // every one so a rename can't silently change operator-facing text.
        assert_eq!(StructuredOutputMode::Native.label(), "native");
        assert_eq!(StructuredOutputMode::Guided.label(), "guided");
        assert_eq!(StructuredOutputMode::None_.label(), "none");
        assert_eq!(StructuredOutputMode::Unknown.label(), "unknown");
    }
}
