// Tests for `domain::wording` — task 2.10, the configuration law extended to text.
//
// Two things need proving here, and the second is the one that would rot:
//
//   1. The placeholder machinery does what the refusal promises.
//   2. The test fixture still says what the MIGRATION seeds. A fixture compared
//      only to itself proves nothing — the same argument
//      `the_fixtures_carry_the_values_the_migration_actually_seeds` makes for the
//      seven numbers.

use super::*;
use crate::domain::link_cut::LinkCut;
use std::collections::HashMap;

// The template rules moved to their own module in task 2.11 B2 — they are the
// rules for ALL stored text, not this surface's.
use crate::domain::wording_accusation as accusation;
use crate::domain::wording_authoring as authoring;
use crate::domain::wording_rehearsal as rehearsal;
use crate::domain::wording_rehearsal_chrome as chrome;
use crate::domain::wording_templates::{missing_placeholders, render, REQUIRED_PLACEHOLDERS};

// ── The placeholder rules ────────────────────────────────────────────────────

#[test]
fn a_template_that_keeps_its_placeholders_is_accepted() {
    assert!(
        missing_placeholders(KEY_SUMMARY_TEMPLATE, "Linked to {allegations} — {cut}").is_empty()
    );
    assert!(missing_placeholders(KEY_PROGRESS_TEMPLATE, "{linked}/{total}").is_empty());
    assert!(missing_placeholders(KEY_SHOW_ALL_LABEL, "All {count} of them").is_empty());
}

#[test]
fn a_template_that_drops_a_placeholder_is_named_as_missing_it() {
    // The defect this exists to stop: a grammatical sentence with the FACT
    // removed. "You linked this to  · they'll use it against us."
    let missing = missing_placeholders(KEY_SUMMARY_TEMPLATE, "You linked this to it · {cut}.");
    assert_eq!(missing, vec!["{allegations}"]);

    let both = missing_placeholders(KEY_SUMMARY_TEMPLATE, "You linked this to something.");
    assert_eq!(both, vec!["{allegations}", "{cut}"]);
}

#[test]
fn a_key_with_no_required_placeholders_never_reports_one() {
    // Most keys are plain sentences. They must not be dragged into the template
    // rules by accident — "Cancel" is a complete label.
    assert!(missing_placeholders(KEY_CANCEL_LABEL, "Cancel").is_empty());
    assert!(missing_placeholders(KEY_PANEL_INTRO, "anything at all").is_empty());
    assert!(missing_placeholders("not_a_key_at_all", "").is_empty());
}

#[test]
fn every_key_with_required_placeholders_is_a_real_key() {
    // Anti-drift: a requirement filed against a renamed key would silently stop
    // guarding anything, because `missing_placeholders` returns empty for a key
    // it does not find.
    //
    // All SEVEN stored-string lists count (2.10, 2.11 B1/B2, 2.11 C, the
    // 2026-08-07 scenario-authoring block, and 2.15's scan block): this table is
    // the one place the write path looks, so it carries every surface's templates
    // as well as this module's own.
    for (key, required) in REQUIRED_PLACEHOLDERS {
        assert!(
            WORDING_KEYS.contains(key)
                || accusation::ACCUSATION_WORDING_KEYS.contains(key)
                || rehearsal::REHEARSAL_WORDING_KEYS.contains(key)
                || chrome::REHEARSAL_CHROME_KEYS.contains(key)
                || authoring::AUTHORING_WORDING_KEYS.contains(key)
                || crate::domain::wording_scenario_authoring::SCENARIO_AUTHORING_WORDING_KEYS
                    .contains(key)
                || crate::domain::wording_scan::SCAN_WORDING_KEYS.contains(key),
            "{key} has placeholder requirements but is not a stored wording key"
        );
        assert!(!required.is_empty(), "{key} declares an empty requirement");
    }
}

