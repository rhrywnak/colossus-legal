//! The guard, tested against an in-memory fixture — no database, no network.
//!
//! The gate the instruction sets is explicit: "a wrong `old_quote` STOPs; a
//! missing node STOPs; a count ≠ 76 rolls back". The first two are `guard`, and
//! live here; the third is `count_matches`, tested at the bottom.

use super::*;

/// The real first entry of `OCR_REPAIR_v1.json`'s `apply` array, used as the
/// fixture so the tests exercise the actual text — curly apostrophe, the `--`
/// interruption, the blank line Surya left mid-quote — and not a tidy invention.
fn fixture() -> Repair {
    Repair {
        id: "doc-hearing-10-14-2010:evidence:2f1758b8".to_string(),
        document: "doc-hearing-10-14-2010".to_string(),
        page: 5,
        how: "hand_read".to_string(),
        old_quote: "her. to--to owed money some there\u{2019}s And COURT: THE\n\nestate. the on claimants the one of She\u{2019}s".to_string(),
        new_quote: "THE COURT: And there's some money owed to--to her. She's one of the claimants on the estate.".to_string(),
    }
}

fn node_holding(quote: &str) -> Vec<NodeState> {
    vec![NodeState {
        source_document: "doc-hearing-10-14-2010".to_string(),
        page_number: Some(5),
        quote: quote.to_string(),
    }]
}

#[test]
fn the_untouched_node_passes() {
    let repair = fixture();
    assert_eq!(guard(&repair, &node_holding(&repair.old_quote)), Ok(()));
}

#[test]
fn whitespace_differences_alone_do_not_stop_the_write() {
    // Domain note: the quote is stored with the line breaks Surya emitted, and
    // an export/re-import round trip can turn `\n\n` into `\r\n\r\n` or a run of
    // spaces without a human touching a word. That is not tampering, so the
    // guard normalises before comparing — and ONLY whitespace is forgiven.
    let repair = fixture();
    let mangled = repair.old_quote.replace("\n\n", "  \r\n \t ");
    assert_eq!(guard(&repair, &node_holding(&mangled)), Ok(()));
}

#[test]
fn a_wrong_old_quote_stops_and_prints_both_texts() {
    let repair = fixture();
    let error = guard(&repair, &node_holding("Somebody retyped this card."))
        .expect_err("a changed quote must STOP");
    match &error {
        Stop::QuoteChanged {
            id,
            expected,
            actual,
        } => {
            assert_eq!(id, &repair.id);
            assert_eq!(actual, "Somebody retyped this card.");
            assert!(expected.starts_with("her. to--to owed"));
        }
        other => panic!("expected QuoteChanged, got {other:?}"),
    }
    // Rule 1: the operator must be able to see BOTH strings in the log line.
    let rendered = error.to_string();
    assert!(rendered.contains("Somebody retyped this card."));
    assert!(rendered.contains("her. to--to owed"));
}

#[test]
fn one_changed_character_is_enough_to_stop() {
    // The narrow case that matters: casefolding here would wave this through.
    let repair = fixture();
    // The stored text reads `... And COURT: THE`, so this is a real one-word
    // recasing of text that is actually there — not a no-op replace.
    let recased = repair.old_quote.replace("COURT", "Court");
    assert_ne!(recased, repair.old_quote);
    assert!(matches!(
        guard(&repair, &node_holding(&recased)),
        Err(Stop::QuoteChanged { .. })
    ));
}

#[test]
fn a_missing_node_stops_and_names_the_id() {
    let repair = fixture();
    let error = guard(&repair, &[]).expect_err("a missing node must STOP");
    assert_eq!(
        error,
        Stop::NotFound {
            id: repair.id.clone()
        }
    );
    assert!(error.to_string().contains(&repair.id));
}

#[test]
fn two_nodes_on_one_id_stop_separately_from_none() {
    let repair = fixture();
    let mut two = node_holding(&repair.old_quote);
    two.push(two[0].clone());
    let error = guard(&repair, &two).expect_err("a duplicated id must STOP");
    assert_eq!(
        error,
        Stop::NotUnique {
            id: repair.id.clone(),
            found: 2
        }
    );
    // Rule 1: the count has to survive into the text, or the operator cannot
    // tell "two nodes" from "eleven nodes" without re-running the query.
    let rendered = error.to_string();
    assert!(rendered.contains(&repair.id));
    assert!(
        rendered.contains('2'),
        "the duplicate count is missing: {rendered}"
    );
}

