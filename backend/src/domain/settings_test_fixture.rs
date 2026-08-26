//! `Settings::for_test()` — the fixture every unit test builds from.
//!
//! Split out of `domain::settings` on 2026-08-25, when Phase B's two new fields
//! would have carried that file past Rule 17's 300-line limit. Nothing about the
//! fixture changed in the move.
//!
//! ## Rust Learning: an inherent `impl` in another module
//!
//! `impl Settings` is written HERE while `struct Settings` is declared in the
//! parent. Rust allows that for any type defined in the same CRATE — coherence
//! is a rule about crates, not modules — so a type's data and one coherent
//! family of its methods can be filed separately with no trait and no wrapper.
//! `dto::practice_wording_map` makes the same move for the same reason.

use super::*;

/// A snapshot for TESTS ONLY.
///
/// ## Why this is `#[cfg(test)]` and not a `Default` impl
///
/// A `Default for Settings` would be a compiled-in set of parameters — the exact
/// defect v2 §2b bans — and worse, it would be reachable from production code by
/// accident (`..Default::default()`, `unwrap_or_default()`). Gating it on
/// `cfg(test)` means it cannot exist in a release binary at all: production has
/// one way to obtain a `Settings`, and that is to read the store.
///
/// The values match the migration's seed so a test reads the way the product
/// behaves. `settings_store_tests` separately asserts the seed still produces
/// exactly these numbers, so the two cannot drift apart silently.
#[cfg(test)]
impl Settings {
    pub fn for_test() -> Self {
        Settings {
            confidence_band_high: 0.80,
            confidence_band_medium: 0.50,
            quote_context_window_chars: 240,
            talking_points_cap: 3,
            readiness_item_threshold_n: 5,
            card_test_ratio: Ratio {
                numerator: 9,
                denominator: 10,
            },
            reanchor_close_match_tolerance: 0.85,
            link_short_list_max: 8,
            wording: Wording::for_test(),
            accusation_wording: AccusationWording::for_test(),
            rehearsal_wording: RehearsalWording::for_test(),
            rehearsal_timeline_min_distinct_dates: 2,
            rehearsal_chrome_wording: RehearsalChromeWording::for_test(),
            authoring_wording: AuthoringWording::for_test(),
            scenario_authoring_wording: ScenarioAuthoringWording::for_test(),
            theme_scan_prompt_file: "theme_scan_prompt_v3.md".to_string(),
            theme_scan_max_tokens: 8192,
            theme_scan_default_model: "claude-opus-5".to_string(),
            theme_scan_prefilter_min_chars: 60,
            theme_scan_prefilter_statement_types: vec!["referral".to_string()],
            scan_wording: ScanWording::for_test(),
            rehearsal_instance_rows_expand_max: 3,
            card_grammar_wording: CardGrammarWording::for_test(),
            model_params_wording: ModelParamsWording::for_test(),
            chronology_wording: ChronologyWording::for_test(),
            chronology_phase_window_events: 4,
            chronology_document_picker_max: 20,
            matrix_wording: MatrixWording::for_test(),
            war_room_wording: WarRoomWording::for_test(),
            practice_wording: PracticeWording::for_test(),
            practice_report_wording: PracticeReportWording::for_test(),
            practice_read: PracticeReadParams::for_test(),
            evidence_tier_map: EvidenceTierMap::for_test(),
            card_question_truncate_chars: 110,
            card_element_chips_visible_k: 2,
        }
    }
}