#[test]
fn the_seeded_defaults_satisfy_their_own_placeholder_rules() {
    // If they did not, the migration would seed a store that the write path would
    // refuse — a value nobody could edit back to its own default.
    let mut values = Wording::for_test_values();
    // The table spans both stored-string lists (task 2.11), so the fixture it is
    // checked against has to as well — otherwise this test would look up an
    // accusation key in the link-panel fixture and fail on the lookup rather than
    // on the rule it exists to check.
    values.extend(accusation::AccusationWording::for_test_values());
    values.extend(rehearsal::RehearsalWording::for_test_values());
    values.extend(chrome::RehearsalChromeWording::for_test_values());
    values.extend(authoring::AuthoringWording::for_test_values());
    values.extend(
        crate::domain::wording_scenario_authoring::ScenarioAuthoringWording::for_test_values(),
    );
    values.extend(crate::domain::wording_scan::ScanWording::for_test_values());
    for (key, _) in REQUIRED_PLACEHOLDERS {
        let seeded = values.get(key).expect("a seeded value for every key");
        assert!(
            missing_placeholders(key, seeded).is_empty(),
            "the seeded default for {key} does not satisfy its own rule: {seeded}"
        );
    }
}

// ── Rendering ────────────────────────────────────────────────────────────────

#[test]
fn rendering_fills_every_placeholder_it_is_given() {
    let out = render(
        "You linked this to {allegations} · {cut}.",
        &[
            ("allegations", "\u{b6}41 — refused to divide the property"),
            ("cut", "they'll use it against us"),
        ],
    );
    assert_eq!(
        out,
        "You linked this to \u{b6}41 — refused to divide the property · they'll use it against us."
    );
}

#[test]
fn a_placeholder_nobody_supplied_stays_visible_rather_than_vanishing() {
    // Deliberate, and the honest end of the chain: a template can only lose a
    // value through an edit the write path refuses, so a `{cut}` reaching a screen
    // means somebody edited the store around the API. Substituting "" would give a
    // confident sentence with a hole in it and nothing to notice.
    let out = render(
        "You linked this to {allegations} · {cut}.",
        &[("allegations", "\u{b6}41")],
    );
    assert!(out.contains("{cut}"), "{out}");
}

#[test]
fn rendering_a_template_with_nothing_to_fill_returns_it_unchanged() {
    assert_eq!(render("Cancel", &[]), "Cancel");
}

#[test]
fn a_value_containing_braces_is_not_re_scanned() {
    // Replacement walks the placeholder list once, so a value that happens to
    // contain `{cut}` cannot be substituted into by a later pass. An accusation's
    // own text is arbitrary human prose and must never be treated as a template.
    let out = render(
        "{allegations} · {cut}",
        &[
            ("allegations", "\u{b6}9 — the {cut} clause"),
            ("cut", "it supports us"),
        ],
    );
    assert_eq!(out, "\u{b6}9 — the {cut} clause · it supports us");
}

// ── The enum meets its two sentence forms ────────────────────────────────────

#[test]
fn each_cut_gets_its_own_stored_sentence_form() {
    let wording = Wording::for_test();
    assert_eq!(wording.cut_phrase(LinkCut::Supports), "it supports us");
    assert_eq!(
        wording.cut_phrase(LinkCut::Against),
        "they'll use it against us"
    );
}

#[test]
fn the_sentence_form_is_a_stored_row_and_not_a_lowercased_label() {
    // The rule stated as a test: nothing in code changes the case of a word a
    // human wrote. If someone deleted the phrase rows and lowercased the labels,
    // this is what would fail.
    let mut wording = Wording::for_test();
    wording.link_cut_supports_phrase = "it helps our side".to_string();
    assert_eq!(wording.cut_phrase(LinkCut::Supports), "it helps our side");
    assert_ne!(
        wording.cut_phrase(LinkCut::Supports),
        wording.link_cut_supports_label.to_lowercase()
    );
}

// ── The fixture is checked against the MIGRATION, not against itself ─────────

/// The migrations that between them decide what the store holds.
///
/// TWO of them now, and the order matters: 2.10 seeds the rows and 2.12
/// CORRECTS one. `link_save_label` was seeded 'Save and next' and is updated to
/// 'Save', so the effective value is the INSERT as amended — a reader that
/// stopped at the INSERT would assert this fixture into disagreeing with the
/// product, which is the exact failure task 2.12 item A calls "a lying button".
const SEED_MIGRATION: &str =
    "pipeline_migrations/20260804132730_create_evidence_allegation_links.sql";
