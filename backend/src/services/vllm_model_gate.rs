//! vLLM `/v1/models` hard gate (LLM Config Chunk B, design §3 hard-gate).
//!
//! Before a scan dispatches ANY candidate to a vLLM model, the backend confirms
//! the endpoint is reachable AND that the model actually loaded there is the one
//! the operator selected. This is a HARD gate, not a warning: for legal work
//! "which model judged this" is load-bearing, and a benchmark run against the
//! wrong loaded model is worse than no run. The Anthropic path SKIPS this entirely
//! (a hosted API has no "which model is loaded" ambiguity).
//!
//! There is no pre-existing `/v1/models` call anywhere in the app — the CURL test
//! proved the ENDPOINT answers, not that the app calls it. This is that call,
//! written fresh with an explicit short timeout (Rule 13: every HTTP call has a
//! timeout; a pre-flight probe must fail fast, before any spend).

use std::time::Duration;

use serde::Deserialize;

/// OpenAI-compatible model-list path (vLLM serves it).
const V1_MODELS_PATH: &str = "/v1/models";

// CONST: a fixed fail-fast timeout for the pre-flight probe — deliberately NOT a
// per-deployment tunable. The vLLM endpoint is LAN-only (design §5.10 — no-auth,
// same homelab segment), so there is no WAN latency to accommodate; the gate's
// whole job is to REFUSE quickly before spending budget on a dead/wrong endpoint,
// and a larger value would only delay that refusal. 5s bounds a hung endpoint
// with generous headroom for a healthy LAN /v1/models call. (Not an env var per
// the Chunk B "no new env vars" scope; flagged for the future central LLM-config
// chunk should a deployment ever need it tunable.)
const GATE_TIMEOUT_SECS: u64 = 5;

/// A gate refusal, independent of any app-specific error type.
///
/// ## Rust Learning: keeping a reusable helper's errors LOCAL
///
/// The probe below is generic OpenAI-compatible logic with zero legal-domain
/// knowledge — another Colossus deployment (colossus-ai) would want it verbatim.
/// Returning a module-local `VllmGateError` (not the app's `ThemeScanError`)
/// keeps it that way: the caller maps this into its own taxonomy at the boundary,
/// so the helper can move to the shared workspace with only a crate change
/// (reusability checkpoint, rule 11).
#[derive(Debug, thiserror::Error)]
pub(crate) enum VllmGateError {
    /// The endpoint did not answer (network, timeout, non-2xx, unparseable body).
    #[error("vLLM endpoint '{endpoint}' did not answer the model gate: {detail}")]
    Unreachable { endpoint: String, detail: String },
    /// It answered, but the loaded model is not the selected one (names both).
    #[error(
        "vLLM endpoint '{endpoint}' has the wrong model loaded: selected '{selected}' \
         but loaded '{loaded}'"
    )]
    Mismatch {
        endpoint: String,
        selected: String,
        loaded: String,
    },
}

/// The `/v1/models` response shape — only the `data[].id` list is needed.
// serde: allows unknown fields (NO deny_unknown_fields) because the vLLM
// `/v1/models` response carries fields we deliberately ignore (`object`, and
// per-entry `created`, `owned_by`, `permission`, ...). We read only what the gate
// needs; denying unknown fields would make a well-formed response fail to parse.
#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelEntry>,
}

// serde: allows unknown fields — same reason as ModelsResponse; a vLLM model
// entry has many fields beyond `id` and we intentionally read only `id`.
#[derive(Debug, Deserialize)]
struct ModelEntry {
    id: String,
}

/// Poll the vLLM endpoint's `/v1/models` and REFUSE unless the selected model is
/// the one loaded there.
///
/// Two distinct refusals (Standing Rule 1), which the caller maps to HTTP 503:
/// - [`VllmGateError::Unreachable`] — the endpoint did not answer (network,
///   timeout, or non-2xx / unparseable body), naming the endpoint;
/// - [`VllmGateError::Mismatch`] — it answered, but the loaded model id is not the
///   selected one, naming BOTH.
pub(crate) async fn assert_vllm_model_loaded(
    http: &reqwest::Client,
    endpoint: &str,
    selected_model_id: &str,
) -> Result<(), VllmGateError> {
    // trim a trailing slash so `http://host:8000/` + `/v1/models` is well-formed.
    let url = format!("{}{}", endpoint.trim_end_matches('/'), V1_MODELS_PATH);
    let unreachable = |detail: String| VllmGateError::Unreachable {
        endpoint: endpoint.to_string(),
        detail,
    };

    let resp = http
        .get(&url)
        .timeout(Duration::from_secs(GATE_TIMEOUT_SECS))
        .send()
        .await
        .map_err(|e| unreachable(e.to_string()))?;

    let status = resp.status();
    if !status.is_success() {
        return Err(unreachable(format!(
            "GET {V1_MODELS_PATH} returned HTTP {status}"
        )));
    }

    let body: ModelsResponse = resp
        .json()
        .await
        .map_err(|e| unreachable(format!("GET {V1_MODELS_PATH} body did not parse: {e}")))?;

    let loaded: Vec<String> = body.data.into_iter().map(|m| m.id).collect();
    evaluate_loaded(&loaded, selected_model_id, endpoint)
}

/// Pure verdict: is `selected` among the `loaded` model ids? Split out so the
/// match/mismatch decision is unit-testable without a live endpoint.
fn evaluate_loaded(loaded: &[String], selected: &str, endpoint: &str) -> Result<(), VllmGateError> {
    if loaded.iter().any(|id| id == selected) {
        Ok(())
    } else {
        Err(VllmGateError::Mismatch {
            endpoint: endpoint.to_string(),
            selected: selected.to_string(),
            loaded: if loaded.is_empty() {
                "<none>".to_string()
            } else {
                loaded.join(", ")
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_when_selected_is_loaded() {
        let loaded = vec!["qwen-14b".to_string(), "other".to_string()];
        assert!(evaluate_loaded(&loaded, "qwen-14b", "http://x:8000").is_ok());
    }

    #[test]
    fn refuses_naming_both_on_mismatch() {
        let loaded = vec!["qwen-7b".to_string()];
        let err = evaluate_loaded(&loaded, "qwen-14b", "http://x:8000")
            .expect_err("mismatch must refuse");
        let msg = err.to_string();
        assert!(msg.contains("qwen-14b"), "names the selected model: {msg}");
        assert!(msg.contains("qwen-7b"), "names the loaded model: {msg}");
    }

    #[test]
    fn refuses_on_empty_model_list() {
        let err = evaluate_loaded(&[], "qwen-14b", "http://x:8000")
            .expect_err("no loaded model must refuse");
        assert!(err.to_string().contains("qwen-14b"));
    }
}
