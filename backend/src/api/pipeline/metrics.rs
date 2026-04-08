//! GET /api/admin/pipeline/metrics — aggregate pipeline statistics.

use std::collections::HashMap;

use axum::{extract::State, Json};
use serde::Serialize;
use sqlx::PgPool;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::state::AppState;

use super::state_machine::PIPELINE_STAGE_ORDER;

#[derive(Debug, Serialize)]
pub struct MetricsResponse {
    pub total_documents: i64,
    pub documents_by_status: HashMap<String, i64>,
    pub total_cost_usd: f64,
    pub avg_cost_per_document: f64,
    pub avg_grounding_rate: f64,
    pub total_steps_executed: i64,
    pub failed_steps: i64,
    pub step_performance: HashMap<String, StepMetrics>,
    pub estimates: EstimatesResponse,
}

#[derive(Debug, Serialize)]
pub struct EstimatesResponse {
    pub avg_cost_per_document: Option<f64>,
    pub avg_total_duration_per_document_secs: Option<f64>,
    pub documents_remaining: i64,
    pub estimated_remaining_cost_usd: Option<f64>,
    pub estimated_remaining_time_secs: Option<f64>,
    pub confidence: String,
}

#[derive(Debug, Serialize)]
pub struct StepMetrics {
    /// Human-readable label from PIPELINE_STAGE_ORDER
    pub label: String,
    /// Display order (1-8), 0 for unknown steps
    pub order: u8,
    pub count: i64,
    pub avg_duration_secs: f64,
    pub min_duration_secs: f64,
    pub max_duration_secs: f64,
    pub failure_count: i64,
}

/// GET /metrics
pub async fn metrics_handler(
    user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<MetricsResponse>, AppError> {
    require_admin(&user)?;
    let pool = &state.pipeline_pool;

    let documents_by_status = query_documents_by_status(pool).await?;
    let total_documents: i64 = documents_by_status.values().sum();
    let total_cost_usd = query_total_cost(pool).await?;
    let avg_cost = if total_documents > 0 {
        total_cost_usd / total_documents as f64
    } else {
        0.0
    };
    let avg_grounding_rate = query_avg_grounding_rate(pool).await?;
    let (total_steps, failed_steps, step_performance) = query_step_performance(pool).await?;
    let estimates = query_estimates(pool).await?;

    Ok(Json(MetricsResponse {
        total_documents,
        documents_by_status,
        total_cost_usd,
        avg_cost_per_document: avg_cost,
        avg_grounding_rate,
        total_steps_executed: total_steps,
        failed_steps,
        step_performance,
        estimates,
    }))
}

async fn query_documents_by_status(pool: &PgPool) -> Result<HashMap<String, i64>, AppError> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT status, COUNT(*) as count FROM documents GROUP BY status",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal { message: format!("Documents query: {e}") })?;
    Ok(rows.into_iter().collect())
}

async fn query_total_cost(pool: &PgPool) -> Result<f64, AppError> {
    let row: (Option<f64>,) = sqlx::query_as(
        "SELECT COALESCE(SUM(cost_usd::float8), 0.0) FROM extraction_runs WHERE status = 'COMPLETED'",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal { message: format!("Cost query: {e}") })?;
    Ok(row.0.unwrap_or(0.0))
}

