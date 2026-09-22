// What comes back, pinned — v2.2.1's half: no citation key in the prose Marie
// reads (CC_TASK_PRACTICE_FIXES_v2.2.1, Fix 3).
//
// A sibling of `practice_read_parse_tests.rs` rather than more of it: that file
// was already over Rule 17's 300 lines before this task, and the guard is one
// rule with its own tests — the `practice_read_payload_addendum_tests` precedent.

use super::*;

fn rules() -> ReadRules<'static> {
    ReadRules {
        max_words_call: 12,
        max_words_why: 55,
        max_words_pointer: 20,
        max_pointers: 3,
        max_words_after_fine: 6,
        fine_token: "Fine.",
    }
}

fn citable() -> BTreeSet<String> {
    ["P1", "P2", "P3", "R1", "R2", "R3", "S1", "S2"]
        .iter()
        .map(|k| (*k).to_string())
        .collect()
}

fn parse(raw: &str) -> Result<(ReadReply, Vec<Overrun>), ReplyRejection> {
    parse_reply(raw, rules(), &citable())
}

fn parts_of(reply: ReadReply) -> ReadParts {
    match reply {
        ReadReply::Parts(parts) => parts,
        ReadReply::Abstain(reason) => panic!("expected a judgement, got an abstain: {reason}"),
    }
}

/// The reply Roman screenshotted on DEV: keys in the prose Marie reads.
///
/// "they argued it (S1), and R2 says the property fight drove the fees (P2)"
/// is three labels that mean nothing to her. The prompt forbids it; this is the
/// check that does not trust the prompt.
#[test]
fn a_reply_naming_a_key_in_why_is_rejected() {
    let rejection = parse(
        r#"{"call": "You conceded the fees.", "why": "They argued it, and R2 says the property fight drove the fees.", "pointers": [], "keys": ["R2"]}"#,
    )
    .expect_err("a key in `why` is not shown to Marie");
    assert_eq!(
        rejection,
        ReplyRejection::KeyInProse {
            part: "why".to_string(),
            token: "R2".to_string()
        }
    );
}

/// The same guard on the call.
#[test]
fn a_reply_naming_a_key_in_the_call_is_rejected() {
    let rejection = parse(r#"{"call": "You skipped S1.", "keys": ["S1"]}"#)
        .expect_err("a key in `call` is not shown to Marie");
    assert_eq!(
        rejection,
        ReplyRejection::KeyInProse {
            part: "call".to_string(),
            token: "S1".to_string()
        }
    );
}

/// And on each pointer, named by its position — the correction quotes it.
#[test]
fn a_reply_naming_a_key_in_a_pointer_is_rejected_and_the_pointer_is_named() {
    let rejection = parse(
        r#"{"call": "You let the braid stand.", "pointers": ["Take the second question only.", "Lead with P3."], "keys": ["P3"]}"#,
    )
    .expect_err("a key in a pointer is not shown to Marie");
    assert_eq!(
        rejection,
        ReplyRejection::KeyInProse {
            part: "pointer 2".to_string(),
            token: "P3".to_string()
        }
    );
}

/// A reply wrong in both ways is sent back for the one Marie would SEE.
#[test]
fn a_key_in_the_prose_is_named_before_an_unknown_key() {
    let rejection = parse(r#"{"call": "Cite R9.", "keys": ["R9"]}"#).expect_err("two faults");
    assert!(
        matches!(rejection, ReplyRejection::KeyInProse { .. }),
        "{rejection:?}"
    );
}

/// Keys in `keys` are where they belong, and a clean read passes.
#[test]
fn keys_only_in_the_keys_field_are_accepted() {
    let (parsed, _) = parse(
        r#"{"call": "You let the braid stand.", "why": "Their own sworn answer says there were no such conversations.", "pointers": ["Put the date on it — you have the certified letter of 16 Nov 2009."], "keys": ["S2", "R1"]}"#,
    )
    .expect("keys in `keys` only");
    assert_eq!(parts_of(parsed).keys, vec!["S2", "R1"]);
}

/// A word that merely CONTAINS the shape is prose, not a key.
#[test]
fn a_key_like_word_is_not_a_key() {
    for prose in [
        "PR2x is not a key.",
        "S1mple words.",
        "Section 12 of the statute.",
        "R2D2 and XR2 are not keys either.",
        "P is a letter; 12 is a number.",
        "éR2 is one word.",
        "",
    ] {
        assert_eq!(find_key_token(prose, KEY_FAMILIES), None, "{prose:?}");
    }
    for (prose, key) in [
        ("R2", "R2"),
        ("(S1)", "S1"),
        ("drove the fees (P2).", "P2"),
        ("see R12, then R3", "R12"),
        ("R2—the fee order", "R2"),
    ] {
        assert_eq!(find_key_token(prose, KEY_FAMILIES), Some(key), "{prose:?}");
    }
}

/// The scanner IS `\b[PRS]\d+\b`, checked against the real `regex` crate.
///
/// `regex` is a dev-dependency only, so the production guard is a scanner; this
/// test is what keeps the two readings of the pattern from drifting.
#[test]
fn the_scanner_agrees_with_the_regex_it_implements() {
    let pattern = regex::Regex::new(r"\b[PRS][0-9]+\b").expect("test pattern compiles");
    let corpus = [
        "They argued it (S1), and R2 says the property fight drove the fees (P2).",
        "PR2x S1mple Section 12 R2D2 XR2 P 12",
        "Lead with P3.",
        "R10 and R1",
        "_R2 R2_ R2",
        "Put the date on it — you have the letter of 16 Nov 2009.",
        "S",
        "S1",
        "1S1",
        "a-R7-b",
        "éR2 R2é",
    ];
    for text in corpus {
        let expected = pattern.find(text).map(|m| m.as_str());
        assert_eq!(find_key_token(text, KEY_FAMILIES), expected, "{text:?}");
    }
}

/// Every key the payload can hand out is caught by the guard.
///
/// A real citable set from the payload fixture (`S1`/`S2` minted by
/// `citable_sources`, `P`/`R` in the shape `practice_read_gather` formats). A
/// fourth key family added to the payload without a letter in the guard fails
/// here instead of reaching Marie's screen.
#[test]
fn every_citable_key_matches_the_prose_guard() {
    let payload = crate::services::practice_read_payload::tests::cross_payload();
    let keys = payload.citable_keys();
    assert!(
        keys.len() >= 8,
        "the fixture carries every family: {keys:?}"
    );
    for key in &keys {
        let prose = format!("As {key} shows.");
        assert_eq!(
            find_key_token(&prose, KEY_FAMILIES),
            Some(key.as_str()),
            "{key} slips past the guard"
        );
    }
}

/// The scanner holds no domain letters: the caller's families decide.
#[test]
fn the_scanner_reads_only_the_families_it_is_given() {
    assert_eq!(find_key_token("See Q7 and R2.", &['Q']), Some("Q7"));
    assert_eq!(find_key_token("See Q7 and R2.", &[]), None);
    assert_eq!(find_key_token("See Q7 and R2.", KEY_FAMILIES), Some("R2"));
}
