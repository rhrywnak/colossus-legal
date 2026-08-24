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
use crate::domain::practice_params::{
    KEY_PRACTICE_READ_MAX_POINTERS, KEY_PRACTICE_READ_MAX_TOKENS, KEY_PRACTICE_READ_MAX_WORDS,
    KEY_PRACTICE_READ_MAX_WORDS_AFTER_FINE, KEY_PRACTICE_READ_MAX_WORDS_CALL,
    KEY_PRACTICE_READ_MAX_WORDS_POINTER, KEY_PRACTICE_READ_MAX_WORDS_WHY, PRACTICE_PARAM_KEYS,
};
use crate::domain::wording::WORDING_KEYS;
use crate::domain::wording_accusation::ACCUSATION_WORDING_KEYS;
use crate::domain::wording_authoring::AUTHORING_WORDING_KEYS;
use crate::domain::wording_card_grammar::CARD_GRAMMAR_WORDING_KEYS;
use crate::domain::wording_matrix::MATRIX_WORDING_KEYS;
use crate::domain::wording_model_params::MODEL_PARAMS_WORDING_KEYS;
use crate::domain::wording_practice::PRACTICE_WORDING_KEYS;
use crate::domain::wording_practice_editor::PRACTICE_EDITOR_WORDING_KEYS;
use crate::domain::wording_practice_flow::PRACTICE_FLOW_WORDING_KEYS;
use crate::domain::wording_practice_list::PRACTICE_LIST_WORDING_KEYS;
use crate::domain::wording_practice_print::PRACTICE_PRINT_WORDING_KEYS;
use crate::domain::wording_practice_report::PRACTICE_REPORT_WORDING_KEYS;
use crate::domain::wording_practice_row::PRACTICE_ROW_WORDING_KEYS;
use crate::domain::wording_rehearsal::REHEARSAL_WORDING_KEYS;
use crate::domain::wording_rehearsal_chrome::REHEARSAL_CHROME_KEYS;
use crate::domain::wording_scan::SCAN_WORDING_KEYS;
use crate::domain::wording_scenario_authoring::SCENARIO_AUTHORING_WORDING_KEYS;
use crate::domain::wording_war_room::WAR_ROOM_WORDING_KEYS;

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

/// The whole store as the migrations seed it: every not-wording parameter and
/// every stored string, from the same fixtures the product's own tests use.
///
/// ## Why the wording rows come from `Wording::for_test_values` (task 2.10)
///
/// A second hand-typed copy of twenty sentences here would be a fixture nothing
/// pins to the product. `wording_tests` pins `Wording::for_test` to the migration
/// that seeds it, so borrowing the values keeps ONE chain: migration → wording
/// fixture → this store fixture → `Settings::for_test`. Every link in it is
/// asserted.
/// The three tier-map rows, verbatim as the .396 migration seeds them.
///
/// Named constants rather than inline literals because
/// `the_fixtures_carry_the_values_the_migration_actually_seeds` compares these
/// against the SQL on disk character for character — a stray line break inside
/// one of them would fail with a diff nobody could read.
const MATRIX_TIER_STRONG_PAIRS: &str =
    "admission+sworn_party_admission, court_finding+court_finding, court_order+court_order";
const MATRIX_TIER_HEDGED_PAIRS: &str =
    "partial_admission+sworn_party_admission, partial_admission+sworn_party_evasion";
const MATRIX_TIER_OTHER_PAIRS: &str = "factual_assertion+sworn_testimony";

