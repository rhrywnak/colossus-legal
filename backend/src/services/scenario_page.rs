// =============================================================================
// backend/src/services/scenario_page.rs
// =============================================================================
//
// Scenario page-assembly + a stubbed no-op cache-aside seam (task 0.4).
//
// Two collaborators, each behind a trait so the caller depends on behavior, not
// a concrete type (Standing Rule 10):
//
//   ScenarioPageComposer — "produce a page from params". The real
//       ScenarioPageAssembler calls the three 0.3 ScenarioRepository methods and
//       assembles their results. Abstracting it lets the cache stay agnostic to
//       WHAT it composes, and lets the no-op be unit-tested with a fake composer
//       (no live graph needed).
//
//   ScenarioPageCache — the cache-aside contract the eventual caller asks:
//       get-or-compose (check store; on miss compose live; store; return). The
//       only implementation today is NoOpScenarioPageCache — always a miss,
//       always composes live, stores nothing.
//
// Graph-only for now: the page composes ONLY the three graph traversals from
// 0.3. The Postgres-backed sections (scenario definition, confirmed-reference
// set, authored responses) do not exist yet (Phase 1) and are NOT reached here.
// =============================================================================

use std::sync::Arc;

use async_trait::async_trait;
use neo4rs::Graph;

use crate::dto::scenario::{ScenarioPage, ScenarioPageParams};
use crate::repositories::scenario_repository::{ScenarioRepository, ScenarioRepositoryError};

// ─────────────────────────────────────────────────────────────────────────────
// Error
// ─────────────────────────────────────────────────────────────────────────────

