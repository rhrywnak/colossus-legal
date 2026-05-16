//! Tests for the pipeline configuration registry.
//!
//! Kept in a sibling file (`#[path = "registry_tests.rs"] mod tests`)
//! because inline tests would push `registry.rs` past the 300-line
//! module-size budget (CLAUDE.md Rule 17). Imports the parent module
//! via `super::*` and shares its `serial_test`-free isolation by
//! relying on tempfile-scoped directories rather than process-global
//! state — every test that touches `std::env` saves/restores the var.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::*;

// ── Fixture helpers ────────────────────────────────────────────

/// Build a four-directory layout in a TempDir and write the named
/// profile files.
///
/// Returns the tempdir handle (drop it to clean up the layout) and the
/// resolved absolute path of each directory. Callers compose YAML
/// strings that reference these paths and feed them through
/// `serde_yaml::from_str` + `validate()`.
struct Layout {
    _tmp: TempDir,
    profiles: String,
    schemas: String,
    templates: String,
    system_prompts: String,
}

impl Layout {
    fn new(profile_files: &[(&str, &str)]) -> Self {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        let profiles = root.join("profiles");
        let schemas = root.join("schemas");
        let templates = root.join("templates");
        let system_prompts = root.join("system_prompts");
        for d in [&profiles, &schemas, &templates, &system_prompts] {
            fs::create_dir_all(d).unwrap();
        }
        for (name, body) in profile_files {
            fs::write(profiles.join(name), body).unwrap();
        }
        Self {
            profiles: profiles.to_string_lossy().into_owned(),
            schemas: schemas.to_string_lossy().into_owned(),
            templates: templates.to_string_lossy().into_owned(),
            system_prompts: system_prompts.to_string_lossy().into_owned(),
            _tmp: tmp,
        }
    }
}

fn minimal_profile_yaml(name: &str) -> String {
    format!(
        "name: {name}\n\
         display_name: \"{name} display\"\n\
         schema_file: schema.yaml\n\
         template_file: template.md\n\
         extraction_model: claude-sonnet-4-6\n"
    )
}

fn registry_yaml(layout: &Layout, document_types_section: &str) -> String {
    format!(
        "directories:\n  \
           profiles: {profiles}\n  \
           schemas: {schemas}\n  \
           templates: {templates}\n  \
           system_prompts: {system_prompts}\n\
         document_types:\n{document_types_section}\
         {step_labels_section}",
        profiles = layout.profiles,
        schemas = layout.schemas,
        templates = layout.templates,
        system_prompts = layout.system_prompts,
        step_labels_section = default_step_labels_yaml(),
    )
}

/// YAML block for the registry's `step_labels:` section, matching the
/// production `backend/config/pipeline_registry.yaml`. Tests that load
/// registry YAML must include this section (or YAML parsing fails with
/// `missing field 'step_labels'`), so the helper appends it
/// automatically.
fn default_step_labels_yaml() -> &'static str {
    "step_labels:\n  \
       extract_text:\n    label: \"Extracting text\"\n    label_full: \"Extracting text\"\n    label_chunk: \"Extracting text\"\n    percent_start: 5\n    percent_end: 10\n  \
       llm_extract_pass1:\n    label: \"Pass 1\"\n    label_full: \"Pass 1 (full document)\"\n    label_chunk: \"Pass 1 chunk {current}/{total}\"\n    percent_start: 10\n    percent_end: 60\n  \
       llm_extract_pass2:\n    label: \"Pass 2\"\n    label_full: \"Pass 2 (full document)\"\n    label_chunk: \"Pass 2 chunk {current}/{total}\"\n    percent_start: 60\n    percent_end: 70\n  \
       verify:\n    label: \"Verifying\"\n    label_full: \"Verifying\"\n    label_chunk: \"Verifying\"\n    percent_start: 70\n    percent_end: 80\n  \
       auto_approve:\n    label: \"Auto-approving\"\n    label_full: \"Auto-approving\"\n    label_chunk: \"Auto-approving\"\n    percent_start: 80\n    percent_end: 82\n  \
       ingest:\n    label: \"Ingesting\"\n    label_full: \"Ingesting\"\n    label_chunk: \"Ingesting\"\n    percent_start: 82\n    percent_end: 90\n  \
       index:\n    label: \"Indexing\"\n    label_full: \"Indexing\"\n    label_chunk: \"Indexing\"\n    percent_start: 90\n    percent_end: 95\n  \
       completeness:\n    label: \"Finalizing\"\n    label_full: \"Finalizing\"\n    label_chunk: \"Finalizing\"\n    percent_start: 95\n    percent_end: 100\n"
}