fn seeded() -> HashMap<String, AppSettingRecord> {
    let mut rows = numeric_rows();
    // All eight stored-string blocks, chained (2.10, 2.11 B1/B2, 2.11 C, the
    // 2026-08-07 scenario-authoring block, 2.15's scan block, and the
    // one-card-grammar block): the lists key ONE table, and a fixture holding
    // only some of them would let a snapshot build that the real store could
    // not.
    let text_rows = crate::domain::wording::Wording::for_test_values()
        .into_iter()
        .chain(crate::domain::wording_accusation::AccusationWording::for_test_values())
        .chain(crate::domain::wording_rehearsal::RehearsalWording::for_test_values())
        .chain(crate::domain::wording_rehearsal_chrome::RehearsalChromeWording::for_test_values())
        .chain(crate::domain::wording_authoring::AuthoringWording::for_test_values())
        .chain(
            crate::domain::wording_scenario_authoring::ScenarioAuthoringWording::for_test_values(),
        )
        .chain(crate::domain::wording_scan::ScanWording::for_test_values())
        .chain(crate::domain::wording_card_grammar::CardGrammarWording::for_test_values())
        .chain(crate::domain::wording_model_params::ModelParamsWording::for_test_values())
        .chain(crate::domain::wording_matrix::MatrixWording::for_test_values())
        .chain(crate::domain::wording_war_room::WarRoomWording::for_test_values())
        .chain(crate::domain::wording_practice::PracticeWording::for_test_values())
        // PRACTICE flow v1 (mockup v3): nested inside `PracticeWording` on the
        // struct, but a SEPARATE table here — the store keys one flat table, and
        // a fixture missing these thirty-two rows would let a snapshot build
        // that the real store could not.
        .chain(crate::domain::wording_practice_flow::PracticeFlowWording::for_test_values())
        // PRACTICE v1 (the Chuck review): nested inside `PracticeWording` for the
        // same reason `flow` is, and listed here for the same reason — one flat
        // table, and a fixture missing these twelve rows would let a snapshot
        // build that the real store could not.
        .chain(crate::domain::wording_practice_row::PracticeRowWording::for_test_values())
        // PRACTICE v1 Part B: the deck editor's words and the review page's,
        // nested on the struct for the same Rule 17 reason and listed here for
        // the same reason as their siblings — one flat table.
        .chain(crate::domain::wording_practice_editor::PracticeEditorWording::for_test_values())
        .chain(crate::domain::wording_practice_report::PracticeReportWording::for_test_values())
        .chain(crate::domain::wording_practice_print::PracticePrintWording::for_test_values())
        .chain(crate::domain::wording_practice_list::PracticeListWording::for_test_values())
        // Task 2.15 Tier 2: two TEXT rows that are not wording — one names a
        // file, one holds a comma-separated list — so they are seeded here rather
        // than borrowed from a `for_test_values` block.
        .chain([
            (
                KEY_THEME_SCAN_PROMPT_FILE,
                "theme_scan_prompt_v3.md".to_string(),
            ),
            (KEY_PREFILTER_STATEMENT_TYPES, "referral".to_string()),
            // Task R2 / 10e: a TEXT row that is not wording — it names a model,
            // so like its two neighbours it is seeded here rather than borrowed
            // from a `for_test_values` block.
            ("theme_scan_default_model", "claude-opus-5".to_string()),
            // Task 396 P1: three TEXT rows that are not wording — they carry
            // extraction vocabulary rather than sentences — so like their four
            // neighbours above they are seeded here. The values are the six pairs
            // measured on DEV, and `EvidenceTierMap::for_test` holds the same six;
            // `wording_matrix`'s seed test pins BOTH to the migration.
            (
                "matrix_tier_strong_pairs",
                MATRIX_TIER_STRONG_PAIRS.to_string(),
            ),
            (
                "matrix_tier_hedged_pairs",
                MATRIX_TIER_HEDGED_PAIRS.to_string(),
            ),
            (
                "matrix_tier_other_pairs",
                MATRIX_TIER_OTHER_PAIRS.to_string(),
            ),
            // PRACTICE v0: three more TEXT rows that are not wording — one names
            // a prompt file, one names a model, one carries the seven-card
            // vocabulary. Same reason as their neighbours above: nobody reads
            // them on a screen.
            (
                "practice_read_prompt_file",
                // v3 as of 2026-08-20 (T1): the read returns three parts and is
                // given her receipts, which is a change to what the model is TOLD
                // and to what it may say back — so a new file and a pointer moved.
                // v1 and v2 both stay on disk; pointing this row back at v2 is the
                // whole of the T1 rollback.
                "practice_read_prompt_v3.md".to_string(),
            ),
            ("practice_read_model", "claude-opus-5".to_string()),
            // The case's own timezone — what "today" means on a deck row. Case
            // data, so a stored row (hotfix, 2026-08-19); Postgres does the
            // comparing, which is why nothing parses it here.
            ("practice_case_timezone", "America/Detroit".to_string()),
            // The OK word, coupled to the prompt file. Text, and not wording:
            // nobody reads it on a screen — the model writes it and the parser
            // recognises it.
            ("practice_read_fine_token", "Fine.".to_string()),
            (
                "practice_tactic_names",
                "broad generalization,half-truth,character jab,false premise,compound,\
                 authority borrow,echo"
                    .to_string(),
            ),
        ]);
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

/// The numeric rows as the migrations seed them (the not-wording text rows are
/// chained in by `seeded`, beside the wording blocks).
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
        // ONE_CARD_GRAMMAR: how much of a question shows, and how many element
        // chips stand before the fold. Bounds mirror the migration's — the
        // question needs at least one visible character to be ellipsizable at
        // all, while zero visible element chips is a legitimate choice (the
        // count chip and "+N more" still say the elements are there).
        row(
            KEY_CARD_QUESTION_TRUNCATE,
            "110",
            ValueKind::Count,
            Some(1.0),
            Some(2000.0),
        ),
        row(
            KEY_CARD_ELEMENT_CHIPS_K,
            "2",
            ValueKind::Count,
            Some(0.0),
            Some(50.0),
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
        // Task 2.15 Tier 2: the shortest unanchored quote that still reaches the
        // judge. Minimum 0, which is the documented "rule off" value.
        row(
            KEY_PREFILTER_MIN_CHARS,
            "60",
            ValueKind::Count,
            Some(0.0),
            None,
        ),
        // 2026-08-09: the judge's per-candidate token budget, thinking included.
        // Bounds 256..=64000 — the floor is above the largest observed successful
        // reply with room to think, the ceiling is the lowest Anthropic
        // `max_output_tokens` in the registry, above which `constrain` would clamp
        // and the stored number would stop describing what is sent.
        row(
            KEY_SCAN_MAX_TOKENS,
            "8192",
            ValueKind::Count,
            Some(256.0),
            Some(64000.0),
        ),
        // PRACTICE v0: the one-sentence read's output cap. Bounds 64..=8192 —
        // the floor is well above one sentence, the ceiling stays under every
        // active model's `max_output_tokens` (including the two 2048-token vLLM
        // rows) because `constrain` REFUSES a cap above the ceiling rather than
        // clamping it.
        row(
            KEY_PRACTICE_READ_MAX_TOKENS,
            "1024",
            ValueKind::Count,
            Some(64.0),
            Some(8192.0),
        ),
        // PRACTICE v0: the read's two word caps. They REFUSE a longer reply
        // rather than shortening it, so the floors are above one useful sentence
        // and the ceilings are well below a paragraph.
        row(
            KEY_PRACTICE_READ_MAX_WORDS,
            "25",
            ValueKind::Count,
            Some(5.0),
            Some(60.0),
        ),
        // T1's three per-part ceilings and the pointer count (2026-08-20). The
        // read stopped being one sentence, so the single whole-reply cap above
        // stopped being the rule — it is kept because pointing prompt_file back
        // at v2 is the rollback, and the v2 path reads it.
        row(
            KEY_PRACTICE_READ_MAX_WORDS_CALL,
            "12",
            ValueKind::Count,
            Some(3.0),
            Some(40.0),
        ),
        row(
            KEY_PRACTICE_READ_MAX_WORDS_WHY,
            "55",
            ValueKind::Count,
            Some(10.0),
            Some(150.0),
        ),
        row(
            KEY_PRACTICE_READ_MAX_WORDS_POINTER,
            "20",
            ValueKind::Count,
            Some(5.0),
            Some(60.0),
        ),
        row(
            KEY_PRACTICE_READ_MAX_POINTERS,
            "3",
            ValueKind::Count,
            Some(0.0),
            Some(5.0),
        ),
        row(
            KEY_PRACTICE_READ_MAX_WORDS_AFTER_FINE,
            "6",
            ValueKind::Count,
            Some(0.0),
            Some(30.0),
        ),
    ])
}