const CORRECTION_MIGRATION: &str =
    "pipeline_migrations/20260804162538_card_and_facts_usability_wording.sql";
/// Task 2.13 slice 1's fourteen rows — the facts list as a prep surface, plus the
/// queue summary split that stops a header claiming the work is done before it
/// has counted. A THIRD file rather than an edit to either of the two above:
/// `ON CONFLICT (key) DO NOTHING` means editing an applied migration's VALUES
/// list changes nothing on any environment that has already run it (2.12's
/// lesson), so new rows can only arrive in a new migration.
const SLICE1_MIGRATION: &str = "pipeline_migrations/20260805092240_facts_prep_slice1_wording.sql";
/// The queue's counting-state row — the third state 2.13 was missing.
const COUNTING_MIGRATION: &str =
    "pipeline_migrations/20260805120900_queue_counting_state_wording.sql";
/// Task 2.13c's five rows — the answer label, the background-move notice, the
/// weights hint, the honest footer, and the un-place control.
const POLISH_MIGRATION: &str = "pipeline_migrations/20260805145731_facts_polish_2_13c_wording.sql";
/// Task 2.15 Tier 2: the raw-pool opt-in, seeded with the scan work whose defect
/// it answers rather than in a migration of its own.
const TIER2_MIGRATION: &str =
    "pipeline_migrations/20260808084539_theme_scan_tier2_settings_and_scan_wording.sql";
/// Scan → ruling: the queue's proposal heading and the three things a PROPOSED
/// card says. A seventh file for the reason the third one gives — `ON CONFLICT
/// (key) DO NOTHING` makes editing an applied migration a no-op everywhere it has
/// already run, so new rows can only ever arrive in a new file.
const PROJECTION_MIGRATION: &str = "pipeline_migrations/20260808141052_scan_to_ruling_wording.sql";
/// The ruling-acknowledgment rows: every ruling says what it did, and a locked
/// card states its condition on its face. Seeded after the measured beta.385
/// silent-defer walk.
// STRUCTURAL: a migration filename, not a deployment value. This test reads the
// seed off disk to prove the fixture below says what the migration actually
// seeds — a declared key with no row is a BOOT REFUSAL, and reading the file is
// the only thing that catches that before a deploy takes DEV down (Rule 21).
// The path is therefore part of the assertion itself: it can only change when
// somebody renames the file in this repo, and it cannot vary per environment.
// The seven sibling constants above are the same shape and predate this task.
const ACKNOWLEDGMENT_MIGRATION: &str =
    "pipeline_migrations/20260808171630_ruling_acknowledgment_wording.sql";

// =============================================================================
// ⚑ THE RULE FOR ANYTHING IN THIS REPOSITORY THAT READS SOURCE OR SQL
// =============================================================================
//
// **Strip comments before you look.**
//
// Not "be careful" — a mechanical step, every time, in every scanner. The root
// cause is not a mistake anyone will stop making, because it is caused by
// something this codebase does deliberately and should keep doing:
//
//   THIS CODEBASE DOCUMENTS ITS RULES NEXT TO ITS RULES.
//
// A migration that parses `SET value         = '` explains that format in its
// own header. A wording fixture reads a seed migration whose comments quote the
// syntax it is parsing. A test that asserts a function name is ABSENT sits in a
// file explaining why that function was deleted. Every one of those is good
// practice, and every one of them puts the string a naive parser is hunting for
// EARLIER in the file than the thing it actually wants.
//
// Measured, three times on 2026-08-23:
//
//   1. A migration's header quoted `SET value         = '`; an overlay parser
//      captured the prose and printed a migration's comments onto S-5's
//      redirect sheet.
//   2. `corrected_value_in` below survives the same decoy ONLY because it uses
//      `rfind` — the decoy always happens to come first. That is luck, not
//      design, and it is logged for the T2 window.
//   3. `api::practice_answers::tests` failed on the handler's own comment
//      explaining a deletion, and now strips comments before scanning.
//
// The three sites point HERE rather than each carrying a copy, because the
// fourth scanner will be in a file nobody thought to copy it into.

