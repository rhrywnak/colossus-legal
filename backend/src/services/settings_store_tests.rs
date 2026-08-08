//! Unit tests for [`super`] — the settings store's snapshot and its refusals.
//!
//! `build_settings` and `validate_candidate` are pure, so the whole failure law
//! is exercised here without a database: missing parameter, wrong declared kind,
//! out-of-bounds value, crossed bands. The transactional write is DEV-verified,
//! the house convention.
//!
//! These tests are the configuration law's teeth. Each one asserts that the
//! system REFUSES rather than falling back — because a fallback is precisely the
//! compiled-in default task 1.6 exists to delete.

use super::*;

// The three key LISTS moved with the boot loader (task 2.11 B2 split); this
// module still counts them, because the count is what proves the seed and the
// code describe the same store.
use crate::domain::wording::WORDING_KEYS;
use crate::domain::wording_accusation::ACCUSATION_WORDING_KEYS;
use crate::domain::wording_authoring::AUTHORING_WORDING_KEYS;
use crate::domain::wording_rehearsal::REHEARSAL_WORDING_KEYS;
use crate::domain::wording_rehearsal_chrome::REHEARSAL_CHROME_KEYS;
use crate::domain::wording_scenario_authoring::SCENARIO_AUTHORING_WORDING_KEYS;

use chrono::Utc;

/// One stored row, with the shape the migration seeds.
fn row(
    key: &str,
    value: &str,
    kind: ValueKind,
    min: Option<f64>,
    max: Option<f64>,
) -> AppSettingRecord {
    AppSettingRecord {
        key: key.to_string(),
        value: value.to_string(),
        value_kind: kind.code().to_string(),
        default_value: value.to_string(),
        min_value: min,
        max_value: max,
        meaning: "a parameter".to_string(),
        consumed_by: None,
        updated_at: Utc::now(),
        updated_by: "system (seed)".to_string(),
    }
}

/// The whole store as the migrations seed it: ten numbers and 170 stored strings.
///
/// ## Why the wording rows come from `Wording::for_test_values` (task 2.10)
///
/// A second hand-typed copy of twenty sentences here would be a fixture nothing
/// pins to the product. `wording_tests` pins `Wording::for_test` to the migration
/// that seeds it, so borrowing the values keeps ONE chain: migration → wording
/// fixture → this store fixture → `Settings::for_test`. Every link in it is
/// asserted.
fn seeded() -> HashMap<String, AppSettingRecord> {
    let mut rows = numeric_rows();
    // All six stored-string blocks, chained (2.10, 2.11 B1/B2, 2.11 C, and the
    // 2026-08-07 scenario-authoring block): the lists key ONE table, and a
    // fixture holding only some of them would let a snapshot build that the real
    // store could not.
    let text_rows = crate::domain::wording::Wording::for_test_values()
        .into_iter()
        .chain(crate::domain::wording_accusation::AccusationWording::for_test_values())
        .chain(crate::domain::wording_rehearsal::RehearsalWording::for_test_values())
        .chain(crate::domain::wording_rehearsal_chrome::RehearsalChromeWording::for_test_values())
        .chain(crate::domain::wording_authoring::AuthoringWording::for_test_values())
        .chain(
            crate::domain::wording_scenario_authoring::ScenarioAuthoringWording::for_test_values(),
        );
    for (key, value) in text_rows {
        // Text rows carry no bounds: `min_value` / `max_value` are numeric
        // comparisons, and the migration leaves them NULL.
        rows.insert(
            key.to_string(),
            row(key, &value, ValueKind::Text, None, None),
        );
    }
    rows
}

