//! Pipeline configuration types — profile loading and config resolution.
//!
//! Processing profiles are YAML files on mounted storage (not database rows).
//! This module loads them, validates references, and resolves the three-level
//! configuration hierarchy: system defaults → profile → per-document overrides.
//!
//! Design: DOC_PROCESSING_CONFIG_DESIGN_v2.md Sections 3.1, 3.2.2, 3.7.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

// ── Processing Profile ──────────────────────────────────────────

/// A processing profile loaded from a YAML file on mounted storage.
///
/// Each profile defines default extraction parameters for a document type:
/// which template, schema, model, chunking mode, and LLM parameters to use.
/// Profiles reference other files by filename — they never embed content.
///
/// ## Why YAML files instead of a database table?
///
/// Grounded in AWS IDP Accelerator architecture: YAML is the authoring
/// format, filesystem is the runtime store. Our backend runs on a
/// persistent VM with mounted storage. Editing a YAML file takes effect
/// immediately — no migration, no restart, no container rebuild.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingProfile {
    pub name: String,
    pub display_name: String,
    #[serde(default)]
    pub description: String,

    // Schema
    pub schema_file: String,

    // Prompts
    pub template_file: String,
    #[serde(default)]
    pub system_prompt_file: Option<String>,
    /// Global-rules fragment file appended to the prompt at the
    /// `{{global_rules}}` placeholder. When absent, the placeholder is
    /// substituted with the empty string so legacy profiles that don't
    /// opt in to global rules continue to work without leaking literal
    /// `{{global_rules}}` text into the LLM prompt.
    ///
    /// ## Rust Learning: `#[serde(default)]` on `Option<String>`
    ///
    /// Marks the field as optional in YAML deserialization. Profiles
    /// authored before this field existed deserialize cleanly — serde
    /// fills in `None` rather than failing with "missing field". The
    /// substitution code reads `Option<&str>` and treats `None` as
    /// "skip this rule fragment, substitute empty."
    #[serde(default)]
    pub global_rules_file: Option<String>,
    /// Pass-2 relationship-extraction prompt template.
    ///
    /// When present, the manually-invoked pass 2 path reads
    /// `{template_dir}/{pass2_template_file}` and substitutes the
    /// pass-1 entity list into `{{entities_json}}`. Absent for profiles
    /// that only run the single-pass extraction. Task 3 will use the
    /// `run_pass2` flag to decide when to invoke pass 2 automatically;
    /// for now this field is only consulted when the pass-2 entry
    /// point is triggered directly.
    #[serde(default)]
    pub pass2_template_file: Option<String>,

    // Model — must match an id in the llm_models table
    pub extraction_model: String,
    /// Pass-2 relationship-extraction model override.
    ///
    /// Pass 2 does a fundamentally different job from pass 1 (reasoning
    /// over an entity list to identify relationships vs. parsing raw
    /// text to extract entities). Operators often want a stronger /
    /// different model for that task. When absent, pass 2 reuses the
    /// `extraction_model` value. When present, must match an id in the
    /// `llm_models` table.
    #[serde(default)]
    pub pass2_extraction_model: Option<String>,
    #[serde(default)]
    pub synthesis_model: Option<String>,

    // Chunking
    #[serde(default = "default_chunking_mode")]
    pub chunking_mode: String,
    #[serde(default)]
    pub chunk_size: Option<i32>,
    #[serde(default)]
    pub chunk_overlap: Option<i32>,

    /// Flexible chunking parameters for the intelligent-chunking pipeline.
    ///
    /// Holds keys like `mode`, `strategy`, `units_per_chunk`, `unit_overlap`,
    /// `request_timeout_secs` — the exact set is owned by Group 2's
    /// `ConfigAccess` reader, not by the type system here. Storing
    /// `serde_json::Value` instead of typed fields keeps this struct
    /// schema-free so adding a new chunking knob never requires a struct
    /// change in colossus-legal — only a new key in YAML and a new reader
    /// in colossus-extract.
    ///
    /// ## Rust Learning: `#[serde(default)]` on a `HashMap` field
    ///
    /// When YAML profiles authored before this field existed are loaded,
    /// `serde_yaml` calls `HashMap::default()` (which produces an empty
    /// map) to fill in the missing key. Without `#[serde(default)]`
    /// deserialization would *fail* with "missing field `chunking_config`",
    /// breaking every legacy profile in storage. The empty-map default
    /// is also the natural "use all defaults" sentinel.
    #[serde(default)]
    pub chunking_config: HashMap<String, serde_json::Value>,

    /// Flexible cross-document context parameters used by pass 2.
    ///
    /// Same shape and rationale as `chunking_config` — keys like
    /// `traversal_depth` and `always_include_foundation` live here so
    /// Group 2 can introduce new pass-2 knobs without struct churn.
    #[serde(default)]
    pub context_config: HashMap<String, serde_json::Value>,

    // LLM parameters
    #[serde(default = "default_max_tokens")]
    pub max_tokens: i32,
    #[serde(default)]
    pub temperature: f64,

    // Behavior
    #[serde(default = "default_true")]
    pub auto_approve_grounded: bool,
    #[serde(default)]
    pub run_pass2: bool,

    #[serde(default)]
    pub is_default: bool,
}

