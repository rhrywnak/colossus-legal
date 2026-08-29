use crate::api::pipeline::RestatePurgePolicy;
use crate::domain::llm_effort::{LlmEffortPolicy, DEFAULT_EXTRACTION_EFFORT};
use crate::domain::quote_gap::GapPolicy;
use crate::llm_retry_policy::LlmRetryPolicy;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub neo4j_uri: String,
    pub neo4j_user: String,
    pub neo4j_password: String,
    pub document_storage_path: String,
    pub qdrant_url: String,
    pub fastembed_cache_path: String,
    /// Anthropic API key — None means /ask returns 503 but the rest of the app works.
    pub anthropic_api_key: Option<String>,
    /// Anthropic model id for the Chat / RAG synthesis endpoint.
    /// Read from `ANTHROPIC_MODEL` at startup — required (no
    /// in-binary default).
    pub anthropic_model: String,
    /// Minimum cosine similarity for reranking graph-expanded nodes.
    /// Graph-expanded chunks below this threshold are filtered out.
    /// Default: 0.3 (conservative — keeps most chunks).
    pub rerank_threshold: f32,
    /// Anthropic model id for query decomposition (typically a fast
    /// model). Read from `DECOMPOSER_MODEL` at startup — required (no
    /// in-binary default).
    pub decomposer_model: String,
    /// PostgreSQL connection URL for analytical data (ratings, feedback).
    pub postgres_url: String,
    /// Directory containing prompt template files (synthesis.md,
    /// decomposition.md). Read from `PROMPTS_DIR` at startup —
    /// required (no in-binary default).
    pub prompts_dir: PathBuf,
    /// PostgreSQL connection URL for the pipeline v2 database (clean room).
    /// Separate from postgres_url which connects to colossus_legal.
    pub pipeline_database_url: String,
    /// Path to extraction schema YAML files directory.
    pub extraction_schema_dir: String,
    /// Path to extraction prompt template files directory.
    pub extraction_template_dir: String,
    /// Path to extraction config files directory (models.yaml, etc.).
    pub extraction_config_dir: String,
    /// Path to the processing-profile YAML directory.
    /// Consumed by the pipeline (`AppContext.profile_dir`) and by admin
    /// handlers that need to check profile references (e.g. DELETE /models).
    pub processing_profile_dir: String,
    /// Path to the system-prompt directory.
    pub system_prompt_dir: String,
    /// Deployment environment name (e.g. "dev", "prod").
    /// Read from COLOSSUS_ENVIRONMENT, defaults to "unknown".
    pub environment: String,
    /// Restate admin endpoint base URL (e.g. `http://10.10.100.220:9070`
    /// on DEV).
    ///
    /// Used by the dual-cancel handler in
    /// [`crate::api::pipeline::cancel::cancel_handler`] to call Restate's
    /// `DELETE /invocations/DocumentPipeline/{doc_id}?mode=cancel`
    /// endpoint. `None` is a first-class state: when the env var is unset
    /// the Restate-side cancel is silently skipped, so the Cancel button
    /// continues to work during the transition period (when only the
    /// legacy Worker has cancel coverage). Once Restate cancel is fully
    /// rolled out, deployments will set `RESTATE_ADMIN_URL` and the
    /// silent-skip branch becomes unreachable in practice.
    ///
    /// We deliberately do not hardcode a fallback URL here — case-specific
    /// infrastructure addresses live in configuration, never in code
    /// (Standing Rule 2).
    pub restate_admin_url: Option<String>,
    /// Which `output_config.effort` each LLM call family sends, from
    /// `LLM_EXTRACTION_EFFORT` and `LLM_SCAN_EFFORT`.
    ///
    /// Extraction is turned down to `low` by default; scans send no field at all
    /// unless asked to. See [`crate::domain::llm_effort`] for the 727-second
    /// all-reasoning-blocks incident this answers, and for why the two families
    /// get different treatment. Carried on the config for the same reason
    /// `llm_retry_policy` is: one startup read, so the services and the pipeline
    /// cannot come to disagree.
    pub llm_effort_policy: LlmEffortPolicy,

    /// The two automatic-retry caps, from `LLM_RETRY_MAX` and
    /// `LLM_RATE_LIMIT_RETRY_MAX`.
    ///
    /// Zero and five by ruling (2026-08-28): a failed LLM call marks the step
    /// failed and waits for a human, EXCEPT a pre-generation rejection (HTTP
    /// 429 / 529), which billed nothing and is retried on its own budget.
    /// Carried on the config rather than read at each call site so the
    /// extraction pipeline, the Theme Scan, and the practice reader cannot come
    /// to disagree about the policy, and so the Restate terminal-vs-retryable
    /// classification reads the same numbers the retry loop does. See
    /// [`crate::llm_retry_policy`] for why the defaults are what they are.
    pub llm_retry_policy: LlmRetryPolicy,

    /// Thresholds for the verifier's second-chance (one-gap) match.
    ///
    /// `VERIFY_MAX_GAP_CHARS` (default 240) · `VERIFY_MIN_HALF_FRACTION`
    /// (default 0.05) · `VERIFY_MIN_HALF_WORDS` (default 3).
    ///
    /// Configuration and not literals because these are thresholds a human
    /// tunes against real documents. Measured on the Phillips default motion,
    /// the six footnote-interrupted quotes needed gaps of 44–169 characters.
    ///
    /// The fraction was ruled on 2026-08-17. The obvious value is 0.40, and it
    /// was the first proposal; measured against those same six failures it
    /// recovers 2, because their gaps fall near an end (short halves of 5%,
    /// 29%, 16%, 6%). 0.05 recovers 5. Nothing is weakened by the change: the
    /// two halves ARE the whole quote, so a match still means every word is
    /// present, in order, separated by one gap no larger than the maximum
    /// above. Setting `VERIFY_MIN_HALF_FRACTION=0.40` reverts it with no code
    /// change and no rebuild.
    ///
    /// The absolute word floor is what refuses item 9402, whose "head" is the
    /// single word "For" — a common word matching by coincidence 99 characters
    /// before the rest of the quote. A fraction alone cannot express that: 1
    /// word of 22 is 4.5%, and so is 3 words of 66.
    pub verify_gap_policy: GapPolicy,

    /// Restate ingress endpoint base URL (e.g. `http://10.10.100.220:8080`
    /// on DEV).
    ///
    /// Used by the Process Document handler in
    /// [`crate::api::pipeline::process::process_handler`] to invoke the
    /// `DocumentPipeline` workflow via Restate's ingress API. Unlike
    /// [`Self::restate_admin_url`] (which silently skips when unset),
    /// the process handler **requires** this value — when `None` the
    /// handler returns HTTP 503 Service Unavailable. Restate-driven
    /// document processing is the only supported processing path; a
    /// missing ingress URL means the deployment cannot start new
    /// document processing at all and the operator must fix it.
    ///
    /// We deliberately do not hardcode a fallback URL here — case-specific
    /// infrastructure addresses live in configuration, never in code
    /// (Standing Rule 2).
    pub restate_ingress_url: Option<String>,

    /// How hard DELETE chases a killed Restate invocation before reporting the
    /// manual remedy.
    ///
    /// `RESTATE_PURGE_RETRY_ATTEMPTS` (default 4) ·
    /// `RESTATE_PURGE_RETRY_DELAY_MS` (default 250).
    ///
    /// Configuration rather than constants because the window they cover — how
    /// long Restate takes to apply an asynchronously-accepted kill — is a
    /// property of the deployment, not of the protocol. See
    /// [`crate::api::pipeline::RestatePurgePolicy`].
    pub restate_purge_policy: RestatePurgePolicy,

    /// Optional case-specific subject name to pre-select in the Bias Explorer's
    /// "About" filter on first page render.
    ///
    /// Read from `CASE_DEFAULT_SUBJECT_NAME`. The backend matches this against
    /// the subject list returned to the frontend and surfaces the matching
    /// subject's id as `AvailableFilters.default_subject_id`. The match is
    /// exact (case-sensitive), to avoid surprises when two case-specific
    /// names share a prefix.
    ///
    /// `None` is a first-class state: when the env var is unset we expose
    /// no default at all, the frontend renders "All subjects", and a
    /// `console.warn` notes the absence. We deliberately do not hardcode a
    /// fallback name here — case-specific data lives in configuration, never
    /// in code (Standing Rule 2).
    pub case_default_subject_name: Option<String>,

    /// Case slug identifying which authored (Tier-1) entities the pipeline
    /// works against, read from `CASE_SLUG`.
    ///
    /// Used by the Pass-2 cross-document context loader to scope
    /// `authored_entities` (canonical Elements / LegalCounts) to this
    /// matter — e.g. `awad_v_catholic_family_service`. The pipeline DB has
    /// no document→case mapping, so the current case is a deployment
    /// setting rather than something derivable from a `document_id`.
    ///
    /// `None` is a first-class state: when `CASE_SLUG` is unset, Pass 2
    /// simply loads no authored context (logged, not silent) and behaves
    /// exactly as before this feature. We deliberately do not hardcode a
    /// fallback slug — case-specific data lives in configuration, never in
    /// code (Standing Rule 2).
    pub case_slug: Option<String>,

    /// Anthropic model id the Theme Scan judge (D2b) uses, read from
    /// `THEME_SCAN_MODEL`.
    ///
    /// `None` is a first-class state meaning "use the Chat default model"
    /// (`DEFAULT_CHAT_MODEL`, resolved at startup in `main.rs`). We keep the
    /// scan's model SEPARATE from Chat's rather than hardcoding them equal: the
    /// scan (a deterministic relevance judge) and Chat (a natural-variation
    /// synthesis endpoint) are different jobs, and sharing one id would let a
    /// "tune the chat model" change silently alter scan judgments — the
    /// mixed-provenance hazard. Setting `THEME_SCAN_MODEL` lets them diverge; a
    /// value here overrides the Chat default for the scan only.
    pub theme_scan_model: Option<String>,

    /// Maximum number of Theme Scan LLM verdict calls in flight at once, read
    /// from `THEME_SCAN_CONCURRENCY` (default 4).
    ///
    /// The scan judges every candidate quote with an independent LLM call and
    /// drives them concurrently. This bounds that fan-out via a DEDICATED
    /// semaphore — deliberately NOT the pipeline's `llm_semaphore`, so a running
    /// scan cannot starve document extraction (and vice-versa). The provider's
    /// own rate-limit-retry wrapper absorbs any 429 from the higher combined
    /// concurrency. A magic default is disallowed (Standing Rule 2); the `4`
    /// here is the documented forward-compatible default, overridable per
    /// deployment without a rebuild.
    pub theme_scan_concurrency: usize,
    // RETIRED 2026-08-08 (task 2.15 Tier 2, Roman's amendment): the Theme Scan
    // judging-prompt filename was `theme_scan_prompt_file` here, read from
    // `THEME_SCAN_PROMPT_FILE` with a compiled-in default of
    // `theme_scan_prompt_v2.md`. It is now the `theme_scan_prompt_file` SETTINGS
    // ROW, read from the live snapshot at scan start (see
    // `services::theme_scan_validate::load_scan_prompt`) and asserted at boot in
    // `main.rs`. The env var is no longer read at all — measured on DEV, it was
    // never set, so the compiled default silently decided which prompt judged
    // every scan, which is the invisibility the row removes. No Ansible template
    // change is owed for the same reason: there was nothing to remove there.
}