/// The ten numeric rows as the migrations seed them.
fn numeric_rows() -> HashMap<String, AppSettingRecord> {
    by_key(vec![
        row(
            KEY_BAND_HIGH,
            "0.80",
            ValueKind::Float,
            Some(0.0),
            Some(1.0),
        ),
        row(
            KEY_BAND_MEDIUM,
            "0.50",
            ValueKind::Float,
            Some(0.0),
            Some(1.0),
        ),
        row(KEY_CONTEXT_WINDOW, "240", ValueKind::Count, Some(0.0), None),
        row(
            KEY_TALKING_POINTS_CAP,
            "3",
            ValueKind::Count,
            Some(1.0),
            None,
        ),
        row(KEY_READINESS_N, "5", ValueKind::Count, Some(1.0), None),
        row(KEY_CARD_TEST_RATIO, "9/10", ValueKind::Ratio, None, None),
        row(
            KEY_REANCHOR_TOLERANCE,
            "0.85",
            ValueKind::Float,
            Some(0.0),
            Some(1.0),
        ),
        // Task 2.10: how many accusations the link panel's short list offers.
        row(
            KEY_LINK_SHORT_LIST_MAX,
            "8",
            ValueKind::Count,
            Some(1.0),
            None,
        ),
        // Task 2.11 B2: distinct dates needed before the rehearsal timeline is
        // drawn. Minimum 2 — a threshold of one draws a timeline from one point.
        row(
            KEY_TIMELINE_MIN_DATES,
            "2",
            ValueKind::Count,
            Some(2.0),
            None,
        ),
        // Task 2.11 C: how many instances a scenario may have before the
        // rehearsal page's rows arrive compact. Minimum 1 — zero would fold the
        // only row on a single-instance scenario.
        row(KEY_ROWS_EXPAND_MAX, "3", ValueKind::Count, Some(1.0), None),
    ])
}

// ── The snapshot ─────────────────────────────────────────────────────────────

/// The seed produces exactly the values in force before this task.
///
/// This is the "changes no behaviour" claim, asserted rather than asserted-in-a-
/// commit-message: the four numbers deleted from code must come back out of the
/// store identical, or task 1.6 silently re-tuned the product.
#[test]
fn the_seeded_store_reproduces_the_values_that_were_in_code() {
    let settings = build_settings(&seeded()).expect("the seed is valid");

    assert!((settings.confidence_band_high - 0.80).abs() < f32::EPSILON);
    assert!((settings.confidence_band_medium - 0.50).abs() < f32::EPSILON);
    assert_eq!(settings.quote_context_window_chars, 240);
    assert_eq!(settings.talking_points_cap, 3);
}

#[test]
fn the_dormant_parameters_load_too() {
    // They have no consumer until Phase 2, but they must be READABLE now —
    // otherwise the first task to consume them discovers the store is broken.
    let settings = build_settings(&seeded()).expect("valid");
    assert_eq!(settings.readiness_item_threshold_n, 5);
    assert_eq!(settings.card_test_ratio.to_string(), "9/10");
    assert!((settings.reanchor_close_match_tolerance - 0.85).abs() < f32::EPSILON);
}

// ── The failure law ──────────────────────────────────────────────────────────

/// Every required parameter is required. No exceptions, no defaults.
#[test]
fn any_missing_parameter_refuses_the_whole_snapshot() {
    for key in REQUIRED_KEYS {
        let mut rows = seeded();
        rows.remove(*key);

        let Err(error) = build_settings(&rows) else {
            panic!("a store missing {key} must not produce a snapshot");
        };
        assert!(
            error.to_string().contains(key),
            "the refusal must name the missing parameter: {error}"
        );
    }
}

