//! backend/tests/index_payload_parity.rs
//!
//! ⚑ Every path that writes a Qdrant point carries the document id.
//!
//! Originally the two per-document index paths; extended to the corpus re-embed
//! and the two delete call sites, which is where the real hazard lived — a point
//! written without `document_id` is undeletable by the only filter that removes
//! a document's vectors.
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

/// EVERY path that writes a point payload goes through the shared builder.
///
/// This is the guardrail the whole task exists for. The corpus re-embed used to
/// assemble its own payload and wrote no `document_id` at all — one press of
/// `POST /admin/reindex` would have left every point in the collection
/// undeletable by document, silently, because the delete path is best-effort and
/// a zero-count delete looked exactly like a clean one.
#[test]
fn every_payload_writing_path_uses_the_shared_builder() {
    for path in [
        "src/pipeline/steps/index.rs",
        "src/api/pipeline/index.rs",
        "src/services/embedding_pipeline.rs",
    ] {
        let src =
            std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
        assert!(
            src.contains("qdrant_payload::build_point_payload"),
            "{path} writes Qdrant points and must build the required keys through the \
             shared builder, or its points can lack document_id and be undeletable"
        );
    }
}

/// Both delete call sites filter by the CONSTANT, not a hand-spelled key.
///
/// The filter key and the payload key have to be the same string. Spelling
/// either by hand is how they stop being.
#[test]
fn both_delete_call_sites_filter_by_the_constant() {
    for path in [
        "src/api/pipeline/delete.rs",
        "src/pipeline/steps/cleanup.rs",
    ] {
        let src =
            std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
        assert!(
            src.contains("QDRANT_DOCUMENT_ID_FIELD"),
            "{path} must filter by the constant so the delete key cannot drift from the \
             payload key"
        );
    }
}

/// A zero-count delete warns, at BOTH call sites, and ONLY when it is zero.
///
/// A zero is ambiguous: a document that was never indexed, or a collection whose
/// points carry no `document_id` at all. The second is invisible without this —
/// the delete reports success and removes nothing.
///
/// ## Why this does not simply scan for `tracing::warn!`
///
/// It did, and for `delete.rs` that assertion was tautological: the file already
/// contained an unrelated `tracing::warn!` (a PDF deletion failure), so the scan
/// passed whether or not the new zero-count branch was a warning at all. The
/// test now anchors on the message and checks the macro that ENCLOSES it, so
/// mis-levelling the new branch to `info!` fails.
///
/// It also asserts the warning is ZERO-GUARDED, which is how "a non-zero delete
/// does not warn" is proved: the message appears exactly once per file, inside
/// the branch that a zero reaches and a non-zero cannot.
#[test]
fn a_zero_count_delete_warns_and_only_when_it_is_zero() {
    for (path, zero_guard) in [
        ("src/api/pipeline/delete.rs", "Ok(0)"),
        ("src/pipeline/steps/cleanup.rs", "count == 0"),
    ] {
        let src =
            std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"));

        assert_eq!(
            src.matches("NO vectors").count(),
            1,
            "{path} must carry exactly one zero-count message, or this test cannot reason \
             about which branch encloses it"
        );
        let message = src.find("NO vectors").expect("just counted it");
        let before = &src[..message];

        // The macro that encloses the message is the LAST one opened before it.
        let macro_start = before
            .rfind("tracing::")
            .unwrap_or_else(|| panic!("{path}: the zero-count message is not inside a log call"));
        assert!(
            before[macro_start..].starts_with("tracing::warn!"),
            "{path} must report a zero-count delete at WARN — an unrelated warn elsewhere in \
             the file must not be able to satisfy this"
        );

        // …and that log call is reachable only when the count is zero, which is
        // what makes a non-zero delete silent.
        assert!(
            before[..macro_start].contains(zero_guard),
            "{path}: the zero-count warning must be guarded by `{zero_guard}`, so a \
             legitimate non-zero delete does not warn"
        );
    }
}

/// A non-zero delete still logs, so a successful delete is not silent either.
#[test]
fn a_non_zero_delete_still_logs_its_count() {
    for path in [
        "src/api/pipeline/delete.rs",
        "src/pipeline/steps/cleanup.rs",
    ] {
        let src =
            std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
        assert!(
            src.contains("count,"),
            "{path} must still log the count on the non-zero path — the zero warning replaces \
             nothing"
        );
    }
}

/// The corpus re-embed never writes an empty-string document id.
///
/// An empty string would match a filter for `""` and be undeletable by the real
/// document id — strictly worse than omitting the key, because it looks present.
///
/// Asserted on the two things that make that true, rather than on the ABSENCE of
/// `unwrap_or_default` anywhere in the file: that call appears legitimately in
/// this module for a display title, and a test that fails on it would be failing
/// for the wrong reason.
#[test]
fn the_corpus_re_embed_never_writes_an_empty_document_id() {
    let src = std::fs::read_to_string("src/services/embedding_pipeline.rs").expect("readable");
    assert!(
        src.contains("document_id.map(String::as_str)"),
        "the document id must reach the builder as an Option, so a missing one omits the \
         key instead of defaulting to the empty string"
    );
    assert!(
        src.contains("has no document linkage"),
        "a node with no document id must be NAMED in a warning, not quietly given one"
    );
}

/// The corpus re-embed spells no payload-only key by hand.
///
/// Narrower than the scan applied to the two index paths, and deliberately so:
/// this module legitimately reads NODE PROPERTIES called `title` and `name`, and
/// that is not the same thing as spelling a payload key. What it must never do is
/// name a key that only exists in the stored payload — those belong to the
/// shared builder alone.
#[test]
fn the_corpus_re_embed_spells_no_payload_only_key() {
    let src = std::fs::read_to_string("src/services/embedding_pipeline.rs").expect("readable");
    for key in [
        "\"node_id\"",
        "\"node_type\"",
        "\"source_document\"",
        "\"page_number\"",
        "\"document_id\"",
    ] {
        assert!(
            !src.contains(key),
            "src/services/embedding_pipeline.rs names the payload key {key} — payload keys \
             belong to services::qdrant_payload, through pipeline::constants"
        );
    }
}
