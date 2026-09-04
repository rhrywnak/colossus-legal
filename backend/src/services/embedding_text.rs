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

use std::borrow::Cow;
use std::collections::HashMap;

// STRUCTURAL: nomic-embed-text's asymmetric prefixes. The model was trained
// with these two exact strings, so they are model protocol, not a setting —
// and they are a MATCHED PAIR. Text stored under `search_document:` must be
// searched with `search_query:`; using the same prefix on both sides, or
// omitting one, still returns vectors and still returns results, just worse
// ones. There is no error and no empty list to notice, which is why they are
// named here and asserted against each other in a test rather than typed out
// at each of the fourteen call sites.
pub const DOCUMENT_PREFIX: &str = "search_document: ";
pub const QUERY_PREFIX: &str = "search_query: ";

/// Build the embedding text for a node based on its type and properties.
///
/// Returns a "search_document: ..." prefixed string ready for embedding.
/// If the resulting text is empty after trimming, falls back to
/// "search_document: {node_type}" so we never produce an empty embedding.
pub fn build_embedding_text(node_type: &str, props: &HashMap<String, String>) -> String {
    let text = match node_type {
        "Evidence" => evidence_text(props),

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

/// The Evidence arm, lifted out of the match.
///
/// It is the only arm with a rule rather than a format string, and lifting it
/// keeps that rule beside [`compose_request_and_answer`] where its reasoning
/// lives — the other six arms stay one line each and say everything they do.
fn evidence_text(props: &HashMap<String, String>) -> String {
    format!(
        "search_document: {}. {}. Significance: {}",
        get_prop(props, "title"),
        compose_request_and_answer(
            get_prop(props, "question"),
            get_prop(props, "verbatim_quote"),
        ),
        get_prop(props, "significance"),
    )
}

/// Put a discovery response back together with the request it answers.
///
/// ## Domain note: why "Admitted." is not a searchable sentence
///
/// 367 Evidence cards carry a `question` — the interrogatory or request for
/// admission the card answers — and 99 of those have an answer-only
/// `verbatim_quote`: `Admitted.`, `Denied as untrue.`, `No.` Standing alone
/// that is a correct extraction of a sworn discovery response and completely
/// unretrievable: the vector for "Admitted." is the vector for every other
/// "Admitted.", and it carries nothing about the $50,000 check the request
/// actually named. Measured 2026-09-01: six of the seven $50,000 admissions were
/// reachable only through their `title`.
///
/// The card DTO and the scan judge already show the two together. This is the
/// same pairing, for the text that gets embedded.
///
/// ## The rule
///
/// A non-blank `question` yields `Request: {question} Answer: {quote}`. A blank
/// or absent one yields the quote unchanged — byte for byte, which
/// `pinned_evidence_text_without_a_question_2026_09_04` exists to hold, because
/// every vector already in Qdrant was built from that exact text.
///
/// ## Rust Learning: `Cow<str>` — borrow when you can, own when you must
///
/// The no-question path has nothing to build: the answer is the string that was
/// passed in, and copying it to satisfy a `String` return type would allocate
/// once per card on the commonest path (842 of 1,209 cards have no question).
/// `Cow` — "clone on write" — lets the two arms return different things through
/// one type: `Cow::Borrowed` hands back the caller's own `&str` with no
/// allocation, `Cow::Owned` hands back the `format!`ed one. Both deref to `&str`,
/// so `format!` at the call site cannot tell them apart.
pub fn compose_request_and_answer<'a>(question: &str, quote: &'a str) -> Cow<'a, str> {
    if question.trim().is_empty() {
        Cow::Borrowed(quote)
    } else {
        // STRUCTURAL: this is the embedding's retrieval protocol, not a setting.
        // Changing the two labels or the spacing changes the text every Evidence
        // vector is built from, which makes the stored vectors and the newly
        // built ones incomparable — the exact failure the pinned tests in this
        // file exist to catch. It varies by a deliberate re-embed decision, never
        // by deployment, so a config knob here would be a way to break retrieval
        // by editing YAML.
        Cow::Owned(format!("Request: {question} Answer: {quote}"))
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

    /// The defect this task fixes, in one assertion: an answer-only card whose
    /// request names the $50,000 check must carry the figure into its vector.
    #[test]
    fn an_answer_only_card_carries_its_request_into_the_embedding() {
        let mut props = HashMap::new();
        props.insert("title".into(), "Phillips RFA 12".into());
        props.insert(
            "question".into(),
            "Admit that the $50,000 check was an asset of the estate.".into(),
        );
        props.insert("verbatim_quote".into(), "Admitted.".into());
        props.insert("significance".into(), "Concedes the check.".into());

        let text = build_embedding_text("Evidence", &props);
        assert_eq!(
            text,
            concat!(
                "search_document: Phillips RFA 12. ",
                "Request: Admit that the $50,000 check was an asset of the estate. ",
                "Answer: Admitted.. Significance: Concedes the check."
            )
        );
        // The two things a reader of the vector store actually needs to be true.
        assert!(text.contains("$50,000"));
        assert!(text.contains("Admitted."));
    }

    /// The guard on the pinned path: adding a `question` key must be the ONLY
    /// thing that changes the Evidence text. Same props with and without it.
    #[test]
    fn the_question_is_the_only_difference_from_the_pinned_text() {
        let mut without = HashMap::new();
        without.insert("title".into(), "Phillips Q73".into());
        without.insert("verbatim_quote".into(), "I took the money.".into());
        without.insert("significance".into(), "Admission of conversion.".into());

        let mut with = without.clone();
        with.insert("question".into(), "Where did the money go?".into());

        let plain = build_embedding_text("Evidence", &without);
        let composed = build_embedding_text("Evidence", &with);
        assert_ne!(plain, composed);
        assert_eq!(
            plain,
            "search_document: Phillips Q73. I took the money.. Significance: Admission of conversion."
        );
        assert_eq!(
            composed,
            "search_document: Phillips Q73. Request: Where did the money go? Answer: I took the \
             money.. Significance: Admission of conversion."
        );
    }

    /// A blank or whitespace-only question is the same as no question at all.
    /// It matters because the graph read omits empty properties but a future
    /// caller building the map by hand may not.
    #[test]
    fn a_blank_question_leaves_the_text_exactly_as_it_was() {
        let mut base = HashMap::new();
        base.insert("title".into(), "Phillips Q73".into());
        base.insert("verbatim_quote".into(), "I took the money.".into());
        base.insert("significance".into(), "Admission of conversion.".into());
        let pinned = build_embedding_text("Evidence", &base);

        for blank in ["", "   ", "\n\t "] {
            let mut props = base.clone();
            props.insert("question".into(), blank.into());
            assert_eq!(
                build_embedding_text("Evidence", &props),
                pinned,
                "a {blank:?} question must not change the text"
            );
        }
    }

    /// `question` reaches ONLY the Evidence arm. An Allegation or a Person that
    /// happened to carry the property must embed exactly as it does today.
    #[test]
    fn the_question_does_not_leak_into_other_node_types() {
        let mut props = HashMap::new();
        props.insert("title".into(), "T".into());
        props.insert("allegation".into(), "A".into());
        props.insert("verbatim_quote".into(), "Q".into());
        props.insert("claim_text".into(), "C".into());
        props.insert("significance".into(), "S".into());
        props.insert("description".into(), "D".into());
        props.insert("name".into(), "N".into());
        props.insert("role".into(), "R".into());
        props.insert("question".into(), "Should not appear.".into());

        for node_type in [
            "ComplaintAllegation",
            "MotionClaim",
            "Harm",
            "Document",
            "Person",
            "Organization",
            "Whatever",
        ] {
            assert!(
                !build_embedding_text(node_type, &props).contains("Should not appear"),
                "{node_type} leaked the question"
            );
        }
    }

    /// The pure rule, on its own, including the borrow/own split.
    #[test]
    fn compose_borrows_when_there_is_no_question_and_owns_when_there_is() {
        assert!(matches!(
            compose_request_and_answer("", "Admitted."),
            Cow::Borrowed("Admitted.")
        ));
        assert!(matches!(
            compose_request_and_answer("   ", "Admitted."),
            Cow::Borrowed("Admitted.")
        ));
        assert_eq!(
            compose_request_and_answer("Admit the check.", "Admitted."),
            "Request: Admit the check. Answer: Admitted."
        );
        // An empty quote with a real question is still worth composing: the
        // request is the only retrievable text such a card has.
        assert_eq!(
            compose_request_and_answer("Admit the check.", ""),
            "Request: Admit the check. Answer: "
        );
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