#[test]
fn the_required_key_list_matches_what_the_snapshot_actually_reads() {
    // Anti-drift: a parameter added to `Settings` but forgotten in REQUIRED_KEYS
    // would not be checked at boot, and would fail later as a missing-key error
    // at whatever moment it happened to be read.
    assert_eq!(
        REQUIRED_KEYS.len(),
        10,
        "seven numbers, 2.10's short-list cap, 2.11 B2's timeline threshold, and \
         2.11 C's row-expand cap"
    );
    assert_eq!(
        WORDING_KEYS.len(),
        48,
        "22 from 2.10, five from 2.12, fourteen from 2.13, one latch fix, five from 2.13c"
    );
    assert_eq!(
        ACCUSATION_WORDING_KEYS.len(),
        27,
        "task 2.11's accusation section, every word of it a row"
    );
    assert_eq!(
        REHEARSAL_WORDING_KEYS.len(),
        41,
        "task 2.11 B2's rehearsal page plus 2.11 C's fifth section state"
    );
    assert_eq!(
        REHEARSAL_CHROME_KEYS.len(),
        18,
        "task 2.11 C: the rebuilt page's controls, tags and attribution lines"
    );
    assert_eq!(
        AUTHORING_WORDING_KEYS.len(),
        23,
        "task 2.11 C, ruling C4b: the literals that left two React components"
    );
    assert_eq!(
        SCENARIO_AUTHORING_WORDING_KEYS.len(),
        13,
        "2026-08-07: the create form's two new fields, the identity modal's \
         target control, and the no-target notice"
    );
    assert_eq!(
        seeded().len(),
        REQUIRED_KEYS.len()
            + WORDING_KEYS.len()
            + ACCUSATION_WORDING_KEYS.len()
            + REHEARSAL_WORDING_KEYS.len()
            + REHEARSAL_CHROME_KEYS.len()
            + AUTHORING_WORDING_KEYS.len()
            + SCENARIO_AUTHORING_WORDING_KEYS.len(),
        "the seed and the seven required lists must describe the same store"
    );
}

/// A missing ACCUSATION row refuses the snapshot exactly as the others do.
///
/// Its own test rather than a widened loop above: the two lists are read by two
/// calls, and a `build_settings` that forgot the second call would still pass
/// every assertion written against the first.
#[test]
fn a_missing_accusation_wording_row_refuses_the_snapshot_too() {
    for key in ACCUSATION_WORDING_KEYS {
        let mut rows = seeded();
        rows.remove(*key);

        let Err(error) = build_settings(&rows) else {
            panic!("a store missing {key} must not produce a snapshot");
        };
        assert!(
            error.to_string().contains(key),
            "the refusal must name the missing string: {error}"
        );
    }
}

/// A missing WORDING row refuses the snapshot exactly as a missing number does.
///
/// The point of Roman's ruling is that a string is a stored parameter with the
/// same standing as a threshold. If a missing label degraded to an empty button
/// instead of refusing to boot, it would be a compiled-in default by omission.
#[test]
fn a_missing_wording_row_refuses_the_snapshot_too() {
    for key in WORDING_KEYS {
        let mut rows = seeded();
        rows.remove(*key);

        let Err(error) = build_settings(&rows) else {
            panic!("a store missing {key} must not produce a snapshot");
        };
        assert!(
            error.to_string().contains(key),
            "the refusal must name the missing string: {error}"
        );
    }
}

/// A BLANK stored string refuses too — it would render as an invisible control.
#[test]
fn a_blank_label_is_refused_rather_than_shown_as_nothing() {
    let mut rows = seeded();
    let key = WORDING_KEYS[0];
    rows.insert(
        key.to_string(),
        row(key, "   ", ValueKind::Text, None, None),
    );

    let Err(error) = build_settings(&rows) else {
        panic!("a blank label must not reach a screen");
    };
    let message = error.to_string();
    assert!(message.contains(key), "{message}");
    assert!(message.contains("blank"), "{message}");
}

/// A wording row declaring the wrong kind is a drifted store, and says so.
#[test]
fn a_wording_row_declaring_a_number_is_refused_as_a_drifted_store() {
    let mut rows = seeded();
    let key = WORDING_KEYS[0];
    rows.insert(
        key.to_string(),
        row(key, "Some words", ValueKind::Count, None, None),
    );

    let Err(error) = build_settings(&rows) else {
        panic!("a kind mismatch must refuse");
    };
    assert!(
        error.to_string().contains("different kind"),
        "the refusal must say the STORE disagrees with the code: {error}"
    );
}

