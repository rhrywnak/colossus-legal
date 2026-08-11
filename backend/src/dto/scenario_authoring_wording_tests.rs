//! Wire-parity tests for the two authoring-wording DTOs (task R4, P1a + P8).
//!
//! ## What these exist to catch
//!
//! Task R2 added nine stored rows for the unified identity vocabulary. They
//! reached the domain block, the migration and the frontend type — and stopped
//! one layer short of [`ScenarioIdentityWordingDto`], the only struct that
//! actually crosses the wire. The scenario page then rendered `undefined` into
//! each of its four labels and shipped the identity block as four unlabelled
//! paragraphs, with every stated absence blank behind them.
//!
//! Nothing in the build said a word. The frontend type declares
//! `attack_label: string`; a field the server never serialises arrives
//! `undefined` and satisfies no compile-time check on either side of the wire.
//! The gap was found by a human reading the page.
//!
//! ## Why the test walks VALUES rather than listing field names
//!
//! A test that asserted "these nine fields are present" would be a second copy
//! of the mapping, maintained by the same hand that forgot the first one — it
//! would have been written complete and gone stale identically.
//!
//! So the fixture builds the wording block with **each key as its own value**,
//! serialises the DTO, and asserts every key's text is findable in the JSON.
//! Adding a stored row and forgetting to map it fails here by construction,
//! because the new key is in [`SCENARIO_AUTHORING_WORDING_KEYS`] and its value is
//! nowhere in the payload. That is the property worth protecting: not "the nine
//! fields of August 2026", but "no stored word the identity surface reads gets
//! left behind the wire again".

use crate::domain::wording_scenario_authoring::{
    build_scenario_authoring_wording, ScenarioAuthoringWording, SCENARIO_AUTHORING_WORDING_KEYS,
};
use crate::dto::scenario_authoring_wording::{create_wording, identity_wording};

/// A wording block whose every field carries its own KEY as its text.
///
/// ## Rust Learning: an infallible closure and the `Result` that proves it
///
/// `build_scenario_authoring_wording` is generic over the error its reader can
/// return (`impl Fn(&str) -> Result<String, E>`), which is what lets one builder
/// serve the settings store and a test fixture alike. This reader cannot fail —
/// it hands back the key it was given — so `E` is inferred from the `Ok` arm and
/// the `expect` below can never fire. It is written as a `Result` anyway rather
/// than as a second infallible builder, because a fixture that took a different
/// code path from production would stop testing production's path.
fn keys_as_values() -> ScenarioAuthoringWording {
    build_scenario_authoring_wording(|key| Ok::<String, std::convert::Infallible>(key.to_string()))
        .expect("the reader above returns Ok for every key")
}

/// The stored rows that deliberately do NOT ride the identity payload.
///
/// Each one is withheld for a stated reason, not by oversight:
///
/// * the seven `scenario_create_*` rows belong to the create form on the Trial
///   Prep dashboard, which is a different dialog on a different page;
/// * `scenario_no_target_notice` rides the gather and cards payloads instead,
///   because there its PRESENCE is the signal that no target is set — a client
///   holding it unconditionally could render it beside a full queue;
/// * `scenario_never_scanned_notice` rides the cards payload for the same
///   reason.
///
/// Adding a key here is how a future author says "withheld on purpose". Adding
/// one by accident is what the test below refuses to let happen quietly.
const NOT_ON_THE_IDENTITY_PAYLOAD: &[&str] = &[
    "scenario_create_target_label",
    "scenario_create_target_helper",
    "scenario_create_target_unset_option",
    "scenario_create_accusation_label",
    "scenario_create_accusation_helper",
    "scenario_create_target_required_refusal",
    "scenario_create_accusation_required_refusal",
    "scenario_no_target_notice",
    "scenario_never_scanned_notice",
];

#[test]
fn the_identity_payload_carries_every_word_the_identity_surface_reads() {
    let wording = keys_as_values();
    let json = serde_json::to_string(&identity_wording(&wording))
        .expect("the DTO derives Serialize and holds only Strings");

    let missing: Vec<&str> = SCENARIO_AUTHORING_WORDING_KEYS
        .iter()
        .copied()
        .filter(|key| !NOT_ON_THE_IDENTITY_PAYLOAD.contains(key))
        // The fixture put each key IN as its own value, so finding the key's text
        // in the JSON is finding the mapped field. A key that is absent was read
        // from the store, built into the domain block, and then dropped at the
        // wire — which is exactly the P1a defect.
        .filter(|key| !json.contains(*key))
        .collect();

    assert!(
        missing.is_empty(),
        "these stored rows never reach the browser — map them in `identity_wording`, \
         or list them in NOT_ON_THE_IDENTITY_PAYLOAD with a reason: {missing:?}",
    );
}

#[test]
fn the_nine_unified_identity_rows_are_on_the_wire_by_name() {
    // The walking test above proves nothing was dropped. This one pins the four
    // labels and their four absences by the names the browser destructures, so a
    // RENAME on this side fails here rather than in front of a reader — a
    // renamed field serialises perfectly and arrives as `undefined` just the
    // same, which is the second half of how P1a stayed invisible.
    let dto = identity_wording(&keys_as_values());

    assert_eq!(dto.attack_label, "scenario_identity_attack_label");
    assert_eq!(dto.attack_absent, "scenario_identity_attack_absent");
    assert_eq!(dto.theme_label, "scenario_identity_theme_label");
    assert_eq!(dto.theme_absent, "scenario_identity_theme_absent");
    assert_eq!(dto.theme_helper, "scenario_identity_theme_helper");
    assert_eq!(dto.motivation_label, "scenario_identity_motivation_label");
    assert_eq!(dto.motivation_absent, "scenario_identity_motivation_absent");
    assert_eq!(dto.bears_on_label, "scenario_identity_bears_on_label");
    assert_eq!(dto.bears_on_absent, "scenario_identity_bears_on_absent");
}

#[test]
fn the_create_form_still_gets_its_own_seven_and_nothing_else() {
    // The sibling mapping, checked for the same disease in the other direction:
    // the create form must not silently acquire identity vocabulary, which would
    // put "what they say" on a dialog that has no such field to label.
    let dto = create_wording(&keys_as_values());
    let json = serde_json::to_string(&dto).expect("Strings only");

    assert!(json.contains("scenario_create_target_label"));
    assert!(json.contains("scenario_create_accusation_required_refusal"));
    assert!(
        !json.contains("scenario_identity_"),
        "the create form is speaking the identity modal's words: {json}",
    );
}
