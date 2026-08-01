//! Unit tests for [`super`] — the anchored-ruling write path.
//!
//! Everything here exercises the REFUSAL LAW and the anchor extraction, both of
//! which are pure: they take a `BiasInstance` and a `RulingKind` and decide. The
//! transactional write itself needs a live Postgres and a live Neo4j and is
//! covered by DEV verification, exactly as the sibling repository writes are.
//!
//! These are the tests that matter most in this task. The law they pin is the one
//! that stops an unanchorable ruling from being written, and an unanchorable
//! ruling written is precisely the 2026-07-24 data loss.

use super::*;
use crate::bias::dto::{ActorOption, DocumentRef};

/// A graph instance with everything an anchor needs. Individual tests knock out
/// the one field they are about, so each test's SUBJECT is the line it edits.
fn full_instance() -> BiasInstance {
    BiasInstance {
        evidence_id: "ev-1".to_string(),
        title: "Interrogatory answer 14".to_string(),
        verbatim_quote: Some("  I do NOT   recall  ".to_string()),
        question: None,
        page_number: Some(14),
        pattern_tags: Vec::new(),
        stated_by: Some(ActorOption {
            id: "actor-1".to_string(),
            name: "R. Phillips".to_string(),
            actor_type: "person".to_string(),
            tagged_statement_count: 3,
        }),
        about: Vec::new(),
        document: Some(DocumentRef {
            id: "doc-7".to_string(),
            title: "CFS interrogatory responses".to_string(),
            document_type: Some("discovery_response".to_string()),
        }),
    }
}

// ── extract_anchor: the happy path ────────────────────────────────────────────

#[test]
fn a_complete_instance_yields_a_complete_anchor() {
    let anchor = extract_anchor(&full_instance(), RulingKind::Include)
        .expect("a complete instance must anchor");

    assert_eq!(anchor.document_id, "doc-7");
    assert_eq!(anchor.page, Some(14));
    assert_eq!(anchor.speaker.as_deref(), Some("R. Phillips"));
    // The VERBATIM quote is preserved exactly as the graph held it, whitespace and
    // capitals included — it is what gets read aloud. Normalization happens at the
    // write, into a separate column, never in place.
    assert_eq!(
        anchor.quote_verbatim.as_deref(),
        Some("  I do NOT   recall  ")
    );
}

// ── extract_anchor: the document law ──────────────────────────────────────────

#[test]
fn an_item_with_no_document_cannot_be_ruled_on_at_all() {
    // Mandatory for EVERY ruling, including defer: an item with no source document
    // is not record evidence, and no anchor to it could ever be cited or matched.
    let mut instance = full_instance();
    instance.document = None;

    for kind in RulingKind::ALL {
        let result = extract_anchor(&instance, *kind);
        assert!(
            matches!(result, Err(RulingError::NoDocument)),
            "{} on a document-less item must be refused, got {result:?}",
            kind.code()
        );
    }
}

// ── extract_anchor: the citability law ────────────────────────────────────────

#[test]
fn include_and_exclude_are_refused_without_a_quote() {
    let mut instance = full_instance();
    instance.verbatim_quote = None;

    for kind in [RulingKind::Include, RulingKind::Exclude] {
        let result = extract_anchor(&instance, kind);
        assert!(
            matches!(result, Err(RulingError::NotCitable)),
            "{} on an unquotable item must be refused, got {result:?}",
            kind.code()
        );
    }
}

#[test]
fn a_whitespace_only_quote_is_no_quote() {
    // The subtle case: the column is populated, so a naive `is_some()` check would
    // pass it and store an anchor whose normalized quote is "". That empty
    // comparison surface would match everything or nothing in task 2.5 — worse than
    // an honest refusal, because it looks anchored.
    let mut instance = full_instance();
    instance.verbatim_quote = Some("   \n\t  ".to_string());

    assert!(matches!(
        extract_anchor(&instance, RulingKind::Include),
        Err(RulingError::NotCitable)
    ));
}

#[test]
fn defer_is_always_permitted_on_an_unquotable_item() {
    // This is the escape hatch the refusal message promises. If this ever fails,
    // a human meeting an unquotable candidate has NO legal move — they can neither
    // include it, exclude it, nor park it — and the workbench is stuck.
    let mut instance = full_instance();
    instance.verbatim_quote = None;

    let anchor =
        extract_anchor(&instance, RulingKind::Defer).expect("defer must survive a missing quote");
    assert_eq!(anchor.quote_verbatim, None);
    // The rest of the anchor is still captured — a defer records everything the
    // record does have, so 2.5 can still find it by document and page.
    assert_eq!(anchor.document_id, "doc-7");
    assert_eq!(anchor.page, Some(14));
}

#[test]
fn undrop_is_permitted_on_an_unquotable_item() {
    // Undrop returns a candidate to the pool; refusing it for lack of a quote would
    // trap the item in the dropped tray permanently.
    let mut instance = full_instance();
    instance.verbatim_quote = None;
    assert!(extract_anchor(&instance, RulingKind::Undrop).is_ok());
}

// ── extract_anchor: the optional fields ───────────────────────────────────────

#[test]
fn a_missing_page_is_recorded_not_refused() {
    // Page is nullable by law: a missing page is a degraded anchor, not a useless
    // one — document plus quote still identify the passage.
    let mut instance = full_instance();
    instance.page_number = None;

    let anchor = extract_anchor(&instance, RulingKind::Include).expect("no page is not a refusal");
    assert_eq!(anchor.page, None);
}