/// A template edited into one that lost its facts is refused, by name.
#[test]
fn a_template_that_lost_a_placeholder_is_refused_on_write() {
    let record = row(
        "link_summary_template",
        "You linked this to {allegations} · {cut}.",
        ValueKind::Text,
        None,
        None,
    );

    // Keeping both placeholders is fine, in any wording.
    assert!(validate_candidate(&record, "Bears on {allegations} — {cut}").is_ok());

    // Dropping one is not, and the refusal names it — otherwise the human is told
    // "invalid" about a sentence that reads perfectly well.
    let Err(error) = validate_candidate(&record, "You linked this to it · {cut}.") else {
        panic!("a template with the fact removed must be refused");
    };
    let message = error.to_string();
    assert!(message.contains("{allegations}"), "{message}");
    assert!(message.contains("link_summary_template"), "{message}");
}

/// A plain label has no placeholder rules to trip over.
#[test]
fn an_ordinary_label_can_be_reworded_freely() {
    let record = row(
        "link_save_label",
        "Save and next",
        ValueKind::Text,
        None,
        None,
    );
    assert!(validate_candidate(&record, "Save it and move on").is_ok());
    // But it still cannot be blanked.
    assert!(validate_candidate(&record, "   ").is_err());
}

#[test]
fn an_unreadable_value_refuses_rather_than_falling_back_to_the_default() {
    // The row carries `default_value`. Nothing reads it to recover — that would
    // be a compiled-in default wearing a database costume.
    let mut rows = seeded();
    rows.insert(
        KEY_TALKING_POINTS_CAP.to_string(),
        row(
            KEY_TALKING_POINTS_CAP,
            "three",
            ValueKind::Count,
            Some(1.0),
            None,
        ),
    );

    let Err(error) = build_settings(&rows) else {
        panic!("an unreadable value must refuse");
    };
    assert!(
        error.to_string().contains(KEY_TALKING_POINTS_CAP),
        "{error}"
    );
}

#[test]
fn an_out_of_bounds_value_refuses_and_names_the_bound() {
    let mut rows = seeded();
    rows.insert(
        KEY_TALKING_POINTS_CAP.to_string(),
        row(
            KEY_TALKING_POINTS_CAP,
            "0",
            ValueKind::Count,
            Some(1.0),
            None,
        ),
    );

    let Err(error) = build_settings(&rows) else {
        panic!("a cap of zero would forbid talking points entirely");
    };
    assert!(error.to_string().contains("at least 1"), "{error}");
}

/// A row whose declared kind disagrees with how this build reads it.
///
/// Reported as a store/code drift rather than as a parse failure, so a human is
/// not sent hunting through a value that is fine for the kind it claims to be.
#[test]
fn a_kind_that_disagrees_with_the_code_is_refused() {
    let mut rows = seeded();
    rows.insert(
        KEY_TALKING_POINTS_CAP.to_string(),
        row(KEY_TALKING_POINTS_CAP, "3/4", ValueKind::Ratio, None, None),
    );

    let Err(error) = build_settings(&rows) else {
        panic!("a ratio where the code reads a count must refuse");
    };
    assert!(
        error.to_string().contains("different kind"),
        "the refusal must say the STORE disagrees, not that the value is bad: {error}"
    );
}

#[test]
fn an_unknown_kind_token_is_refused() {
    let mut rows = seeded();
    let mut bad = row(
        KEY_TALKING_POINTS_CAP,
        "3",
        ValueKind::Count,
        Some(1.0),
        None,
    );
    bad.value_kind = "percentage".to_string();
    rows.insert(KEY_TALKING_POINTS_CAP.to_string(), bad);

    assert!(build_settings(&rows).is_err());
}

