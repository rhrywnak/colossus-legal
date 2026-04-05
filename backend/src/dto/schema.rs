use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::state::{EntityTypeInfo, RelationshipTypeInfo};

/// Response for GET /schema - database discovery + extraction schema metadata.
///
/// Combines live graph statistics (node/relationship counts from Neo4j) with
/// extraction schema metadata (entity types and relationship types from the
/// YAML schema loaded at startup).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaResponse {
    // Live graph statistics
    pub total_nodes: i64,
    pub total_relationships: i64,
    pub node_counts: HashMap<String, i64>,
    pub relationship_counts: HashMap<String, i64>,

    // Extraction schema metadata
    /// The document type this schema handles (e.g., "general_legal")
    pub document_type: String,
    /// Entity types defined in the extraction schema
    pub entity_types: Vec<EntityTypeInfo>,
    /// Relationship types defined in the extraction schema
    pub relationship_types: Vec<RelationshipTypeInfo>,
}