#[test]
fn the_wrong_document_stops_before_the_quote_is_compared() {
    let repair = fixture();
    let elsewhere = vec![NodeState {
        source_document: "doc-some-other-transcript".to_string(),
        page_number: Some(5),
        quote: repair.old_quote.clone(),
    }];
    let error = guard(&repair, &elsewhere).expect_err("the wrong document must STOP");
    assert_eq!(
        error,
        Stop::WrongDocument {
            id: repair.id.clone(),
            expected: repair.document.clone(),
            actual: "doc-some-other-transcript".to_string(),
        }
    );
    // Both document names have to reach the terminal: which one the audit meant
    // and which one the node actually sits on are different next actions.
    let rendered = error.to_string();
    assert!(rendered.contains(&repair.id));
    assert!(rendered.contains("doc-some-other-transcript"));
    assert!(rendered.contains(&repair.document));
}

#[test]
fn the_wrong_page_stops_before_the_quote_is_compared() {
    let repair = fixture();
    let wrong_page = vec![NodeState {
        source_document: repair.document.clone(),
        page_number: Some(6),
        quote: repair.old_quote.clone(),
    }];
    let error = guard(&repair, &wrong_page).expect_err("the wrong page must STOP");
    assert_eq!(
        error,
        Stop::WrongPage {
            id: repair.id.clone(),
            expected: 5,
            actual: Some(6),
        }
    );
    let rendered = error.to_string();
    assert!(rendered.contains(&repair.id));
    assert!(
        rendered.contains('5') && rendered.contains('6'),
        "{rendered}"
    );
}

#[test]
fn a_card_with_no_page_at_all_stops_and_says_so() {
    let repair = fixture();
    let no_page = vec![NodeState {
        source_document: repair.document.clone(),
        page_number: None,
        quote: repair.old_quote.clone(),
    }];
    let error = guard(&repair, &no_page).expect_err("a null page must STOP");
    assert_eq!(
        error,
        Stop::WrongPage {
            id: repair.id.clone(),
            expected: 5,
            actual: None,
        }
    );
    // A card with no page must not render as though it were on page 0.
    let rendered = error.to_string();
    assert!(
        rendered.contains("None"),
        "a null page must say so: {rendered}"
    );
    assert!(rendered.contains('5'));
}

#[test]
fn normalise_collapses_and_trims_but_never_casefolds() {
    assert_eq!(
        normalise("  THE   COURT: \n\n And\tso "),
        "THE COURT: And so"
    );
    assert_eq!(normalise(" \r\n\t "), "");
    assert_ne!(normalise("THE COURT"), normalise("the court"));
}

/// The B8 replica, pinned to the same §C1 worked examples the audit's own tests
/// use. If the two copies ever drift, this fails.
#[test]
fn the_b8_replica_matches_the_audit() {
    assert!(has_ocr_damage("that's not our busi-\nness."));
    assert!(has_ocr_damage("for--as--ashe's"));
    assert!(has_ocr_damage(
        "let Mr. Phillips finish his\n9\nexplanation"
    ));
    assert!(!has_ocr_damage(
        "The court ordered the $50,000 returned to the estate."
    ));
    assert!(!has_ocr_damage("a well-known conservator-ship question"));
    // And the corrected quote from the audit's own worked example still carries
    // the `--`, because that is how the court reporter writes an interruption.
    // Domain note: this is exactly why 60 cards are `false_alarm_dash_only` —
    // B8 will keep flagging them after the repair, correctly.
    assert!(has_ocr_damage(&fixture().new_quote));
}

#[test]
fn the_count_check_accepts_only_the_declared_number() {
    assert!(crate::count_matches(76, 76).is_ok());
    assert!(crate::count_matches(75, 76).is_err());
    assert!(crate::count_matches(77, 76).is_err());
    assert!(crate::count_matches(0, 76).is_err());
    // The message has to name both numbers or the operator cannot act on it.
    let message = crate::count_matches(75, 76)
        .expect_err("a short count must STOP")
        .to_string();
    assert!(message.contains("75") && message.contains("76"));
}