/// The cross-row invariant a column CHECK cannot express.
#[test]
fn bands_that_cross_are_refused_at_load() {
    let mut rows = seeded();
    rows.insert(
        KEY_BAND_HIGH.to_string(),
        row(
            KEY_BAND_HIGH,
            "0.40",
            ValueKind::Float,
            Some(0.0),
            Some(1.0),
        ),
    );

    let Err(error) = build_settings(&rows) else {
        panic!("high <= medium makes the medium band unreachable");
    };
    let message = error.to_string();
    assert!(message.contains("medium band"), "{message}");
}

#[test]
fn equal_bands_are_refused_too() {
    // Not just "high below medium": if they are EQUAL, no score can land in
    // medium either, and the off-by-one is the easy mistake to make by hand.
    let mut rows = seeded();
    rows.insert(
        KEY_BAND_HIGH.to_string(),
        row(
            KEY_BAND_HIGH,
            "0.50",
            ValueKind::Float,
            Some(0.0),
            Some(1.0),
        ),
    );
    assert!(build_settings(&rows).is_err());
}

/// The load-time band check exists because `psql` bypasses the write path.
///
/// Both halves are asserted: a hand-edited store is caught on the next boot, and
/// the same rule is applied to a candidate before it is ever written.
#[test]
fn the_band_rule_is_enforced_at_load_as_well_as_on_write() {
    // Load side: proven by `bands_that_cross_are_refused_at_load` above.
    // Write side: a single-row candidate check cannot see the other band, so the
    // write path re-loads after committing — this pins that the load-side rule
    // is the one that would catch it.
    let mut rows = seeded();
    rows.insert(
        KEY_BAND_MEDIUM.to_string(),
        row(
            KEY_BAND_MEDIUM,
            "0.95",
            ValueKind::Float,
            Some(0.0),
            Some(1.0),
        ),
    );
    assert!(
        build_settings(&rows).is_err(),
        "raising medium above high must be caught by the reload after a write"
    );
}

// ── Candidate validation (the write path's gate) ─────────────────────────────

#[test]
fn a_valid_candidate_passes_its_rows_declared_bounds() {
    let record = row(
        KEY_TALKING_POINTS_CAP,
        "3",
        ValueKind::Count,
        Some(1.0),
        None,
    );
    assert!(validate_candidate(&record, "5").is_ok());
}

#[test]
fn a_candidate_below_the_rows_minimum_is_refused_before_any_write() {
    let record = row(
        KEY_TALKING_POINTS_CAP,
        "3",
        ValueKind::Count,
        Some(1.0),
        None,
    );
    let Err(error) = validate_candidate(&record, "0") else {
        panic!("refused");
    };
    assert!(error.to_string().contains("at least 1"), "{error}");
}

#[test]
fn a_candidate_above_the_rows_maximum_is_refused_before_any_write() {
    let record = row(
        KEY_BAND_HIGH,
        "0.80",
        ValueKind::Float,
        Some(0.0),
        Some(1.0),
    );
    assert!(validate_candidate(&record, "1.4").is_err());
}

#[test]
fn a_candidate_of_the_wrong_shape_is_refused_per_kind() {
    let ratio = row(KEY_CARD_TEST_RATIO, "9/10", ValueKind::Ratio, None, None);
    // A decimal where a ratio is stored: readable as a number, wrong as a value.
    assert!(validate_candidate(&ratio, "0.9").is_err());
    assert!(validate_candidate(&ratio, "8/10").is_ok());

    let count = row(KEY_CONTEXT_WINDOW, "240", ValueKind::Count, Some(0.0), None);
    assert!(validate_candidate(&count, "240.5").is_err());
    assert!(validate_candidate(&count, "300").is_ok());
}