fn default_chunking_mode() -> String {
    "chunked".to_string()
}

fn default_max_tokens() -> i32 {
    8000
}

fn default_true() -> bool {
    true
}

impl ProcessingProfile {
    /// Load a processing profile from a YAML file.
    ///
    /// Returns a descriptive error if the file doesn't exist, can't be
    /// read, or contains invalid YAML. The error message includes the
    /// file path so the caller knows which profile failed.
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read profile '{}': {e}", path.display()))?;
        let profile: Self = serde_yaml::from_str(&content)
            .map_err(|e| format!("Failed to parse profile '{}': {e}", path.display()))?;
        Ok(profile)
    }

    /// Load a profile by name from the profile directory.
    ///
    /// Appends ".yaml" to the name and looks in `profile_dir`.
    /// If the named profile doesn't exist, tries "default.yaml".
    /// If neither exists, returns an error.
    pub fn load(profile_dir: &str, profile_name: &str) -> Result<Self, String> {
        let primary = Path::new(profile_dir).join(format!("{profile_name}.yaml"));
        if primary.exists() {
            return Self::from_file(&primary);
        }

        let fallback = Path::new(profile_dir).join("default.yaml");
        if fallback.exists() {
            tracing::warn!(
                requested = profile_name,
                "Profile not found, falling back to default.yaml"
            );
            return Self::from_file(&fallback);
        }

        Err(format!(
            "Profile '{profile_name}' not found at '{}' and no default.yaml exists",
            primary.display()
        ))
    }
}

// ── Resolved Config ─────────────────────────────────────────────

/// The fully resolved extraction configuration for a single document.
///
/// This struct represents the final merged result of the three-level
/// hierarchy: system defaults → profile → per-document overrides.
/// It is serialized to JSON and stored in `extraction_runs.processing_config`
/// as the audit trail of exactly what parameters were used.
///
/// The `overrides_applied` field lists which parameters the user changed
/// from the profile defaults, making it clear what was customized.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedConfig {
    pub profile_name: String,
    pub model: String,
    /// Pass-2 model id resolved from override → profile. `None` means
    /// pass 2 falls back to `model` (the pass-1 model). Consulted only
    /// by `run_pass2_extraction`.
    pub pass2_model: Option<String>,
    pub template_file: String,
    pub template_hash: Option<String>,
    /// Pass-2 relationship-extraction prompt template filename, if the
    /// profile declares one. Resolved from
    /// `ProcessingProfile.pass2_template_file` — per-document overrides
    /// are not plumbed for this field (it's a profile-level authoring
    /// concern, not an operator knob).
    pub pass2_template_file: Option<String>,
    pub system_prompt_file: Option<String>,
    /// SHA-256 of the loaded system prompt file. `None` when
    /// `system_prompt_file` is absent. Populated at runtime after the file
    /// is loaded so the audit snapshot can prove exactly which system
    /// prompt was sent to the provider.
    pub system_prompt_hash: Option<String>,
    /// Global-rules fragment filename resolved from the profile. Same
    /// shape and semantics as `system_prompt_file` — `None` means the
    /// profile didn't opt in to global rules; the substitution code
    /// uses an empty string in that case so the placeholder doesn't
    /// leak as literal text.
    pub global_rules_file: Option<String>,
    pub schema_file: String,
    pub chunking_mode: String,
    pub chunk_size: Option<i32>,
    pub chunk_overlap: Option<i32>,
    /// Resolved chunking parameters — the merged result of the profile's
    /// `chunking_config` and any per-document override map.
    /// Serialized to JSONB in `extraction_runs.processing_config` as part
    /// of the audit trail for what knobs were used on this document.
    #[serde(default)]
    pub chunking_config: HashMap<String, serde_json::Value>,
    /// Resolved cross-document context parameters — same merge semantics
    /// as `chunking_config`. Consumed by pass 2 for graph-based relevance
    /// filtering.
    #[serde(default)]
    pub context_config: HashMap<String, serde_json::Value>,
    pub max_tokens: i32,
    pub temperature: f64,
    pub auto_approve_grounded: bool,
    pub run_pass2: bool,
    pub overrides_applied: Vec<String>,
}

