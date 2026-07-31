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

use colossus_legal_backend::pipeline::config::ProcessingProfile;
use colossus_legal_backend::pipeline::registry::PipelineRegistry;

/// Workspace-relative path to the registry YAML.
const REGISTRY_PATH: &str = "config/pipeline_registry.yaml";

/// Workspace-relative path to the committed profiles directory.
const PROFILES_DIR: &str = "profiles";

/// Workspace-relative path to the committed extraction schemas.
const SCHEMAS_DIR: &str = "extraction_schemas";

/// Workspace-relative path to the committed prompt templates.
///
/// Note this one directory backs THREE registry-declared directories.
/// `pipeline_registry.yaml` names `templates` and `system_prompts` as
/// separate production paths under `/data/documents`, but in the repo
/// the pass-1/pass-2 templates, the global-rules fragment and the
/// system prompt are all committed side by side here. Deployment
/// flattens them. The test resolves against the committed layout,
/// because that is the layout a reviewer can actually check.
const TEMPLATES_DIR: &str = "extraction_templates";

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

/// The prompt/schema artifacts a profile names, as
/// `(field_name, resolve_dir, value)` triples.
///
/// Two of the five profile fields (`schema_file`, `template_file`) are
/// required `String`s and are always `Some`; the other three are
/// `Option<String>` and may legitimately be absent.
///
/// ## Rust Learning: `Option<&String>` and `as_deref`
///
/// `as_deref()` turns an `&Option<String>` into an `Option<&str>`
/// without cloning the heap-allocated `String` — it dereferences
/// through the `Option`. The caller then skips a `None` rather than
/// treating it as an empty filename: an absent optional artifact and a
/// MISSING named artifact are different states and must not collapse
/// into one.
///
/// ## Rust Learning: elided lifetime in the return type
///
/// The `Option<&str>` in each triple borrows from `profile`, so the
/// returned array cannot outlive it. The elided lifetime ties the
/// array to the `&ProcessingProfile` argument, which is exactly what we
/// want — the array is consumed inside the same loop iteration.
fn profile_artifact_refs(
    profile: &ProcessingProfile,
) -> [(&'static str, &'static str, Option<&str>); 5] {
    [
        (
            "schema_file",
            SCHEMAS_DIR,
            Some(profile.schema_file.as_str()),
        ),
        (
            "template_file",
            TEMPLATES_DIR,
            Some(profile.template_file.as_str()),
        ),
        (
            "pass2_template_file",
            TEMPLATES_DIR,
            profile.pass2_template_file.as_deref(),
        ),
        (
            "global_rules_file",
            TEMPLATES_DIR,
            profile.global_rules_file.as_deref(),
        ),
        (
            "system_prompt_file",
            TEMPLATES_DIR,
            profile.system_prompt_file.as_deref(),
        ),
    ]
}

/// Every artifact a registered profile names must exist on disk.
///
/// This closes the gap the profile-existence test above leaves open.
/// `PipelineRegistry::validate()` checks that a registry entry's
/// `profile_file` resolves — and stops there. It never opens the
/// profile, so a profile naming a schema, template, pass-2 template,
/// global-rules fragment or system prompt that isn't committed passes
/// startup, passes every other test, and surfaces only when an
/// operator triggers the first extraction run.
///
/// That failure lands directly on the spend gate: the document has
/// already been uploaded and the run authorised before anything
/// notices the file is missing. Standing Rule 1 — a missing required
/// artifact is a startup-class error, not a runtime surprise — and the
/// cheapest place to observe it is here, at merge time.
///
/// Direction is registry → profile → artifacts. Uncommitted extra
/// schemas and templates are legitimate (superseded/ holds several),
/// so this test never asserts the reverse.
#[test]
fn every_registered_profile_artifact_exists_on_disk() {
    let registry_body =
        std::fs::read_to_string(REGISTRY_PATH).expect("registry YAML must be readable");
    let registry: PipelineRegistry =
        serde_yaml::from_str(&registry_body).expect("registry YAML must parse");

    let mut missing: Vec<String> = Vec::new();

    for entry in &registry.document_types {
        let profile_path = PathBuf::from(PROFILES_DIR).join(&entry.profile_file);
        let Ok(body) = std::fs::read_to_string(&profile_path) else {
            // The sibling test above owns the "profile file is missing"
            // failure and reports it with its own message. Skipping here
            // keeps one failure reported once, rather than as two
            // findings that look like two separate problems.
            continue;
        };
        let profile = ProcessingProfile::from_yaml_str(&body).unwrap_or_else(|e| {
            panic!(
                "profile '{}' (referenced by registry entry '{}') must parse: {e}",
                entry.profile_file, entry.name
            )
        });

        for (field, dir, value) in profile_artifact_refs(&profile) {
            let Some(filename) = value else {
                continue; // optional and not set — a legitimate state
            };
            let on_disk = PathBuf::from(dir).join(filename);
            if !on_disk.exists() {
                missing.push(format!(
                    "registry entry '{}' → profile '{}' → {} = '{}' does not exist at '{}'",
                    entry.name,
                    entry.profile_file,
                    field,
                    filename,
                    on_disk.display()
                ));
            }
        }
    }

    assert!(
        missing.is_empty(),
        "Registered profiles reference artifacts that don't exist on disk:\n  {}",
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