/// Pull one key's seeded value out of the migration's INSERT.
///
/// ## Why this handles doubled quotes and the 1.6 version did not
///
/// SQL escapes a literal apostrophe by doubling it, and these are sentences:
/// "isn''t", "They''ll", "can''t". A scan that stopped at the first `'` would
/// report `link_panel_intro` as "This statement isn" and the comparison below
/// would fail on every row that contains an apostrophe — which is most of the
/// interesting ones. So this walks the literal, treating `''` as one character.
pub(crate) fn seeded_value_in(sql: &str, key: &str) -> Option<String> {
    let marker = format!("('{key}',");
    let at = sql.find(&marker)?;
    let rest = &sql[at + marker.len()..];
    // The value literal starts at the first quote after the key.
    let open = rest.find('\'')?;
    let mut out = String::new();
    let mut chars = rest[open + 1..].chars();
    while let Some(c) = chars.next() {
        if c != '\'' {
            out.push(c);
            continue;
        }
        // A doubled quote is one literal apostrophe; a single one ends the value.
        match chars.next() {
            Some('\'') => out.push('\''),
            _ => return Some(out),
        }
    }
    None
}

/// The value a key ends up with: seeded by one migration, possibly corrected by
/// a later one.
///
/// Deliberately crude, like its INSERT sibling: it looks for
/// `SET value = '…'` inside a statement whose WHERE names this key. A migration
/// that corrected a row some other way would not be found — and the count
/// assertion below is what would catch that, by leaving the fixture disagreeing.
pub(crate) fn corrected_value_in(sql: &str, key: &str) -> Option<String> {
    // `rfind` and not `find`: the caller concatenates the migrations IN ORDER, and
    // a key corrected TWICE ends up holding the LAST correction. Searching
    // forwards returned the first one — which was harmless for as long as no key
    // had been corrected twice, and stopped being harmless on 2026-08-20 when T1
    // moved `practice_read_prompt_file` from v2 to v3 and this reported v2. A
    // fixture pinned to a superseded correction is the exact drift these helpers
    // exist to catch, reported as a pass.
    let at = sql.rfind(&format!("key           = '{key}'"))?;
    // The SET clause precedes the WHERE in an UPDATE, so scan backwards.
    let before = &sql[..at];
    let set = before.rfind("SET value         = '")?;
    let rest = &before[set + "SET value         = '".len()..];
    let end = rest.find('\'')?;
    Some(rest[..end].to_string())
}

/// The value the store actually ends up holding for one key.
fn effective_value(seed_sql: &str, correction_sql: &str, key: &str) -> Option<String> {
    corrected_value_in(correction_sql, key).or_else(|| seeded_value_in(seed_sql, key))
}

#[test]
fn the_wording_fixture_carries_the_values_the_migration_actually_seeds() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let sql = std::fs::read_to_string(root.join(SEED_MIGRATION))
        .expect("the wording migration is on disk");
    let correction = std::fs::read_to_string(root.join(CORRECTION_MIGRATION))
        .expect("the correction migration is on disk");
    let slice1 = std::fs::read_to_string(root.join(SLICE1_MIGRATION))
        .expect("the 2.13 slice-1 wording migration is on disk");
    let counting = std::fs::read_to_string(root.join(COUNTING_MIGRATION))
        .expect("the queue counting-state wording migration is on disk");
    let polish = std::fs::read_to_string(root.join(POLISH_MIGRATION))
        .expect("the 2.13c polish wording migration is on disk");
    let tier2 = std::fs::read_to_string(root.join(TIER2_MIGRATION))
        .expect("the 2.15 Tier-2 scan migration is on disk");
    let projection = std::fs::read_to_string(root.join(PROJECTION_MIGRATION))
        .expect("the scan-to-ruling wording migration is on disk");
    let acknowledgment = std::fs::read_to_string(root.join(ACKNOWLEDGMENT_MIGRATION))
        .expect("the ruling-acknowledgment wording migration is on disk");

    let fixture = Wording::for_test_values();
    let mut checked = 0usize;

    for key in WORDING_KEYS {
        let seeded = effective_value(&sql, &correction, key)
            .or_else(|| seeded_value_in(&correction, key))
            .or_else(|| seeded_value_in(&slice1, key))
            .or_else(|| seeded_value_in(&counting, key))
            .or_else(|| seeded_value_in(&polish, key))
            .or_else(|| seeded_value_in(&tier2, key))
            .or_else(|| seeded_value_in(&projection, key))
            .or_else(|| seeded_value_in(&acknowledgment, key))
            .unwrap_or_else(|| panic!("{key} is not seeded by any migration"));
        let in_fixture = fixture
            .get(*key)
            .unwrap_or_else(|| panic!("{key} is missing from the test fixture"));

        assert_eq!(
            in_fixture, &seeded,
            "the fixture has {key} = '{in_fixture}' but the migration seeds \
             '{seeded}'. One moved without the other, and every test that asserts \
             on this wording is now describing something the product does not say."
        );
        checked += 1;
    }

    // Anti-vacuity: a parsing change that stopped finding rows would otherwise
    // make this test pass while comparing nothing.
    assert_eq!(
        checked,
        WORDING_KEYS.len(),
        "every stored string must be compared — counted from the list itself so \
         adding a key cannot quietly skip it"
    );
    assert_eq!(WORDING_KEYS.len(), 58);
}

