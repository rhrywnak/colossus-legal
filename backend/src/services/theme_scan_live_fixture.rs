//! The FIXTURE the live-database scan proofs share: a stub judge, a seeded
//! scenario and run, and the three reads the assertions make.
//!
//! Split from `theme_scan_judge_live_tests.rs` when that file crossed the
//! 300-line module limit. The seam is the ordinary one for a test suite — what a
//! test SETS UP versus what it CLAIMS — and it keeps the four proofs readable as
//! four paragraphs rather than four paragraphs interleaved with a stub provider.
//!
//! `pub(super)` throughout: the only consumer is the sibling test module, and
//! `#[cfg(test)]` on the `mod` declaration keeps all of it out of a release
//! build.
//!
//! Every test here is `#[ignore]` because it needs a real `colossus_legal_v2`
//! PostgreSQL database — there is no `#[sqlx::test]` fixture infrastructure in
//! this project, so CI does not run them (the same convention
//! `repositories::pipeline_repository::config_overrides` uses for its live-DB
//! block, and `tests/scan_run_history_integration.rs` for its whole file).
//!
//! They live INSIDE the library rather than in `tests/` for one mechanical
//! reason: `judge_all` is `pub(crate)`, so an integration test — a separate
//! crate — cannot call it. What is being proved is the loop's own behaviour, so
//! the test has to be where the loop is.
//!
//! Run them against a live pipeline database with:
//!
//! ```text
//! cargo test --lib theme_scan_judge_live -- --ignored --test-threads=1
//! ```
//!
//! ## They must not be pointed at DEV's real database
//!
//! Each test writes a scenario, a `scan_runs` row and verdict rows, and deletes
//! the scenario afterwards (the run and its verdicts cascade). That is safe
//! against a scratch copy of the schema and is NOT something to run against
//! `colossus_legal_v2` itself — point `PIPELINE_DATABASE_URL` at a throwaway
//! database schema-copied from it. The slug every test uses is prefixed
//! `__test_` for the same reason the history suite's is.
//!
//! No LLM is called. The provider is a stub that returns canned verdict JSON, so
//! these cost nothing and take milliseconds.

use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use colossus_extract::{LlmProvider, LlmResponse, PipelineError};
use serde_json::json;
use sqlx::PgPool;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::bias::dto::BiasInstance;
use crate::config::AppConfig;
use crate::domain::llm_params::ResolvedLlmParams;
use crate::llm_retry_policy::LlmRetryPolicy;
use crate::repositories::pipeline_repository::{
    delete_scenario, insert_scan_run_stub, insert_scenario, promote_scan_run_running,
    PromoteOutcome, ScanRunStart, ScanRunStub,
};
use crate::services::theme_scan_judge::{judge_all, JudgeInputs, JudgeRun};
use crate::services::theme_scan_prefilter::CandidateGroup;

pub(super) type TestResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// The real matter's slug with a per-test suffix, so the destructive cleanup can
/// only ever reach rows this test wrote (the history suite's convention).
pub(super) fn test_slug(tag: &str) -> String {
    format!("awad_v_catholic_family_service__test_{tag}")
}

/// Connect to the live pipeline database from the environment.
pub(super) async fn pipeline_pool() -> TestResult<PgPool> {
    // best-effort: a missing .env is normal when the URL comes from the shell
    // (which is how a scratch database is pointed at); the connect below fails
    // loudly if it is unset either way.
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env()?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&config.pipeline_database_url)
        .await?;
    Ok(pool)
}

/// A stub judge. Returns the same admitted verdict for every candidate, and —
/// when `cancel_after` is set — pulls the stop cord once it has answered that
/// many calls.
///
/// ## Why the provider is what cancels
///
/// A stop has to land WHILE the loop is running, and a test that cancelled from
/// the outside would be racing a loop that finishes in microseconds. Cancelling
/// from inside the Nth call makes the timing exact: N groups are judged, and
/// every group the loop had not yet started is skipped.
pub(super) struct StubJudge {
    calls: Mutex<usize>,
    cancel_after: Option<(usize, CancellationToken)>,
}

impl StubJudge {
    pub(super) fn new() -> Self {
        Self {
            calls: Mutex::new(0),
            cancel_after: None,
        }
    }

    pub(super) fn cancelling_after(n: usize, token: CancellationToken) -> Self {
        Self {
            calls: Mutex::new(0),
            cancel_after: Some((n, token)),
        }
    }

    pub(super) fn calls(&self) -> usize {
        *self.calls.lock().expect("test mutex")
    }
}

#[async_trait]
impl LlmProvider for StubJudge {
    async fn invoke(&self, _prompt: &str, _max_tokens: u32) -> Result<LlmResponse, PipelineError> {
        let seen = {
            let mut calls = self.calls.lock().expect("test mutex");
            *calls += 1;
            *calls
        };
        if let Some((after, token)) = &self.cancel_after {
            if seen >= *after {
                token.cancel();
            }
        }
        Ok(LlmResponse {
            text: json!({
                "relevant": true,
                "proposed_role": "supports",
                "reason": "stub verdict",
                "confidence": 0.9
            })
            .to_string(),
            input_tokens: Some(10),
            output_tokens: Some(5),
        })
    }

