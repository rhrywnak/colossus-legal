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

    /// The Evidence text this builder produced on 2026-09-04, byte for byte.
    ///
    /// ## Why an `assert_eq!` on a whole string and not a set of `contains`
    ///
    /// Everything already in Qdrant was embedded from THIS text. A change to it
    /// that nobody notices does not fail anything — it silently makes the stored
    /// vectors and the newly-built ones incomparable, and the only symptom is
    /// retrieval quietly getting worse. So the exact bytes are pinned here BEFORE
    /// the `question` composition is added, and the no-question path has to keep
    /// producing them afterwards. The doubled full stop is real: `verbatim_quote`
    /// ends in one and the format string adds another. It is pinned as-is
    /// deliberately — tidying it would be a re-embed, not a test fix.
    #[test]
    fn pinned_evidence_text_without_a_question_2026_09_04() {
        let mut props = HashMap::new();
        props.insert("title".into(), "Phillips Q73".into());
        props.insert("verbatim_quote".into(), "I took the money.".into());
        props.insert("significance".into(), "Admission of conversion.".into());

        assert_eq!(
            build_embedding_text("Evidence", &props),
            "search_document: Phillips Q73. I took the money.. Significance: Admission of conversion."
        );
    }

    /// The empty-props Evidence text, also pinned. It is not pretty — three
    /// separators around nothing — but it is what is in the index, and the point
    /// of a pin is to record what IS, not what ought to be.
    #[test]
    fn pinned_evidence_text_with_no_properties_at_all_2026_09_04() {
        let props = HashMap::new();
        assert_eq!(
            build_embedding_text("Evidence", &props),
            "search_document: . . Significance:"
        );
    }

    /// The other six node types, pinned in one place. This task touches only the
    /// Evidence arm; if a later edit reaches sideways into another arm, the
    /// vectors for that type go stale too, and this is what says so.
    #[test]
    fn pinned_non_evidence_texts_2026_09_04() {
        let mut props = HashMap::new();
        props.insert("title".into(), "T".into());
        props.insert("allegation".into(), "A".into());
        props.insert("verbatim_quote".into(), "Q".into());
        props.insert("claim_text".into(), "C".into());
        props.insert("significance".into(), "S".into());
        props.insert("description".into(), "D".into());
        props.insert("document_type".into(), "DT".into());
        props.insert("name".into(), "N".into());
        props.insert("role".into(), "R".into());

        for (node_type, expected) in [
            ("ComplaintAllegation", "search_document: T. A. Q"),
            ("MotionClaim", "search_document: T. C. Significance: S"),
            ("Harm", "search_document: T. D"),
            ("Document", "search_document: T (DT)"),
            ("Person", "search_document: N (R). D"),
            ("Organization", "search_document: N (R). D"),
            ("Whatever", "search_document: T"),
        ] {
            assert_eq!(
                build_embedding_text(node_type, &props),
                expected,
                "{node_type}"
            );
        }
    }

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