/// Read an optional numeric env var, failing loudly on a malformed value.
///
/// ## Rust Learning: generic over `T: FromStr`
///
/// The bound says "any type that knows how to parse itself from a string" —
/// `usize` and `f64` both do, so one helper serves both. `T::Err: Display` is
/// the second half of it: without knowing the error type can be printed, the
/// message below could not include *why* the parse failed.
///
/// The three states stay distinct, which is the whole point: unset → the
/// documented default; set and valid → that value; set and invalid → a startup
/// error naming the key and the offending text.
/// The verifier's gap thresholds, from the environment, with
/// [`GapPolicy::default`] filling every value the operator did not set.
///
/// The ONE reader. Both startup paths call it — `AppConfig::from_env` for the
/// HTTP verify path and `AppContext::from_deps_and_env` for the pipeline step —
/// so the two can neither disagree about a default nor disagree about what
/// counts as a malformed value.
pub(crate) fn verify_gap_policy_from_env() -> Result<GapPolicy, String> {
    let d = GapPolicy::default();
    Ok(GapPolicy {
        max_gap_chars: parse_env_or("VERIFY_MAX_GAP_CHARS", d.max_gap_chars)?,
        min_half_fraction: parse_env_or("VERIFY_MIN_HALF_FRACTION", d.min_half_fraction)?,
        min_half_words: parse_env_or("VERIFY_MIN_HALF_WORDS", d.min_half_words)?,
    })
}

