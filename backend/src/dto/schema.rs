use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Response for GET /schema - database discovery endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaResponse {
    pub total_nodes: i64,
    pub total_relationships: i64,
    pub node_counts: HashMap<String, i64>,
    pub relationship_counts: HashMap<String, i64>,
}
