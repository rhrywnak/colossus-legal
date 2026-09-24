// Tests for `domain::wording_ai_jobs`, and the fixture every other suite uses.
//
// Same law as its siblings: a key declared to the boot loader with no row in the
// migration makes the backend REFUSE TO START, and reading the migration off
// disk is the only thing that catches it before a deploy does (Rule 21).

use super::*;
use crate::domain::wording::tests::seeded_value_in;
use std::collections::HashMap;

/// The one migration that seeds this block.
const SEED_MIGRATION: &str =
    "pipeline_migrations/20260924145151_ai_jobs_panel_and_last_run_wording.sql";

/// The seeded values, for TESTS ONLY — kept beside the test that pins them to
/// the migration file, so a fixture and its proof cannot drift apart.
const TEST_SEED: &[(&str, &str)] = &[
    (KEY_AI_JOBS_TITLE, "Which model and which instructions each AI job uses"),
    (KEY_AI_JOBS_INTRO, "A change takes effect on the next use. No rebuild, no redeploy, no restart."),
    (KEY_AI_JOBS_MODEL_LABEL, "Model"),
    (KEY_AI_JOBS_INSTRUCTIONS_LABEL, "Instructions"),
    (KEY_AI_JOBS_EFFORT_LABEL, "Thinking"),
    (KEY_AI_JOBS_SAVE_LABEL, "Save"),
    (KEY_AI_JOBS_EMPTY_CONTROL, "—"),
    (KEY_AI_JOBS_BUILT_IN, "Built in, not changeable here"),
    (KEY_AI_JOBS_FIXED_BY_SERVER, "Fixed by the server: {model}"),
    (KEY_AI_JOBS_FIXED_META, "Set in the server's configuration. Change it there, or remove that line to choose here."),
    (KEY_AI_JOBS_META_INSTALL, "Set by the install, never changed"),
    (KEY_AI_JOBS_META_CHANGED, "Changed by {who}, {when}"),
    (KEY_AI_JOBS_META_MODEL_CHANGED, "Model changed by {who}, {when}"),
    (KEY_AI_JOBS_META_INSTRUCTIONS_CHANGED, "Instructions changed by {who}, {when}"),
    (KEY_AI_JOBS_FOOT_GROUNDED, "Not listed: models that are switched off or have no \"Quotes checked\" tick."),
    (KEY_AI_JOBS_FOOT_ACTIVE, "Not listed: models that are switched off."),
    (KEY_AI_JOBS_FOOT_ANTHROPIC, "Not listed: models that are switched off, and local models, which the Chat page cannot call."),
    (KEY_AI_JOBS_FOOT_SCAN, "Not listed: models that are switched off or not offered for scans."),
    (KEY_AI_JOBS_FOOT_FILES, "New versions of these files arrive with a release. A version in use can't be edited or deleted."),
    (KEY_AI_JOBS_MARKER_IN_USE, "in use"),
    (KEY_AI_JOBS_MARKER_USED_BEFORE, "used before"),
    (KEY_AI_JOBS_MARKER_NEW, "new, never used"),
    (KEY_AI_JOBS_MARKER_EARLIER, "earlier version"),
    (KEY_AI_JOBS_EFFORT_LOW, "Low"),
    (KEY_AI_JOBS_EFFORT_MEDIUM, "Medium"),
    (KEY_AI_JOBS_EFFORT_HIGH, "High"),
    (KEY_AI_JOBS_EFFORT_XHIGH, "Extra high"),
    (KEY_AI_JOBS_EFFORT_MAX, "Max"),
    (KEY_AI_JOBS_EFFORT_UNSET, "The model's own default"),
    (KEY_AI_JOBS_WARN_MODEL_OFF, "{model} is switched off on Prompt Management → Models, so {job} can't run. Pick another model."),
    (KEY_AI_JOBS_WARN_MODEL_MISSING, "{model} is not in the list on Prompt Management → Models, so {job} can't run. Pick another model."),
    (KEY_AI_JOBS_WARN_MODEL_UNQUOTED, "{model} has no \"Quotes checked\" tick on Prompt Management → Models, so {job} can't run. Pick another model."),
    (KEY_AI_JOBS_WARN_MODEL_LOCAL, "{model} is a local model, which {job} cannot call. Pick another model."),
    (KEY_AI_JOBS_WARN_MODEL_NOT_SCAN, "{model} is not offered for scans on Prompt Management → Models, so {job} can't start on it. Pick another model."),
    (KEY_AI_JOBS_WARN_FILE_MISSING, "{file} is not in the instructions folder, so {job} can't run. Pick another version."),
    (KEY_AI_JOBS_SAVED, "{job} now uses {value}, from the next use."),
    (KEY_AI_JOBS_SAVED_RELOAD, "The next question reloads the case file (about {cost})."),
    (KEY_AI_JOBS_SAVED_RELOAD_UNPRICED, "The next question reloads the case file."),
    (KEY_AI_JOBS_WAS, "was {value}"),
    (KEY_AI_JOBS_SWITCH_BACK, "Switch back"),
    (KEY_AI_JOBS_REFUSE_FIXED, "{job} takes its model from the server's configuration. Change it there, or remove that line to choose here."),
    (KEY_AI_JOBS_REFUSE_NOTHING, "Nothing changed: {job} already uses that."),
    (KEY_AI_JOBS_REFUSE_NO_HISTORY, "There is nothing to switch back to: {job} has not been changed here."),
    (KEY_AI_JOBS_REFUSE_NOT_JOB, "{setting} is not one of {job}'s settings."),
    (KEY_AI_JOBS_REFUSE_PARTIAL, "{saved} was saved; the rest was not: {reason}"),
    (KEY_AI_JOBS_REFUSE_MODEL_IN_USE, "{model} is used by {jobs}. Pick another model for that job on Admin → Overview first."),
    (KEY_AI_JOBS_REFUSE_FILE_IN_USE, "{file} is used by {jobs}, so it can't be changed or deleted."),
    (KEY_AI_JOBS_REFUSE_FILES_READ_ONLY, "Instruction files can't be changed here. New versions of these files arrive with a release."),
    (KEY_AI_JOBS_FILES_NOTE, "New versions of these files arrive with a release."),
    (KEY_AI_JOBS_USED_FOR_LABEL, "Used for"),
    (KEY_AI_JOBS_SETTINGS_POINTER, "Change this on Admin → Overview → Which model and which instructions each AI job uses."),
    (KEY_AI_JOBS_OTHERS_TITLE, "Other settings that name a model"),
    (KEY_AI_JOBS_OTHERS_LINE, "{setting} names {model}. It is not on this panel yet."),
    (KEY_AI_JOBS_LINK_MODELS_LEAD, "All models:"),
    (KEY_AI_JOBS_LINK_MODELS, "Prompt Management → Models"),
    (KEY_AI_JOBS_LINK_FILES_LEAD, "All instruction files:"),
    (KEY_AI_JOBS_LINK_FILES, "Prompt Management → Prompts"),
    (KEY_AI_JOBS_LINK_DATA_LEAD, "Document loading numbers:"),
    (KEY_AI_JOBS_LINK_DATA, "Admin → Data"),
];