/// The automatic-LLM-retry caps, from the environment, defaulting to
/// [`LlmRetryPolicy::default`].
///
/// The ONE reader. Both startup paths call it — `AppConfig::from_env` for the
/// service paths and `AppContext::from_deps_and_env` for the pipeline steps — so
/// the two can neither disagree about the values nor disagree about what counts
/// as a malformed one. A malformed value is a STARTUP error naming the key, not
/// a silent fallback: an operator who typed `LLM_RETRY_MAX=three` believing they
/// had raised the cap must not be quietly left at zero, and one who typed it
/// believing they had lowered it must not be quietly left spending money.
pub(crate) fn llm_retry_policy_from_env() -> Result<LlmRetryPolicy, String> {
    let d = LlmRetryPolicy::default();
    Ok(LlmRetryPolicy {
        max_retries: parse_env_or("LLM_RETRY_MAX", d.max_retries)?,
        rate_limit_max_retries: parse_env_or("LLM_RATE_LIMIT_RETRY_MAX", d.rate_limit_max_retries)?,
    })
}

/// Which effort each LLM call family sends, from the environment.
///
/// The ONE reader. Both startup paths call it — `AppConfig::from_env` for the
/// service paths and `AppContext::from_deps_and_env` for the pipeline steps.
///
/// A malformed value is a STARTUP error naming the key, not a silent fallback.
/// That matters more here than for most tunables: an unrecognised effort string
/// is rejected by the API as an HTTP 400, so the alternative to failing the boot
/// is failing every extraction, one paid request at a time.
pub(crate) fn llm_effort_policy_from_env() -> Result<LlmEffortPolicy, String> {
    let d = LlmEffortPolicy::default();
    Ok(LlmEffortPolicy {
        // Always sent. The default is `low`; an operator can raise it.
        extraction: Some(parse_env_or(
            "LLM_EXTRACTION_EFFORT",
            // `Default` already carries the shipped level; reaching through it
            // rather than naming the constant again keeps ONE place to change.
            d.extraction.unwrap_or(DEFAULT_EXTRACTION_EFFORT),
        )?),
        // Sent ONLY if asked for. Unset means the key is absent from the request
        // body, which is not the same as sending the provider's current default
        // — see the module doc.
        scan: optional_env("LLM_SCAN_EFFORT")?,
    })
}

