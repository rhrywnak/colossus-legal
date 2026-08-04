// Tests for `domain::link_cut` — task 2.10.
//
// The same five-test shape `HumanFactKind` carries, for the same reason: these
// tokens ARE the database column. A drift between an enum variant and its stored
// string would not fail to compile — it would quietly file a hazard as a weapon.

use super::*;

#[test]
fn cut_tokens_match_serde() {
    for &cut in LinkCut::ALL {
        assert_eq!(
            serde_json::json!(cut),
            serde_json::json!(cut.code()),
            "{} drifted from its serde token",
            cut.code()
        );
    }
}

#[test]
fn an_unknown_cut_is_a_loud_error_not_a_default() {
    // There is no flattering default here and that is the point: guessing
    // "supports" for an unreadable value would tell a lawyer a landmine is a
    // weapon.
    assert!(LinkCut::try_from("helpful").is_err());
    assert!(LinkCut::try_from("").is_err());
    assert!(LinkCut::try_from("SUPPORTS").is_err(), "tokens are exact");
}

#[test]
fn the_cut_parse_error_names_the_token_and_the_whole_vocabulary() {
    // `is_err()` proves the branch; only the message proves an operator reading
    // the log can see WHICH value the database held and what was allowed.
    let Err(error) = LinkCut::try_from("helpful") else {
        panic!("an unknown cut must be refused");
    };
    let message = error.to_string();
    assert!(message.contains("helpful"), "the bad token: {message}");
    for known in LinkCut::ALL {
        assert!(
            message.contains(known.code()),
            "{} must be offered: {message}",
            known.code()
        );
    }
}

#[test]
fn every_known_cut_round_trips() {
    for &cut in LinkCut::ALL {
        assert_eq!(
            LinkCut::try_from(cut.code()).expect("its own token parses"),
            cut
        );
    }
}

#[test]
fn the_cut_vocabulary_is_the_one_the_column_stores() {
    // Pinning the literals means a rename of an enum VARIANT cannot quietly
    // change what is already written in `evidence_allegation_links.cut`.
    assert_eq!(LinkCut::Supports.code(), "supports");
    assert_eq!(LinkCut::Against.code(), "against");
    assert_eq!(LinkCut::ALL.len(), 2);
    // The favourable reading is offered first, matching the panel's button order.
    assert_eq!(LinkCut::ALL[0], LinkCut::Supports);
}

// ── The ledger's vocabulary ──────────────────────────────────────────────────

#[test]
fn action_tokens_match_serde() {
    for &action in LinkAction::ALL {
        assert_eq!(
            serde_json::json!(action),
            serde_json::json!(action.code()),
            "{} drifted from its serde token",
            action.code()
        );
    }
}

#[test]
fn an_unknown_action_is_a_loud_error_naming_the_vocabulary() {
    let Err(error) = LinkAction::try_from("relinked") else {
        panic!("an unknown action must be refused");
    };
    let message = error.to_string();
    assert!(message.contains("relinked"), "{message}");
    for known in LinkAction::ALL {
        assert!(message.contains(known.code()), "{message}");
    }
}

#[test]
fn every_known_action_round_trips() {
    for &action in LinkAction::ALL {
        assert_eq!(
            LinkAction::try_from(action.code()).expect("its own token parses"),
            action
        );
    }
}

#[test]
fn only_an_unlink_carries_no_cut() {
    // The ledger's nullable `cut` column, as a rule stated once. An unlink that
    // recorded the old cut would read as though the withdrawal had asserted
    // something about the statement.
    assert!(LinkAction::Link.carries_cut());
    assert!(LinkAction::Recut.carries_cut());
    assert!(!LinkAction::Unlink.carries_cut());
    assert_eq!(LinkAction::ALL.len(), 3);
}

#[test]
fn the_action_vocabulary_is_the_one_the_column_stores() {
    assert_eq!(LinkAction::Link.code(), "link");
    assert_eq!(LinkAction::Recut.code(), "recut");
    assert_eq!(LinkAction::Unlink.code(), "unlink");
}
