//! backend/tests/index_payload_parity.rs
//!
//! ⚑ Both per-document index paths store the SAME payload for the same node.
//!
//! ## Why parity is proved this way
//!
//! The strongest possible proof of "two paths agree" is that there is only one
//! implementation to disagree with — and after this change there is:
//! `services::qdrant_payload::build_point_payload`. Its behaviour is pinned by
//! six pure unit tests in that module, which need no graph, no Qdrant and no
//! `AppState`.
//!
//! What those cannot pin is that both paths still CALL it. Driving either path
//! for real is out of reach here: Path A needs a Restate runtime and Path B
//! needs an `AppState` carrying two pools, a graph and a Qdrant client, and this
//! project has no tier that builds either. So the calling half is asserted
//! against the source, the same discipline
//! `api::timeline_subsets::writes::tests` uses for its write-guard proof.
//!
//! Together: the builder's behaviour is tested directly, and these tests fail
//! the moment a path stops using it or grows a payload of its own again.

/// Neither path constructs a payload map any more.
///
/// This is the regression that matters. The two paths previously each built a
/// `json!({...})` literal and then conditionally inserted `page_number` into it,
/// and nothing failed if the two copies drifted — the stored records would just
/// differ depending on which path last touched the document. A new `json!` with
/// payload keys in either file means that has started again.
#[test]
fn neither_index_path_builds_a_payload_of_its_own() {
    for path in ["src/pipeline/steps/index.rs", "src/api/pipeline/index.rs"] {
        let src =
            std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
        for key in [
            "\"node_id\"",
            "\"node_type\"",
            "\"title\"",
            "\"document_id\"",
            "\"source_document\"",
            "\"page_number\"",
        ] {
            assert!(
                !src.contains(key),
                "{path} contains the literal payload key {key} — every key must come from \
                 pipeline::constants, through services::qdrant_payload"
            );
        }
    }
}

/// Neither path names a payload-key CONSTANT either.
///
/// Closes the gap the literal check above leaves: a rogue payload rebuilt as
/// `json!({QDRANT_NODE_ID_FIELD: node.id, …})` would contain no string literal
/// and would slip past it. Neither path has any business naming a payload key by
/// any route — the builder owns the whole key set. `QDRANT_COLLECTION_NAME` is
/// deliberately not in this list: it names the collection, not a payload field,
/// and the workflow step legitimately logs it.
#[test]
fn neither_index_path_names_a_payload_key_constant() {
    for path in ["src/pipeline/steps/index.rs", "src/api/pipeline/index.rs"] {
        let src =
            std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
        for constant in [
            "QDRANT_NODE_ID_FIELD",
            "QDRANT_NODE_TYPE_FIELD",
            "QDRANT_TITLE_FIELD",
            "QDRANT_DOCUMENT_ID_FIELD",
            "QDRANT_SOURCE_DOCUMENT_FIELD",
            "QDRANT_PAGE_NUMBER_FIELD",
        ] {
            assert!(
                !src.contains(constant),
                "{path} names the payload key constant {constant} — payload keys belong to \
                 services::qdrant_payload and nowhere else"
            );
        }
    }
}

/// Both paths call the shared builder.
#[test]
fn both_index_paths_call_the_shared_builder() {
    for path in ["src/pipeline/steps/index.rs", "src/api/pipeline/index.rs"] {
        let src =
            std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
        assert!(
            src.contains("qdrant_payload::build_point_payload"),
            "{path} must build its point payload through the shared builder, or the two \
             paths can store different records for the same node"
        );
    }
}

/// The builder is reachable from both without either importing the other.
///
/// It lives in `services`, which both already depend on. If it ever moved into
/// one path's module, the other would have to import that path — which is the
/// coupling the eventual reconciliation is meant to remove, not create.
#[test]
fn the_builder_sits_where_neither_path_depends_on_the_other() {
    let a = std::fs::read_to_string("src/pipeline/steps/index.rs").expect("readable");
    let b = std::fs::read_to_string("src/api/pipeline/index.rs").expect("readable");

    assert!(
        !a.contains("use crate::api::pipeline::index"),
        "the workflow step must not import the API path"
    );
    assert!(
        !b.contains("use crate::pipeline::steps::index"),
        "the API path must not import the workflow step"
    );
    for src in [&a, &b] {
        assert!(
            src.contains("use crate::services::qdrant_payload"),
            "each path reaches the builder through services, which both already depend on"
        );
    }
}