// ── Item A's correction: the one that can ship a lying button ────────────────

/// The save label no longer promises to move the human anywhere.
///
/// THE POINT OF ITEM A, as an assertion on the stored text rather than on the
/// code: saving a link now leaves the selection where it is, so a label reading
/// "and next" would be the button describing behaviour it no longer has. This
/// checks the EFFECTIVE value — the seed as amended by the correction — so it
/// fails if the UPDATE is dropped, mis-keyed, or silently matched nothing.
#[test]
fn the_stored_save_label_does_not_promise_navigation() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let seed = std::fs::read_to_string(root.join(SEED_MIGRATION)).expect("seed on disk");
    let correction =
        std::fs::read_to_string(root.join(CORRECTION_MIGRATION)).expect("correction on disk");

    let effective = effective_value(&seed, &correction, KEY_SAVE_LABEL).expect("a value");
    assert_eq!(effective, "Save");
    assert!(
        !effective.to_lowercase().contains("next"),
        "the button no longer moves anyone: {effective}"
    );
    // And the seed really did hold the old text — otherwise this test would pass
    // by accident on a migration that never needed correcting.
    assert_eq!(
        seeded_value_in(&seed, KEY_SAVE_LABEL).as_deref(),
        Some("Save and next"),
        "the 2.10 seed is what makes the UPDATE necessary"
    );
}

/// The correction fires ONLY on a row still holding the old default.
///
/// A human may have edited this label since 2.10 shipped, and their words are
/// not ours to overwrite. The guard is the VALUE itself rather than
/// `updated_by`: somebody who edited the label and set it back would be wrongly
/// protected by an attribution check, and somebody who changed another column
/// would be wrongly overwritten by one.
#[test]
fn the_save_label_correction_only_fires_on_the_untouched_default() {
    let sql = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(CORRECTION_MIGRATION),
    )
    .expect("correction on disk");

    let stmt = sql
        .split_once("UPDATE app_settings")
        .map(|(_, rest)| rest.split_once(';').map(|(s, _)| s).unwrap_or(rest))
        .expect("the correction is an UPDATE");

    assert!(
        stmt.contains("value         = 'Save and next'"),
        "the WHERE must name the old value: {stmt}"
    );
    assert!(
        stmt.contains("default_value = 'Save and next'"),
        "and the old default, so a half-edited row is left alone: {stmt}"
    );
    assert!(
        !stmt.contains("updated_by"),
        "a migration is not a person; the attribution column stays as it was: {stmt}"
    );
}

/// The correction moves the DEFAULT as well as the value.
///
/// Otherwise the Settings page advertises "Default Save and next" for a default
/// that no longer exists, and the stored row falls out of step with
/// `Wording::for_test`.
#[test]
fn the_correction_moves_the_default_as_well_as_the_value() {
    let sql = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(CORRECTION_MIGRATION),
    )
    .expect("correction on disk");
    assert!(sql.contains("SET value         = 'Save'"));
    assert!(sql.contains("default_value = 'Save',"));
}