// ── The snapshot ─────────────────────────────────────────────────────────────

/// The seed produces exactly the values in force before this task.
///
/// This is the "changes no behaviour" claim, asserted rather than asserted-in-a-
/// commit-message: the four numbers deleted from code must come back out of the
/// store identical, or task 1.6 silently re-tuned the product.
/// A pair a Settings typo put under two tiers ranks as the WEAKER one — through
/// the real boot path, not just through `from_entries`.
///
/// `EvidenceTierMap::from_entries` guarantees "last group supplied wins", and
/// `evidence_tier`'s own test pins that. It only produces the SAFE reading if the
/// boot path supplies the groups strongest-first — and nothing enforces that
/// except three hand-written lines in `build_evidence_tier_map`. Reorder them and
/// a pair fat-fingered into the strong row would promote itself into the headline
/// number Chuck reads as "cannot be disputed", with every other test still green.
///
/// This is the assertion that would notice.
#[test]
fn a_pair_seeded_under_two_tiers_ranks_as_the_weaker_one() {
    use crate::domain::evidence_tier::EvidenceTier;

    let mut rows = seeded();
    // The strong list keeps its seeded contents; the hedged list ALSO claims the
    // firm-admission pair, which is exactly the shape of a copy-paste slip on the
    // Settings page.
    let duplicated = format!("{MATRIX_TIER_HEDGED_PAIRS}, admission+sworn_party_admission");
    rows.insert(
        "matrix_tier_hedged_pairs".to_string(),
        row(
            "matrix_tier_hedged_pairs",
            &duplicated,
            ValueKind::Text,
            None,
            None,
        ),
    );

    let settings = build_settings(&rows).expect("a duplicate is a typo, not a boot refusal");
    assert_eq!(
        settings
            .evidence_tier_map
            .tier_for(Some("admission"), Some("sworn_party_admission")),
        Some(EvidenceTier::Hedged),
        "a pair listed in two tiers must land in the WEAKER one — the boot path \
         has to supply the groups strongest-first for that to hold",
    );
}

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

