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

        // RESTATE_INGRESS_URL is read here as Option<String>; the handler
        // layer (process::process_handler) enforces presence at use time
        // and returns HTTP 503 when None. The read here is intentionally
        // permissive so the binary still starts on a deployment without
        // the env var set — only the Process Document endpoint becomes
        // unavailable, which is preferable to refusing to boot.
        // best-effort: env-var-unset → None is the documented intermediate path here
        let restate_ingress_url = std::env::var("RESTATE_INGRESS_URL").ok();

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
            restate_ingress_url,
            case_default_subject_name,
            case_slug,
            theme_scan_model,
            theme_scan_concurrency,
        })
    }
}