fn write_registry_yaml(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
}

// ── Loading and validation ─────────────────────────────────────

#[test]
fn test_registry_loads_from_valid_yaml() {
    let layout = Layout::new(&[
        ("complaint.yaml", &minimal_profile_yaml("complaint")),
        ("default.yaml", &minimal_profile_yaml("default")),
    ]);
    let yaml = registry_yaml(
        &layout,
        "  - name: complaint\n    \
                display_name: \"Complaint\"\n    \
                profile_file: complaint.yaml\n    \
                description: \"Initiating document\"\n    \
                sort_order: 1\n  \
              - name: default\n    \
                display_name: \"Other\"\n    \
                profile_file: default.yaml\n    \
                is_default: true\n    \
                sort_order: 99\n",
    );
    let tmp_yaml = tempfile::NamedTempFile::new().unwrap();
    write_registry_yaml(tmp_yaml.path(), &yaml);

    let registry = PipelineRegistry::from_file(tmp_yaml.path().to_str().unwrap())
        .expect("registry should load");
    assert_eq!(registry.directories.profiles, layout.profiles);
    assert_eq!(registry.document_types.len(), 2);
    let complaint = registry.document_type("complaint").unwrap();
    assert_eq!(complaint.display_name, "Complaint");
    assert_eq!(complaint.profile_file, "complaint.yaml");
    assert_eq!(complaint.sort_order, 1);
}

#[test]
fn test_registry_validate_missing_directory() {
    let registry = PipelineRegistry {
        directories: PipelineDirectories {
            profiles: "/nonexistent/path/profiles".to_string(),
            schemas: "/tmp".to_string(),
            templates: "/tmp".to_string(),
            system_prompts: "/tmp".to_string(),
        },
        document_types: vec![],
        step_labels: super::legacy_default_step_labels(),
        recovery_hints: std::collections::HashMap::new(),
    };
    let err = registry.validate().unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("/nonexistent/path/profiles") && msg.contains("profiles"),
        "error must name the missing directory; got: {msg}"
    );
}

#[test]
fn test_registry_validate_missing_profile_file() {
    let layout = Layout::new(&[]); // no profile files
    let registry = PipelineRegistry {
        directories: PipelineDirectories {
            profiles: layout.profiles.clone(),
            schemas: layout.schemas.clone(),
            templates: layout.templates.clone(),
            system_prompts: layout.system_prompts.clone(),
        },
        document_types: vec![DocumentTypeEntry {
            name: "discovery_response".to_string(),
            display_name: "Discovery Response".to_string(),
            profile_file: "discovery_response.yaml".to_string(),
            description: String::new(),
            is_default: true,
            sort_order: 0,
        }],
        step_labels: super::legacy_default_step_labels(),
        recovery_hints: std::collections::HashMap::new(),
    };
    let err = registry.validate().unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("discovery_response") && msg.contains("discovery_response.yaml"),
        "error must name the document type AND missing path; got: {msg}"
    );
}

#[test]
fn test_registry_validate_no_default() {
    let layout = Layout::new(&[("complaint.yaml", &minimal_profile_yaml("complaint"))]);
    let registry = PipelineRegistry {
        directories: PipelineDirectories {
            profiles: layout.profiles.clone(),
            schemas: layout.schemas.clone(),
            templates: layout.templates.clone(),
            system_prompts: layout.system_prompts.clone(),
        },
        document_types: vec![DocumentTypeEntry {
            name: "complaint".to_string(),
            display_name: "Complaint".to_string(),
            profile_file: "complaint.yaml".to_string(),
            description: String::new(),
            is_default: false,
            sort_order: 1,
        }],
        step_labels: super::legacy_default_step_labels(),
        recovery_hints: std::collections::HashMap::new(),
    };
    let err = registry.validate().unwrap_err();
    assert!(
        err.to_string().contains("found 0"),
        "error must report the default count; got: {err}"
    );
}