/// The judge's cap has no fallback: no row, no boot (§2b).
///
/// ## Why this gets its own test when the loop below already covers it
///
/// The loop proves every key in `REQUIRED_KEYS` refuses. This proves the cap is
/// IN that list — which is the thing that could quietly stop being true. The cap
/// arrived by deleting a compiled-in constant, and the failure mode worth naming
/// is the obvious "fix" for a boot refusal: put the 512 back as a default and let
/// a missing row fall through to it. That would restore, silently, exactly the
/// state that truncated 7 of 104 verdicts on 2026-08-09 — and every other test in
/// this file would still pass.
///
/// It also pins the BOUNDS, because a cap the store accepts at 8 is a cap that
/// cannot produce a verdict at all, and a cap it accepts at 200000 is one
/// `constrain` would silently clamp — leaving a stored number that no longer
/// describes what is sent.
#[test]
fn a_missing_cap_row_refuses_boot() {
    let mut rows = seeded();
    rows.remove(KEY_SCAN_MAX_TOKENS);

    let Err(error) = build_settings(&rows) else {
        panic!(
            "a store with no judge cap must not produce a snapshot — there is \
                no compiled-in 512 left to fall back to, and restoring one is the \
                regression this test exists to catch"
        );
    };
    assert!(
        error.to_string().contains(KEY_SCAN_MAX_TOKENS),
        "the refusal must name the missing cap: {error}"
    );

    // The bounds the row carries are enforced on the way in, not merely stored.
    for (value, why) in [
        ("8", "a cap that cannot fit a verdict at all"),
        (
            "200000",
            "a cap above every model ceiling, which constrain would clamp",
        ),
    ] {
        let mut rows = seeded();
        rows.insert(
            KEY_SCAN_MAX_TOKENS.to_string(),
            row(
                KEY_SCAN_MAX_TOKENS,
                value,
                ValueKind::Count,
                Some(256.0),
                Some(64000.0),
            ),
        );
        assert!(
            build_settings(&rows).is_err(),
            "{value} must be refused — {why}"
        );
    }

    // ANTI-VACUITY: the seeded value itself passes, so the refusals above are
    // about these values and not about the row being unreadable in general.
    assert_eq!(
        build_settings(&seeded())
            .expect("the seeded cap is valid")
            .theme_scan_max_tokens,
        8192
    );
}

/// A token budget too large for the field it feeds is a REFUSAL, not a wrap.
///
/// ## Why this branch is tested when the seeded row can never reach it
///
/// `token_count_of` narrows a `usize` row to the `u32` the provider seam speaks,
/// and the judge cap's own `max_value` of 64000 makes the overflow unreachable —
/// today. The reason the branch exists at all is that the bound is DATA: a `psql`
/// edit, or a future migration widening the ceiling, exposes it without touching
/// a line of code. A branch whose justification is "a data change can reach this"
/// is a branch a test has to reach now, or the justification is decoration.
///
/// What it guards against concretely: written `count as u32`, a stored
/// 5_000_000_000 arrives at the wire as 705_032_704 — a different, plausible,
/// silently wrong budget, on the one parameter this whole build exists to stop
/// being silently wrong.
#[test]
fn a_cap_too_large_for_the_wire_is_refused_rather_than_wrapped() {
    use crate::services::settings_row_readers::token_count_of;

    // `max_value: None` is the state a widened bound leaves behind — the row's own
    // ceiling is gone, so the narrowing is the only thing left standing.
    let widened = row(
        KEY_SCAN_MAX_TOKENS,
        "5000000000",
        ValueKind::Count,
        Some(0.0),
        None,
    );

    let Err(error) = token_count_of(&widened) else {
        panic!(
            "5000000000 does not fit a u32 and must be refused — an `as` cast \
             would have returned 705032704 and sent it"
        );
    };
    assert!(
        matches!(error, SettingError::AboveMaximum { .. }),
        "a number too large for its field is exactly an above-maximum: {error}"
    );
    assert!(
        error.to_string().contains(KEY_SCAN_MAX_TOKENS),
        "the refusal must name the row: {error}"
    );

    // ANTI-VACUITY. With the same absent ceiling, a value that DOES fit is read
    // normally — so the refusal above is about the width of the field and not
    // about `max_value: None` being rejected outright.
    assert_eq!(
        token_count_of(&row(
            KEY_SCAN_MAX_TOKENS,
            "8192",
            ValueKind::Count,
            Some(0.0),
            None,
        ))
        .expect("8192 fits a u32"),
        8192
    );
}