impl AiJobsWording {
    /// The fixture, built through the PRODUCTION builder — so a fixture the real
    /// builder would reject cannot exist.
    pub fn for_test() -> Self {
        build_ai_jobs_wording::<String>(|key| {
            TEST_SEED
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
                .ok_or_else(|| format!("{key} is missing from TEST_SEED"))
        })
        .expect("every key in AI_JOBS_WORDING_KEYS is in TEST_SEED")
    }

    /// The fixture as a key→value map, in the shape the store reads.
    pub fn for_test_values() -> HashMap<&'static str, String> {
        TEST_SEED
            .iter()
            .map(|(k, v)| (*k, (*v).to_string()))
            .collect()
    }
}

/// Every declared key is seeded by the migration, with the value this build
/// expects.
#[test]
fn every_declared_key_is_seeded_with_the_value_this_build_expects() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let sql = std::fs::read_to_string(root.join(SEED_MIGRATION))
        .expect("the ai jobs wording migration is on disk");

    assert_eq!(TEST_SEED.len(), AI_JOBS_WORDING_KEYS.len());
    for key in AI_JOBS_WORDING_KEYS {
        let seeded = seeded_value_in(&sql, key)
            .unwrap_or_else(|| panic!("{key} is declared to the boot loader but not seeded"));
        let fixture = TEST_SEED
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| (*v).to_string())
            .unwrap_or_else(|| panic!("{key} is missing from TEST_SEED"));
        assert_eq!(
            seeded, fixture,
            "the migration and the fixture disagree about {key}"
        );
    }
}

/// A missing row is named, not defaulted.
#[test]
fn a_missing_row_is_refused_by_name() {
    let refused = build_ai_jobs_wording::<String>(|key| {
        if key == KEY_AI_JOBS_LINK_DATA {
            Err("missing".to_string())
        } else {
            Ok("x".to_string())
        }
    });
    assert_eq!(refused, Err("missing".to_string()));
}

/// Every key carries this block's prefix, and none is declared twice.
#[test]
fn every_key_is_prefixed_and_unique() {
    let mut seen = std::collections::HashSet::new();
    for key in AI_JOBS_WORDING_KEYS {
        assert!(key.starts_with("ai_jobs_"), "{key} lacks the block prefix");
        assert!(seen.insert(*key), "{key} is declared twice");
    }
}
