// Tests for `domain::wording_scenario_authoring`.
//
// ## Why there is only one behavioural test here
//
// Roman's standing law of 2026-08-07: no test that would not fail when
// user-visible behavior breaks. A test asserting `KEYS.len() == 13`, or that
// each field reads its own key, restates the code — the compiler already
// enforces both.
//
// The test below is not that. `SCENARIO_AUTHORING_WORDING_KEYS` is what the boot
// loader demands rows for, and a declared key the migration does not seed makes
// the backend REFUSE TO START (v2 §2b — there are no compiled-in defaults). So
// this test catches a deploy that would take DEV down, by reading the migration
// file off disk and comparing it against the fixture the rest of the suite runs
// on. It is the disk/code consistency pattern of Rule 21.

use super::*;
use crate::domain::wording::tests::seeded_value_in;
use std::collections::HashMap;

/// The migrations that seed these rows, concatenated before the search.
///
/// Three files since task R1: the original thirteen, the never-scanned notice
/// that arrived with the scan work whose defect it answers, and .390's two — the
/// second definition-loss refusal and the gated rehearsal control's reason.
/// Concatenated rather than searched one at a time because WHERE a row is seeded
/// is a fact about migration history — only its VALUE is what this test pins.
const SEED_MIGRATIONS: &[&str] = &[
    // PRACTICE v0 (2026-08-17): the one string this block gained from another
    // task — the label on the control that opens Marie's drill. It is seeded by
    // the practice migration because that is the task that introduced it; it is
    // READ from this block because the scenario page is what speaks it.
    "pipeline_migrations/20260817213319_practice_session_v0.sql",
    "pipeline_migrations/20260807155419_scenario_authoring_wording.sql",
    "pipeline_migrations/20260808084539_theme_scan_tier2_settings_and_scan_wording.sql",
    "pipeline_migrations/20260810094435_r1_390_rehearsal_gate_wording_and_response_uniqueness.sql",
    "pipeline_migrations/20260810114629_r2_391_unified_names_one_attack_box_and_scan_default_model.sql",
];

/// The seeded values, for TESTS ONLY — kept beside the test that pins them to
/// the migration file, so a fixture and its proof cannot drift apart.
const TEST_SEED: &[(&str, &str)] = &[
    (KEY_CREATE_TARGET_LABEL, "Who this scenario is about"),
    (
        KEY_CREATE_TARGET_HELPER,
        "Evidence is gathered about this person and nobody else. A scenario \
         with no target gathers nothing.",
    ),
    (KEY_CREATE_TARGET_UNSET, "Choose a person…"),
    (
        KEY_CREATE_ACCUSATION_LABEL,
        "The accusation, in plain language",
    ),
    (
        KEY_CREATE_ACCUSATION_HELPER,
        "What the other side is actually saying about this person. The scan \
         judges every candidate fact against these words, so write them the way \
         you would say them out loud.",
    ),
    (
        KEY_CREATE_TARGET_REQUIRED,
        "Choose who this scenario is about — a scenario with no target cannot \
         gather any evidence.",
    ),
    (
        KEY_CREATE_ACCUSATION_REQUIRED,
        "Write the accusation in plain language — without it the scan has \
         nothing to judge candidate facts against.",
    ),
    (
        KEY_NO_TARGET_NOTICE,
        "No target defined — this scenario cannot gather evidence. Use Edit \
         identity to name who it is about.",
    ),
    (
        KEY_NEVER_SCANNED_NOTICE,
        "No scan has run yet. Run a theme scan above, or browse the raw evidence \
         pool.",
    ),
    (KEY_IDENTITY_TARGET_LABEL, "Who this scenario is about"),
    (
        KEY_IDENTITY_TARGET_HELPER,
        "Changing this changes which evidence the scenario gathers. Rulings you \
         have already made are kept.",
    ),
    (KEY_IDENTITY_TARGET_UNSET, "Not set — gathers nothing"),
    (
        KEY_TARGET_OPTIONS_FAILED,
        "Could not load the people in this case. Close and reopen — do not \
         save, or this scenario's target would be cleared.",
    ),
    (
        KEY_TARGET_NEEDS_ATTACK_TEXT,
        "Write what they say before choosing who this is about — a scenario \
         stores the two together, and saving now would lose the person you \
         picked.",
    ),
    (
        KEY_MEANING_NEEDS_ATTACK_TEXT,
        "Write what they say before writing what it is meant to imply — a \
         scenario stores the two together, and saving now would lose the words \
         you just typed.",
    ),
    // The unified identity vocabulary (task R2) — ONE row per idea, rendered by
    // both the read-only block and the editor.
    (
        KEY_IDENTITY_ATTACK_LABEL,
        "The attack — what they claim, in their words",
    ),
    (
        KEY_IDENTITY_ATTACK_ABSENT,
        "No attack text written yet — the pencil opens the editor.",
    ),
    (KEY_IDENTITY_THEME_LABEL, "Our theme — one sentence"),
    (KEY_IDENTITY_THEME_ABSENT, "No theme written yet."),
    (KEY_IDENTITY_THEME_HELPER, "Read aloud in rehearsal mode."),
    (
        KEY_IDENTITY_MOTIVATION_LABEL,
        "Their motivation — what they want the jury to believe",
    ),
    (KEY_IDENTITY_MOTIVATION_ABSENT, "No motivation written yet."),
    (KEY_IDENTITY_BEARS_ON_LABEL, "Bears on"),
    (KEY_IDENTITY_BEARS_ON_ABSENT, "No allegations linked yet."),
    (
        KEY_REHEARSAL_LINK_BLOCKED,
        "Not in rehearsal — this scenario is {status}. Switch it to Ready on \
         this page first.",
    ),
    (KEY_PRACTICE_LINK_LABEL, "Practice \u{25b8}"),
];

