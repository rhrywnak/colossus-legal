//! Builds the text string that gets embedded for each node type.
//!
//! ## Pattern: HashMap<String, String> as flexible property bag
//! Instead of creating a separate struct for each of the 7 node types,
//! we use a `HashMap<String, String>` to hold whatever properties each
//! node has. This is a common pattern for cross-cutting concerns where
//! the exact set of fields varies. The `get_prop()` helper safely
//! returns "" for missing keys — no unwrap(), no panic.
//!
//! ## Nomic prefix convention
//! The nomic-embed-text model uses prefixes to distinguish indexed
//! documents from search queries:
//! - "search_document: ..." for text being stored (indexing time)
//! - "search_query: ..." for text being searched (query time, used in H.2)
//!
//! All texts built here use "search_document:" since they're going into Qdrant.

use std::collections::HashMap;

/// Build the embedding text for a node based on its type and properties.
///
/// Returns a "search_document: ..." prefixed string ready for embedding.
/// If the resulting text is empty after trimming, falls back to
/// "search_document: {node_type}" so we never produce an empty embedding.
pub fn build_embedding_text(node_type: &str, props: &HashMap<String, String>) -> String {
    let text = match node_type {
        "Evidence" => format!(
            "search_document: {}. {}. Significance: {}",
            get_prop(props, "title"),
            get_prop(props, "verbatim_quote"),
            get_prop(props, "significance"),
        ),

        "ComplaintAllegation" => format!(
            "search_document: {}. {}. {}",
            get_prop(props, "title"),
            get_prop(props, "allegation"),
            get_prop(props, "verbatim_quote"),
        ),

        "MotionClaim" => format!(
            "search_document: {}. {}. Significance: {}",
            get_prop(props, "title"),
            get_prop(props, "claim_text"),
            get_prop(props, "significance"),
        ),

        "Harm" => format!(
            "search_document: {}. {}",
            get_prop(props, "title"),
            get_prop(props, "description"),
        ),

        "Document" => format!(
            "search_document: {} ({})",
            get_prop(props, "title"),
            get_prop(props, "document_type"),
        ),

        "Person" => format!(
            "search_document: {} ({}). {}",
            get_prop(props, "name"),
            get_prop(props, "role"),
            get_prop(props, "description"),
        ),

        "Organization" => format!(
            "search_document: {} ({}). {}",
            get_prop(props, "name"),
            get_prop(props, "role"),
            get_prop(props, "description"),
        ),

        // Unknown node type — use whatever title or name is available
        _ => format!("search_document: {}", get_prop(props, "title"),),
    };

    let trimmed = text.trim().to_string();

    // Fallback: if everything was empty, at least include the node type
    if trimmed == "search_document:" || trimmed.is_empty() {
        format!("search_document: {node_type}")
    } else {
        trimmed
    }
}

/// Safely get a property value, returning "" if the key is missing.
fn get_prop<'a>(props: &'a HashMap<String, String>, key: &str) -> &'a str {
    props.get(key).map(|s| s.as_str()).unwrap_or("")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evidence_text() {
        let mut props = HashMap::new();
        props.insert("title".into(), "Phillips Q73".into());
        props.insert("verbatim_quote".into(), "I took the money.".into());
        props.insert("significance".into(), "Admission of conversion.".into());

        let text = build_embedding_text("Evidence", &props);
        assert!(text.starts_with("search_document:"));
        assert!(text.contains("Phillips Q73"));
        assert!(text.contains("I took the money."));
    }

    #[test]
    fn test_missing_fields_dont_panic() {
        let props = HashMap::new();
        let text = build_embedding_text("Evidence", &props);
        assert!(text.starts_with("search_document:"));
    }

    #[test]
    fn test_unknown_node_type_fallback() {
        let props = HashMap::new();
        let text = build_embedding_text("UnknownType", &props);
        assert_eq!(text, "search_document: UnknownType");
    }

    #[test]
    fn test_person_text() {
        let mut props = HashMap::new();
        props.insert("name".into(), "Marie Awad".into());
        props.insert("role".into(), "plaintiff".into());
        props.insert("description".into(), "Estate beneficiary.".into());

        let text = build_embedding_text("Person", &props);
        assert!(text.contains("Marie Awad"));
        assert!(text.contains("plaintiff"));
    }
}
