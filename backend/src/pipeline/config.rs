//! Pipeline configuration types — profile loading and config resolution.
//!
//! Processing profiles are YAML files on mounted storage (not database rows).
//! This module loads them, validates references, and resolves the three-level
//! configuration hierarchy: system defaults → profile → per-document overrides.
//!
//! Design: DOC_PROCESSING_CONFIG_DESIGN_v2.md Sections 3.1, 3.2.2, 3.7.

use serde::{Deserialize, Serialize};
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

    // Model — must match an id in the llm_models table
    pub extraction_model: String,
    #[serde(default)]
    pub synthesis_model: Option<String>,

    // Chunking
    #[serde(default = "default_chunking_mode")]
    pub chunking_mode: String,
    #[serde(default)]
    pub chunk_size: Option<i32>,
    #[serde(default)]
    pub chunk_overlap: Option<i32>,

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
    pub template_file: String,
    pub template_hash: Option<String>,
    pub system_prompt_file: Option<String>,
    pub schema_file: String,
    pub chunking_mode: String,
    pub chunk_size: Option<i32>,
    pub chunk_overlap: Option<i32>,
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
    pub template_file: Option<String>,
    pub system_prompt_file: Option<String>,
    pub chunking_mode: Option<String>,
    pub chunk_size: Option<i32>,
    pub chunk_overlap: Option<i32>,
    pub max_tokens: Option<i32>,
    pub temperature: Option<f64>,
    pub run_pass2: Option<bool>,
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

    ResolvedConfig {
        profile_name: overrides.profile_name.clone()
            .unwrap_or_else(|| profile.name.clone()),
        model,
        template_file,
        template_hash: None, // Set at runtime after loading the file
        system_prompt_file,
        schema_file: profile.schema_file.clone(),
        chunking_mode,
        chunk_size,
        chunk_overlap,
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
            extraction_model: "claude-sonnet-4-6".into(),
            synthesis_model: None,
            chunking_mode: "full".into(),
            chunk_size: None,
            chunk_overlap: None,
            max_tokens: 32000,
            temperature: 0.0,
            auto_approve_grounded: true,
            run_pass2: false,
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
    fn resolve_with_overrides() {
        let profile = ProcessingProfile {
            name: "complaint".into(),
            display_name: "Complaint".into(),
            description: String::new(),
            schema_file: "complaint_v2.yaml".into(),
            template_file: "pass1_complaint.md".into(),
            system_prompt_file: None,
            extraction_model: "claude-sonnet-4-6".into(),
            synthesis_model: None,
            chunking_mode: "full".into(),
            chunk_size: None,
            chunk_overlap: None,
            max_tokens: 32000,
            temperature: 0.0,
            auto_approve_grounded: true,
            run_pass2: false,
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
            template_file: "pass1_complaint.md".into(),
            template_hash: Some("abc123".into()),
            system_prompt_file: None,
            schema_file: "complaint_v2.yaml".into(),
            chunking_mode: "full".into(),
            chunk_size: None,
            chunk_overlap: None,
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
}