#[test]
fn test_registry_validate_multiple_defaults() {
    let layout = Layout::new(&[
        ("a.yaml", &minimal_profile_yaml("a")),
        ("b.yaml", &minimal_profile_yaml("b")),
    ]);
    let registry = PipelineRegistry {
        directories: PipelineDirectories {
            profiles: layout.profiles.clone(),
            schemas: layout.schemas.clone(),
            templates: layout.templates.clone(),
            system_prompts: layout.system_prompts.clone(),
        },
        document_types: vec![
            DocumentTypeEntry {
                name: "a".to_string(),
                display_name: "A".to_string(),
                profile_file: "a.yaml".to_string(),
                description: String::new(),
                is_default: true,
                sort_order: 1,
            },
            DocumentTypeEntry {
                name: "b".to_string(),
                display_name: "B".to_string(),
                profile_file: "b.yaml".to_string(),
                description: String::new(),
                is_default: true,
                sort_order: 2,
            },
        ],
        step_labels: super::legacy_default_step_labels(),
        recovery_hints: std::collections::HashMap::new(),
    };
    let err = registry.validate().unwrap_err();
    assert!(
        err.to_string().contains("found 2"),
        "error must report the default count; got: {err}"
    );
}

#[test]
fn test_registry_validate_duplicate_names() {
    let layout = Layout::new(&[("complaint.yaml", &minimal_profile_yaml("complaint"))]);
    let registry = PipelineRegistry {
        directories: PipelineDirectories {
            profiles: layout.profiles.clone(),
            schemas: layout.schemas.clone(),
            templates: layout.templates.clone(),
            system_prompts: layout.system_prompts.clone(),
        },
        document_types: vec![
            DocumentTypeEntry {
                name: "complaint".to_string(),
                display_name: "Complaint v4".to_string(),
                profile_file: "complaint.yaml".to_string(),
                description: String::new(),
                is_default: false,
                sort_order: 1,
            },
            DocumentTypeEntry {
                name: "complaint".to_string(),
                display_name: "Complaint v5".to_string(),
                profile_file: "complaint.yaml".to_string(),
                description: String::new(),
                is_default: true,
                sort_order: 2,
            },
        ],
        step_labels: super::legacy_default_step_labels(),
        recovery_hints: std::collections::HashMap::new(),
    };
    let err = registry.validate().unwrap_err();
    assert!(
        err.to_string().contains("'complaint'"),
        "error must name the duplicate; got: {err}"
    );
}

#[test]
fn test_registry_validate_empty_name() {
    let layout = Layout::new(&[("anon.yaml", &minimal_profile_yaml("anon"))]);
    let registry = PipelineRegistry {
        directories: PipelineDirectories {
            profiles: layout.profiles.clone(),
            schemas: layout.schemas.clone(),
            templates: layout.templates.clone(),
            system_prompts: layout.system_prompts.clone(),
        },
        document_types: vec![DocumentTypeEntry {
            name: String::new(),
            display_name: "anon".to_string(),
            profile_file: "anon.yaml".to_string(),
            description: String::new(),
            is_default: true,
            sort_order: 0,
        }],
        step_labels: super::legacy_default_step_labels(),
        recovery_hints: std::collections::HashMap::new(),
    };
    let err = registry.validate().unwrap_err();
    assert!(
        err.to_string().contains("empty name"),
        "error must report empty name; got: {err}"
    );
}

#[test]
fn test_registry_validate_empty_profile_file() {
    let layout = Layout::new(&[]);
    let registry = PipelineRegistry {
        directories: PipelineDirectories {
            profiles: layout.profiles.clone(),
            schemas: layout.schemas.clone(),
            templates: layout.templates.clone(),
            system_prompts: layout.system_prompts.clone(),
        },
        document_types: vec![DocumentTypeEntry {
            name: "complaint".to_string(),
            display_name: "Complaint".to_string(),
            profile_file: String::new(),
            description: String::new(),
            is_default: true,
            sort_order: 0,
        }],
        step_labels: super::legacy_default_step_labels(),
        recovery_hints: std::collections::HashMap::new(),
    };
    let err = registry.validate().unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("complaint") && msg.contains("empty profile_file"),
        "error must name the document type AND describe the empty field; got: {msg}"
    );
}

// ── Lookup and sort behavior ───────────────────────────────────

