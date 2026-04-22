//! step_recorder.rs
//!
//! Colossus-legal's implementation of the StepRecorder trait from
//! colossus-pipeline. Wraps the existing record_step_start/complete/failure
//! functions from pipeline_repository::steps.
//!
//! The pipeline framework calls these automatically around each step
//! execution — individual steps no longer manage their own lifecycle
//! recording. This eliminates the error-path recording gaps identified
//! in PIPELINE_CODEBASE_AUDIT.md Section 6.
//!
//! ## Rust Learning: newtype wrapper for trait implementation
//!
//! PgStepRecorder wraps a PgPool and implements the StepRecorder trait.
//! This is the "newtype pattern" — creating a new type to carry a trait
//! implementation for an existing type. We can't impl StepRecorder
//! directly on PgPool because both the trait and PgPool are defined in
//! external crates (Rust's orphan rule forbids this).

use async_trait::async_trait;
use colossus_pipeline::StepRecorder;
use sqlx::PgPool;
use uuid::Uuid;

use crate::repositories::pipeline_repository::steps;

/// PostgreSQL-backed step recorder for colossus-legal.
///
/// Delegates to the existing `record_step_start`, `record_step_complete`,
/// and `record_step_failure` functions in pipeline_repository::steps.
pub struct PgStepRecorder {
    pool: PgPool,
}

impl PgStepRecorder {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl StepRecorder for PgStepRecorder {
    async fn on_step_start(
        &self,
        _job_id: Uuid,
        job_key: &str,
        step_name: &str,
    ) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
        let step_id = steps::record_step_start(
            &self.pool,
            job_key,
            step_name,
            "worker",
            &serde_json::json!({}),
        )
        .await?;
        Ok(step_id as i64)
    }

    async fn on_step_success(
        &self,
        step_handle: i64,
        duration_secs: f64,
        result_summary: &serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        steps::record_step_complete(
            &self.pool,
            step_handle as i32,
            duration_secs,
            result_summary,
        )
        .await?;
        Ok(())
    }

    async fn on_step_failure(
        &self,
        step_handle: i64,
        duration_secs: f64,
        error_message: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        steps::record_step_failure(
            &self.pool,
            step_handle as i32,
            duration_secs,
            error_message,
        )
        .await?;
        Ok(())
    }
}