/// Read an optional env var, failing loudly on a malformed value.
///
/// The sibling of [`parse_env_or`] for a key whose ABSENCE is meaningful rather
/// than a stand-in for a default. Three states, three outcomes, all distinct
/// (Standing Rule 1): unset → `Ok(None)`; set and valid → `Ok(Some(v))`; set and
/// invalid → a startup error naming the key and the offending text.
///
/// ## Rust Learning: why this cannot just be `parse_env_or::<Option<T>>`
///
/// `Option<T>` does not implement `FromStr` — there is no string that parses to
/// `None`, because absence is not a value. The distinction has to live in the
/// reader's control flow, which is what this function is.
pub(crate) fn optional_env<T>(key: &str) -> Result<Option<T>, String>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    // best-effort: env-var-unset → None is the documented "send nothing" path
    // here; a PRESENT value still goes through `parse_present`, so a typo is an
    // error rather than being quietly treated as absence.
    optional_or(std::env::var(key).ok().as_deref(), key)
}

/// The decision [`optional_env`] makes, without touching the environment.
///
/// Split out for the same reason [`parse_or`] is: a test that called `set_var`
/// would race every other test in the binary.
fn optional_or<T>(raw: Option<&str>, key: &str) -> Result<Option<T>, String>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match raw {
        None => Ok(None),
        Some(raw) => parse_present(raw, key).map(Some),
    }
}

