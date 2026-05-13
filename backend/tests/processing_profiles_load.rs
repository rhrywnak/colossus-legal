//! Disk-consistency invariants for the committed profile YAMLs.
//!
//! Every `*.yaml` file under `backend/profiles/` must round-trip
//! through `ProcessingProfile::from_yaml_str`. This is the test that
//! catches the regression class:
//!
//! - A YAML typo (`extration_model` instead of `extraction_model`) —
//!   `#[serde(deny_unknown_fields)]` on the struct surfaces this as a
//!   parse failure now (Bug #9).
//! - A profile referencing a field that's been renamed since the YAML
//!   was authored — same parse failure.
//! - Mass renaming of a profile field that updates the struct but
//!   forgets to update every committed YAML — caught here.
//!
//! Direction is YAML → code, not the reverse: profiles may omit
//! optional fields (`#[serde(default)]`) and they still load.

use std::fs;
use std::path::Path;

use colossus_legal_backend::pipeline::config::ProcessingProfile;

const PROFILES_DIR: &str = "profiles";

#[test]
fn every_committed_profile_yaml_parses() {
    let dir = Path::new(PROFILES_DIR);
    let entries = fs::read_dir(dir).expect("profiles directory must exist");

    let mut count = 0;
    let mut failures: Vec<String> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        // Skip `*.yaml.inactive` and other suffixes — only live `.yaml`
        // files are loaded by the runtime.
        if path.extension().and_then(|s| s.to_str()) != Some("yaml") {
            continue;
        }
        let body = fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!("failed to read profile {}: {e}", path.display());
        });
        match ProcessingProfile::from_yaml_str(&body) {
            Ok(_) => count += 1,
            Err(e) => failures.push(format!("{}: {e}", path.display())),
        }
    }

    assert!(
        failures.is_empty(),
        "profile YAMLs failed to parse with deny_unknown_fields:\n{}",
        failures.join("\n")
    );
    assert!(
        count >= 7,
        "expected at least 7 profile YAMLs, parsed {count} successfully"
    );
}

/// Bug #3 anchor: discovery_response.yaml must reference model IDs that
/// exist in the seed `llm_models` rows (claude-sonnet-4-6 and
/// claude-opus-4-6 per migration 20260420_config_system.sql). The full
/// `llm_models`-membership check is a DB-backed test elsewhere; here
/// we verify the YAML literal text is the canonical, non-date-suffixed
/// model ID — preventing the regression where the file was edited back
/// to `claude-sonnet-4-20250514`.
#[test]
fn discovery_response_references_canonical_model_ids() {
    let body = fs::read_to_string("profiles/discovery_response.yaml")
        .expect("discovery_response.yaml exists");
    assert!(
        body.contains("extraction_model: claude-sonnet-4-6"),
        "extraction_model must be the canonical claude-sonnet-4-6"
    );
    assert!(
        body.contains("pass2_extraction_model: claude-opus-4-6"),
        "pass2_extraction_model must be the canonical claude-opus-4-6"
    );
    assert!(
        !body.contains("claude-sonnet-4-20250514"),
        "stale date-suffixed sonnet ID must be gone"
    );
    assert!(
        !body.contains("claude-opus-4-20250115"),
        "stale date-suffixed opus ID must be gone"
    );
}

/// Bug #6 anchor: the consolidation migration that drops the dead
/// `pass1_model` / `pass2_model` / `pass1_max_tokens` / `pass2_max_tokens`
/// columns must exist in `pipeline_migrations/` with the expected SQL
/// statements. A reviewer who deletes the migration by accident, or a
/// rebase that loses it, would otherwise ship a broken release where
/// the Rust code expects the columns gone but the DB still has them.
///
/// Verifies the migration source text — not the live DB. The
/// DB-backed "column actually dropped" check belongs with the rest of
/// the live-infra tests under `cleanup_integration.rs`'s pattern and
/// is annotated `#[ignore]` so CI can skip it.
#[test]
fn test_pass1_model_column_removed_migration_exists() {
    let migration_path =
        "pipeline_migrations/20260513_consolidate_model_columns_and_add_overrides.sql";
    let body = fs::read_to_string(migration_path).unwrap_or_else(|e| {
        panic!("consolidation migration must exist at {migration_path}: {e}");
    });

    for stmt in [
        "DROP COLUMN IF EXISTS pass1_model",
        "DROP COLUMN IF EXISTS pass2_model",
        "DROP COLUMN IF EXISTS pass1_max_tokens",
        "DROP COLUMN IF EXISTS pass2_max_tokens",
        "ADD COLUMN IF NOT EXISTS auto_approve_grounded",
        "ADD COLUMN IF NOT EXISTS global_rules_file",
    ] {
        assert!(
            body.contains(stmt),
            "migration is missing `{stmt}`. Full body:\n{body}"
        );
    }
}

/// Bug #10 anchor: no hardcoded model literal in repository SQL.
/// Specifically the `'claude-sonnet-4-6'` COALESCE default that used
/// to live in `insert_pipeline_config`. Catches a regression where
/// someone reintroduces a hardcoded fallback in repository SQL —
/// model selection must flow exclusively through the profile →
/// override path.
#[test]
fn test_no_hardcoded_model_default_in_repository_sql() {
    let body = fs::read_to_string("src/repositories/pipeline_repository/mod.rs")
        .expect("pipeline_repository::mod must exist");
    assert!(
        !body.contains("COALESCE($2, 'claude-sonnet-4-6')"),
        "hardcoded model COALESCE default must not return; \
         model selection flows through the profile/override path"
    );
}