#[test]
fn validation_is_the_same_rule_the_snapshot_applies() {
    // The gate before the write and the check after it must not disagree — a
    // value accepted by one and refused by the other would write a store that
    // then refuses to boot.
    let record = row(
        KEY_TALKING_POINTS_CAP,
        "3",
        ValueKind::Count,
        Some(1.0),
        None,
    );
    for candidate in ["0", "-1", "two", "2.5"] {
        assert!(
            validate_candidate(&record, candidate).is_err(),
            "{candidate} must be refused by the write gate"
        );

        let mut rows = seeded();
        rows.insert(
            KEY_TALKING_POINTS_CAP.to_string(),
            row(
                KEY_TALKING_POINTS_CAP,
                candidate,
                ValueKind::Count,
                Some(1.0),
                None,
            ),
        );
        assert!(
            build_settings(&rows).is_err(),
            "{candidate} must also be refused by the snapshot"
        );
    }
}

// ── The fixture is checked against the MIGRATION, not against itself ─────────

/// The seed values this file asserts against are the ones the migration writes.
///
/// ## Why this test exists
///
/// `seeded()` and `Settings::for_test()` are hand-written copies of the migration's
/// `INSERT`. Without this test, "task 1.6 changes no behaviour" was asserted
/// against those copies — so editing the migration's `VALUES` and forgetting the
/// fixtures would leave every test green while DEV and PROD ran different numbers.
/// A fixture compared only to itself proves nothing.
///
/// Reading the migration makes it the ground truth, which is the same move
/// `the_listing_puts_live_parameters_before_dormant_ones` makes for the ORDER BY.
#[test]
fn the_fixtures_carry_the_values_the_migration_actually_seeds() {
    // FOUR migrations now seed numeric parameters: 1.6's original seven, 2.10's
    // short-list cap, 2.11 B2's timeline threshold and 2.11 C's row-expand cap.
    // Concatenated rather than searched one at a time so a key moving between
    // files is not a failure — where a parameter is seeded is a fact about
    // migration history, and only its VALUE is what this pins.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let sql: String = [
        "pipeline_migrations/20260801225147_create_app_settings_store.sql",
        "pipeline_migrations/20260804132730_create_evidence_allegation_links.sql",
        // Task 2.11 B2's timeline threshold — the ninth number, seeded with the
        // rehearsal page's wording because it is that page's tunable.
        "pipeline_migrations/20260806100704_rehearsal_render_wording.sql",
        // Task 2.11 C's row-expand cap — the tenth number, seeded with the
        // rebuilt page's wording for the same reason.
        "pipeline_migrations/20260806135509_rehearsal_visual_2_11c.sql",
    ]
    .iter()
    .map(|relative| {
        std::fs::read_to_string(root.join(relative))
            .unwrap_or_else(|_| panic!("{relative} is on disk"))
    })
    .collect::<Vec<_>>()
    .join("\n");

    let fixture = seeded();
    let mut checked = 0usize;

    for key in REQUIRED_KEYS {
        let seeded_value = seeded_value_in(&sql, key)
            .unwrap_or_else(|| panic!("{key} is not seeded by the migration"));

        let in_fixture = &fixture
            .get(*key)
            .unwrap_or_else(|| panic!("{key} is missing from the test fixture"))
            .value;

        assert_eq!(
            *in_fixture, seeded_value,
            "the test fixture has {key} = '{in_fixture}' but the migration seeds \
             '{seeded_value}'. One of them moved without the other; the \
             'changes no behaviour' claim is only as good as this match."
        );
        checked += 1;
    }

    // Anti-vacuity: a parsing change that stopped finding rows would otherwise
    // make this test pass while comparing nothing.
    assert_eq!(checked, 10, "all ten numeric parameters must be compared");
}

