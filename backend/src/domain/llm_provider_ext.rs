// =============================================================================
// backend/src/domain/llm_provider_ext.rs — the params-aware call seam for
// `LlmProvider` (LLM Configuration Method, Chunk B, design §7 step 3 / 7b)
// =============================================================================
//
// Chunk B needs every LLM provider to accept a *resolved* parameter set
// ([`ResolvedLlmParams`], produced by Chunk A's `resolve`/`constrain`) at the
// call site — the scan resolves temperature/timeout/max_tokens per run, then
// invokes. The obvious move (add the method to the `LlmProvider` trait) is
// STRUCTURALLY impossible here, and this module is the Rust-idiomatic answer.
//
// ## Rust Learning: extending a trait you cannot edit (extension trait + blanket impl)
//
// The full teaching writeup lives in `COLOSSUS_RUST_LEARNING_LOG_v1.md` §7 — read
// it there, not here. The one-paragraph why: `LlmProvider` is defined in the
// `colossus-extract` git dep, and `ResolvedLlmParams` lives HERE in colossus-legal,
// which *depends on* colossus-extract. A crate can never name a type from a crate
// downstream of it, so a trait method whose signature mentions `ResolvedLlmParams`
// cannot be added to `LlmProvider` without first moving the type down into the dep.
// That is a structural boundary, not a version-bump inconvenience.
//
// The seam: define a LOCAL trait ([`LlmProviderExt`]) — the crate that CAN name
// `ResolvedLlmParams` — and give it a BLANKET impl over every `LlmProvider`. One
// impl, zero per-provider boilerplate, and a provider added tomorrow gets it free.
//
// ## The ceiling (deliberate, deferred to convergence — design §7 step 6)
//
// The delegate can only forward what the existing `invoke(prompt, max_tokens)` /
// `invoke_with_system(system, prompt, max_tokens)` methods accept — so ONLY
// `params.max_tokens` reaches the wire through this seam. `temperature` and
// `timeout_secs` from `ResolvedLlmParams` cannot. That is acceptable for Chunk B:
// temperature is pinned to 0 on both benchmark providers already (determinism is
// the goal, not per-run variation), and timeout is handled at CONSTRUCTION in
// `provider_for_model`, not per-call (Chunk B ruling 2). Full per-invocation
// temperature/timeout on the wire is the convergence step, when the real trait is
// changed and the types move down — NOT here.

use async_trait::async_trait;
use colossus_extract::{LlmProvider, LlmResponse, PipelineError};

use crate::domain::llm_params::ResolvedLlmParams;

/// A params-aware call surface layered over any [`LlmProvider`].
///
/// Callers hold `Arc<dyn LlmProvider>` and bring this trait into scope to gain
/// the `*_with_params` methods. The methods forward to the underlying provider's
/// existing `invoke` / `invoke_with_system`, threading `params.max_tokens`.
///
/// ## Rust Learning: `#[async_trait]` to match the underlying trait
///
/// `LlmProvider` is declared with `#[async_trait]` (async methods desugared to
/// boxed futures so the trait is object-safe behind `Arc<dyn ...>`). This
/// extension trait uses the same macro so its async methods compose with the
/// same `dyn`/`Arc` values — mixing native `async fn` in traits here would not
/// line up with the boxed-future shape the providers already expose.
#[async_trait]
pub trait LlmProviderExt {
    /// Invoke with a single prompt, using a resolved parameter set.
    ///
    /// Threads `params.max_tokens` into the underlying [`LlmProvider::invoke`].
    /// See the module docs for why the other resolved fields do not (yet) reach
    /// the wire.
    ///
    /// # Errors
    ///
    /// Propagates the underlying provider's error taxonomy verbatim
    /// ([`PipelineError::LlmProvider`], [`PipelineError::RateLimited`]).
    async fn invoke_with_params(
        &self,
        prompt: &str,
        params: &ResolvedLlmParams,
    ) -> Result<LlmResponse, PipelineError>;

    /// Invoke with a separate system prompt, using a resolved parameter set.
    ///
    /// Threads `params.max_tokens` into [`LlmProvider::invoke_with_system`], so a
    /// provider with a native system field (Anthropic Messages API, vLLM's
    /// `role: "system"` first message) populates it. The Theme Scan routes
    /// through THIS variant because its judging prompt is a system prompt
    /// (`theme_scan_prompt_v1.md`) — the system/user split must survive.
    ///
    /// # Errors
    ///
    /// Same taxonomy as [`invoke_with_params`](Self::invoke_with_params).
    async fn invoke_with_system_and_params(
        &self,
        system: &str,
        prompt: &str,
        params: &ResolvedLlmParams,
    ) -> Result<LlmResponse, PipelineError>;
}