/// Per-document overrides from the pipeline_config table.
///
/// All fields are optional — None means "use the profile default."
/// These come from the nullable columns added to pipeline_config
/// by migration 20260420_config_system.sql.
#[derive(Debug, Clone, Default)]
pub struct PipelineConfigOverrides {
    pub profile_name: Option<String>,
    pub extraction_model: Option<String>,
    /// Per-document pass-2 model override. `None` means "use the
    /// profile's `pass2_extraction_model` (or fall back to the pass-1
    /// model)." Populated from the `pass2_extraction_model` column on
    /// `pipeline_config`.
    pub pass2_extraction_model: Option<String>,
    pub template_file: Option<String>,
    pub system_prompt_file: Option<String>,
    pub chunking_mode: Option<String>,
    pub chunk_size: Option<i32>,
    pub chunk_overlap: Option<i32>,
    pub max_tokens: Option<i32>,
    pub temperature: Option<f64>,
    pub run_pass2: Option<bool>,
    /// Per-document chunking config override — merged on top of the
    /// profile's `chunking_config` at the *key level*.
    ///
    /// ## Rust Learning: `Option<HashMap<…>>` vs. plain `HashMap<…>`
    ///
    /// We need to distinguish three states:
    ///   - `None`           — no override provided; use the profile's map verbatim.
    ///   - `Some(empty)`    — override exists but contains no entries; merge result equals the profile's map.
    ///   - `Some(non-empty)`— extend/overwrite the profile's map with these entries.
    ///
    /// A bare `HashMap` would collapse the first two cases together —
    /// every document would look like it had an override, and the
    /// `overrides_applied` audit list would always include
    /// `"chunking_config"`. Wrapping in `Option` preserves the
    /// "no override" signal that the audit trail depends on.
    pub chunking_config: Option<HashMap<String, serde_json::Value>>,
    /// Per-document context config override. Same `Option` rationale as
    /// `chunking_config` above.
    pub context_config: Option<HashMap<String, serde_json::Value>>,
}

/// System-wide defaults used when no profile or override provides a value.
///
/// These match the constants in DOC_PROCESSING_CONFIG_DESIGN_v2.md Section 3.1.
/// They are NOT hardcoded in llm_extract.rs — they live here as the single
/// source of truth for system defaults.
pub struct SystemDefaults;

impl SystemDefaults {
    pub fn model() -> String {
        std::env::var("LLM_MODEL").unwrap_or_else(|_| "claude-sonnet-4-6".to_string())
    }
    pub fn template_file() -> String {
        "chunk_extract.md".to_string()
    }
    pub fn chunking_mode() -> String {
        "chunked".to_string()
    }
    pub fn chunk_size() -> i32 {
        8000
    }
    pub fn chunk_overlap() -> i32 {
        500
    }
    pub fn max_tokens() -> i32 {
        8000
    }
    pub fn temperature() -> f64 {
        0.0
    }
}