/// The snapshot built from the fixture matches the one `Settings::for_test`
/// hands to every other module's tests.
///
/// Two hand-written copies of the seed exist — `seeded()` here and
/// `Settings::for_test()` in the domain, which `confidence_band`, `scenario_card`
/// and `scenario_augmentation` tests all use. If they drifted, those suites would
/// be asserting against different numbers than this one, and the cutoff and cap
/// assertions scattered across the codebase would quietly stop describing the
/// product. The test above pins `seeded()` to the migration; this pins the other
/// copy to `seeded()`, so all three are one chain.
#[test]
fn the_domain_test_snapshot_matches_the_seeded_store() {
    assert_eq!(
        build_settings(&seeded()).expect("the seed is valid"),
        Settings::for_test(),
        "Settings::for_test() has drifted from the seeded store"
    );
}

/// Pull one key's seeded value out of the migration's INSERT.
///
/// Deliberately crude — the rows have a fixed shape (`('key', 'value', …`), and a
/// SQL parser would be a lot of machinery to read seven literals. It returns
/// `None` rather than guessing when the shape does not match, so the caller can
/// fail with a message naming the key.
fn seeded_value_in(sql: &str, key: &str) -> Option<String> {
    let marker = format!("('{key}', '");
    let at = sql.find(&marker)?;
    let rest = &sql[at + marker.len()..];
    let end = rest.find('\'')?;
    Some(rest[..end].to_string())
}

// ── The cross-row gate, proven at the level that actually guards it ──────────

/// A crossed pair is caught by rebuilding the store, not by the single-row check.
///
/// This is the shape of the defect the trial snapshot closes. `validate_candidate`
/// ACCEPTS `confidence_band_high = 0.40` — it is a perfectly good float in [0, 1]
/// for its own row — and only a whole-store rebuild sees that medium sits above
/// it. Before the trial snapshot existed, that gap meant: 400 to the operator,
/// value already committed, and a database that would refuse the next boot.
#[test]
fn the_single_row_check_accepts_a_value_the_whole_store_rejects() {
    let high = row(
        KEY_BAND_HIGH,
        "0.80",
        ValueKind::Float,
        Some(0.0),
        Some(1.0),
    );

    // Step 1 passes: 0.40 is a valid float within this row's declared bounds.
    assert!(
        validate_candidate(&high, "0.40").is_ok(),
        "the row's own bounds cannot see the sibling band — that is the gap"
    );

    // Step 2 refuses: with medium at 0.50, the store no longer builds.
    let mut trial = seeded();
    trial.insert(
        KEY_BAND_HIGH.to_string(),
        row(
            KEY_BAND_HIGH,
            "0.40",
            ValueKind::Float,
            Some(0.0),
            Some(1.0),
        ),
    );
    let Err(error) = build_settings(&trial) else {
        panic!("the trial snapshot must refuse a crossed pair BEFORE it is written");
    };
    assert!(error.to_string().contains("medium band"), "{error}");
}

/// The trial runs the REAL `build_settings`, so future rules are covered for free.
///
/// If the trial had reimplemented the band comparison, a cross-row invariant added
/// to `build_settings` later would silently not be pre-checked, and the
/// commit-then-discover sequence would come back for the new rule.
#[test]
fn the_trial_path_is_the_same_builder_the_boot_uses() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/services/settings_store.rs"),
    )
    .expect("readable");

    let trial_fn = source
        .split_once("async fn trial_snapshot(")
        .map(|(_, rest)| rest)
        .and_then(|rest| rest.split_once("\n}"))
        .map(|(body, _)| body)
        .expect("trial_snapshot exists");

    assert!(
        trial_fn.contains("build_settings(&trial)"),
        "the pre-write trial must call the real builder, so any cross-row rule \
         added there is pre-checked automatically: {trial_fn}"
    );
}

/// A valid change still passes the trial.
#[test]
fn a_legitimate_change_survives_the_whole_store_rebuild() {
    let mut trial = seeded();
    trial.insert(
        KEY_TALKING_POINTS_CAP.to_string(),
        row(
            KEY_TALKING_POINTS_CAP,
            "5",
            ValueKind::Count,
            Some(1.0),
            None,
        ),
    );
    let settings = build_settings(&trial).expect("raising the cap is legitimate");
    assert_eq!(settings.talking_points_cap, 5);
}

