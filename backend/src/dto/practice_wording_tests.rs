// Tests for `dto::practice_wording`.
//
// Moved out of the module on 2026-08-23, when L3's twenty-three wording keys
// took the mirror to exactly Rule 17's limit. The seam is the honest one: the
// ## ⚑ THIS IS NOT A GENERAL ESCAPE VALVE
//
// Moving tests out of a module is a way to make ANY file pass Rule 17 forever,
// and that is not what happened here. It is legitimate in this ONE case for a
// reason the next module cannot borrow: the struct genuinely CANNOT be split by
// content. The
// mirror is ONE struct — one field per stored key, by design, because the
// browser has one page and should not have to know which backend block a
// sentence lives in — so it cannot be split by content. Its tests can.
use super::*;
use crate::domain::wording_practice::PracticeWording;
use crate::domain::wording_practice::PRACTICE_WORDING_KEYS;
use crate::domain::wording_practice_editor::PRACTICE_EDITOR_WORDING_KEYS;
use crate::domain::wording_practice_flow::PRACTICE_FLOW_WORDING_KEYS;
use crate::domain::wording_practice_list::PRACTICE_LIST_WORDING_KEYS;
use crate::domain::wording_practice_print::PRACTICE_PRINT_WORDING_KEYS;
use crate::domain::wording_practice_report::PracticeReportWording;
use crate::domain::wording_practice_report::PRACTICE_REPORT_WORDING_KEYS;
use crate::domain::wording_practice_row::PRACTICE_ROW_WORDING_KEYS;

fn mirror() -> PracticeWordingDto {
    PracticeWordingDto::from_blocks(
        &PracticeWording::for_test(),
        &PracticeReportWording::for_test(),
    )
}

/// The mirror carries every declared key from BOTH blocks — so a field added
/// to either and forgotten here fails at `cargo test` rather than as an
/// `undefined` in the middle of Marie's session.
#[test]
fn the_mirror_carries_every_declared_key_from_both_blocks() {
    let value = serde_json::to_value(mirror()).expect("the mirror serializes");
    assert_eq!(
        value.as_object().expect("an object body").len(),
        PRACTICE_WORDING_KEYS.len()
            + PRACTICE_FLOW_WORDING_KEYS.len()
            + PRACTICE_REPORT_WORDING_KEYS.len()
            + PRACTICE_PRINT_WORDING_KEYS.len()
            + PRACTICE_ROW_WORDING_KEYS.len()
            + PRACTICE_EDITOR_WORDING_KEYS.len()
            + PRACTICE_LIST_WORDING_KEYS.len()
    );
}

/// Every wire name is a stored key without its `practice_` prefix, and comes
/// from one of the two blocks.
#[test]
fn every_wire_key_is_a_stored_key_without_its_prefix() {
    let value = serde_json::to_value(mirror()).expect("the mirror serializes");
    for key in value.as_object().expect("an object body").keys() {
        let stored = format!("practice_{key}");
        assert!(
            PRACTICE_WORDING_KEYS.contains(&stored.as_str())
                || PRACTICE_FLOW_WORDING_KEYS.contains(&stored.as_str())
                || PRACTICE_REPORT_WORDING_KEYS.contains(&stored.as_str())
                || PRACTICE_PRINT_WORDING_KEYS.contains(&stored.as_str())
                || PRACTICE_ROW_WORDING_KEYS.contains(&stored.as_str())
                || PRACTICE_EDITOR_WORDING_KEYS.contains(&stored.as_str())
                || PRACTICE_LIST_WORDING_KEYS.contains(&stored.as_str()),
            "wire field '{key}' implies stored key '{stored}', which is not declared",
        );
    }
}

/// No wire value is blank.
///
/// The fixtures are built through the production builders, which refuse a
/// blank row — so this asserts the CLONE did not lose one. A `String::new()`
/// left by a mapping line would render an empty button on a witness screen,
/// and nothing else in the stack would notice.
#[test]
fn no_string_arrives_blank() {
    let value = serde_json::to_value(mirror()).expect("the mirror serializes");
    for (key, v) in value.as_object().expect("an object body") {
        assert!(
            !v.as_str().unwrap_or_default().trim().is_empty(),
            "{key} arrived blank"
        );
    }
}
