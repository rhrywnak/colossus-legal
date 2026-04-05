use axum::{extract::State, http::StatusCode, Json};

use crate::auth::AuthUser;
use crate::dto::SchemaResponse;
use crate::repositories::SchemaRepository;
use crate::state::AppState;

/// GET /schema — Returns live graph statistics + extraction schema metadata.
///
/// Combines two data sources:
/// 1. **Live graph stats** — node/relationship counts from Neo4j (via SchemaRepository)
/// 2. **Schema metadata** — entity types and relationship types from the extraction
///    schema YAML (loaded at startup, stored in AppState)
pub async fn get_schema(
    user: Option<AuthUser>,
    State(state): State<AppState>,
) -> Result<Json<SchemaResponse>, StatusCode> {
    if let Some(ref u) = user {
        tracing::info!("{} GET /schema", u.username);
    }
    let repo = SchemaRepository::new(state.graph.clone());

    match repo.get_schema_stats().await {
        Ok(graph_stats) => {
            let response = SchemaResponse {
                total_nodes: graph_stats.total_nodes,
                total_relationships: graph_stats.total_relationships,
                node_counts: graph_stats.node_counts,
                relationship_counts: graph_stats.relationship_counts,
                document_type: state.schema_metadata.document_type.clone(),
                entity_types: state.schema_metadata.entity_types.clone(),
                relationship_types: state.schema_metadata.relationship_types.clone(),
            };
            Ok(Json(response))
        }
        Err(e) => {
            tracing::error!("Schema query failed: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