/// Error surface for page assembly.
///
/// ## Why a page-level error instead of reusing `ScenarioRepositoryError`
///
/// Today the assembly only calls the graph repository, so reusing its error
/// would compile. But the page is explicitly designed to gain Postgres-backed
/// sections later (scenario definition / confirmed-set / responses, Phase 1).
/// A page-level error that `#[from]`-wraps `ScenarioRepositoryError` now means
/// those future sections add a `Pg(sqlx::Error)` variant WITHOUT reshaping the
/// composer/cache signatures — the same additive discipline the page DTO
/// follows. `#[from]` keeps `?` transparent; `thiserror` keeps the failure
/// observable with `{}` (Standing Rule 1).
#[derive(Debug, thiserror::Error)]
pub enum ScenarioPageError {
    /// A graph traversal underneath the page failed. Carries the repository
    /// error verbatim so the offending column / Neo4j cause is preserved.
    #[error("scenario repository failed: {0}")]
    Repository(#[from] ScenarioRepositoryError),
}

// ─────────────────────────────────────────────────────────────────────────────
// Composer trait + real assembler
// ─────────────────────────────────────────────────────────────────────────────

/// Produces a `ScenarioPage` from its identity parameters.
///
/// ## Rust Learning: `#[async_trait]` for a `dyn`-held async trait
///
/// Native async-in-trait is not yet object-safe, so to hold this behind an
/// `Arc<dyn ScenarioPageComposer>` (the way the cache injects it) we use
/// `#[async_trait]`, which desugars the `async fn` to a boxed future. This is
/// the established backend idiom (`ExtractionEngine`, `LlmProvider`).
#[async_trait]
pub trait ScenarioPageComposer: Send + Sync {
    /// Compose the page live from the graph.
    async fn compose(&self, params: &ScenarioPageParams)
        -> Result<ScenarioPage, ScenarioPageError>;
}

/// The real, graph-backed page composer.
///
/// Holds a `ScenarioRepository` (which is a cheap `Clone` over the Neo4j
/// connection pool). Build it from `state.graph.clone()` at the call site, the
/// same way handlers construct repositories elsewhere.
#[derive(Clone)]
pub struct ScenarioPageAssembler {
    repo: ScenarioRepository,
}

impl ScenarioPageAssembler {
    /// Construct an assembler over a shared Neo4j connection.
    pub fn new(repo: ScenarioRepository) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ScenarioPageComposer for ScenarioPageAssembler {
    /// Call the three 0.3 traversals and assemble the graph-only page.
    ///
    /// `?` propagates any `ScenarioRepositoryError` into `ScenarioPageError`
    /// via the `#[from]` impl — no failure is swallowed. The three calls run
    /// sequentially; the graph is tiny (design v2 §3) so this is already fast,
    /// and sequential keeps the error attribution simple (the first failing
    /// section is the one reported).
    async fn compose(
        &self,
        params: &ScenarioPageParams,
    ) -> Result<ScenarioPage, ScenarioPageError> {
        let rebuttal_facts = self.repo.rebuttal_facts(&params.wielder_id).await?;
        let contradictions = self
            .repo
            .contradictions_against_wielder(&params.wielder_id)
            .await?;
        let related_allegations = self.repo.related_allegations(&params.anchor_id).await?;

        Ok(ScenarioPage {
            rebuttal_facts,
            contradictions,
            related_allegations,
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Cache-aside trait + no-op impl
// ─────────────────────────────────────────────────────────────────────────────

/// Cache-aside contract for scenario pages: get-or-compose.
///
/// The caller depends on THIS trait, never on a concrete cache (Rule 10), so a
/// real in-process memory cache can be swapped in later with zero calling-code
/// change. "Refresh" and "open" are the same operation through this seam — both
/// call `get_or_compose`; with the no-op both always compose live.
#[async_trait]
pub trait ScenarioPageCache: Send + Sync {
    /// Return the page for `params`, composing it live on a cache miss.
    async fn get_or_compose(
        &self,
        params: &ScenarioPageParams,
    ) -> Result<ScenarioPage, ScenarioPageError>;
}

/// The only cache implementation today: a no-op that always misses.
///
/// Holds the composer it delegates to — injected as a trait object so the cache
/// does not hardcode what it composes (and so tests can inject a fake).
pub struct NoOpScenarioPageCache {
    composer: Arc<dyn ScenarioPageComposer>,
}

impl NoOpScenarioPageCache {
    /// Wrap a composer in the no-op cache.
    pub fn new(composer: Arc<dyn ScenarioPageComposer>) -> Self {
        Self { composer }
    }
}

#[async_trait]
impl ScenarioPageCache for NoOpScenarioPageCache {
    /// Always a miss: compose live every call, store nothing, return the fresh
    /// page.
    ///
    /// Why: design v2 §3 — the graph is tiny (~250 Evidence nodes; the whole DB
    /// fits in Neo4j's own RAM page cache), so live composition is already fast.
    /// A real cache now would be premature optimization plus an invalidation
    /// burden. We build the SEAM (this trait) so a real in-process memory cache
    /// can drop in later with zero calling-code change THE DAY measurement
    /// proves a page is genuinely slow — not before. Do not "finish" or
    /// "optimize" this stub into a real cache without that measurement.
    async fn get_or_compose(
        &self,
        params: &ScenarioPageParams,
    ) -> Result<ScenarioPage, ScenarioPageError> {
        // Miss is unconditional — there is no store to consult.
        self.composer.compose(params).await
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Wiring
// ─────────────────────────────────────────────────────────────────────────────

/// Build the scenario-page cache seam over a Neo4j connection.
///
/// Wires `NoOp( Assembler( ScenarioRepository ) )` and returns it as the cache
/// trait object, so a future Phase-5 handler asks the cache for a page in one
/// call and depends only on `ScenarioPageCache` — never on the assembler or the
/// no-op concretely. Pass `state.graph.clone()`.
pub fn build_scenario_page_cache(graph: Graph) -> Arc<dyn ScenarioPageCache> {
    let assembler = ScenarioPageAssembler::new(ScenarioRepository::new(graph));
    Arc::new(NoOpScenarioPageCache::new(Arc::new(assembler)))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests — the cache seam (the assembler's own compose needs a live graph and is
// integration-tested, like the 0.3 query methods)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::dto::scenario::{
        ScenarioContradictionsResponse, ScenarioRebuttalFactsResponse,
        ScenarioRelatedAllegationsResponse,
    };

    /// A composer test double that counts how many times it composed, so we can
    /// prove the no-op runs the composition on every call.
    struct CountingComposer {
        calls: Arc<AtomicUsize>,
        fail: bool,
    }

    #[async_trait]
    impl ScenarioPageComposer for CountingComposer {
        async fn compose(
            &self,
            params: &ScenarioPageParams,
        ) -> Result<ScenarioPage, ScenarioPageError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                // Surface a real repository error through the page error so the
                // propagation path (not a fabricated variant) is exercised.
                return Err(ScenarioRepositoryError::MissingRequired {
                    column: "evidence_id".to_string(),
                }
                .into());
            }
            Ok(empty_page(params))
        }
    }

    /// An empty-but-well-formed page echoing the requested ids.
    fn empty_page(params: &ScenarioPageParams) -> ScenarioPage {
        ScenarioPage {
            rebuttal_facts: ScenarioRebuttalFactsResponse {
                wielder_id: params.wielder_id.clone(),
                facts: Vec::new(),
            },
            contradictions: ScenarioContradictionsResponse {
                anchor_id: params.wielder_id.clone(),
                contradictions: Vec::new(),
                total: 0,
            },
            related_allegations: ScenarioRelatedAllegationsResponse {
                anchor_id: params.anchor_id.clone(),
                allegations: Vec::new(),
            },
        }
    }

    fn sample_params() -> ScenarioPageParams {
        ScenarioPageParams {
            wielder_id: "doc-x:person:abc".to_string(),
            anchor_id: "doc-x:evidence:def".to_string(),
        }
    }

    #[tokio::test]
    async fn noop_composes_live_on_every_call() {
        let calls = Arc::new(AtomicUsize::new(0));
        let composer = Arc::new(CountingComposer {
            calls: Arc::clone(&calls),
            fail: false,
        });
        let cache = NoOpScenarioPageCache::new(composer);
        let params = sample_params();

        // Two opens (or open + refresh — identical through the no-op) must BOTH
        // compose live: the stub never serves a stored page.
        let _ = cache.get_or_compose(&params).await.expect("first compose");
        let _ = cache.get_or_compose(&params).await.expect("second compose");

        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn noop_returns_assembled_sections() {
        let calls = Arc::new(AtomicUsize::new(0));
        let composer = Arc::new(CountingComposer { calls, fail: false });
        let cache = NoOpScenarioPageCache::new(composer);
        let params = sample_params();

        let page = cache.get_or_compose(&params).await.expect("compose");

        // The three sections are present and carry the requested ids through.
        assert_eq!(page.rebuttal_facts.wielder_id, params.wielder_id);
        assert_eq!(page.related_allegations.anchor_id, params.anchor_id);
        assert_eq!(page.contradictions.total, 0);
    }

    #[tokio::test]
    async fn noop_propagates_composition_error() {
        let calls = Arc::new(AtomicUsize::new(0));
        let composer = Arc::new(CountingComposer {
            calls: Arc::clone(&calls),
            fail: true,
        });
        let cache = NoOpScenarioPageCache::new(composer);

        let result = cache.get_or_compose(&sample_params()).await;

        // The error is surfaced, not swallowed, and still composed (miss path ran).
        assert!(matches!(
            result,
            Err(ScenarioPageError::Repository(
                ScenarioRepositoryError::MissingRequired { .. }
            ))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
