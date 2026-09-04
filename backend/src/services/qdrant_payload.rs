//! The ONE builder for a Qdrant point's payload, for both per-document index
//! paths. (A third, different builder serves the full-corpus re-embed — see the
//! scope note at the end of this comment.)
//!
//! ## Why this module exists
//!
//! Two independent implementations index a document into Qdrant —
//! `pipeline::steps::index::run_index` (the Restate workflow's step 7) and
//! `api::pipeline::index::run_index_core` (the `POST /documents/:id/index` route
//! and the delta ingest's inline trigger). Each built its own payload map, and
//! the two were held in agreement only by the fact that somebody had copied one
//! into the other. Nothing failed if they drifted; the stored records would
//! simply differ depending on which path last touched the document, and a
//! payload read would return a different answer for no reason a reader could
//! predict.
//!
//! ## Where it lives, and why here
//!
//! `services`, beside `embedding_text` — which answers the sibling question,
//! "how does a node become the TEXT we embed?". This answers "how does a node
//! become the PAYLOAD we store?". Both index paths already depend on `services`
//! (`qdrant_service`, `embedding_text`), and neither `pipeline::steps` nor
//! `api::pipeline` has to import the other to reach it. Putting it in either
//! path's own module would have made one of them depend on the other, which is
//! the coupling the reconciliation is supposed to remove later, not add now.
//!
//! ## Scope: this is the payload only
//!
//! It deliberately does NOT unify how the two paths fetch nodes, embed them, or
//! decide a point id — those stay as they are. Reconciling the two index
//! implementations is an owed item for after court.
//!
//! ## ⚑ "ONE builder" means one for the two INDEX paths — there is a third
//!
//! `services::embedding_pipeline::run_embedding_pipeline` — the full-corpus
//! re-embed behind the CLI, `POST /admin/embed-all` and `POST /admin/reindex` —
//! builds its own payload and is **deliberately not changed here**. It is not a
//! per-document path and it writes a materially different shape: `node_id`,
//! `node_type`, `title`, then every node property copied in wholesale. For
//! Evidence that yields `verbatim_quote`, `significance`, `page_number`,
//! `statement_type`, `stated_by` and more — the nine extra fields
//! `qdrant_service::search_points` reads and that this builder does not write.
//!
//! **It also writes NEITHER `document_id` NOR `source_document`.** Its Evidence
//! query projects `e.document_id`, and no Evidence node carries that property
//! (measured 2026-09-01: 0 of 1209; they carry `source_document`), so the value
//! is null, the empty string is skipped by the fetch, and the key never reaches
//! the payload. `qdrant_service::delete_points_by_filter` filters on
//! `document_id` — so a corpus re-embed would leave every point undeletable by
//! document. That is a live latent defect, reported in
//! CC_REPORT_INDEX_PAYLOAD_PARITY_v1, and it is why the three builders were NOT
//! collapsed into one here: making them agree requires deciding which shape is
//! right, which is a ruling, not a refactor.

use serde_json::{json, Map, Value};

use crate::pipeline::constants::{
    QDRANT_DOCUMENT_ID_FIELD, QDRANT_NODE_ID_FIELD, QDRANT_NODE_TYPE_FIELD,
    QDRANT_PAGE_NUMBER_FIELD, QDRANT_SOURCE_DOCUMENT_FIELD, QDRANT_TITLE_FIELD,
};
use crate::repositories::embedding_repository::EmbeddableNode;

/// The node property a title is taken from, and the one it falls back to.
///
// STRUCTURAL: Neo4j property vocabulary, not a setting. Different labels name
// their human-readable field differently — `Evidence` has `title`, `Person` has
// `name` — and the fallback is what lets one builder serve every label.
const TITLE_PROP: &str = "title";
const NAME_PROP: &str = "name";
const PAGE_NUMBER_PROP: &str = "page_number";

/// The display title for a node: its `title`, else its `name`, else empty.
///
/// Empty string rather than an omitted key, because that is what both paths did
/// before this builder existed and this change removes a difference rather than
/// introducing one. A point written last week and a point written today must
/// still compare equal.
fn title_of(node: &EmbeddableNode) -> String {
    node.properties
        .get(TITLE_PROP)
        .or_else(|| node.properties.get(NAME_PROP))
        .cloned()
        .unwrap_or_default()
}