pub(crate) fn parse_env_or<T>(key: &str, default: T) -> Result<T, String>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    // best-effort: env-var-unset → None is the documented default path here;
    // `parse_or` below turns None into the default and a malformed value into an
    // error, so the three states stay distinct.
    parse_or(std::env::var(key).ok().as_deref(), key, default)
}

/// The decision [`parse_env_or`] makes, without touching the environment.
///
/// Split out so it is testable: a test that called `set_var` would race every
/// other test in the binary, because the environment is process-global. This
/// takes the raw value as an argument instead, and the one-line wrapper above
/// is the only code that reads the real environment.
fn parse_or<T>(raw: Option<&str>, key: &str, default: T) -> Result<T, String>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match raw {
        None => Ok(default),
        Some(raw) => parse_present(raw, key),
    }
}

/// Parse a value that IS present, or say why it could not be.
///
/// The ONE owner of the malformed-env-var wording, shared by the
/// default-bearing [`parse_or`] and the absence-preserving [`optional_or`]. Two
/// hand-kept copies of this sentence is how the two readers come to report the
/// same mistake differently.
fn parse_present<T>(raw: &str, key: &str) -> Result<T, String>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    raw.trim()
        .parse::<T>()
        .map_err(|e| format!("Invalid env var {key}=\"{raw}\": {e}"))
}

#[cfg(test)]
mod parse_or_tests {
    use super::{optional_or, parse_or};

    #[test]
    fn an_unset_var_takes_the_documented_default() {
        assert_eq!(
            parse_or::<usize>(None, "VERIFY_MAX_GAP_CHARS", 240),
            Ok(240)
        );
        assert_eq!(
            parse_or::<f64>(None, "VERIFY_MIN_HALF_FRACTION", 0.05),
            Ok(0.05)
        );
    }

    #[test]
    fn a_set_var_wins_over_the_default() {
        assert_eq!(
            parse_or::<usize>(Some("400"), "VERIFY_MAX_GAP_CHARS", 240),
            Ok(400)
        );
        // Surrounding whitespace from a .env file is not an operator error.
        assert_eq!(
            parse_or::<f64>(Some(" 0.40 "), "VERIFY_MIN_HALF_FRACTION", 0.05),
            Ok(0.40)
        );
    }

    #[test]
    fn a_malformed_var_is_a_startup_error_naming_the_key_and_the_value() {
        // The letter O typed for a zero — the exact mistake that a silent
        // fallback would hide, leaving the operator convinced they had raised
        // the cap when they had not.
        let err = parse_or::<usize>(Some("24O"), "VERIFY_MAX_GAP_CHARS", 240)
            .expect_err("a malformed value must not fall back to the default");
        assert!(err.contains("VERIFY_MAX_GAP_CHARS"), "message was: {err}");
        assert!(err.contains("24O"), "message was: {err}");
    }

