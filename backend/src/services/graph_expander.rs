//! Graph-aware context assembly for LLM synthesis.
//!
//! Takes seed node IDs from semantic search (H.2), expands through Neo4j
//! relationships (1-2 hops), deduplicates, and assembles formatted text
//! for LLM consumption (H.4).
//!
//! ## Pattern: String budget management
//! We build formatted text incrementally, node by node, estimating token
//! count as we go. If the total exceeds the budget, we truncate by
//! removing entire nodes from the lowest priority types first.
//! Priority: Evidence > Allegation > MotionClaim > Harm > Person/Org > Document.

use neo4rs::Graph;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use super::graph_expansion_minor;
use super::graph_expansion_queries;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A node from the graph with its properties.
#[derive(Debug, Clone, Serialize)]
pub struct ExpandedNode {
    pub id: String,
    pub node_type: String,
    pub title: String,
    pub properties: HashMap<String, String>,
}

/// A relationship connecting two expanded nodes.
#[derive(Debug, Clone, Serialize)]
pub struct ExpandedRelationship {
    pub from_id: String,
    pub to_id: String,
    pub rel_type: String,
    pub properties: HashMap<String, String>,
}

impl ExpandedRelationship {
    pub fn new(from_id: &str, to_id: &str, rel_type: &str) -> Self {
        Self {
            from_id: from_id.to_string(),
            to_id: to_id.to_string(),
            rel_type: rel_type.to_string(),
            properties: HashMap::new(),
        }
    }
}