/// Moving a band to a value that stays ordered is accepted.
#[test]
fn a_band_change_that_keeps_the_order_is_accepted() {
    let mut trial = seeded();
    trial.insert(
        KEY_BAND_HIGH.to_string(),
        row(
            KEY_BAND_HIGH,
            "0.90",
            ValueKind::Float,
            Some(0.0),
            Some(1.0),
        ),
    );
    let settings = build_settings(&trial).expect("0.90 > 0.50 is fine");
    assert!((settings.confidence_band_high - 0.90).abs() < f32::EPSILON);
}

// ── The refusal messages ─────────────────────────────────────────────────────

#[test]
fn the_unknown_key_refusal_names_the_key() {
    let error = SettingsError::UnknownKey {
        key: "invented_parameter".to_string(),
    };
    assert!(error.to_string().contains("invented_parameter"), "{error}");
}

#[test]
fn the_no_op_refusal_names_the_current_value() {
    let error = SettingsError::Unchanged {
        key: KEY_TALKING_POINTS_CAP.to_string(),
        value: "3".to_string(),
    };
    let message = error.to_string();
    assert!(message.contains("already '3'"), "{message}");
    assert!(message.contains("nothing to change"), "{message}");
}

/// A failed read and a failed write say different things.
///
/// The 1.4 lesson, held: collapsing them told a human "failed to save" when a
/// LOAD had failed — a message about an action they never took.
#[test]
fn a_failed_read_and_a_failed_write_are_different_sentences() {
    let source = || PipelineRepoError::from(sqlx::Error::RowNotFound);
    let read = SettingsError::Read { source: source() }.to_string();
    let write = SettingsError::Write { source: source() }.to_string();

    assert!(read.contains("read the configuration store"), "{read}");
    assert!(write.contains("record the configuration change"), "{write}");
    assert_ne!(read, write);
}

/// "Saved but not live" is its own sentence, not a read failure wearing one.
///
/// The two are opposite states. A pre-write read failure means nothing happened
/// and the fix is to retry; this one means the value IS stored and a retry would
/// answer "already that value — nothing to change", contradicting the error the
/// operator just saw. The remedy differs too: restart, not retry.
#[test]
fn a_change_that_saved_but_did_not_go_live_says_so_and_says_what_to_do() {
    let error = SettingsError::SavedButStale {
        key: KEY_TALKING_POINTS_CAP.to_string(),
        value: "5".to_string(),
        source: Box::new(SettingsError::Read {
            source: PipelineRepoError::from(sqlx::Error::RowNotFound),
        }),
    };
    let message = error.to_string();

    // What happened: the value IS stored.
    assert!(message.contains("was saved"), "{message}");
    assert!(message.contains(KEY_TALKING_POINTS_CAP), "{message}");
    assert!(message.contains('5'), "{message}");
    // What to do: reload to confirm, and expect the old value until a restart.
    assert!(message.contains("restarted"), "{message}");

    // And it must not read like the failure where nothing was written.
    let plain_read = SettingsError::Read {
        source: PipelineRepoError::from(sqlx::Error::RowNotFound),
    }
    .to_string();
    assert_ne!(message, plain_read);
}

/// An invalid parameter reaches the human with the domain's own words.
///
/// Rewrapping it as "invalid configuration" would lose the key, the value and the
/// bound — the only three things that tell someone what to type instead.
#[test]
fn an_invalid_parameter_keeps_the_detail_that_makes_it_fixable() {
    let error = SettingsError::from(SettingError::BelowMinimum {
        key: KEY_TALKING_POINTS_CAP.to_string(),
        value: "0".to_string(),
        min: 1.0,
    });
    let message = error.to_string();
    assert!(message.contains(KEY_TALKING_POINTS_CAP), "{message}");
    assert!(message.contains("at least 1"), "{message}");
}