    #[test]
    fn an_optional_var_keeps_absence_distinct_from_a_value() {
        use crate::domain::llm_effort::Effort;

        // The three states `LLM_SCAN_EFFORT` has to hold apart. Unset means "send
        // no effort field at all", which is NOT the same as sending the
        // provider's current default — see `domain::llm_effort`.
        assert_eq!(optional_or::<Effort>(None, "LLM_SCAN_EFFORT"), Ok(None));
        assert_eq!(
            optional_or::<Effort>(Some("max"), "LLM_SCAN_EFFORT"),
            Ok(Some(Effort::Max))
        );

        // And a typo is a startup error, not a fall-through to absence: an
        // unrecognised effort string reaches the API as an HTTP 400, so failing
        // the boot is the cheap version of failing every paid request.
        let err = optional_or::<Effort>(Some("maxx"), "LLM_SCAN_EFFORT")
            .expect_err("a malformed optional value must not degrade to absent");
        assert!(err.contains("LLM_SCAN_EFFORT"), "message was: {err}");
        assert!(err.contains("maxx"), "message was: {err}");
    }

    #[test]
    fn an_empty_var_is_an_error_not_a_default() {
        // `VERIFY_MIN_HALF_WORDS=` in a .env file is a half-finished edit, and
        // it must not look identical to never having written the line.
        assert!(parse_or::<usize>(Some(""), "VERIFY_MIN_HALF_WORDS", 3).is_err());
    }
}

