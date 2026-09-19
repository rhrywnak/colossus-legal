//! The backend half of the Settings page wire contract.
//!
//! ## Why a second contract file
//!
//! The Include contract exists because two languages disagreed about a REQUEST
//! body for months with both test suites green. This is the same hazard pointed
//! the other way: the Settings page is a RESPONSE whose shape the browser has to
//! know field for field, and the page renders 860-odd rows grouped by two fields
//! — `area_id` and `block_id` — that did not exist until today. A rename on
//! either side would not break a build. It would produce a page where every row
//! landed in "Undeclared — read by nothing", which looks like a data problem and
//! is not.
//!
//! `contracts/settings_page.json` is the shared artifact. This file asserts the
//! bytes deserialize as [`SettingsPageDto`] with nothing left over; the
//! frontend's `settingsContract.test.ts` asserts the same bytes survive its own
//! parse and group the way the page will group them.
//!
//! ## The `deny_unknown_fields` is doing real work here
//!
//! Every DTO on this page carries it, so a field ADDED on the backend and not
//! added to the contract fails this test rather than silently travelling. That is
//! the direction of drift a response contract has to catch: the server is the
//! side that grows fields.

use super::settings::SettingsPageDto;

/// The exact bytes the page is served.
fn contract_bytes() -> String {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../contracts/settings_page.json");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()))
}

/// The contract bytes are a whole settings page. (M)
#[test]
fn the_contract_bytes_deserialize_as_the_settings_page() {
    let body = contract_bytes();
    let page: SettingsPageDto = serde_json::from_str(body.trim())
        .unwrap_or_else(|e| panic!("the settings page contract must parse here: {e}\n{body}"));

    assert_eq!(page.settings.len(), 2);
    assert_eq!(page.areas.len(), 2);

    // The row that has moved off its default carries the phrase AND the old
    // value — the landing list is built from exactly this field.
    let cap = &page.settings[0];
    assert_eq!(cap.key, "talking_points_cap");
    assert_eq!(
        cap.changed_from_default.as_deref(),
        Some("Changed — default: 3")
    );
    assert_eq!(cap.area_id, "core");
    assert_eq!(cap.block_id, "core");
}

/// The dead group travels with its note, and its rows point at it. (M)
///
/// The ruling of 2026-09-19 is that rows no block declares are SHOWN, at the
/// bottom, labelled dead. That is a property of the response, so it is pinned in
/// the response contract rather than in a component.
#[test]
fn the_undeclared_group_travels_with_the_note_that_says_it_is_dead() {
    let page: SettingsPageDto =
        serde_json::from_str(contract_bytes().trim()).expect("the contract parses");

    let last = page.areas.last().expect("the rail is never empty");
    assert_eq!(last.id, "undeclared");
    assert!(
        last.note
            .as_deref()
            .is_some_and(|note| note.contains("dead")),
        "the group must carry its own explanation: a heading a human cannot \
         interpret is how these rows stayed invisible for a month"
    );

    let stray = &page.settings[1];
    assert_eq!(stray.area_id, last.id);
    assert!(
        stray.changed_from_default.is_none(),
        "a dead row sitting on its default must not appear in the landing list"
    );
}

/// A field the contract does not declare is refused. (M)
///
/// The anti-vacuity guard for this whole file: without it, a parse that quietly
/// ignored unknown fields would make the two tests above assertions about a
/// shape nobody was actually holding to.
#[test]
fn a_field_the_contract_does_not_declare_is_refused() {
    let body = contract_bytes();
    let widened = body.trim().replacen(
        r#"{"key":"talking_points_cap""#,
        r#"{"surprise":true,"key":"talking_points_cap""#,
        1,
    );
    assert_ne!(
        widened,
        body.trim(),
        "the mutation must actually change the bytes"
    );

    let parsed = serde_json::from_str::<SettingsPageDto>(&widened);
    assert!(
        parsed.is_err(),
        "an undeclared field must fail the parse — deny_unknown_fields is what \
         makes this contract a contract rather than a sample"
    );
}