    async fn invoke_with_system(
        &self,
        _system: &str,
        prompt: &str,
        max_tokens: u32,
    ) -> Result<LlmResponse, PipelineError> {
        self.invoke(prompt, max_tokens).await
    }

    fn provider_name(&self) -> &str {
        "stub"
    }
    fn model_name(&self) -> &str {
        "stub-judge"
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

pub(super) fn group(node_id: &str) -> CandidateGroup {
    CandidateGroup {
        representative: BiasInstance {
            evidence_id: node_id.to_string(),
            title: String::new(),
            verbatim_quote: Some("a quote".to_string()),
            question: None,
            statement_type: None,
            page_number: None,
            pattern_tags: Vec::new(),
            stated_by: None,
            about: Vec::new(),
            document: None,
        },
        members: vec![node_id.to_string()],
    }
}

pub(super) fn params() -> ResolvedLlmParams {
    ResolvedLlmParams {
        temperature: Some(0.0),
        timeout_secs: 30,
        max_tokens: 512,
    }
}

/// A scenario and a `running` run under it, through the real two-step start path.
pub(super) async fn seed_running_run(
    pool: &PgPool,
    tag: &str,
    total: i32,
) -> TestResult<(Uuid, Uuid)> {
    let (scenario_id, _code) = insert_scenario(
        pool,
        &format!("scan server state {tag}"),
        "offense",
        "draft",
        &test_slug(tag),
        None,
        None,
        &json!({ "schema_v": 1 }),
    )
    .await?;

    let run_id = Uuid::new_v4();
    insert_scan_run_stub(
        pool,
        &ScanRunStub {
            run_id,
            scenario_id,
            requested_model_id: Some("stub-judge".to_string()),
            started_at: chrono::Utc::now(),
        },
    )
    .await?;
    let promoted = promote_scan_run_running(
        pool,
        &ScanRunStart {
            run_id,
            model_id: "stub-judge".to_string(),
            resolved_params: json!({ "max_tokens": 512 }),
            candidates_total: total,
            candidates_read: total,
        },
    )
    .await?;
    assert_eq!(
        promoted,
        PromoteOutcome::Promoted,
        "the stub row just written must be promotable to running"
    );
    Ok((scenario_id, run_id))
}

/// Delete the test's own scenario, taking its runs and verdicts with it.
///
/// `delete_scenario` is scoped by BOTH the id and the case slug, which is the
/// safety this suite depends on: even a mis-typed id cannot reach a row outside
/// the `__test_<tag>` slug this test wrote.
pub(super) async fn cleanup(pool: &PgPool, scenario_id: Uuid, tag: &str) -> TestResult<()> {
    let deleted = delete_scenario(pool, scenario_id, &test_slug(tag)).await?;
    assert_eq!(deleted, 1, "the test's own scenario must be cleaned up");
    Ok(())
}

/// Judge `n` synthetic groups against `run_id`, through the real loop.
///
/// The three proofs all set up the same way — N groups, a stub judge, the shared
/// semaphore — and differ only in what they then assert. Naming the setup once
/// keeps each test readable as its claim rather than as its scaffolding, and keeps
/// each within the function-size limit.
///
/// `concurrency` is a parameter because it is load-bearing in exactly one test: at
/// 1, the cancel proof's arithmetic is exact (the stub stops the token from inside
/// its Nth call, so N are judged and the rest are never started).
pub(super) async fn judge_n(
    pool: &PgPool,
    run_id: Uuid,
    prefix: &str,
    n: usize,
    concurrency: usize,
    provider: Arc<dyn LlmProvider>,
    cancel: CancellationToken,
) -> JudgeRun {
    let groups: Vec<CandidateGroup> = (1..=n).map(|i| group(&format!("{prefix}-{i}"))).collect();
    judge_all(
        JudgeInputs {
            provider,
            semaphore: Arc::new(Semaphore::new(concurrency)),
            concurrency,
            scan_prompt: Arc::from("system"),
            scan_criteria: Arc::from("criteria"),
            params: params(),
            pool: pool.clone(),
            run_id,
            policy: LlmRetryPolicy::default(),
            cancel,
        },
        groups,
    )
    .await
}

pub(super) async fn verdict_count(pool: &PgPool, run_id: Uuid) -> TestResult<i64> {
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM scan_run_verdicts WHERE run_id = $1")
        .bind(run_id)
        .fetch_one(pool)
        .await?;
    Ok(n)
}

/// `(status, candidates_judged, relevant_count, error, summary present)`.
pub(super) async fn run_state(
    pool: &PgPool,
    run_id: Uuid,
) -> TestResult<(String, i32, i32, Option<String>, bool)> {
    let row: (String, i32, i32, Option<String>, Option<serde_json::Value>) = sqlx::query_as(
        "SELECT status, candidates_judged, relevant_count, error, summary_json \
         FROM scan_runs WHERE run_id = $1",
    )
    .bind(run_id)
    .fetch_one(pool)
    .await?;
    Ok((row.0, row.1, row.2, row.3, row.4.is_some()))
}