impl AppConfig {
    pub fn from_env() -> Result<Self, String> {
        let neo4j_uri =
            std::env::var("NEO4J_URI").map_err(|_| "Missing env var: NEO4J_URI".to_string())?;
        let neo4j_user =
            std::env::var("NEO4J_USER").map_err(|_| "Missing env var: NEO4J_USER".to_string())?;
        let neo4j_password = std::env::var("NEO4J_PASSWORD")
            .map_err(|_| "Missing env var: NEO4J_PASSWORD".to_string())?;

        let document_storage_path = std::env::var("DOCUMENT_STORAGE_PATH")
            .unwrap_or_else(|_| "./data/documents".to_string());

        // Qdrant vector database URL (used by H.1 embedding pipeline)
        let qdrant_url =
            std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6333".to_string());

        // fastembed model cache directory (ONNX model weights stored here)
        let fastembed_cache_path =
            std::env::var("FASTEMBED_CACHE_PATH").unwrap_or_else(|_| "/data/models".to_string());

        // Anthropic API key — optional so the app starts without it.
        // If absent, POST /ask returns 503 Service Unavailable.
        // best-effort: env-var-unset → None is the documented success path here
        let anthropic_api_key = std::env::var("ANTHROPIC_API_KEY").ok();

        // ANTHROPIC_MODEL is required: the model identifier is a
        // deployment decision (model availability, cost tier) that
        // must be set explicitly per environment. No fallback is
        // hardcoded — the binary refuses to start if the env var is
        // unset, the same way DATABASE_URL etc. do.
        let anthropic_model = std::env::var("ANTHROPIC_MODEL")
            .map_err(|_| "Missing env var: ANTHROPIC_MODEL".to_string())?;

        // Each `.ok()` below is annotated inline with `// best-effort:`
        // so the marker satisfies the same-line-or-immediately-preceding
        // placement requirement for both calls. Distinct from `let _ =
        // ...`: each `.ok()` feeds the next combinator and the final
        // value is captured by `.unwrap_or(0.3)`.
        let rerank_threshold: f32 = std::env::var("RERANK_THRESHOLD")
            .ok() // best-effort: env-var-unset → None → unwrap_or(0.3)
            .and_then(|v| v.parse().ok()) // best-effort: parse-failure → None → unwrap_or(0.3)
            .unwrap_or(0.3);

        // DECOMPOSER_MODEL is required for the same reason as
        // ANTHROPIC_MODEL above — the decomposer reads its own env
        // var so the two model selections can drift independently.
        let decomposer_model = std::env::var("DECOMPOSER_MODEL")
            .map_err(|_| "Missing env var: DECOMPOSER_MODEL".to_string())?;

        let postgres_url = std::env::var("DATABASE_URL")
            .map_err(|_| "Missing env var: DATABASE_URL".to_string())?;

        // PROMPTS_DIR is required: the prompt directory is a
        // deployment-specific filesystem path (container bind-mount on
        // DEV/PROD, repo-relative on local dev). No fallback is
        // hardcoded — the binary refuses to start if the env var is
        // unset, the same way DOCUMENT_STORAGE_PATH would if it were
        // declared without `unwrap_or_else`.
        let prompts_dir = PathBuf::from(
            std::env::var("PROMPTS_DIR").map_err(|_| "Missing env var: PROMPTS_DIR".to_string())?,
        );

        let pipeline_database_url = std::env::var("PIPELINE_DATABASE_URL")
            .map_err(|_| "Missing env var: PIPELINE_DATABASE_URL".to_string())?;

        let extraction_schema_dir = std::env::var("EXTRACTION_SCHEMA_DIR")
            .unwrap_or_else(|_| "./extraction_schemas".to_string());

        let extraction_template_dir = std::env::var("EXTRACTION_TEMPLATE_DIR")
            .unwrap_or_else(|_| "./extraction_templates".to_string());

        let extraction_config_dir =
            std::env::var("EXTRACTION_CONFIG_DIR").unwrap_or_else(|_| "./config".to_string());

        let processing_profile_dir =
            std::env::var("PROCESSING_PROFILE_DIR").unwrap_or_else(|_| "./profiles".to_string());

        let system_prompt_dir =
            std::env::var("SYSTEM_PROMPT_DIR").unwrap_or_else(|_| "./system_prompts".to_string());

        let environment =
            std::env::var("COLOSSUS_ENVIRONMENT").unwrap_or_else(|_| "unknown".to_string());

        // RESTATE_ADMIN_URL is optional. `.ok()` converts "env var
        // unset" → `None`, which the dual-cancel handler treats as
        // "Restate cancel is not configured; skip silently and rely on
        // the legacy path." Once Restate is fully rolled out this env
        // var becomes mandatory in practice, but keeping it optional
        // avoids breaking deployments still on the legacy-only path.
        // Distinct from `let _ = ...`: the value is captured and the
        // call-site handles the `None` arm explicitly (see
        // `cancel::try_restate_cancel`).
        // best-effort: env-var-unset → None is the documented success path here
        let restate_admin_url = std::env::var("RESTATE_ADMIN_URL").ok();

        // The verifier's second-chance thresholds. A value that is PRESENT but
        // unparseable is a startup error, not a silent fall back to the default
        // (Standing Rule 1): an operator who typed `VERIFY_MAX_GAP_CHARS=24O`
        // must be told, not quietly given 240.
        let verify_gap_policy = verify_gap_policy_from_env()?;

        // The automatic-LLM-retry caps. Same treatment and the same reasoning as
        // the thresholds above: a present-but-unparseable value fails startup.
        let llm_retry_policy = llm_retry_policy_from_env()?;

        // Which effort each call family sends. Same treatment and the same
        // reasoning as the caps above: a present-but-unparseable value fails
        // startup rather than reaching the API as a 400.
        let llm_effort_policy = llm_effort_policy_from_env()?;

        // RESTATE_INGRESS_URL is read here as Option<String>; the handler
        // layer (process::process_handler) enforces presence at use time
        // and returns HTTP 503 when None. The read here is intentionally
        // permissive so the binary still starts on a deployment without
        // the env var set — only the Process Document endpoint becomes
        // unavailable, which is preferable to refusing to boot.
        // best-effort: env-var-unset → None is the documented intermediate path here
        let restate_ingress_url = std::env::var("RESTATE_INGRESS_URL").ok();

        // Restate purge/kill retry policy. Like the verifier thresholds above, a
        // PRESENT but unparseable value is a startup error rather than a silent
        // fall back to the default (Standing Rule 1) — an operator who typed
        // `RESTATE_PURGE_RETRY_ATTEMPTS=four` must be told.
        let purge_defaults = RestatePurgePolicy::default();
        let restate_purge_policy = RestatePurgePolicy {
            retry_attempts: parse_env_or(
                "RESTATE_PURGE_RETRY_ATTEMPTS",
                purge_defaults.retry_attempts,
            )?,
            retry_delay: std::time::Duration::from_millis(parse_env_or(
                "RESTATE_PURGE_RETRY_DELAY_MS",
                purge_defaults.retry_delay.as_millis() as u64,
            )?),
        };

        // CASE_DEFAULT_SUBJECT_NAME — optional. We use `.ok()` rather than
        // `unwrap_or` because we need to distinguish "unset" (no default
        // applied, frontend stays at All subjects) from "set to empty
        // string" (which we treat as unset below to keep the wire contract
        // simple — see `Some(name) if !name.trim().is_empty()` in the
        // bias handler).
        // best-effort: env-var-unset → None is the documented success path here
        let case_default_subject_name = std::env::var("CASE_DEFAULT_SUBJECT_NAME").ok();

        // CASE_SLUG — optional, same posture as CASE_DEFAULT_SUBJECT_NAME.
        // `.ok()` maps "unset" → `None`, which the Pass-2 context loader
        // treats as "no authored entity context configured; load none."
        // No hardcoded fallback slug (case-specific data → config only,
        // Standing Rule 2).
        // best-effort: env-var-unset → None is the documented success path here
        let case_slug = std::env::var("CASE_SLUG").ok();

        // THEME_SCAN_MODEL — optional. `.ok()` maps "unset" → `None`, which
        // main.rs resolves to `DEFAULT_CHAT_MODEL` when it builds the scan
        // provider. Kept separate from the Chat model so the two jobs can
        // diverge (see the field doc). No hardcoded model id here — model
        // selection is a deployment decision (Standing Rule 2).
        // best-effort: env-var-unset → None is the documented success path here
        let theme_scan_model = std::env::var("THEME_SCAN_MODEL").ok();

        // THEME_SCAN_CONCURRENCY — optional, defaults to 4. Same best-effort
        // parse shape as RERANK_THRESHOLD above: unset OR unparseable → the
        // documented default. Each `.ok()`/`.and_then` feeds the next
        // combinator and the final value is captured by `.unwrap_or(4)` — this
        // is combinator chaining, NOT a discarded `Result` (Standing Rule 1).
        let theme_scan_concurrency: usize = std::env::var("THEME_SCAN_CONCURRENCY")
            .ok() // best-effort: env-var-unset → None → unwrap_or(4)
            .and_then(|v| v.parse().ok()) // best-effort: parse-failure → None → unwrap_or(4)
            .filter(|&n| n > 0) // a zero cap would deadlock the semaphore — treat as unset
            .unwrap_or(4);

        Ok(Self {
            neo4j_uri,
            neo4j_user,
            neo4j_password,
            document_storage_path,
            qdrant_url,
            fastembed_cache_path,
            anthropic_api_key,
            anthropic_model,
            rerank_threshold,
            decomposer_model,
            postgres_url,
            pipeline_database_url,
            extraction_schema_dir,
            extraction_template_dir,
            extraction_config_dir,
            processing_profile_dir,
            system_prompt_dir,
            prompts_dir,
            environment,
            restate_admin_url,
            llm_retry_policy,
            llm_effort_policy,
            verify_gap_policy,
            restate_ingress_url,
            restate_purge_policy,
            case_default_subject_name,
            case_slug,
            theme_scan_model,
            theme_scan_concurrency,
        })
    }
}

// No `#[cfg(test)] mod tests` here any more. Its four tests all exercised
// `resolve_theme_scan_prompt_file`, which task 2.15 retired along with the env var
// it resolved — the prompt filename is a settings row now, and the store's own
// parse/bounds tests cover it. Deleting them beat keeping tests for a function
// that no longer exists (Rule 6: a test asserts behaviour, and this behaviour has
// moved).
