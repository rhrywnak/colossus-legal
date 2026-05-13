//! Disk-consistency invariants for the committed pipeline registry.
//!
//! Catches the regression class: "added a `document_type` entry to
//! `backend/config/pipeline_registry.yaml`, forgot to create or rename
//! the profile YAML it points at." Production startup catches this via
//! `PipelineRegistry::validate()`, but a CI test gates the merge —
//! you find out at code-review time instead of at deploy time.
//!
//! The direction is registry → disk, not the reverse: extra profile
//! YAMLs that aren't registered are legitimate (legacy versions, work
//! in progress, the explicit `profile_version` override path). Only
//! missing-on-disk-but-referenced-by-registry is a hard error.
//!
//! ## Rust Learning: `serde_yaml::from_str` instead of `from_file`
//!
//! We parse the registry YAML directly rather than going through
//! [`PipelineRegistry::from_file`] because `from_file` runs
//! `validate()`, which requires `directories.profiles` to exist as a
//! real directory. The committed registry points at the production
//! path `/data/documents/profiles`, which doesn't exist on a CI
//! runner or a developer laptop. Parsing bypasses validation and
//! lets us re-map paths to the workspace-local `backend/profiles`
//! before doing the existence checks.

use std::collections::HashSet;
use std::path::PathBuf;

use colossus_legal_backend::pipeline::registry::PipelineRegistry;

/// Workspace-relative path to the registry YAML.
const REGISTRY_PATH: &str = "config/pipeline_registry.yaml";

/// Workspace-relative path to the committed profiles directory.
const PROFILES_DIR: &str = "profiles";

/// Every `document_types[*].profile_file` in the committed registry
/// must exist as a real file under `backend/profiles/`. A missing
/// file would let the backend boot the wrong profile or panic the
/// upload route at first POST — both worse than failing a CI test.
#[test]
fn every_registry_entry_has_a_matching_profile_yaml_on_disk() {
    let registry_body = std::fs::read_to_string(REGISTRY_PATH).unwrap_or_else(|e| {
        panic!(
            "Expected {REGISTRY_PATH} to be readable from the test cwd ({e}). \
                Cargo runs integration tests with cwd = crate root, so a missing \
                file here indicates a renamed or deleted registry."
        )
    });
    let registry: PipelineRegistry = serde_yaml::from_str(&registry_body)
        .expect("Committed registry YAML must parse — see PipelineRegistry definition");

    let mut missing: Vec<String> = Vec::new();
    for entry in &registry.document_types {
        let on_disk: PathBuf = PathBuf::from(PROFILES_DIR).join(&entry.profile_file);
        if !on_disk.exists() {
            missing.push(format!(
                "registry entry '{}' references '{}' which does not exist at '{}'",
                entry.name,
                entry.profile_file,
                on_disk.display()
            ));
        }
    }

    assert!(
        missing.is_empty(),
        "Registry references profile files that don't exist on disk:\n  {}",
        missing.join("\n  ")
    );
}

/// Catches a different drift: two registry entries pointing at the
/// same `profile_file` is almost always a copy-paste mistake. The
/// `name` field is checked by `PipelineRegistry::validate()` already,
/// but `profile_file` duplicates can slip through.
#[test]
fn no_two_registry_entries_share_a_profile_file() {
    let registry_body =
        std::fs::read_to_string(REGISTRY_PATH).expect("registry YAML must be readable");
    let registry: PipelineRegistry =
        serde_yaml::from_str(&registry_body).expect("registry YAML must parse");

    let mut seen: HashSet<&str> = HashSet::new();
    let mut duplicates: Vec<&str> = Vec::new();
    for entry in &registry.document_types {
        if !seen.insert(entry.profile_file.as_str()) {
            duplicates.push(entry.profile_file.as_str());
        }
    }
    assert!(
        duplicates.is_empty(),
        "Registry has multiple entries pointing at the same profile_file: {duplicates:?}"
    );
}

/// The committed registry must have exactly one default entry. This
/// duplicates `PipelineRegistry::validate()`'s "exactly one default"
/// check, but at test time so a broken commit is rejected before
/// review, not at backend startup.
#[test]
fn exactly_one_default_in_committed_registry() {
    let registry_body =
        std::fs::read_to_string(REGISTRY_PATH).expect("registry YAML must be readable");
    let registry: PipelineRegistry =
        serde_yaml::from_str(&registry_body).expect("registry YAML must parse");

    let defaults: Vec<&str> = registry
        .document_types
        .iter()
        .filter(|e| e.is_default)
        .map(|e| e.name.as_str())
        .collect();
    assert_eq!(
        defaults.len(),
        1,
        "Committed registry must have exactly one is_default entry, found {} ({:?})",
        defaults.len(),
        defaults
    );
}