/// Resolve the three-level configuration hierarchy for a document.
///
/// Priority: per-document override → profile → system default.
/// Each field is resolved independently — a document can override the model
/// but use the profile's template and the system default chunk size.
///
/// The `overrides_applied` field tracks which values came from per-document
/// overrides so the audit trail shows what was customized.
pub fn resolve_config(
    profile: &ProcessingProfile,
    overrides: &PipelineConfigOverrides,
) -> ResolvedConfig {
    let mut applied = Vec::new();

    let model = match &overrides.extraction_model {
        Some(v) => { applied.push("extraction_model".to_string()); v.clone() }
        None => profile.extraction_model.clone(),
    };

    // Pass-2 model: per-doc override wins; otherwise profile's
    // `pass2_extraction_model`; otherwise `None` (caller falls back to
    // pass-1 `model`).
    let pass2_model = match &overrides.pass2_extraction_model {
        Some(v) => { applied.push("pass2_extraction_model".to_string()); Some(v.clone()) }
        None => profile.pass2_extraction_model.clone(),
    };

    let template_file = match &overrides.template_file {
        Some(v) => { applied.push("template_file".to_string()); v.clone() }
        None => profile.template_file.clone(),
    };

    let system_prompt_file = match &overrides.system_prompt_file {
        Some(v) => { applied.push("system_prompt_file".to_string()); Some(v.clone()) }
        None => profile.system_prompt_file.clone(),
    };

    let chunking_mode = match &overrides.chunking_mode {
        Some(v) => { applied.push("chunking_mode".to_string()); v.clone() }
        None => profile.chunking_mode.clone(),
    };

    let chunk_size = match overrides.chunk_size {
        Some(v) => { applied.push("chunk_size".to_string()); Some(v) }
        None => profile.chunk_size,
    };

    let chunk_overlap = match overrides.chunk_overlap {
        Some(v) => { applied.push("chunk_overlap".to_string()); Some(v) }
        None => profile.chunk_overlap,
    };

    let max_tokens = match overrides.max_tokens {
        Some(v) => { applied.push("max_tokens".to_string()); v }
        None => profile.max_tokens,
    };

    let temperature = match overrides.temperature {
        Some(v) => { applied.push("temperature".to_string()); v }
        None => profile.temperature,
    };

    let run_pass2 = match overrides.run_pass2 {
        Some(v) => { applied.push("run_pass2".to_string()); v }
        None => profile.run_pass2,
    };

    // ## Rust Learning: `HashMap::extend()` is upsert
    //
    // `extend()` walks an iterator of `(K, V)` pairs and inserts each
    // into the map — if the key already exists, the value is *overwritten*;
    // if not, it's inserted. That is exactly the merge semantics we want:
    // start with the profile's full config, then let the per-document
    // override change individual keys without having to re-state every
    // other key. The alternative — replacing the whole map — would force
    // every override to repeat the entire chunking config just to nudge
    // one parameter.
    let chunking_config = match &overrides.chunking_config {
        Some(overrides_map) => {
            applied.push("chunking_config".to_string());
            let mut merged = profile.chunking_config.clone();
            merged.extend(overrides_map.iter().map(|(k, v)| (k.clone(), v.clone())));
            merged
        }
        None => profile.chunking_config.clone(),
    };

    let context_config = match &overrides.context_config {
        Some(overrides_map) => {
            applied.push("context_config".to_string());
            let mut merged = profile.context_config.clone();
            merged.extend(overrides_map.iter().map(|(k, v)| (k.clone(), v.clone())));
            merged
        }
        None => profile.context_config.clone(),
    };

    ResolvedConfig {
        profile_name: overrides.profile_name.clone()
            .unwrap_or_else(|| profile.name.clone()),
        model,
        pass2_model,
        template_file,
        template_hash: None, // Set at runtime after loading the file
        pass2_template_file: profile.pass2_template_file.clone(),
        system_prompt_file,
        system_prompt_hash: None, // Set at runtime after loading the file
        // Global rules are profile-level only — no per-document override path.
        // Operators change them by editing the profile YAML; per-document
        // tweaking didn't seem worth the override-column surface area.
        global_rules_file: profile.global_rules_file.clone(),
        schema_file: profile.schema_file.clone(),
        chunking_mode,
        chunk_size,
        chunk_overlap,
        chunking_config,
        context_config,
        max_tokens,
        temperature,
        auto_approve_grounded: profile.auto_approve_grounded,
        run_pass2,
        overrides_applied: applied,
    }
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp_profile(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn load_valid_profile() {
        let yaml = r#"
name: test_profile
display_name: Test Profile
schema_file: test.yaml
template_file: test_template.md
extraction_model: claude-sonnet-4-6
chunking_mode: full
max_tokens: 32000
temperature: 0.0
"#;
        let f = write_temp_profile(yaml);
        let profile = ProcessingProfile::from_file(f.path()).unwrap();
        assert_eq!(profile.name, "test_profile");
        assert_eq!(profile.chunking_mode, "full");
        assert_eq!(profile.max_tokens, 32000);
        assert!(profile.system_prompt_file.is_none());
        assert!(profile.auto_approve_grounded); // default true
    }

    #[test]
    fn load_profile_with_defaults() {
        let yaml = r#"
name: minimal
display_name: Minimal
schema_file: s.yaml
template_file: t.md
extraction_model: claude-sonnet-4-6
"#;
        let f = write_temp_profile(yaml);
        let profile = ProcessingProfile::from_file(f.path()).unwrap();
        assert_eq!(profile.chunking_mode, "chunked"); // default
        assert_eq!(profile.max_tokens, 8000); // default
        assert_eq!(profile.temperature, 0.0); // default
        assert!(profile.auto_approve_grounded); // default true
        assert!(!profile.run_pass2); // default false
        assert!(!profile.is_default); // default false
    }

    #[test]
    fn load_missing_profile_returns_error() {
        let result = ProcessingProfile::from_file(Path::new("/nonexistent/profile.yaml"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to read profile"));
    }

    #[test]
    fn load_invalid_yaml_returns_error() {
        let f = write_temp_profile("not: [valid: yaml: {{{}}}");
        let result = ProcessingProfile::from_file(f.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to parse profile"));
    }

    #[test]
    fn resolve_no_overrides() {
        let profile = ProcessingProfile {
            name: "complaint".into(),
            display_name: "Complaint".into(),
            description: String::new(),
            schema_file: "complaint_v2.yaml".into(),
            template_file: "pass1_complaint.md".into(),
            system_prompt_file: None,
            global_rules_file: None,
            extraction_model: "claude-sonnet-4-6".into(),
            pass2_extraction_model: None,
            synthesis_model: None,
            chunking_mode: "full".into(),
            chunk_size: None,
            chunk_overlap: None,
            chunking_config: HashMap::new(),
            context_config: HashMap::new(),
            max_tokens: 32000,
            temperature: 0.0,
            auto_approve_grounded: true,
            run_pass2: false,
            pass2_template_file: None,
            is_default: false,
        };
        let overrides = PipelineConfigOverrides::default();
        let resolved = resolve_config(&profile, &overrides);

        assert_eq!(resolved.profile_name, "complaint");
        assert_eq!(resolved.model, "claude-sonnet-4-6");
        assert_eq!(resolved.template_file, "pass1_complaint.md");
        assert_eq!(resolved.chunking_mode, "full");
        assert_eq!(resolved.max_tokens, 32000);
        assert!(resolved.overrides_applied.is_empty());
    }

    #[test]
    fn resolve_surfaces_system_prompt_file_from_profile() {
        // Guard against regression of the silent-ignore bug fixed alongside
        // this test: the resolver must pass system_prompt_file through so the
        // step can load it and route through invoke_with_system.
        let profile = ProcessingProfile {
            name: "complaint".into(),
            display_name: "Complaint".into(),
            description: String::new(),
            schema_file: "complaint_v2.yaml".into(),
            template_file: "pass1_complaint.md".into(),
            system_prompt_file: Some("legal_extraction_system.md".into()),
            global_rules_file: None,
            extraction_model: "claude-sonnet-4-6".into(),
            pass2_extraction_model: None,
            synthesis_model: None,
            chunking_mode: "full".into(),
            chunk_size: None,
            chunk_overlap: None,
            chunking_config: HashMap::new(),
            context_config: HashMap::new(),
            max_tokens: 32000,
            temperature: 0.0,
            auto_approve_grounded: true,
            run_pass2: false,
            pass2_template_file: None,
            is_default: false,
        };
        let overrides = PipelineConfigOverrides::default();
        let resolved = resolve_config(&profile, &overrides);
        assert_eq!(
            resolved.system_prompt_file.as_deref(),
            Some("legal_extraction_system.md"),
            "resolver must surface profile.system_prompt_file when no override"
        );
        assert!(
            resolved.system_prompt_hash.is_none(),
            "hash is set by the step after loading, not by the resolver"
        );

        // Per-doc override wins.
        let overrides = PipelineConfigOverrides {
            system_prompt_file: Some("custom_system.md".into()),
            ..Default::default()
        };
        let resolved = resolve_config(&profile, &overrides);
        assert_eq!(
            resolved.system_prompt_file.as_deref(),
            Some("custom_system.md")
        );
        assert!(resolved
            .overrides_applied
            .contains(&"system_prompt_file".to_string()));
    }

    #[test]
    fn resolve_surfaces_pass2_template_file_from_profile() {
        // The resolver must pass pass2_template_file through so the manually-
        // invoked pass-2 path can locate the relationship-extraction prompt.
        // No per-document override is plumbed — it's a profile-level concern.
        let profile = ProcessingProfile {
            name: "complaint".into(),
            display_name: "Complaint".into(),
            description: String::new(),
            schema_file: "complaint_v2.yaml".into(),
            template_file: "pass1_complaint.md".into(),
            system_prompt_file: None,
            global_rules_file: None,
            pass2_template_file: Some("pass2_complaint.md".into()),
            extraction_model: "claude-sonnet-4-6".into(),
            pass2_extraction_model: None,
            synthesis_model: None,
            chunking_mode: "full".into(),
            chunk_size: None,
            chunk_overlap: None,
            chunking_config: HashMap::new(),
            context_config: HashMap::new(),
            max_tokens: 32000,
            temperature: 0.0,
            auto_approve_grounded: true,
            run_pass2: true,
            is_default: false,
        };
        let overrides = PipelineConfigOverrides::default();
        let resolved = resolve_config(&profile, &overrides);
        assert_eq!(
            resolved.pass2_template_file.as_deref(),
            Some("pass2_complaint.md"),
            "resolver must surface profile.pass2_template_file"
        );
    }

    #[test]
    fn resolve_surfaces_pass2_extraction_model_from_profile() {
        // Profile-level default flows through.
        let yaml = r#"
name: complaint
display_name: Complaint
schema_file: complaint_v2.yaml
template_file: pass1_complaint.md
extraction_model: claude-sonnet-4-6
pass2_extraction_model: claude-opus-4-7
"#;
        let f = write_temp_profile(yaml);
        let profile = ProcessingProfile::from_file(f.path()).unwrap();
        assert_eq!(
            profile.pass2_extraction_model.as_deref(),
            Some("claude-opus-4-7")
        );
        let resolved = resolve_config(&profile, &PipelineConfigOverrides::default());
        assert_eq!(resolved.pass2_model.as_deref(), Some("claude-opus-4-7"));
        assert!(
            resolved.overrides_applied.is_empty(),
            "no per-doc override was set, applied list must be empty"
        );
    }

    #[test]
    fn resolve_per_doc_pass2_model_override_wins_and_is_recorded() {
        let yaml = r#"
name: complaint
display_name: Complaint
schema_file: complaint_v2.yaml
template_file: pass1_complaint.md
extraction_model: claude-sonnet-4-6
pass2_extraction_model: claude-opus-4-7
"#;
        let f = write_temp_profile(yaml);
        let profile = ProcessingProfile::from_file(f.path()).unwrap();
        let overrides = PipelineConfigOverrides {
            pass2_extraction_model: Some("claude-opus-4-6".into()),
            ..Default::default()
        };
        let resolved = resolve_config(&profile, &overrides);
        assert_eq!(resolved.pass2_model.as_deref(), Some("claude-opus-4-6"));
        assert!(
            resolved
                .overrides_applied
                .contains(&"pass2_extraction_model".to_string()),
            "per-doc overrides must be tracked in overrides_applied"
        );
    }

    #[test]
    fn pass2_model_is_none_when_neither_profile_nor_override_sets_it() {
        // Back-compat: pre-pass-2 profiles must keep working. When
        // nothing provides a pass-2 model, the resolver returns None so
        // run_pass2_extraction falls back to the pass-1 model.
        let yaml = r#"
name: legacy
display_name: Legacy
schema_file: s.yaml
template_file: t.md
extraction_model: claude-sonnet-4-6
"#;
        let f = write_temp_profile(yaml);
        let profile = ProcessingProfile::from_file(f.path()).unwrap();
        assert!(profile.pass2_extraction_model.is_none());
        let resolved = resolve_config(&profile, &PipelineConfigOverrides::default());
        assert!(resolved.pass2_model.is_none());
    }

    #[test]
    fn pass2_template_file_is_none_when_profile_omits_it() {
        // Profiles that predate pass 2 support must still load — the YAML
        // field is optional and defaults to None without an explicit value.
        let yaml = r#"
name: legacy
display_name: Legacy
schema_file: s.yaml
template_file: t.md
extraction_model: claude-sonnet-4-6
"#;
        let f = write_temp_profile(yaml);
        let profile = ProcessingProfile::from_file(f.path()).unwrap();
        assert!(profile.pass2_template_file.is_none());

        let resolved = resolve_config(&profile, &PipelineConfigOverrides::default());
        assert!(resolved.pass2_template_file.is_none());
    }

    #[test]
    fn resolve_with_overrides() {
        let profile = ProcessingProfile {
            name: "complaint".into(),
            display_name: "Complaint".into(),
            description: String::new(),
            schema_file: "complaint_v2.yaml".into(),
            template_file: "pass1_complaint.md".into(),
            system_prompt_file: None,
            global_rules_file: None,
            extraction_model: "claude-sonnet-4-6".into(),
            pass2_extraction_model: None,
            synthesis_model: None,
            chunking_mode: "full".into(),
            chunk_size: None,
            chunk_overlap: None,
            chunking_config: HashMap::new(),
            context_config: HashMap::new(),
            max_tokens: 32000,
            temperature: 0.0,
            auto_approve_grounded: true,
            run_pass2: false,
            pass2_template_file: None,
            is_default: false,
        };
        let overrides = PipelineConfigOverrides {
            template_file: Some("custom_template.md".into()),
            temperature: Some(0.3),
            ..Default::default()
        };
        let resolved = resolve_config(&profile, &overrides);

        assert_eq!(resolved.template_file, "custom_template.md");
        assert_eq!(resolved.temperature, 0.3);
        // Non-overridden fields come from profile
        assert_eq!(resolved.chunking_mode, "full");
        assert_eq!(resolved.max_tokens, 32000);
        // Overrides tracked
        assert!(resolved.overrides_applied.contains(&"template_file".to_string()));
        assert!(resolved.overrides_applied.contains(&"temperature".to_string()));
        assert_eq!(resolved.overrides_applied.len(), 2);
    }

    #[test]
    fn resolved_config_serializes_to_json() {
        let config = ResolvedConfig {
            profile_name: "complaint".into(),
            model: "claude-sonnet-4-6".into(),
            pass2_model: None,
            template_file: "pass1_complaint.md".into(),
            template_hash: Some("abc123".into()),
            pass2_template_file: None,
            system_prompt_file: None,
            system_prompt_hash: None,
            global_rules_file: None,
            schema_file: "complaint_v2.yaml".into(),
            chunking_mode: "full".into(),
            chunk_size: None,
            chunk_overlap: None,
            chunking_config: HashMap::new(),
            context_config: HashMap::new(),
            max_tokens: 32000,
            temperature: 0.0,
            auto_approve_grounded: true,
            run_pass2: false,
            overrides_applied: vec!["temperature".into()],
        };
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["profile_name"], "complaint");
        assert_eq!(json["overrides_applied"][0], "temperature");
        // This JSON is what gets stored in extraction_runs.processing_config
    }

    // ── chunking_config / context_config (Phase 1b Group 1) ──────────

    #[test]
    fn load_profile_with_chunking_config() {
        let yaml = r#"
name: discovery_response
display_name: Discovery Response
schema_file: discovery_response_v4.yaml
template_file: pass1_discovery_response_v4.md
extraction_model: claude-sonnet-4-6
chunking_config:
  mode: structured
  strategy: qa_pair
  units_per_chunk: 25
  unit_overlap: 0
  request_timeout_secs: 1800
context_config:
  traversal_depth: 2
  always_include_foundation: true
"#;
        let f = write_temp_profile(yaml);
        let profile = ProcessingProfile::from_file(f.path()).unwrap();

        // chunking_config is a HashMap — verify keys exist and values are correct types
        assert_eq!(
            profile.chunking_config.get("mode").and_then(|v| v.as_str()),
            Some("structured")
        );
        assert_eq!(
            profile.chunking_config.get("strategy").and_then(|v| v.as_str()),
            Some("qa_pair")
        );
        assert_eq!(
            profile.chunking_config.get("units_per_chunk").and_then(|v| v.as_i64()),
            Some(25)
        );
        assert_eq!(
            profile
                .chunking_config
                .get("request_timeout_secs")
                .and_then(|v| v.as_i64()),
            Some(1800)
        );

        // context_config
        assert_eq!(
            profile.context_config.get("traversal_depth").and_then(|v| v.as_i64()),
            Some(2)
        );
        assert_eq!(
            profile
                .context_config
                .get("always_include_foundation")
                .and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn load_profile_without_chunking_config_gets_empty_maps() {
        // Backward compatibility: profiles authored before these fields
        // existed must still deserialize. `#[serde(default)]` on the
        // HashMap fields produces empty maps when the YAML keys are absent.
        let yaml = r#"
name: complaint
display_name: Complaint
schema_file: complaint_v2.yaml
template_file: pass1_complaint.md
extraction_model: claude-sonnet-4-6
chunking_mode: full
"#;
        let f = write_temp_profile(yaml);
        let profile = ProcessingProfile::from_file(f.path()).unwrap();

        assert!(
            profile.chunking_config.is_empty(),
            "expected empty chunking_config for legacy profile"
        );
        assert!(
            profile.context_config.is_empty(),
            "expected empty context_config for legacy profile"
        );
        // Legacy field still works
        assert_eq!(profile.chunking_mode, "full");
    }

    #[test]
    fn resolve_config_includes_chunking_config() {
        let mut chunking_config = HashMap::new();
        chunking_config.insert("mode".to_string(), serde_json::json!("structured"));
        chunking_config.insert("strategy".to_string(), serde_json::json!("qa_pair"));
        chunking_config.insert("units_per_chunk".to_string(), serde_json::json!(25));

        let profile = ProcessingProfile {
            name: "discovery".into(),
            display_name: "Discovery".into(),
            description: String::new(),
            schema_file: "disc.yaml".into(),
            template_file: "disc.md".into(),
            system_prompt_file: None,
            global_rules_file: None,
            pass2_template_file: None,
            extraction_model: "claude-sonnet-4-6".into(),
            pass2_extraction_model: None,
            synthesis_model: None,
            chunking_mode: "full".into(),
            chunk_size: None,
            chunk_overlap: None,
            chunking_config: chunking_config.clone(),
            context_config: HashMap::new(),
            max_tokens: 32000,
            temperature: 0.0,
            auto_approve_grounded: true,
            run_pass2: false,
            is_default: false,
        };
        let overrides = PipelineConfigOverrides::default();
        let resolved = resolve_config(&profile, &overrides);

        assert_eq!(
            resolved.chunking_config.get("mode").and_then(|v| v.as_str()),
            Some("structured")
        );
        assert_eq!(
            resolved.chunking_config.get("units_per_chunk").and_then(|v| v.as_i64()),
            Some(25)
        );
        assert!(resolved.context_config.is_empty());
        // chunking_config was NOT overridden — should not appear in overrides_applied
        assert!(!resolved
            .overrides_applied
            .contains(&"chunking_config".to_string()));
    }

    #[test]
    fn resolve_config_merges_chunking_config_overrides() {
        // Verifies the key-level merge contract: per-document override
        // changes one knob (units_per_chunk) without having to re-state the
        // rest of the config (mode, strategy). This is the crux of why
        // `Option<HashMap<...>>` + `extend()` is the right shape — a
        // whole-map replacement would force every override to enumerate
        // every key the profile already set.
        let mut profile_config = HashMap::new();
        profile_config.insert("mode".to_string(), serde_json::json!("structured"));
        profile_config.insert("strategy".to_string(), serde_json::json!("qa_pair"));
        profile_config.insert("units_per_chunk".to_string(), serde_json::json!(25));

        let profile = ProcessingProfile {
            name: "discovery".into(),
            display_name: "Discovery".into(),
            description: String::new(),
            schema_file: "disc.yaml".into(),
            template_file: "disc.md".into(),
            system_prompt_file: None,
            global_rules_file: None,
            pass2_template_file: None,
            extraction_model: "claude-sonnet-4-6".into(),
            pass2_extraction_model: None,
            synthesis_model: None,
            chunking_mode: "full".into(),
            chunk_size: None,
            chunk_overlap: None,
            chunking_config: profile_config,
            context_config: HashMap::new(),
            max_tokens: 32000,
            temperature: 0.0,
            auto_approve_grounded: true,
            run_pass2: false,
            is_default: false,
        };

        // Per-document override changes units_per_chunk but NOT mode or strategy
        let mut override_config = HashMap::new();
        override_config.insert("units_per_chunk".to_string(), serde_json::json!(15));

        let overrides = PipelineConfigOverrides {
            chunking_config: Some(override_config),
            ..Default::default()
        };
        let resolved = resolve_config(&profile, &overrides);

        // units_per_chunk was overridden
        assert_eq!(
            resolved.chunking_config.get("units_per_chunk").and_then(|v| v.as_i64()),
            Some(15)
        );
        // mode and strategy carried through from profile
        assert_eq!(
            resolved.chunking_config.get("mode").and_then(|v| v.as_str()),
            Some("structured")
        );
        assert_eq!(
            resolved.chunking_config.get("strategy").and_then(|v| v.as_str()),
            Some("qa_pair")
        );
        // Tracked as overridden
        assert!(resolved
            .overrides_applied
            .contains(&"chunking_config".to_string()));
    }

    #[test]
    fn resolved_config_with_chunking_config_serializes() {
        let mut chunking_config = HashMap::new();
        chunking_config.insert("mode".to_string(), serde_json::json!("structured"));
        chunking_config.insert("strategy".to_string(), serde_json::json!("qa_pair"));

        let config = ResolvedConfig {
            profile_name: "discovery".into(),
            model: "claude-sonnet-4-6".into(),
            pass2_model: None,
            template_file: "disc.md".into(),
            template_hash: None,
            pass2_template_file: None,
            system_prompt_file: None,
            system_prompt_hash: None,
            global_rules_file: None,
            schema_file: "disc.yaml".into(),
            chunking_mode: "full".into(),
            chunk_size: None,
            chunk_overlap: None,
            chunking_config,
            context_config: HashMap::new(),
            max_tokens: 32000,
            temperature: 0.0,
            auto_approve_grounded: true,
            run_pass2: false,
            overrides_applied: vec![],
        };

        let json = serde_json::to_value(&config).unwrap();

        // chunking_config appears in the JSON (this is what gets stored in
        // extraction_runs.processing_config — the audit trail).
        assert_eq!(json["chunking_config"]["mode"], "structured");
        assert_eq!(json["chunking_config"]["strategy"], "qa_pair");
        // Empty context_config still serializes as an object, not omitted.
        // This matters for the audit trail: a downstream consumer can rely
        // on the key being present and reading `{}` rather than handling
        // both "missing" and "empty" states.
        assert!(json["context_config"].is_object());
    }

    #[test]
    fn profile_preserves_unknown_chunking_config_keys() {
        // ## Rust Learning: forward-compatibility via `serde_json::Value`
        //
        // `HashMap<String, serde_json::Value>` accepts any valid JSON-shaped
        // value as the map value. When future YAML profiles introduce keys
        // we haven't coded a reader for yet, those keys round-trip
        // through deserialization untouched — they aren't rejected (which
        // would break old colossus-legal builds reading new profiles), and
        // they aren't silently dropped (which would lose user intent on
        // re-serialize). This is the property that lets Group 2 add new
        // chunking knobs without coordinated releases of colossus-legal.
        let yaml = r#"
name: future_proof
display_name: Future Proof
schema_file: test.yaml
template_file: test.md
extraction_model: claude-sonnet-4-6
chunking_config:
  mode: structured
  strategy: qa_pair
  future_key_we_dont_know_about_yet: 42
  another_unknown: "hello"
"#;
        let f = write_temp_profile(yaml);
        let profile = ProcessingProfile::from_file(f.path()).unwrap();

        // Known keys work
        assert_eq!(
            profile.chunking_config.get("mode").and_then(|v| v.as_str()),
            Some("structured")
        );
        // Unknown keys are preserved — not rejected, not silently dropped
        assert_eq!(
            profile
                .chunking_config
                .get("future_key_we_dont_know_about_yet")
                .and_then(|v| v.as_i64()),
            Some(42)
        );
        assert_eq!(
            profile
                .chunking_config
                .get("another_unknown")
                .and_then(|v| v.as_str()),
            Some("hello")
        );
    }
}
