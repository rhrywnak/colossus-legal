//! Reading the record a deck's questions bind to.
//!
//! Split from [`super::seed`] on 2026-08-17 when that module passed the 300-line
//! limit (Rule 17). The seam is the natural one: `seed` decides and writes, this
//! reads the scenario the decisions are about — and it is the half that knows
//! what "the third ruled instance" means.
//!
//! ## Domain note: what "in order" means here
//!
//! Both readers return an ORDER, and the deck's positions count in it. For
//! instances that order is the order a human ruled them while reading a document;
//! for points it is the order they are printed beside her. Neither is an
//! arbitrary sort, which is why each query says which it is rather than leaving
//! the reader to infer it from an `ORDER BY`.

use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::seed::SeedError;

/// The scenario a deck names, and the sources its questions can bind to.
pub(super) struct ScenarioSources {
    pub(super) scenario_id: Uuid,
    /// Graph node ids of the ruled accusation instances, in ruling order.
    pub(super) instances: Vec<String>,
    /// `response_items.id` of the talking points, in printed order.
    pub(super) points: Vec<Uuid>,
}

/// Look up the scenario by code, and everything its questions may bind to.
///
/// The code is `S-<code_ordinal>`, which is how `domain::scenario_code` composes
/// it; the parse back is done here rather than by a join on a stored string,
/// because no column holds the printed code.
pub(super) async fn read_sources(pool: &PgPool, code: &str) -> Result<ScenarioSources, SeedError> {
    let scenario_id = scenario_id_for(pool, code).await?;
    Ok(ScenarioSources {
        instances: ruled_instances(pool, scenario_id).await?,
        points: talking_points(pool, scenario_id).await?,
        scenario_id,
    })
}

/// The scenario a printed code names.
///
/// The code is `S-<code_ordinal>`, which is how `domain::scenario_code` composes
/// it; the parse back happens here rather than as a join on a stored string,
/// because no column holds the printed code.
async fn scenario_id_for(pool: &PgPool, code: &str) -> Result<Uuid, SeedError> {
    let ordinal: i32 = code
        .trim()
        .trim_start_matches(['S', 's'])
        .trim_start_matches('-')
        .parse()
        .map_err(|_| SeedError::NoSuchScenario {
            code: code.to_string(),
        })?;

    let row = sqlx::query("SELECT scenario_id FROM scenarios WHERE code_ordinal = $1")
        .bind(ordinal)
        .fetch_optional(pool)
        .await
        .map_err(|source| SeedError::Database { source })?
        .ok_or_else(|| SeedError::NoSuchScenario {
            code: code.to_string(),
        })?;
    row.try_get("scenario_id")
        .map_err(|source| SeedError::Database { source })
}

/// The graph node ids of the scenario's ruled accusation instances, in ruling
/// order.
///
/// Ruling order is creation order: the instances were marked one after another as
/// a human read the document, and that is the order the deck's positions count
/// in. Ties are broken by id so the order is total and stable across runs.
async fn ruled_instances(pool: &PgPool, scenario_id: Uuid) -> Result<Vec<String>, SeedError> {
    sqlx::query(
        "SELECT anchor_graph_node_id FROM scenario_human_facts \
         WHERE scenario_id = $1 AND kind = 'accusation_instance' \
           AND anchor_graph_node_id IS NOT NULL \
         ORDER BY created_at, id",
    )
    .bind(scenario_id)
    .fetch_all(pool)
    .await
    .map_err(|source| SeedError::Database { source })?
    .into_iter()
    .map(|row| row.try_get::<String, _>("anchor_graph_node_id"))
    .collect::<Result<Vec<_>, _>>()
    .map_err(|source| SeedError::Database { source })
}

/// The `response_items.id` of the scenario's talking points, in printed order.
async fn talking_points(pool: &PgPool, scenario_id: Uuid) -> Result<Vec<Uuid>, SeedError> {
    sqlx::query(
        "SELECT i.id FROM scenario_responses r \
         JOIN response_items i ON i.response_id = r.id \
         WHERE r.scenario_id = $1 ORDER BY i.item_index",
    )
    .bind(scenario_id)
    .fetch_all(pool)
    .await
    .map_err(|source| SeedError::Database { source })?
    .into_iter()
    .map(|row| row.try_get::<Uuid, _>("id"))
    .collect::<Result<Vec<_>, _>>()
    .map_err(|source| SeedError::Database { source })
}