#[test]
fn documentary_evidence_with_no_speaker_anchors_cleanly() {
    // A letter or a record genuinely has no speaker. That must anchor, with the
    // absence recorded as a state rather than blocking the ruling.
    let mut instance = full_instance();
    instance.stated_by = None;

    let anchor =
        extract_anchor(&instance, RulingKind::Include).expect("no speaker is not a refusal");
    assert_eq!(anchor.speaker, None);
}

#[test]
fn an_empty_speaker_string_is_treated_as_absent() {
    // `evidence_by_ids` decodes a missing STATED_BY edge with `coalesce(…, '')`, so
    // an empty string IS the graph saying "no speaker". Storing it as a present
    // speaker whose name is "" would be an anchor asserting something false.
    let mut instance = full_instance();
    if let Some(actor) = instance.stated_by.as_mut() {
        actor.name = "   ".to_string();
    }

    let anchor = extract_anchor(&instance, RulingKind::Include).expect("anchors");
    assert_eq!(anchor.speaker, None);
}

// ── validate_reason ───────────────────────────────────────────────────────────

#[test]
fn a_defer_without_a_reason_is_refused() {
    let result = validate_reason(RulingKind::Defer, None);
    assert!(matches!(result, Err(RulingError::ReasonMismatch { .. })));
}

#[test]
fn a_blank_reason_does_not_satisfy_a_defer() {
    // Whitespace is not a reason. Accepting it would let the UI satisfy the law
    // with an empty box and reintroduce the very ambiguity defer exists to remove.
    let result = validate_reason(RulingKind::Defer, Some("   "));
    assert!(matches!(result, Err(RulingError::ReasonMismatch { .. })));
}

#[test]
fn a_reason_on_a_non_defer_is_refused() {
    // The vocabularies must not cross: a reason attached to an include would be
    // stored by neither the ledger's CHECK nor read by anything, so it is a caller
    // bug worth naming rather than silently dropping.
    for kind in [RulingKind::Include, RulingKind::Exclude, RulingKind::Undrop] {
        let result = validate_reason(kind, Some("changed my mind"));
        assert!(
            matches!(result, Err(RulingError::ReasonMismatch { .. })),
            "{} must not accept a defer reason",
            kind.code()
        );
    }
}

#[test]
fn a_defer_with_a_real_reason_passes() {
    assert!(validate_reason(RulingKind::Defer, Some("page missing from scan")).is_ok());
}

#[test]
fn the_other_kinds_pass_with_no_reason() {
    for kind in [RulingKind::Include, RulingKind::Exclude, RulingKind::Undrop] {
        assert!(validate_reason(kind, None).is_ok(), "{}", kind.code());
    }
}

// ── The refusal messages ──────────────────────────────────────────────────────

#[test]
fn the_citability_refusal_offers_defer_as_the_way_forward() {
    // A refusal that only says "no" strands the human on a candidate they still
    // have to deal with. The message must name the alternative, and it is ratified
    // wording — pinned so a later edit cannot quietly drop the second half.
    let message = RulingError::NotCitable.to_string();
    assert!(
        message.contains("no verbatim quote"),
        "the message must name the problem: {message}"
    );
    assert!(
        message.contains("defer"),
        "the message must offer the honest alternative: {message}"
    );
    assert!(
        message.contains("stays in the queue"),
        "the message must say the item is not lost: {message}"
    );
}

#[test]
fn the_node_gone_refusal_tells_the_human_what_to_do() {
    let message = RulingError::NodeGone.to_string();
    assert!(message.contains("reload"), "actionable: {message}");
}

#[test]
fn the_no_document_refusal_names_the_constraint() {
    // This message is the entire 400 body for an item with no source document.
    // If it were ever emptied or truncated the request would still fail — but the
    // human would get a blank explanation and no way to understand why a
    // legitimate-looking candidate cannot be ruled on.
    let message = RulingError::NoDocument.to_string();
    assert!(
        message.contains("no source document"),
        "the message must name what is missing: {message}"
    );
    assert!(
        message.contains("cited") || message.contains("recovered"),
        "the message must say WHY that matters: {message}"
    );
}

#[test]
fn the_defer_without_reason_refusal_explains_what_the_reason_is_for() {
    // `ReasonMismatch` carries a dynamically-built message, so nothing but an
    // assertion on the text stops a future edit from replacing it with something
    // that names no cause.
    let Err(error) = validate_reason(RulingKind::Defer, None) else {
        panic!("a reasonless defer must be refused");
    };
    let message = error.to_string();
    assert!(
        message.contains("must state a reason"),
        "the message must name what is missing: {message}"
    );
    assert!(
        message.contains("nobody has looked at"),
        "the message must explain why the reason is load-bearing — it is what \
         distinguishes a parked candidate from an untouched one: {message}"
    );
}

#[test]
fn the_reason_on_a_non_defer_refusal_names_the_offending_verb() {
    // The message interpolates the kind, so an operator (and the user) can see
    // WHICH ruling wrongly carried a reason rather than a generic complaint.
    let Err(error) = validate_reason(RulingKind::Include, Some("why")) else {
        panic!("a reason on an include must be refused");
    };
    let message = error.to_string();
    assert!(
        message.contains("only accompany a defer"),
        "the message must name the rule: {message}"
    );
    assert!(
        message.contains("include"),
        "the message must name the offending verb: {message}"
    );
}