fn fully_valid_registry() -> (Layout, PipelineRegistry) {
    let layout = Layout::new(&[
        ("complaint.yaml", &minimal_profile_yaml("complaint")),
        ("discovery.yaml", &minimal_profile_yaml("discovery")),
        ("default.yaml", &minimal_profile_yaml("default")),
    ]);
    let registry = PipelineRegistry {
        directories: PipelineDirectories {
            profiles: layout.profiles.clone(),
            schemas: layout.schemas.clone(),
            templates: layout.templates.clone(),
            system_prompts: layout.system_prompts.clone(),
        },
        document_types: vec![
            DocumentTypeEntry {
                name: "discovery_response".to_string(),
                display_name: "Discovery Response".to_string(),
                profile_file: "discovery.yaml".to_string(),
                description: String::new(),
                is_default: false,
                sort_order: 2,
            },
            DocumentTypeEntry {
                name: "complaint".to_string(),
                display_name: "Complaint".to_string(),
                profile_file: "complaint.yaml".to_string(),
                description: String::new(),
                is_default: false,
                sort_order: 1,
            },
            DocumentTypeEntry {
                name: "default".to_string(),
                display_name: "Other".to_string(),
                profile_file: "default.yaml".to_string(),
                description: String::new(),
                is_default: true,
                sort_order: 99,
            },
        ],
        step_labels: super::legacy_default_step_labels(),
        recovery_hints: std::collections::HashMap::new(),
    };
    registry.validate().expect("fixture must validate");
    (layout, registry)
}

#[test]
fn test_registry_document_type_lookup() {
    let (_layout, registry) = fully_valid_registry();
    let entry = registry.document_type("complaint").unwrap();
    assert_eq!(entry.profile_file, "complaint.yaml");
    assert_eq!(entry.display_name, "Complaint");
}

#[test]
fn test_registry_document_type_lookup_missing() {
    let (_layout, registry) = fully_valid_registry();
    assert!(registry.document_type("nonexistent_type").is_none());
}

#[test]
fn test_registry_default_document_type() {
    let (_layout, registry) = fully_valid_registry();
    let entry = registry.default_document_type().unwrap();
    assert_eq!(entry.name, "default");
    assert!(entry.is_default);
}

#[test]
fn test_registry_document_types_sorted() {
    let (_layout, registry) = fully_valid_registry();
    let sorted = registry.document_types_sorted();
    let names: Vec<&str> = sorted.iter().map(|e| e.name.as_str()).collect();
    // Default ("Other") is excluded; remaining entries are ordered by sort_order.
    assert_eq!(names, vec!["complaint", "discovery_response"]);
}

// ── Path construction ──────────────────────────────────────────

#[test]
fn test_registry_path_construction() {
    let (layout, registry) = fully_valid_registry();
    assert_eq!(
        registry.profile_path("x.yaml"),
        Path::new(&layout.profiles)
            .join("x.yaml")
            .to_string_lossy()
            .into_owned()
    );
    assert_eq!(
        registry.schema_path("x.yaml"),
        Path::new(&layout.schemas)
            .join("x.yaml")
            .to_string_lossy()
            .into_owned()
    );
    assert_eq!(
        registry.template_path("x.md"),
        Path::new(&layout.templates)
            .join("x.md")
            .to_string_lossy()
            .into_owned()
    );
    assert_eq!(
        registry.system_prompt_path("x.md"),
        Path::new(&layout.system_prompts)
            .join("x.md")
            .to_string_lossy()
            .into_owned()
    );
}

// ── from_env behavior ──────────────────────────────────────────

/// Save the four legacy env vars + the registry file var, run a
/// closure with them in a chosen state, then restore. Avoids leaking
/// test state between tests that touch `std::env`.
fn with_env_vars<F: FnOnce()>(vars: &[(&str, Option<&str>)], f: F) {
    let saved: Vec<(String, Option<String>)> = vars
        .iter()
        .map(|(k, _)| (k.to_string(), std::env::var(k).ok()))
        .collect();
    for (k, v) in vars {
        match v {
            Some(val) => std::env::set_var(k, val),
            None => std::env::remove_var(k),
        }
    }
    f();
    for (k, prev) in saved {
        match prev {
            Some(val) => std::env::set_var(&k, val),
            None => std::env::remove_var(&k),
        }
    }
}