/// Blanket implementation: EVERY `LlmProvider` is an `LlmProviderExt`.
///
/// ## Rust Learning: `impl<T: LlmProvider + ?Sized> ... for T` — the `?Sized` is load-bearing
///
/// Without `+ ?Sized`, `T` defaults to `Sized`, and a `dyn LlmProvider` (the
/// trait object behind the `Arc<dyn LlmProvider>` the scan actually holds) is
/// `!Sized` — so the blanket impl would silently NOT cover it, and the methods
/// would "not exist" on the `dyn` value. `+ ?Sized` opts the bound out of the
/// implicit `Sized` requirement so the impl covers both concrete providers and
/// trait objects. The orphan rule is satisfied because the *trait*
/// (`LlmProviderExt`) is ours, even though `T` ranges over a foreign trait.
#[async_trait]
impl<T: LlmProvider + ?Sized> LlmProviderExt for T {
    async fn invoke_with_params(
        &self,
        prompt: &str,
        params: &ResolvedLlmParams,
    ) -> Result<LlmResponse, PipelineError> {
        // Delegate: only max_tokens crosses this seam (see module docs).
        self.invoke(prompt, params.max_tokens).await
    }

    async fn invoke_with_system_and_params(
        &self,
        system: &str,
        prompt: &str,
        params: &ResolvedLlmParams,
    ) -> Result<LlmResponse, PipelineError> {
        // Delegate to the native-system-prompt path so the scan's system prompt
        // survives; only max_tokens crosses this seam (see module docs).
        self.invoke_with_system(system, prompt, params.max_tokens)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::llm_params::ResolvedLlmParams;
    use std::sync::Mutex;

    /// A stub provider that records the arguments of the last call so a test can
    /// assert the extension seam threaded the right values. It is NOT a network
    /// client — every method returns a canned [`LlmResponse`].
    #[derive(Default)]
    struct RecordingProvider {
        last: Mutex<Option<Recorded>>,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct Recorded {
        system: Option<String>,
        prompt: String,
        max_tokens: u32,
    }

    #[async_trait]
    impl LlmProvider for RecordingProvider {
        async fn invoke(
            &self,
            prompt: &str,
            max_tokens: u32,
        ) -> Result<LlmResponse, PipelineError> {
            *self.last.lock().expect("test mutex poisoned") = Some(Recorded {
                system: None,
                prompt: prompt.to_string(),
                max_tokens,
            });
            Ok(LlmResponse {
                text: "ok".to_string(),
                input_tokens: None,
                output_tokens: None,
            })
        }

        async fn invoke_with_system(
            &self,
            system: &str,
            prompt: &str,
            max_tokens: u32,
        ) -> Result<LlmResponse, PipelineError> {
            *self.last.lock().expect("test mutex poisoned") = Some(Recorded {
                system: Some(system.to_string()),
                prompt: prompt.to_string(),
                max_tokens,
            });
            Ok(LlmResponse {
                text: "ok".to_string(),
                input_tokens: None,
                output_tokens: None,
            })
        }

        fn provider_name(&self) -> &str {
            "recording"
        }
        fn model_name(&self) -> &str {
            "recording-model"
        }
        fn cost_per_input_token(&self) -> Option<f64> {
            None
        }
        fn cost_per_output_token(&self) -> Option<f64> {
            None
        }
        fn supports_structured_output(&self) -> bool {
            false
        }
    }

    fn params(max_tokens: u32) -> ResolvedLlmParams {
        ResolvedLlmParams {
            temperature: Some(0.0),
            timeout_secs: 600,
            max_tokens,
        }
    }

    #[tokio::test]
    async fn invoke_with_params_threads_max_tokens_no_system() {
        let p = RecordingProvider::default();
        // Call through a `&dyn LlmProvider` to prove the blanket impl covers the
        // trait object (the `?Sized` path), not just the concrete type.
        let dynp: &dyn LlmProvider = &p;
        dynp.invoke_with_params("hello", &params(512))
            .await
            .expect("stub never errors");
        let rec = p.last.lock().unwrap().clone().expect("a call was recorded");
        assert_eq!(rec.system, None, "invoke path must not set a system prompt");
        assert_eq!(rec.prompt, "hello");
        assert_eq!(rec.max_tokens, 512, "params.max_tokens must be threaded");
    }

    #[tokio::test]
    async fn invoke_with_system_and_params_preserves_system_and_max_tokens() {
        let p = RecordingProvider::default();
        let dynp: &dyn LlmProvider = &p;
        dynp.invoke_with_system_and_params("SYSTEM", "user", &params(512))
            .await
            .expect("stub never errors");
        let rec = p.last.lock().unwrap().clone().expect("a call was recorded");
        assert_eq!(
            rec.system.as_deref(),
            Some("SYSTEM"),
            "the system prompt MUST survive the params seam"
        );
        assert_eq!(rec.prompt, "user");
        assert_eq!(rec.max_tokens, 512, "params.max_tokens must be threaded");
    }
}
