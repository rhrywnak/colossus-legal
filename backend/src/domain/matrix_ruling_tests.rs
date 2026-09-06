// Tests for `domain::matrix_ruling`.
//
// Two closed vocabularies whose whole job is to refuse what they do not know.
// Every assertion below is about a value that crosses a boundary — a stored
// token, a wire token, or a rule the writer and the reader must agree on.

use super::*;

/// Every `code()` round-trips through `try_from`. A token written by this build
/// and unreadable by it would be a row nobody can ever display.
#[test]
fn every_ruling_code_parses_back_to_itself() {
    for ruling in MatrixRuling::ALL {
        let code = ruling.code();
        assert_eq!(
            MatrixRuling::try_from(code),
            Ok(*ruling),
            "'{code}' did not round-trip"
        );
    }
}

/// The stored tokens are pinned by literal, because the DATABASE holds them:
/// renaming one in code without a migration would orphan every existing row, and
/// the orphan would read as "unruled" — the machine's order restored over a
/// human's decision, silently.
#[test]
fn the_stored_ruling_tokens_are_exactly_keep_and_remove() {
    assert_eq!(MatrixRuling::Keep.code(), "keep");
    assert_eq!(MatrixRuling::Remove.code(), "remove");
    assert_eq!(MatrixRuling::ALL.len(), 2);
}

/// An unknown token is refused and NAMES itself, so an operator reading the log
/// learns which value the database holds rather than only that one is bad.
#[test]
fn an_unknown_ruling_token_is_refused_by_name() {
    let err = MatrixRuling::try_from("delete").expect_err("'delete' is not a ruling");
    assert_eq!(err.token, "delete");
    assert!(err.to_string().contains("delete"));
    assert!(err.to_string().contains("keep/remove"));
}

/// The empty string is refused like any other unknown token. Called out on its
/// own because a NOT NULL column can still hold `''`, and "" is what a
/// half-written client sends.
#[test]
fn the_empty_ruling_token_is_refused() {
    assert!(MatrixRuling::try_from("").is_err());
}

/// Exactly one of the two verdicts hides its item. This is the rule §3's hide
/// list, the export's filter and the hidden-count all read; if it ever answered
/// `true` for `Keep`, a confirmed item would vanish from a proof surface.
#[test]
fn only_remove_hides_the_item() {
    assert!(MatrixRuling::Remove.hides_the_item());
    assert!(!MatrixRuling::Keep.hides_the_item());
}

/// Every action `code()` round-trips. Same argument as the ruling tokens: the
/// ledger is append-only, so a token this build cannot read is a permanent hole
/// in the record of who decided what.
#[test]
fn every_action_code_parses_back_to_itself() {
    for action in RulingAction::ALL {
        let code = action.code();
        assert_eq!(
            RulingAction::try_from(code),
            Ok(*action),
            "'{code}' did not round-trip"
        );
    }
}

/// The three ledger tokens, pinned by literal for the same reason as the two
/// above.
#[test]
fn the_stored_action_tokens_are_rule_rerule_withdraw() {
    assert_eq!(RulingAction::Rule.code(), "rule");
    assert_eq!(RulingAction::Rerule.code(), "rerule");
    assert_eq!(RulingAction::Withdraw.code(), "withdraw");
    assert_eq!(RulingAction::ALL.len(), 3);
}

/// An unknown action is refused by name.
#[test]
fn an_unknown_action_token_is_refused_by_name() {
    let err = RulingAction::try_from("undo").expect_err("'undo' is not an action");
    assert_eq!(err.token, "undo");
    assert!(err.to_string().contains("rule/rerule/withdraw"));
}

/// Only a withdrawal leaves no verdict behind. This is the rule the ledger's
/// nullable `ruling` column encodes — the writer asks this function instead of
/// remembering it, so a fourth action added later cannot forget to answer.
#[test]
fn only_a_withdrawal_carries_no_ruling() {
    assert!(RulingAction::Rule.carries_ruling());
    assert!(RulingAction::Rerule.carries_ruling());
    assert!(!RulingAction::Withdraw.carries_ruling());
}

/// The two vocabularies do not overlap. A shared token would make one column's
/// value readable as the other's, and the two columns sit side by side in the
/// ledger.
#[test]
fn the_ruling_and_action_vocabularies_are_disjoint() {
    for ruling in MatrixRuling::ALL {
        assert!(
            RulingAction::try_from(ruling.code()).is_err(),
            "'{}' is both a ruling and an action",
            ruling.code()
        );
    }
}

/// The wire form is the stored form. The browser sends `{"ruling":"keep"}` and
/// the column holds `keep`; a serde rename that drifted from `code()` would make
/// the API and the database disagree about the same two words.
#[test]
fn the_wire_token_is_the_stored_token() {
    for ruling in MatrixRuling::ALL {
        let json = serde_json::to_string(ruling).expect("a ruling serializes");
        assert_eq!(json, format!("\"{}\"", ruling.code()));
    }
}