impl ScenarioAuthoringWording {
    /// The fixture, built through the PRODUCTION builder — so a fixture that
    /// the real builder would reject cannot exist.
    pub fn for_test() -> Self {
        build_scenario_authoring_wording::<String>(|key| {
            TEST_SEED
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
                .ok_or_else(|| format!("{key} is missing from TEST_SEED"))
        })
        .expect("every key in SCENARIO_AUTHORING_WORDING_KEYS is in TEST_SEED")
    }

    /// The fixture as a key→value map, in the shape the store reads.
    pub fn for_test_values() -> HashMap<&'static str, String> {
        TEST_SEED
            .iter()
            .map(|(key, value)| (*key, (*value).to_string()))
            .collect()
    }
}

/// Every declared key is seeded, with the value this build expects.
///
/// The failure this prevents is a real one and not hypothetical: a key added to
/// `SCENARIO_AUTHORING_WORDING_KEYS` without a matching row makes
/// `load_at_boot_or_exit` end the process. That surfaces as a backend that will
/// not start after deploy — the most expensive way to learn a string is missing.
#[test]
fn every_declared_key_is_seeded_with_the_value_this_build_expects() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let sql: String = SEED_MIGRATIONS
        .iter()
        .map(|relative| {
            std::fs::read_to_string(root.join(relative))
                .unwrap_or_else(|_| panic!("{relative} is on disk"))
        })
        .collect::<Vec<_>>()
        .join("\n");

    let fixture = ScenarioAuthoringWording::for_test_values();

    for key in SCENARIO_AUTHORING_WORDING_KEYS {
        let seeded = seeded_value_in(&sql, key).unwrap_or_else(|| {
            panic!("{key} is declared to the boot loader but the migration seeds no row for it")
        });
        let in_fixture = fixture
            .get(*key)
            .unwrap_or_else(|| panic!("{key} is missing from TEST_SEED"));

        assert_eq!(
            in_fixture, &seeded,
            "the fixture has {key} = '{in_fixture}' but the migration seeds '{seeded}'."
        );
    }
}

/// No key collides with a sibling block's.
///
/// Also behavioural, not shape-pinning: `app_settings` is keyed by `key` alone,
/// so a collision would make two surfaces read ONE row — editing the create
/// form's label on the Settings page would silently change a rehearsal-page
/// sentence. The compiler cannot see that; only this can.
#[test]
fn no_key_collides_with_another_surface_s_key() {
    for key in SCENARIO_AUTHORING_WORDING_KEYS {
        assert!(
            !crate::domain::wording::WORDING_KEYS.contains(key),
            "{key} is also a curation-surface key"
        );
        assert!(
            !crate::domain::wording_accusation::ACCUSATION_WORDING_KEYS.contains(key),
            "{key} is also a working-view accusation key"
        );
        assert!(
            !crate::domain::wording_rehearsal::REHEARSAL_WORDING_KEYS.contains(key),
            "{key} is also a rehearsal prose key"
        );
        assert!(
            !crate::domain::wording_rehearsal_chrome::REHEARSAL_CHROME_KEYS.contains(key),
            "{key} is also a rehearsal chrome key"
        );
        assert!(
            !crate::domain::wording_authoring::AUTHORING_WORDING_KEYS.contains(key),
            "{key} is also a shared authoring-section key"
        );
    }
}
