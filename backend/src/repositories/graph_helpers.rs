//! Shared helpers for Neo4j repository code.
//!
//! ## Label validation
//!
//! Neo4j Cypher does not support parameterized labels — you can't write
//! `MATCH (n:$label)`. Labels must be interpolated into the query string.
//! `safe_label()` validates that a label contains only alphanumeric chars
//! and underscores to prevent Cypher injection.
//!
//! ## GraphNode property extraction
//!
//! Free functions for extracting typed values from `colossus_graph::GraphNode`
//! properties. These live here (not in colossus-graph) because colossus-graph
//! is an external crate. They'll move there in a future release.

use colossus_graph::GraphNode;

/// Validate a Neo4j label for safe interpolation into Cypher queries.
///
/// Returns the label unchanged if valid, or an error message if it
/// contains characters that could enable Cypher injection.
pub fn safe_label(label: &str) -> Result<&str, String> {
    if label.is_empty() {
        return Err("Label cannot be empty".to_string());
    }
    if label.chars().all(|c| c.is_alphanumeric() || c == '_') {
        Ok(label)
    } else {
        Err(format!("Invalid Neo4j label: '{label}'"))
    }
}

/// Extract a string property from a GraphNode.
pub fn node_str(node: &GraphNode, key: &str) -> Option<String> {
    node.properties.get(key)?.as_str().map(|s| s.to_string())
}

/// Extract a string property, returning empty string if missing.
pub fn node_str_or(node: &GraphNode, key: &str, default: &str) -> String {
    node_str(node, key).unwrap_or_else(|| default.to_string())
}

/// Extract an i64 property from a GraphNode.
pub fn node_i64(node: &GraphNode, key: &str) -> Option<i64> {
    node.properties.get(key)?.as_i64()
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn safe_label_accepts_valid() {
        assert_eq!(safe_label("ComplaintAllegation"), Ok("ComplaintAllegation"));
        assert_eq!(safe_label("LegalCount"), Ok("LegalCount"));
        assert_eq!(safe_label("Person_2"), Ok("Person_2"));
    }

    #[test]
    fn safe_label_rejects_injection() {
        assert!(safe_label("Person})-[:HACK]->()//").is_err());
        assert!(safe_label("Label With Spaces").is_err());
        assert!(safe_label("").is_err());
    }

    #[test]
    fn node_str_extracts_value() {
        let node = GraphNode {
            id: "test".to_string(),
            labels: vec!["Test".to_string()],
            properties: HashMap::from([
                ("name".to_string(), serde_json::json!("Alice")),
                ("count".to_string(), serde_json::json!(42)),
            ]),
        };
        assert_eq!(node_str(&node, "name"), Some("Alice".to_string()));
        assert_eq!(node_str(&node, "missing"), None);
        assert_eq!(node_i64(&node, "count"), Some(42));
        assert_eq!(node_i64(&node, "missing"), None);
    }
}