async fn query_avg_grounding_rate(pool: &PgPool) -> Result<f64, AppError> {
    let row: (Option<f64>,) = sqlx::query_as(
        "SELECT AVG((result_summary->>'grounding_rate')::float)
         FROM pipeline_steps
         WHERE step_name = 'verify' AND status = 'completed'
           AND result_summary->>'grounding_rate' IS NOT NULL",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal { message: format!("Grounding rate query: {e}") })?;
    Ok(row.0.unwrap_or(0.0))
}

#[derive(sqlx::FromRow)]
struct StepRow {
    step_name: String,
    count: i64,
    avg_duration: Option<f64>,
    min_duration: Option<f64>,
    max_duration: Option<f64>,
    failure_count: i64,
}

async fn query_step_performance(
    pool: &PgPool,
) -> Result<(i64, i64, HashMap<String, StepMetrics>), AppError> {
    let rows: Vec<StepRow> = sqlx::query_as(
        "SELECT step_name,
                COUNT(*) as count,
                AVG(duration_secs) as avg_duration,
                MIN(duration_secs) as min_duration,
                MAX(duration_secs) as max_duration,
                COUNT(*) FILTER (WHERE status = 'failed') as failure_count
         FROM pipeline_steps GROUP BY step_name",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal { message: format!("Step perf query: {e}") })?;

    let mut total: i64 = 0;
    let mut failed: i64 = 0;
    let mut map = HashMap::new();

    for row in rows {
        total += row.count;
        failed += row.failure_count;

        // Look up label and order from the canonical stage definitions
        let (label, order) = PIPELINE_STAGE_ORDER.iter()
            .enumerate()
            .find(|(_, &(name, _))| name == row.step_name)
            .map(|(i, &(_, lbl))| (lbl.to_string(), (i + 1) as u8))
            .unwrap_or_else(|| (row.step_name.clone(), 0));

        map.insert(row.step_name, StepMetrics {
            label,
            order,
            count: row.count,
            avg_duration_secs: row.avg_duration.unwrap_or(0.0),
            min_duration_secs: row.min_duration.unwrap_or(0.0),
            max_duration_secs: row.max_duration.unwrap_or(0.0),
            failure_count: row.failure_count,
        });
    }

    Ok((total, failed, map))
}

async fn query_estimates(pool: &PgPool) -> Result<EstimatesResponse, AppError> {
    // Count published and total documents
    let counts: (i64, i64) = sqlx::query_as(
        "SELECT
            COUNT(*) FILTER (WHERE status = 'PUBLISHED') AS published,
            COUNT(*) AS total
         FROM documents",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal { message: format!("Estimates count query: {e}") })?;

    let (published, total) = counts;
    let remaining = total - published;

    if published == 0 {
        return Ok(EstimatesResponse {
            avg_cost_per_document: None,
            avg_total_duration_per_document_secs: None,
            documents_remaining: remaining,
            estimated_remaining_cost_usd: None,
            estimated_remaining_time_secs: None,
            confidence: "none".to_string(),
        });
    }

    // Avg cost per published document (from extraction_runs)
    let avg_cost: (Option<f64>,) = sqlx::query_as(
        "SELECT AVG(doc_cost) FROM (
            SELECT er.document_id, SUM(er.cost_usd::float8) AS doc_cost
            FROM extraction_runs er
            JOIN documents d ON d.id = er.document_id
            WHERE d.status = 'PUBLISHED' AND er.cost_usd IS NOT NULL
            GROUP BY er.document_id
        ) sub",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal { message: format!("Estimates cost query: {e}") })?;

    // Avg total duration per published document (sum of all step durations)
    let avg_duration: (Option<f64>,) = sqlx::query_as(
        "SELECT AVG(doc_duration) FROM (
            SELECT document_id, SUM(duration_secs) AS doc_duration
            FROM pipeline_steps ps
            JOIN documents d ON d.id = ps.document_id
            WHERE d.status = 'PUBLISHED' AND ps.status = 'completed'
            GROUP BY ps.document_id
        ) sub",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal { message: format!("Estimates duration query: {e}") })?;

    let confidence = match published {
        0 => "none",
        1..=2 => "low",
        3..=5 => "medium",
        _ => "high",
    }
    .to_string();

    Ok(EstimatesResponse {
        avg_cost_per_document: avg_cost.0,
        avg_total_duration_per_document_secs: avg_duration.0,
        documents_remaining: remaining,
        estimated_remaining_cost_usd: avg_cost.0.map(|c| c * remaining as f64),
        estimated_remaining_time_secs: avg_duration.0.map(|d| d * remaining as f64),
        confidence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_stage_order_is_accessible() {
        let extract = PIPELINE_STAGE_ORDER.iter()
            .find(|&&(name, _)| name == "extract_text");
        assert!(extract.is_some());
        assert_eq!(extract.unwrap().1, "Read Document");
    }
}