/// Every required parameter is required. No exceptions, no defaults.
#[test]
fn any_missing_parameter_refuses_the_whole_snapshot() {
    for key in REQUIRED_KEYS.iter().chain(PRACTICE_PARAM_KEYS) {
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
        REQUIRED_KEYS.len() + PRACTICE_PARAM_KEYS.len(),
        32,
        "seven numbers, 2.10's short-list cap, 2.11 B2's timeline threshold, \
         2.11 C's row-expand cap, 2.15's three scan parameters (the prompt \
         filename and the two pre-filter dials), the one-card grammar's two fold \
         thresholds (the question's visible length and the element-chip K), and \
         the judge's token budget (a constant until 2026-08-09), R2's scan \
         default model — a decision that used to be made by list order — and \
         .396's three tier-map rows, which carry extraction vocabulary rather \
         than sentences and so are parameters, not wording, and PRACTICE v0's \
         seven — the read's prompt file, its model, its token cap, its two word \
         caps, the word it reserves for \"fine\", and the seven tactic-card names \
         — plus the case's own timezone, which decides what \"today\" means on a \
         deck row (hotfix, 2026-08-19)"
    );
    assert_eq!(
        WORDING_KEYS.len(),
        58,
        "22 from 2.10, five from 2.12, fourteen from 2.13, one latch fix, five \
         from 2.13c, 2.15's raw-pool opt-in, the projection's four, and the five \
         the ruling acknowledgment speaks (saved / left-the-filter / defer \
         recorded / failed, plus the locked card's condition)"
    );
    assert_eq!(
        ACCUSATION_WORDING_KEYS.len(),
        27,
        "task 2.11's accusation section, every word of it a row"
    );
    assert_eq!(
        REHEARSAL_WORDING_KEYS.len(),
        58,
        "task 2.11 B2's rehearsal page, 2.11 C's fifth section state, and R1's \
         picker heading for the rehearsal front door, and R3's seven for the \
         prep page's plural-correct count line plus its four case phases"
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
        26,
        "2026-08-07: the create form's two new fields, the identity modal's \
         target control, and the no-target notice — plus 2.15's never-scanned \
         notice, its sibling, R1's two (the second definition-loss refusal and \
         the gated rehearsal control's reason), and R2's nine: the unified \
         identity vocabulary both identity surfaces now share"
    );
    assert_eq!(
        SCAN_WORDING_KEYS.len(),
        16,
        "2.15 Tier 2's conservation line and two history controls, the \
         projection's eight (the collapsed summary and the seven strings of the \
         numbers-only report), and the failure honesty five: the failed clause, \
         the failed tile, the two status pills and the failed collapsed line"
    );
    assert_eq!(
        CARD_GRAMMAR_WORDING_KEYS.len(),
        33,
        "ONE_CARD_GRAMMAR: the queue frame's seven, the card body's eleven, \
         linking's four, the fact wrapper's nine, the two chip-filter \
         sentences, and .396's already-linked note — the sentence the panel \
         speaks now that it stays after the first link"
    );
    assert_eq!(
        MODEL_PARAMS_WORDING_KEYS.len(),
        7,
        "MODEL_PARAMS (ruling R5, 2026-08-09): the temperature dropdown's label, \
         its help sentence, its three option texts, and the numeric value's label \
         and disabled-help"
    );
    assert_eq!(
        MATRIX_WORDING_KEYS.len(),
        8,
        "task 396 P1: the strong column's label and hint, the depth line, the \
         three tier chips, the duplicate marker, and the ranked-list note"
    );
    assert_eq!(
        WAR_ROOM_WORDING_KEYS.len(),
        4,
        "task 396 P3b: the subtitle R2 ruled and never migrated, plus the three \
         metric tile labels"
    );
    assert_eq!(
        PRACTICE_WORDING_KEYS.len(),
        37,
        "PRACTICE v0, the drill: mockup v2's start card and question screen, \
         plus the named gaps, the way in, the braid suffix on a tactic tag, and \
         the .403 bundle's three side-card terms and redirects sub-header"
    );
    assert_eq!(
        PRACTICE_FLOW_WORDING_KEYS.len(),
        33,
        "PRACTICE flow v1, mockup v3: the deck listed with its two row controls, \
         the resume line, the top bar, and the sheet's flag list and two clauses"
    );
    assert_eq!(
        PRACTICE_ROW_WORDING_KEYS.len(),
        15,
        "PRACTICE v1, the Chuck review (14): the words about ONE question — the \
         way into it alone, its status on the row, the redirect tag and its \
         drawer line, and what she would point to. Plus the one-page work's \
         `answered_on_template`, which becomes the ONLY status a row carries \
         once the marks are retired from the interface"
    );
    assert_eq!(
        PRACTICE_EDITOR_WORDING_KEYS.len(),
        46,
        "PRACTICE v1 Part B (45), plus the nav cleanup's drag grip. Part B's two \
         'Editing as' strings were retired with the picker and the hotfix's busy \
         hint and discard confirm took their places, so the hotfix was net zero; \
         the drag grip is the one addition on top"
    );
    assert_eq!(
        PRACTICE_LIST_WORDING_KEYS.len(),
        33,
        "PRACTICE one-page L2 (4): the practice bar's label, button and hint, plus \
         the footnote that explains why a row now carries at most a date. Plus \
         L3's line under a one-sentence critique, which is the COMMON rendering: \
         12 of 14 stored answers carry no three-part read. Plus Delete and its undo, declared in .408 after .407 rendered the page blank — they were seeded and declared nowhere, so the mirror had no field and the browser never saw them"
    );
    assert_eq!(
        PRACTICE_PRINT_WORDING_KEYS.len(),
        30,
        "Chuck's review sheets (2026-08-22): two controls on the practice page, \
         three on the print view, three sheet titles and two subtitles, the header \
         meta, four how-to lines, the antecedent and its named absence, the footer \
         and the SHEET number, and the six that say what the deck does not contain"
    );
    assert_eq!(
        PRACTICE_REPORT_WORDING_KEYS.len(),
        50,
        "PRACTICE v0, the report: mockup v2's reveal and Chuck's sheet — the two \
         surfaces that answer her back, plus T1's two read lines (2026-08-20): \
         the read declining in its own voice, and the stored line the \
         don't-recall button earns without a model call"
    );
    assert_eq!(
        seeded().len(),
        REQUIRED_KEYS.len()
            + PRACTICE_PARAM_KEYS.len()
            + WORDING_KEYS.len()
            + ACCUSATION_WORDING_KEYS.len()
            + REHEARSAL_WORDING_KEYS.len()
            + REHEARSAL_CHROME_KEYS.len()
            + AUTHORING_WORDING_KEYS.len()
            + SCENARIO_AUTHORING_WORDING_KEYS.len()
            + SCAN_WORDING_KEYS.len()
            + CARD_GRAMMAR_WORDING_KEYS.len()
            + MODEL_PARAMS_WORDING_KEYS.len()
            + MATRIX_WORDING_KEYS.len()
            + WAR_ROOM_WORDING_KEYS.len()
            + PRACTICE_WORDING_KEYS.len()
            + PRACTICE_FLOW_WORDING_KEYS.len()
            + PRACTICE_ROW_WORDING_KEYS.len()
            + PRACTICE_EDITOR_WORDING_KEYS.len()
            + PRACTICE_REPORT_WORDING_KEYS.len()
            + PRACTICE_PRINT_WORDING_KEYS.len()
            + PRACTICE_LIST_WORDING_KEYS.len(),
        "the seed and the twenty required lists must describe the same store"
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
    // SIX migrations now seed numeric parameters: 1.6's original seven, 2.10's
    // short-list cap, 2.11 B2's timeline threshold, 2.11 C's row-expand cap,
    // 2.15's three scan dials, and the one-card grammar's two fold thresholds.
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
        // Task 2.15 Tier 2 — the prompt-file row, the two pre-filter dials, and
        // the scan surface's words.
        "pipeline_migrations/20260808084539_theme_scan_tier2_settings_and_scan_wording.sql",
        // ONE_CARD_GRAMMAR — the question-truncation length and the element-chip
        // K, seeded with the card's own words for the same reason: they are that
        // surface's tunables.
        "pipeline_migrations/20260809121531_one_card_grammar_wording_and_settings.sql",
        // 2026-08-09: the judge's token budget — the eleventh number, and the
        // first that arrived because a compiled-in value was measured failing.
        "pipeline_migrations/20260809210501_scan_max_tokens_setting.sql",
        // Task R2 / 10e: the scan picker's default model — the twelfth, and the
        // first that arrived because the previous fallback was LIST ORDER rather
        // than anybody's decision.
        "pipeline_migrations/20260810114629_r2_391_unified_names_one_attack_box_and_scan_default_model.sql",
        // Task 396 P1: the three tier-map rows — the thirteenth, fourteenth and
        // fifteenth not-wording parameters, and the first that carry extraction
        // vocabulary rather than a number or a name.
        "pipeline_migrations/20260813152536_tuesday_batch_396_matrix_strength_war_room_and_human_fact_completeness.sql",
        // PRACTICE v0: the read's prompt file, its model, its token cap and the
        // seven-card vocabulary — the sixteenth through nineteenth not-wording
        // parameters, arriving with the surface that reads them.
        "pipeline_migrations/20260817213319_practice_session_v0.sql",
        // PRACTICE flow v1: no new not-wording parameter — it is here because
        // `seeded()` below reads this same list for the WORDING rows, and v3's
        // thirty-two navigation strings arrive in this file.
        "pipeline_migrations/20260818093139_practice_flow_v1_deck_controls_and_session_queue.sql",
        // PRACTICE v1 (the Chuck review): no new not-wording parameter either —
        // it is here because it CORRECTS one. `practice_read_prompt_file` moves
        // to v2, and the correction pass below is what sees it.
        "pipeline_migrations/20260819100411_practice_v1_chuck_review_deck_keys_kinds_and_points_to.sql",
        // PRACTICE v1 Part B: no new not-wording parameter, but it CORRECTS the
        // answer box's placeholder, and the correction pass below is what sees
        // it. The two author vocabularies live in the wording lists.
        "pipeline_migrations/20260819113610_practice_v1_part_b_deck_editor_notes_and_review.sql",
        // The attribution hotfix: the twentieth not-wording parameter
        // (practice_case_timezone) and the six hints that came with it.
        "pipeline_migrations/20260819135156_practice_hotfix_attribution_from_login_and_case_timezone.sql",
        // T1's four per-part ceilings. In the list on the day they were written:
        // this list going stale is the drift the test exists to catch, and a
        // parameter seeded by a migration nothing here reads would leave the
        // fixture free to say anything at all about it.
        "pipeline_migrations/20260820165501_practice_read_t1_per_part_storage.sql",
        // The print sheets' twenty-six strings. In the list the day they were
        // written: these go on PAPER that leaves the building, so a fixture that
        // drifted from the migration would be discovered by Chuck, not by a build.
        "pipeline_migrations/20260822154321_practice_print_questions_wording.sql",
        // The seed-question warning: it CORRECTS practice_intro, and the
        // correction pass below is what sees it.
        "pipeline_migrations/20260823101322_practice_seed_question_warning.sql",
        // L1 of the one-page work: `practice_row_answered_on_template`.
        "pipeline_migrations/20260823123657_practice_one_page_l1_answered_on.sql",
        // L2: the list page's new rows, and the three corrections —
        // "George's side", the text-link Edit label, and the how-to
        // sentence about a blue box that no longer prints.
        "pipeline_migrations/20260823134349_practice_one_page_l2_list_and_print_answers.sql",
        // L3: the line under a one-sentence critique.
        "pipeline_migrations/20260823163653_practice_one_page_l3_plain_read_line.sql",
        "pipeline_migrations/20260823164454_practice_one_page_l3_question_page_and_walk.sql",
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

    for key in REQUIRED_KEYS.iter().chain(PRACTICE_PARAM_KEYS) {
        // A later migration may have CORRECTED the row; the store's value is
        // then the correction, not the original insert. Checking only the
        // insert would let this test go green while the live store holds
        // something else — the exact drift it exists to catch.
        let seeded_value = crate::domain::wording::tests::corrected_value_in(&sql, key)
            .or_else(|| seeded_value_in(&sql, key))
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
    assert_eq!(
        checked,
        REQUIRED_KEYS.len() + PRACTICE_PARAM_KEYS.len(),
        "every not-wording parameter must be compared against the migration — \
         counted from BOTH lists, so adding a key to either cannot quietly skip it"
    );
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

// -----------------------------------------------------------------------------
// ⚑ EVERY app_settings INSERT NAMES THE COLUMNS THE TABLE ACTUALLY HAS
// -----------------------------------------------------------------------------
//
// Written after .406 crash-looped DEV. Four migrations inserted into
// `app_settings` naming a column `kind`; the column is `value_kind`. The boot
// migrator hit it, panicked, and the backend restarted forever.
//
// NOTHING IN THIS REPOSITORY EXECUTED A MIGRATION BEFORE THE DEPLOY DID.
// `check-migrations.sh` compares filename prefixes. The wording fixtures PARSE
// the SQL for values and never look at the column list. Four migrations, 38
// inserts, four gate passes, and a column name that never existed.
//
// This is the cheapest possible guard against that exact class: read the
// migrations off disk (Rule 21) and require the column list of every
// `INSERT INTO app_settings` to be the one the CREATE TABLE declares.
//
// ## What it does NOT do
//
// It does not execute anything, so it cannot catch a null violation, a type
// mismatch or a constraint failure. Those need a migration run against a real
// database, which this project has no tier for — the same hole that leaves 113
// routed handlers reachable only by inspection. Until that tier exists, run
// `BEGIN; <file>; ROLLBACK;` against DEV before any deploy carrying a migration.

/// The columns `app_settings` actually has, in declaration order.
// STRUCTURAL: the shape of a table this repository owns, read from its own
// CREATE TABLE below and pinned here so a drift in either is a failure rather
// than a surprise at boot.
const APP_SETTINGS_COLUMNS: &[&str] = &[
    "key",
    "value",
    "value_kind",
    "default_value",
    "min_value",
    "max_value",
    "meaning",
    "consumed_by",
    "updated_at",
    "updated_by",
];

/// Every `.sql` under `pipeline_migrations`, oldest first.
fn every_pipeline_migration() -> Vec<(String, String)> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("pipeline_migrations");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("pipeline_migrations is readable")
        .map(|e| e.expect("dir entry readable").path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("sql"))
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|p| {
            let name = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
                .to_string();
            (
                name,
                std::fs::read_to_string(&p).expect("migration is UTF-8"),
            )
        })
        .collect()
}

/// The declared column list matches the table's own CREATE TABLE.
///
/// ANTI-VACUITY for the test below: it compares inserts against
/// `APP_SETTINGS_COLUMNS`, so a constant that had drifted from the real table
/// would let every wrong insert through while looking authoritative.
#[test]
fn the_pinned_columns_are_the_ones_the_table_declares() {
    let (_, create) = every_pipeline_migration()
        .into_iter()
        .find(|(_, sql)| sql.contains("CREATE TABLE app_settings"))
        .expect("app_settings is created by a migration");

    let body = &create[create.find("CREATE TABLE app_settings").unwrap_or(0)..];
    let body = &body[..body.find("\n);").unwrap_or(body.len())];

    for column in APP_SETTINGS_COLUMNS {
        assert!(
            body.contains(&format!("\n    {column} ")),
            "{column} is pinned here but not declared by CREATE TABLE app_settings"
        );
    }
}

/// No migration inserts into `app_settings` naming a column it does not have.
#[test]
fn every_app_settings_insert_names_real_columns() {
    let mut checked = 0_usize;

    for (name, sql) in every_pipeline_migration() {
        let mut from = 0;
        while let Some(at) = sql[from..].find("INSERT INTO app_settings") {
            let start = from + at;
            let open = sql[start..]
                .find('(')
                .map(|i| start + i)
                .expect("an INSERT names its columns");
            let close = sql[open..]
                .find(')')
                .map(|i| open + i)
                .expect("the column list closes");

            for column in sql[open + 1..close].split(',') {
                let column = column.trim();
                if column.is_empty() {
                    continue;
                }
                assert!(
                    APP_SETTINGS_COLUMNS.contains(&column),
                    "{name} inserts into app_settings naming {column:?}, which the \
                     table does not have. This is what crash-looped DEV on .406: \
                     `kind` where the column is `value_kind`. Real columns: \
                     {APP_SETTINGS_COLUMNS:?}"
                );
            }
            checked += 1;
            from = close;
        }
    }

    assert!(
        checked > 0,
        "no app_settings INSERT was examined — the scan read nothing"
    );
}
