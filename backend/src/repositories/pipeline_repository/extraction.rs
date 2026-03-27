//! Extraction-specific repository functions (runs, items, relationships).

use sqlx::PgPool;

use super::PipelineRepoError;

/// Insert an extraction run record. Returns the auto-generated run ID.
/// Initial status is "RUNNING".
pub async fn insert_extraction_run(
    pool: &PgPool,
    document_id: &str,
    pass_number: i32,
    model_name: &str,
    schema_version: &str,
) -> Result<i32, PipelineRepoError> {
    let row = sqlx::query_scalar::<_, i32>(
        r#"INSERT INTO extraction_runs
           (document_id, pass_number, model_name, schema_version, started_at, raw_output, status)
           VALUES ($1, $2, $3, $4, NOW(), '{}'::jsonb, 'RUNNING')
           RETURNING id"#,
    )
    .bind(document_id)
    .bind(pass_number)
    .bind(model_name)
    .bind(schema_version)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Update an extraction run with results (completed or failed).
pub async fn complete_extraction_run(
    pool: &PgPool,
    run_id: i32,
    raw_output: &serde_json::Value,
    input_tokens: Option<i32>,
    output_tokens: Option<i32>,
    cost_usd: Option<f64>,
    status: &str,
) -> Result<(), PipelineRepoError> {
    // cost_usd is NUMERIC(10,4) in Postgres. We store it as text and cast
    // in SQL because sqlx needs the rust_decimal feature for direct NUMERIC binding.
    let cost_str = cost_usd.map(|c| format!("{c:.4}"));
    sqlx::query(
        r#"UPDATE extraction_runs
           SET raw_output = $1, input_tokens = $2, output_tokens = $3,
               cost_usd = $4::numeric, status = $5, completed_at = NOW()
           WHERE id = $6"#,
    )
    .bind(raw_output)
    .bind(input_tokens)
    .bind(output_tokens)
    .bind(cost_str)
    .bind(status)
    .bind(run_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Insert an extraction item. Returns the auto-generated item ID.
pub async fn insert_extraction_item(
    pool: &PgPool,
    run_id: i32,
    document_id: &str,
    entity_type: &str,
    item_data: &serde_json::Value,
    verbatim_quote: Option<&str>,
) -> Result<i32, PipelineRepoError> {
    let row = sqlx::query_scalar::<_, i32>(
        r#"INSERT INTO extraction_items
           (run_id, document_id, entity_type, item_data, verbatim_quote)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING id"#,
    )
    .bind(run_id)
    .bind(document_id)
    .bind(entity_type)
    .bind(item_data)
    .bind(verbatim_quote)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Insert an extraction relationship.
#[allow(clippy::too_many_arguments)]
pub async fn insert_extraction_relationship(
    pool: &PgPool,
    run_id: i32,
    document_id: &str,
    from_item_id: i32,
    to_item_id: i32,
    relationship_type: &str,
    properties: Option<&serde_json::Value>,
    tier: i32,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        r#"INSERT INTO extraction_relationships
           (run_id, document_id, from_item_id, to_item_id, relationship_type, properties, tier)
           VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
    )
    .bind(run_id)
    .bind(document_id)
    .bind(from_item_id)
    .bind(to_item_id)
    .bind(relationship_type)
    .bind(properties)
    .bind(tier)
    .execute(pool)
    .await?;
    Ok(())
}