#[test]
fn the_seed_parser_reads_an_escaped_apostrophe_as_one_character() {
    // The parser is the only thing standing between the test above and a false
    // green, so it gets its own test rather than being trusted.
    let sql = "VALUES ('a_key', 'it isn''t over', 'text',";
    assert_eq!(
        seeded_value_in(sql, "a_key").as_deref(),
        Some("it isn't over")
    );
}

#[test]
fn no_stored_string_is_blank() {
    // A blank label is an invisible control. The store refuses one on write; this
    // pins the DEFAULTS, which the write path never sees.
    for (key, value) in Wording::for_test_values() {
        assert!(!value.trim().is_empty(), "{key} seeds a blank string");
    }
}

/// The seeded wording, for TESTS ONLY.
///
/// ## Why this is `#[cfg(test)]`, exactly like `Settings::for_test`
///
/// A production-reachable `Default for Wording` would be a compiled-in set of
/// user-facing strings — the precise defect Roman's ruling deletes — and it would
/// be reachable by accident through `unwrap_or_default()`. Gating it on
/// `cfg(test)` means it cannot exist in a release binary at all.
///
/// The values match the migration's seed, and `wording_tests` asserts that
/// against the migration file itself, so the two cannot drift silently.
#[cfg(test)]
impl Wording {
    pub fn for_test() -> Self {
        Wording {
            link_panel_intro: "This statement isn't linked to anything they've accused you of. \
                               Link it and you can rule it."
                .to_string(),
            link_scope_notice: "An accusation link belongs to the statement, not to this \
                                scenario — linking it here links it wherever this item appears."
                .to_string(),
            link_allegations_heading: "What it bears on".to_string(),
            link_show_all_label: "Show all {count}".to_string(),
            link_filter_placeholder: "Type to filter".to_string(),
            link_no_match_notice: "No accusation matches that.".to_string(),
            link_empty_options_notice: "No accusations are stored for this case yet, so there \
                                        is nothing to link to."
                .to_string(),
            link_cut_heading: "How it cuts".to_string(),
            link_cut_supports_label: "It supports us".to_string(),
            link_cut_against_label: "They'll use it against us".to_string(),
            link_cut_supports_phrase: "it supports us".to_string(),
            link_cut_against_phrase: "they'll use it against us".to_string(),
            // Task 2.12 item A: the button no longer moves anyone, so it no
            // longer says it does. The migration UPDATEs the already-seeded row.
            link_save_label: "Save".to_string(),
            link_cancel_label: "Cancel".to_string(),
            link_unlink_label: "Unlink".to_string(),
            link_missing_cut_refusal: "Say which way this cuts before saving — a link with no \
                                       cut can't tell ammunition from hazard."
                .to_string(),
            link_missing_allegation_refusal: "Tick at least one accusation before saving."
                .to_string(),
            link_summary_template: "You linked this to {allegations} · {cut}.".to_string(),
            link_progress_template: "{linked} of {total} linked.".to_string(),
            question_revert_label: "Restore the system's wording".to_string(),
            link_unlink_found_nothing: "That link was already gone — your view was out of \
                                        date, and it has been refreshed."
                .to_string(),
            link_save_failed_template: "That link did not save: {detail} The queue has been \
                                        reloaded from the server, so what you see now is \
                                        what is stored."
                .to_string(),
            link_save_blocks_ruling: "Save the link first.".to_string(),
            fact_remove_label: "Remove".to_string(),
            fact_remove_confirm_template: "Remove {code} from this scenario? It goes back \
                                           to the queue as not ruled."
                .to_string(),
            fact_remove_confirm_yes: "Remove it".to_string(),
            fact_remove_confirm_cancel: "Keep it".to_string(),
            fact_remove_failed_template: "That fact could not be removed: {detail} Reload \
                                          the page to see what is actually stored."
                .to_string(),
            fact_question_label: "Q:".to_string(),
            fact_statement_kind_label: "Kind:".to_string(),
            fact_tier_carries_label: "Carries the scenario".to_string(),
            fact_tier_backup_label: "Backup".to_string(),
            fact_tier_background_label: "Background".to_string(),
            fact_tier_prompt: "How much does this fact carry?".to_string(),
            fact_tier_background_default_state: "collapsed".to_string(),
            fact_background_count_template: "{count} in the background · show".to_string(),
            fact_background_hide_label: "Hide background".to_string(),
            fact_order_drag_hint: "Drag to reorder".to_string(),
            fact_tier_save_failed_template: "Could not save the weight for {code}. {reason}"
                .to_string(),
            fact_order_save_failed_template: "Could not save the new order for {code}. {reason}"
                .to_string(),
            queue_empty_pool_summary: "No candidates gathered yet".to_string(),
            queue_all_ruled_summary: "All candidates ruled".to_string(),
            queue_counting_summary: "Counting candidates…".to_string(),
            fact_answer_label: "A:".to_string(),
            fact_background_move_notice: "{code} moved to the background pile — show".to_string(),
            fact_weights_hint: "Star each fact by how much it carries — carries, backup, \
                                background — and drag to set the order."
                .to_string(),
            fact_footer_template: "{shown} shown · {background} in background".to_string(),
            fact_unplace_label: "Clear my order".to_string(),
            queue_raw_pool_toggle_template: "Browse the raw evidence pool ({count})".to_string(),
            queue_proposed_heading_template:
                "Candidates awaiting ruling — {count} proposed by the {when} scan".to_string(),
            card_proposed_attribution_template: "Proposed by the {when} scan".to_string(),
            card_proposed_role_template: "Scan: {verb}".to_string(),
            card_proposed_covers_template: "×{count} — covers {codes}".to_string(),
            card_ruling_saved_template: "Saved — {code} is now {state}.".to_string(),
            card_ruling_left_filter_template:
                "{code} has left the {filter} list — that is where ruled candidates go.".to_string(),
            card_defer_recorded_template: "Deferred. The reason recorded is: {reason}".to_string(),
            card_ruling_failed_template:
                "{code} could not be saved: {detail} The queue has been reloaded, so what \
                 you see now is what is stored."
                    .to_string(),
            card_locked_condition_label: "Include and Exclude are closed on this card:".to_string(),
        }
    }