#[test]
fn test_registry_from_env_with_registry_file() {
    let layout = Layout::new(&[("default.yaml", &minimal_profile_yaml("default"))]);
    let yaml = registry_yaml(
        &layout,
        "  - name: default\n    \
                display_name: \"Other\"\n    \
                profile_file: default.yaml\n    \
                is_default: true\n    \
                sort_order: 99\n",
    );
    let tmp_yaml = tempfile::NamedTempFile::new().unwrap();
    write_registry_yaml(tmp_yaml.path(), &yaml);

    with_env_vars(
        &[
            (
                "PIPELINE_REGISTRY_FILE",
                Some(tmp_yaml.path().to_str().unwrap()),
            ),
            ("PROCESSING_PROFILE_DIR", None),
            ("EXTRACTION_SCHEMA_DIR", None),
            ("EXTRACTION_TEMPLATE_DIR", None),
            ("SYSTEM_PROMPT_DIR", None),
        ],
        || {
            let registry = PipelineRegistry::from_env().expect("from_env should load");
            assert_eq!(registry.document_types.len(), 1);
            assert!(registry.default_document_type().is_some());
        },
    );
}

#[test]
fn test_registry_from_env_fallback_to_legacy() {
    let layout = Layout::new(&[
        ("complaint.yaml", &minimal_profile_yaml("complaint")),
        ("default.yaml", &minimal_profile_yaml("default")),
    ]);
    with_env_vars(
        &[
            ("PIPELINE_REGISTRY_FILE", None),
            ("PROCESSING_PROFILE_DIR", Some(&layout.profiles)),
            ("EXTRACTION_SCHEMA_DIR", Some(&layout.schemas)),
            ("EXTRACTION_TEMPLATE_DIR", Some(&layout.templates)),
            ("SYSTEM_PROMPT_DIR", Some(&layout.system_prompts)),
        ],
        || {
            let registry = PipelineRegistry::from_env().expect("legacy fallback should succeed");
            assert_eq!(registry.document_types.len(), 2);
            assert!(registry.default_document_type().is_some());
            assert_eq!(
                registry.default_document_type().unwrap().profile_file,
                "default.yaml"
            );
        },
    );
}

#[test]
fn test_registry_from_env_no_vars_at_all() {
    with_env_vars(
        &[
            ("PIPELINE_REGISTRY_FILE", None),
            ("PROCESSING_PROFILE_DIR", None),
            ("EXTRACTION_SCHEMA_DIR", None),
            ("EXTRACTION_TEMPLATE_DIR", None),
            ("SYSTEM_PROMPT_DIR", None),
        ],
        || {
            let err = PipelineRegistry::from_env()
                .expect_err("missing env vars must error, not silently default");
            let msg = err.to_string();
            assert!(
                msg.contains("PIPELINE_REGISTRY_FILE") || msg.contains("PROCESSING_PROFILE_DIR"),
                "error must name the missing var(s); got: {msg}"
            );
        },
    );
}

// ── step_label() lookup ────────────────────────────────────────

#[test]
fn test_step_label_returns_entry_for_each_known_step_name() {
    let (_layout, registry) = fully_valid_registry();
    for step in [
        "extract_text",
        "llm_extract_pass1",
        "llm_extract_pass2",
        "verify",
        "auto_approve",
        "ingest",
        "index",
        "completeness",
    ] {
        let entry = registry
            .step_label(step)
            .unwrap_or_else(|| panic!("step_label({step}) must return Some"));
        assert!(
            entry.percent_start < entry.percent_end,
            "{step}: start ({}) must be < end ({})",
            entry.percent_start,
            entry.percent_end
        );
        assert!(
            entry.percent_end <= 100,
            "{step}: end ({}) must be <= 100",
            entry.percent_end
        );
        assert!(!entry.label.is_empty(), "{step}: label must not be empty");
    }
}

#[test]
fn test_step_label_returns_none_for_unknown_name() {
    let (_layout, registry) = fully_valid_registry();
    assert!(registry.step_label("nonexistent_step").is_none());
    assert!(registry.step_label("").is_none());
}

// ── suggest_recovery() lookup ──────────────────────────────────

fn registry_with_recovery_hints(
    hints: Vec<(&str, Vec<(&str, &str)>)>,
) -> (Layout, PipelineRegistry) {
    let (layout, mut registry) = fully_valid_registry();
    let mut map = std::collections::HashMap::new();
    for (step, patterns) in hints {
        let entries: Vec<super::RecoveryHint> = patterns
            .into_iter()
            .map(|(pat, sug)| super::RecoveryHint {
                error_pattern: pat.to_string(),
                suggestion: sug.to_string(),
            })
            .collect();
        map.insert(step.to_string(), entries);
    }
    registry.recovery_hints = map;
    (layout, registry)
}