/// The complete expanded context from graph traversal.
#[derive(Debug, Clone, Serialize)]
pub struct ExpandedContext {
    pub nodes: Vec<ExpandedNode>,
    pub relationships: Vec<ExpandedRelationship>,
    pub formatted_text: String,
    pub seeds_expanded: usize,
    pub unique_nodes: usize,
    pub approx_tokens: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum GraphExpanderError {
    #[error("Neo4j query error: {0}")]
    Neo4j(#[from] neo4rs::Error),

    #[error("Neo4j deserialization error: {0}")]
    Deserialization(#[from] neo4rs::DeError),
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Expand a list of seed node IDs through the knowledge graph.
///
/// ## Pattern: Enum dispatch for expansion
/// We match on the `node_type` string and call the corresponding expansion
/// function. This is cleaner than an if/else chain and makes it easy to
/// add new node types later.
pub async fn expand_context(
    graph: &Graph,
    seed_node_ids: Vec<(String, String)>, // (node_id, node_type) pairs
    max_tokens: usize,
) -> Result<ExpandedContext, GraphExpanderError> {
    tracing::info!(
        seed_count = seed_node_ids.len(),
        max_tokens,
        "Graph expansion: starting with {} seeds",
        seed_node_ids.len()
    );

    let mut all_nodes: Vec<ExpandedNode> = Vec::new();
    let mut all_rels: Vec<ExpandedRelationship> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut seeds_expanded = 0;

    // ## Pattern: Per-seed error resilience
    // Each seed expansion is wrapped in a match so one failed Neo4j query
    // doesn't kill the entire expansion. Errors are logged and skipped.
    for (node_id, node_type) in &seed_node_ids {
        let result = match node_type.as_str() {
            "Evidence" => {
                graph_expansion_queries::expand_evidence(graph, node_id, &mut seen).await
            }
            "ComplaintAllegation" => {
                graph_expansion_queries::expand_allegation(graph, node_id, &mut seen).await
            }
            "MotionClaim" => {
                graph_expansion_queries::expand_motion_claim(graph, node_id, &mut seen).await
            }
            "Harm" => {
                graph_expansion_minor::expand_harm(graph, node_id, &mut seen).await
            }
            "Document" => {
                graph_expansion_minor::expand_document(graph, node_id, &mut seen).await
            }
            "Person" => {
                graph_expansion_minor::expand_person(graph, node_id, &mut seen).await
            }
            "Organization" => {
                graph_expansion_minor::expand_organization(graph, node_id, &mut seen).await
            }
            _ => {
                tracing::warn!("Unknown node type for expansion: {node_type}");
                continue;
            }
        };

        match result {
            Ok((nodes, rels)) => {
                tracing::info!(
                    node_id,
                    node_type,
                    nodes_found = nodes.len(),
                    rels_found = rels.len(),
                    "Seed expanded successfully"
                );
                all_nodes.extend(nodes);
                all_rels.extend(rels);
                seeds_expanded += 1;
            }
            Err(e) => {
                tracing::warn!(
                    node_id,
                    node_type,
                    error = %e,
                    "Seed expansion failed (skipping)"
                );
            }
        }
    }

    let unique_nodes = all_nodes.len();

    tracing::info!(
        seeds_expanded,
        unique_nodes,
        relationships = all_rels.len(),
        "Graph expansion complete"
    );

    // Format and apply token budget
    let formatted_text = format_context_with_budget(&all_nodes, max_tokens);
    let approx_tokens = estimate_tokens(&formatted_text);

    Ok(ExpandedContext {
        nodes: all_nodes,
        relationships: all_rels,
        formatted_text,
        seeds_expanded,
        unique_nodes,
        approx_tokens,
    })
}

// ---------------------------------------------------------------------------
// Context formatting
// ---------------------------------------------------------------------------

/// Node type priority for budget enforcement (lower = higher priority).
fn type_priority(node_type: &str) -> u8 {
    match node_type {
        "Evidence" => 0,
        "ComplaintAllegation" => 1,
        "MotionClaim" => 2,
        "Harm" => 3,
        "LegalCount" => 4,
        "Person" | "Organization" => 5,
        "Document" => 6,
        _ => 7,
    }
}

/// Format expanded nodes into structured text, enforcing a token budget.
///
/// Sorts nodes by priority (Evidence first, Documents last).
/// If the full text exceeds the budget, drops nodes from the lowest
/// priority end until within budget.
fn format_context_with_budget(nodes: &[ExpandedNode], max_tokens: usize) -> String {
    // Sort by priority
    let mut sorted: Vec<&ExpandedNode> = nodes.iter().collect();
    sorted.sort_by_key(|n| type_priority(&n.node_type));

    let mut output = String::new();
    let mut current_tokens = 0;

    for node in sorted {
        let section = format_node(node);
        let section_tokens = estimate_tokens(&section);

        if current_tokens + section_tokens > max_tokens {
            break; // Budget exhausted — stop adding nodes
        }

        output.push_str(&section);
        output.push('\n');
        current_tokens += section_tokens;
    }

    output.trim().to_string()
}

/// Format a single node into a text section for LLM context.
fn format_node(node: &ExpandedNode) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "=== {}: {} ===", node.node_type.to_uppercase(), node.id);
    let _ = writeln!(s, "Title: {}", node.title);

    let props = &node.properties;

    // Type-specific fields
    match node.node_type.as_str() {
        "Evidence" => {
            if let Some(v) = props.get("verbatim_quote") { let _ = writeln!(s, "Quote: {v}"); }
            if let Some(v) = props.get("significance") { let _ = writeln!(s, "Significance: {v}"); }
            if let Some(v) = props.get("page_number") { let _ = writeln!(s, "Page: {v}"); }
        }
        "ComplaintAllegation" => {
            if let Some(v) = props.get("allegation") { let _ = writeln!(s, "Allegation: {v}"); }
            if let Some(v) = props.get("evidence_status") { let _ = writeln!(s, "Status: {v}"); }
        }
        "MotionClaim" => {
            if let Some(v) = props.get("claim_text") { let _ = writeln!(s, "Claim: {v}"); }
            if let Some(v) = props.get("significance") { let _ = writeln!(s, "Significance: {v}"); }
        }
        "Harm" => {
            if let Some(v) = props.get("description") { let _ = writeln!(s, "Description: {v}"); }
            if let Some(v) = props.get("amount") { let _ = writeln!(s, "Amount: ${v}"); }
        }
        "Person" | "Organization" => {
            if let Some(v) = props.get("name") { let _ = writeln!(s, "Name: {v}"); }
            if let Some(v) = props.get("role") { let _ = writeln!(s, "Role: {v}"); }
        }
        "Document" => {
            if let Some(v) = props.get("document_type") { let _ = writeln!(s, "Type: {v}"); }
        }
        "LegalCount" => {}
        _ => {}
    }

    s
}

/// Approximate token count: ~1.33 tokens per whitespace-delimited word.
fn estimate_tokens(text: &str) -> usize {
    text.split_whitespace().count() * 4 / 3
}