/// Build the payload for one node's Qdrant point.
///
/// `document_id` is the document this node belongs to. `Some` writes it to two
/// keys — `document_id` and `source_document` — because `search_points` reads
/// them separately and points already stored carry both.
///
/// ## Domain note: `None` omits both keys, and the caller must complain
///
/// `None` means the node carries no document linkage at all. The two keys are
/// then OMITTED rather than written as `""`, and this is the important half:
/// `qdrant_service::delete_points_by_filter` filters on `document_id`, so a
/// point stored with an empty-string id would match a filter for `""` and be
/// undeletable by its real document for ever. Omitting leaves it equally
/// undeletable but at least honest, and the caller is expected to log which node
/// it was — see `services::embedding_pipeline`.
///
/// The two per-document index paths always pass `Some`: they are indexing a
/// named document and cannot not know its id. Only the corpus re-embed, which
/// walks every node in the graph, can meet a node with no linkage.
///
/// ## Domain note: `page_number` is OMITTED, never null
///
/// A node with no page produces a payload with no `page_number` key at all,
/// rather than one set to `null`. That is deliberate and it is what both paths
/// already did — this builder preserves the behaviour rather than choosing a new
/// one. The distinction is not cosmetic to Qdrant: a `match` filter on a key
/// matches neither an absent key nor a null one, but `is_empty` and
/// `is_null` are DIFFERENT conditions in the filter language, and points written
/// under one convention would stop matching a filter written for the other. With
/// no key at all, `is_empty` is the single condition that finds them.
///
/// ## Rust Learning: `Map<String, Value>` built directly, not `json!` then patch
///
/// The two call sites this replaces each built a `json!({...})` literal and then
/// conditionally `insert`ed into it via `as_object_mut()` — which returns an
/// `Option` that both of them had to handle for a value they had just built and
/// knew was an object. Building the map directly removes that impossible branch
/// entirely: there is no `Option` to unwrap and no path where the insert could
/// silently not happen.
pub fn build_point_payload(node: &EmbeddableNode, document_id: Option<&str>) -> Value {
    let mut payload = Map::new();
    payload.insert(QDRANT_NODE_ID_FIELD.to_string(), json!(node.id));
    payload.insert(QDRANT_NODE_TYPE_FIELD.to_string(), json!(node.node_type));
    payload.insert(QDRANT_TITLE_FIELD.to_string(), json!(title_of(node)));
    if let Some(document_id) = document_id {
        payload.insert(QDRANT_DOCUMENT_ID_FIELD.to_string(), json!(document_id));
        payload.insert(QDRANT_SOURCE_DOCUMENT_FIELD.to_string(), json!(document_id));
    }

    // Present only when the node has one. See the doc comment: absent, not null.
    if let Some(page) = node.properties.get(PAGE_NUMBER_PROP) {
        payload.insert(QDRANT_PAGE_NUMBER_FIELD.to_string(), json!(page));
    }

    Value::Object(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn node(node_type: &str, props: &[(&str, &str)]) -> EmbeddableNode {
        EmbeddableNode {
            id: "doc-x:evidence:abc".to_string(),
            node_type: node_type.to_string(),
            properties: props
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect::<HashMap<_, _>>(),
        }
    }

    /// A node WITH a page carries every key, page included.
    #[test]
    fn a_node_with_a_page_carries_six_keys() {
        let payload = build_point_payload(
            &node(
                "Evidence",
                &[("title", "Phillips admits"), ("page_number", "22")],
            ),
            Some("doc-x"),
        );
        assert_eq!(payload["node_id"], json!("doc-x:evidence:abc"));
        assert_eq!(payload["node_type"], json!("Evidence"));
        assert_eq!(payload["title"], json!("Phillips admits"));
        assert_eq!(payload["document_id"], json!("doc-x"));
        assert_eq!(payload["source_document"], json!("doc-x"));
        assert_eq!(payload["page_number"], json!("22"));
        assert_eq!(payload.as_object().expect("an object").len(), 6);
    }

    /// A node WITHOUT a page omits the key entirely — it is not set to null.
    ///
    /// Asserted on the key's absence, not on its value, because `payload["x"]`
    /// on a missing key yields `Value::Null` and would pass a `== json!(null)`
    /// check either way. `get()` is the only assertion that can tell them apart.
    #[test]
    fn a_node_without_a_page_omits_the_key_rather_than_nulling_it() {
        let payload = build_point_payload(
            &node("Evidence", &[("title", "No page here")]),
            Some("doc-x"),
        );
        let object = payload.as_object().expect("an object");
        assert!(
            object.get("page_number").is_none(),
            "the key must be ABSENT; a null would need a different Qdrant filter to find"
        );
        assert_eq!(object.len(), 5);
    }

    /// A node with `name` and no `title` uses the name — that is how non-Evidence
    /// labels get a title, and one builder has to serve them all.
    #[test]
    fn a_node_with_only_a_name_titles_itself_from_it() {
        let payload = build_point_payload(
            &node("Person", &[("name", "George Phillips")]),
            Some("doc-x"),
        );
        assert_eq!(payload["title"], json!("George Phillips"));
    }

    /// A node with neither is titled with the empty string, not omitted.
    ///
    /// Preserving what both paths did before. A point written last week and one
    /// written today must still compare equal.
    #[test]
    fn a_node_with_neither_title_nor_name_gets_an_empty_title() {
        let payload = build_point_payload(&node("Chunk", &[]), Some("doc-x"));
        assert_eq!(payload["title"], json!(""));
        assert!(payload
            .as_object()
            .expect("an object")
            .contains_key("title"));
    }

    /// `title` wins over `name` when a node carries both.
    #[test]
    fn title_wins_over_name() {
        let payload = build_point_payload(
            &node("Evidence", &[("title", "the title"), ("name", "the name")]),
            Some("doc-x"),
        );
        assert_eq!(payload["title"], json!("the title"));
    }

    /// No document linkage: BOTH keys are omitted, never written as "".
    ///
    /// The whole point of the Option. `delete_points_by_filter` filters on
    /// `document_id`, so a point stored with `""` would match a filter for the
    /// empty string and be undeletable by its real document for ever. Asserted
    /// on absence via `get()`, because indexing a missing key yields
    /// `Value::Null` and would pass a comparison either way.
    #[test]
    fn a_node_with_no_document_omits_both_keys_rather_than_writing_empty() {
        let payload = build_point_payload(&node("Person", &[("name", "Someone")]), None);
        let object = payload.as_object().expect("an object");
        assert!(object.get("document_id").is_none());
        assert!(object.get("source_document").is_none());
        assert_ne!(
            object.get("document_id"),
            Some(&json!("")),
            "an empty string here would be undeletable by the real document id"
        );
        // The node is still indexed and still searchable — three keys remain.
        assert_eq!(object.len(), 3);
        assert_eq!(payload["node_id"], json!("doc-x:evidence:abc"));
    }

    /// Every key in the payload is one of the declared constants.
    ///
    /// The anti-drift check: a literal added here would be a key nothing else in
    /// the codebase knows the name of, and `search_points` would never read it.
    #[test]
    fn every_key_comes_from_a_constant() {
        let payload = build_point_payload(
            &node("Evidence", &[("title", "t"), ("page_number", "9")]),
            Some("doc-x"),
        );
        let declared = [
            QDRANT_NODE_ID_FIELD,
            QDRANT_NODE_TYPE_FIELD,
            QDRANT_TITLE_FIELD,
            QDRANT_DOCUMENT_ID_FIELD,
            QDRANT_SOURCE_DOCUMENT_FIELD,
            QDRANT_PAGE_NUMBER_FIELD,
        ];
        for key in payload.as_object().expect("an object").keys() {
            assert!(
                declared.contains(&key.as_str()),
                "payload key `{key}` is not one of the declared constants"
            );
        }
    }
}