    /// The test fixture as a key→value map, in the shape the store reads.
    ///
    /// Lets `settings_store_tests` extend its own seeded fixture without holding a
    /// second hand-typed copy of twenty sentences — the copy that matters is
    /// [`Wording::for_test`], and `wording_tests` pins THAT to the migration.
    pub fn for_test_values() -> HashMap<&'static str, String> {
        let w = Wording::for_test();
        WORDING_KEYS
            .iter()
            .map(|key| {
                let value = match *key {
                    KEY_PANEL_INTRO => w.link_panel_intro.clone(),
                    KEY_SCOPE_NOTICE => w.link_scope_notice.clone(),
                    KEY_ALLEGATIONS_HEADING => w.link_allegations_heading.clone(),
                    KEY_SHOW_ALL_LABEL => w.link_show_all_label.clone(),
                    KEY_FILTER_PLACEHOLDER => w.link_filter_placeholder.clone(),
                    KEY_NO_MATCH_NOTICE => w.link_no_match_notice.clone(),
                    KEY_EMPTY_OPTIONS_NOTICE => w.link_empty_options_notice.clone(),
                    KEY_CUT_HEADING => w.link_cut_heading.clone(),
                    KEY_CUT_SUPPORTS_LABEL => w.link_cut_supports_label.clone(),
                    KEY_CUT_AGAINST_LABEL => w.link_cut_against_label.clone(),
                    KEY_CUT_SUPPORTS_PHRASE => w.link_cut_supports_phrase.clone(),
                    KEY_CUT_AGAINST_PHRASE => w.link_cut_against_phrase.clone(),
                    KEY_SAVE_LABEL => w.link_save_label.clone(),
                    KEY_CANCEL_LABEL => w.link_cancel_label.clone(),
                    KEY_UNLINK_LABEL => w.link_unlink_label.clone(),
                    KEY_MISSING_CUT_REFUSAL => w.link_missing_cut_refusal.clone(),
                    KEY_MISSING_ALLEGATION_REFUSAL => w.link_missing_allegation_refusal.clone(),
                    KEY_SUMMARY_TEMPLATE => w.link_summary_template.clone(),
                    KEY_PROGRESS_TEMPLATE => w.link_progress_template.clone(),
                    KEY_QUESTION_REVERT_LABEL => w.question_revert_label.clone(),
                    KEY_UNLINK_FOUND_NOTHING => w.link_unlink_found_nothing.clone(),
                    KEY_SAVE_FAILED_TEMPLATE => w.link_save_failed_template.clone(),
                    KEY_SAVE_BLOCKS_RULING => w.link_save_blocks_ruling.clone(),
                    KEY_FACT_REMOVE_LABEL => w.fact_remove_label.clone(),
                    KEY_FACT_REMOVE_CONFIRM => w.fact_remove_confirm_template.clone(),
                    KEY_FACT_REMOVE_YES => w.fact_remove_confirm_yes.clone(),
                    KEY_FACT_REMOVE_CANCEL => w.fact_remove_confirm_cancel.clone(),
                    KEY_FACT_REMOVE_FAILED => w.fact_remove_failed_template.clone(),
                    KEY_FACT_QUESTION_LABEL => w.fact_question_label.clone(),
                    KEY_FACT_STATEMENT_KIND_LABEL => w.fact_statement_kind_label.clone(),
                    KEY_FACT_TIER_CARRIES => w.fact_tier_carries_label.clone(),
                    KEY_FACT_TIER_BACKUP => w.fact_tier_backup_label.clone(),
                    KEY_FACT_TIER_BACKGROUND => w.fact_tier_background_label.clone(),
                    KEY_FACT_TIER_PROMPT => w.fact_tier_prompt.clone(),
                    KEY_FACT_BACKGROUND_DEFAULT_STATE => {
                        w.fact_tier_background_default_state.clone()
                    }
                    KEY_FACT_BACKGROUND_COUNT => w.fact_background_count_template.clone(),
                    KEY_FACT_BACKGROUND_HIDE => w.fact_background_hide_label.clone(),
                    KEY_FACT_ORDER_DRAG_HINT => w.fact_order_drag_hint.clone(),
                    KEY_FACT_TIER_SAVE_FAILED => w.fact_tier_save_failed_template.clone(),
                    KEY_FACT_ORDER_SAVE_FAILED => w.fact_order_save_failed_template.clone(),
                    KEY_QUEUE_EMPTY_POOL => w.queue_empty_pool_summary.clone(),
                    KEY_QUEUE_ALL_RULED => w.queue_all_ruled_summary.clone(),
                    KEY_QUEUE_COUNTING => w.queue_counting_summary.clone(),
                    KEY_FACT_ANSWER_LABEL => w.fact_answer_label.clone(),
                    KEY_FACT_BG_MOVE_NOTICE => w.fact_background_move_notice.clone(),
                    KEY_FACT_WEIGHTS_HINT => w.fact_weights_hint.clone(),
                    KEY_FACT_FOOTER => w.fact_footer_template.clone(),
                    KEY_FACT_UNPLACE_LABEL => w.fact_unplace_label.clone(),
                    KEY_QUEUE_RAW_POOL_TOGGLE => w.queue_raw_pool_toggle_template.clone(),
                    KEY_QUEUE_PROPOSED_HEADING => w.queue_proposed_heading_template.clone(),
                    KEY_CARD_PROPOSED_ATTRIBUTION => w.card_proposed_attribution_template.clone(),
                    KEY_CARD_PROPOSED_ROLE => w.card_proposed_role_template.clone(),
                    KEY_CARD_PROPOSED_COVERS => w.card_proposed_covers_template.clone(),
                    KEY_CARD_RULING_SAVED => w.card_ruling_saved_template.clone(),
                    KEY_CARD_RULING_LEFT_FILTER => w.card_ruling_left_filter_template.clone(),
                    KEY_CARD_DEFER_RECORDED => w.card_defer_recorded_template.clone(),
                    KEY_CARD_RULING_FAILED => w.card_ruling_failed_template.clone(),
                    KEY_CARD_LOCKED_CONDITION => w.card_locked_condition_label.clone(),
                    // Unreachable by construction: the match above covers every
                    // entry of WORDING_KEYS, and the test below proves it. A panic
                    // here would only fire in a test binary, on a key someone added
                    // to the list and forgot here — which is exactly when a loud
                    // stop is what you want.
                    other => panic!("{other} is in WORDING_KEYS but not in for_test_values"),
                };
                (*key, value)
            })
            .collect()
    }
}