#[test]
fn test_suggest_recovery_returns_first_matching_hint() {
    let (_layout, registry) = registry_with_recovery_hints(vec![(
        "extract_text",
        vec![
            ("OCR confidence", "Re-scan the source at higher DPI"),
            ("permission denied", "Check filesystem permissions"),
        ],
    )]);
    let got = registry
        .suggest_recovery("extract_text", "OCR confidence below threshold for page 3")
        .expect("matching error message must return Some");
    assert_eq!(got, "Re-scan the source at higher DPI");
}

#[test]
fn test_suggest_recovery_returns_first_when_multiple_match() {
    let (_layout, registry) = registry_with_recovery_hints(vec![(
        "ingest",
        vec![
            ("Neo4j", "Check Neo4j connectivity"),
            ("connection", "Check network"),
        ],
    )]);
    let got = registry
        .suggest_recovery("ingest", "Neo4j connection refused")
        .expect("must return first matching hint");
    assert_eq!(got, "Check Neo4j connectivity");
}

#[test]
fn test_suggest_recovery_returns_none_when_no_pattern_matches() {
    let (_layout, registry) = registry_with_recovery_hints(vec![(
        "verify",
        vec![("schema mismatch", "Re-run with --strict")],
    )]);
    assert!(registry
        .suggest_recovery("verify", "Some unrelated runtime panic")
        .is_none());
}

#[test]
fn test_suggest_recovery_returns_none_for_unknown_step() {
    let (_layout, registry) =
        registry_with_recovery_hints(vec![("verify", vec![("anything", "Some suggestion")])]);
    assert!(registry
        .suggest_recovery("step_that_has_no_hints", "anything matches")
        .is_none());
}

#[test]
fn test_suggest_recovery_returns_none_when_recovery_hints_empty() {
    let (_layout, registry) = fully_valid_registry();
    assert!(registry.recovery_hints.is_empty());
    assert!(registry
        .suggest_recovery("extract_text", "any error at all")
        .is_none());
}

// ── validate_step_labels() invariants ──────────────────────────

/// Build a registry whose only flaw is the named step's percent
/// range — used to pin the three invariants enforced by
/// `validate_step_labels`.
fn registry_with_broken_step(
    mutate: impl FnOnce(&mut super::PipelineStepLabels),
) -> (Layout, PipelineRegistry) {
    let (layout, mut registry) = fully_valid_registry();
    mutate(&mut registry.step_labels);
    (layout, registry)
}

#[test]
fn test_validate_step_labels_rejects_start_gte_end() {
    let (_layout, registry) = registry_with_broken_step(|labels| {
        // Make extract_text's start == end.
        labels.extract_text.percent_start = 10;
        labels.extract_text.percent_end = 10;
    });
    let err = registry
        .validate()
        .expect_err("start >= end must fail validation");
    let msg = err.to_string();
    assert!(
        msg.contains("extract_text") && msg.contains("strictly less"),
        "error must name the step and say 'strictly less'; got: {msg}"
    );
}

#[test]
fn test_validate_step_labels_rejects_percent_end_over_100() {
    let (_layout, registry) = registry_with_broken_step(|labels| {
        labels.completeness.percent_end = 101;
    });
    let err = registry
        .validate()
        .expect_err("percent_end > 100 must fail validation");
    let msg = err.to_string();
    assert!(
        msg.contains("completeness") && msg.contains("exceeds 100"),
        "error must name the step and say 'exceeds 100'; got: {msg}"
    );
}

#[test]
fn test_validate_step_labels_rejects_non_monotonic_sequence() {
    let (_layout, registry) = registry_with_broken_step(|labels| {
        // llm_extract_pass2's start (60) is fine; force it BELOW
        // pass1's end (60) so it regresses.
        labels.llm_extract_pass2.percent_start = 50;
        labels.llm_extract_pass2.percent_end = 70;
    });
    let err = registry
        .validate()
        .expect_err("non-monotonic must fail validation");
    let msg = err.to_string();
    assert!(
        msg.contains("llm_extract_pass2") && msg.contains("regress"),
        "error must name the step and say 'regress'; got: {msg}"
    );
}
