// Disk/code consistency test (CLAUDE.md rule 21) for CC_TASK_MODEL_JOBS_PANEL_v1:
// every setting that names a model or an instructions file belongs to an AI job,
// so none can stay configurable only on the Settings page (Law 22).
//
// It reads the migrations off disk: a future migration that seeds `…_model` or
// `…_file` without giving the row an `ai_job` fails here, before a deploy does.

use std::collections::BTreeSet;

// STRUCTURAL (test): the migration that gives the nine job rows their `ai_job`,
// and the one that retired the dock's two such rows.
const JOB_COLUMNS_MIGRATION: &str = "20260924145150_ai_job_columns_on_app_settings.sql";
const DOCK_RETIREMENT_MIGRATION: &str = "20260924154736_retire_the_discuss_dock_settings.sql";

/// Every key a migration seeds that ends in `_model` or `_file`.
fn seeded_model_and_file_keys(dir: &std::path::Path) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .expect("pipeline_migrations is readable")
        .flatten()
        .map(|e| e.path())
        .collect();
    paths.sort();
    for path in paths {
        let sql = std::fs::read_to_string(&path).expect("a migration is readable");
        for piece in sql.split("('").skip(1) {
            let Some(end) = piece.find("',") else {
                continue;
            };
            let key = &piece[..end];
            let is_key = key
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
            if is_key && (key.ends_with("_model") || key.ends_with("_file")) {
                keys.insert(key.to_string());
            }
        }
    }
    keys
}

#[test]
fn every_model_or_file_setting_belongs_to_a_job_or_was_retired() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("pipeline_migrations");
    let jobs = std::fs::read_to_string(dir.join(JOB_COLUMNS_MIGRATION))
        .expect("the job-columns migration is on disk");
    let retired = std::fs::read_to_string(dir.join(DOCK_RETIREMENT_MIGRATION))
        .expect("the dock-retirement migration is on disk");
    let keys = seeded_model_and_file_keys(&dir);
    // Anti-vacuity: eight job rows name a model or a file (the ninth is the
    // thinking level) and the dock retired two more — ten today.
    assert!(keys.len() >= 10, "the scan found only {keys:?}");
    for key in &keys {
        let quoted = format!("'{key}'");
        let in_job = jobs.contains(&format!("WHERE key = {quoted}"));
        let was_retired = retired.contains(&quoted);
        assert!(
            in_job || was_retired,
            "{key} names a model or a file but belongs to no AI job: give it an ai_job \
             in a migration (so Admin → Overview shows it), or retire it"
        );
    }
}
