//! Pipeline Configuration Registry — single source of truth for the
//! extraction pipeline's directory layout and document-type → profile
//! mappings.
//!
//! The registry replaces four independent env vars
//! (`PROCESSING_PROFILE_DIR`, `EXTRACTION_SCHEMA_DIR`,
//! `EXTRACTION_TEMPLATE_DIR`, `SYSTEM_PROMPT_DIR`) and the previous
//! filesystem-scanning approach to profile selection with a single
//! authoritative YAML file. The backend reads `PIPELINE_REGISTRY_FILE`
//! once at startup, validates it loudly, and surfaces an
//! `Arc<PipelineRegistry>` through `AppState` (for HTTP handlers) and
//! `AppContext` (for pipeline steps).
//!
//! ## Why a registry?
//!
//! Before the registry, mapping a document type → profile required three
//! things in lockstep: a profile YAML on disk with `document_type:` set,
//! `is_default: true` in exactly the right place, and the on-disk file
//! living under whichever directory the env var happened to point at. A
//! typo or omission anywhere in that chain caused upload-time fallback
//! to `default.yaml` — silently. The registry makes the mapping
//! explicit, validates it at startup, and fails loudly when a referenced
//! profile file is missing.
//!
//! ## Backward compatibility
//!
//! When `PIPELINE_REGISTRY_FILE` is unset, [`PipelineRegistry::from_env`]
//! falls back to constructing a registry from the four legacy env vars
//! by scanning the profile directory. The fallback path logs a
//! deprecation warning at WARN level. This lets the binary deploy before
//! the registry YAML is created on a target host.
//!
//! ## Standing Rule alignment
//!
//! - **Rule 1 (no silent failures)** — `validate()` returns `Err` with a
//!   message naming the specific problem (missing directory, missing
//!   file, duplicate name, etc.). The backend refuses to start on any
//!   validation failure.
//! - **Rule 2 (no hardcoded values)** — every path comes from YAML or
//!   env vars. No defaults exist for `PIPELINE_REGISTRY_FILE`; the
//!   legacy env vars also have no defaults inside the fallback (the
//!   caller's existing `config.rs` defaults are unchanged but are
//!   bypassed when the registry path is in use).

use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Errors returned by the registry loader.
///
/// ## Rust Learning: `thiserror` + structured fields
///
/// `#[derive(thiserror::Error)]` generates `Display` and `Error` impls
/// from the `#[error("...")]` attribute strings. Structured fields
/// (`path`, `source`) substitute into the format string AND remain
/// available to programmatic callers via match arms — letting any
/// caller distinguish "registry file missing" from "registry YAML
/// malformed" without parsing the error message.
#[derive(Debug, thiserror::Error)]
pub enum PipelineRegistryError {
    /// A configuration invariant failed (missing directory, missing
    /// profile file, duplicate names, no default, etc.). The string
    /// names the specific problem.
    #[error("Pipeline registry config error: {0}")]
    Config(String),

    /// Could not read the registry YAML file from disk.
    #[error("Failed to read pipeline registry file '{path}': {source}")]
    IoError {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// Read succeeded but the YAML body is malformed.
    #[error("Failed to parse pipeline registry YAML '{path}': {source}")]
    ParseError {
        path: String,
        #[source]
        source: serde_yaml::Error,
    },
}

/// Directory paths consumed by every step of the extraction pipeline.
///
/// Surfaces the four directories that previously came from independent
/// env vars. The registry holds them so a single edit in YAML moves a
/// directory without recompiling.
#[derive(Debug, Clone, Deserialize)]
pub struct PipelineDirectories {
    pub profiles: String,
    pub schemas: String,
    pub templates: String,
    pub system_prompts: String,
}

/// One document-type entry in the registry.
///
/// Maps an upload-time `document_type` value (e.g. `"complaint"`,
/// `"discovery_response"`) to the processing-profile YAML the pipeline
/// should load. The entry's `name` is the *registry key* — the
/// document_type the upload route receives. The profile YAML's own
/// `name:` field is irrelevant to this mapping; only the on-disk
/// filename (`profile_file`) matters.
///
/// ## Rust Learning: `#[serde(default)]` on individual fields
///
/// `description`, `is_default`, and `sort_order` are tagged
/// `#[serde(default)]` so the registry YAML can omit them and serde
/// fills in `String::default()` / `bool::default()` / `i32::default()`.
/// `name`, `display_name`, and `profile_file` are mandatory — serde
/// errors with "missing field" if the YAML omits them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentTypeEntry {
    pub name: String,
    pub display_name: String,
    pub profile_file: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub sort_order: i32,
}

/// The registry root.
///
/// Constructed once at startup, wrapped in `Arc`, and shared across all
/// handlers and pipeline steps. Immutable after construction — editing
/// the registry YAML requires a backend restart (matching the lifecycle
/// of any other startup-loaded configuration).
#[derive(Debug, Clone, Deserialize)]
pub struct PipelineRegistry {
    pub directories: PipelineDirectories,
    pub document_types: Vec<DocumentTypeEntry>,
}

impl PipelineRegistry {
    /// Load the registry from the environment.
    ///
    /// Reads `PIPELINE_REGISTRY_FILE` and delegates to [`from_file`]
    /// if set. Otherwise falls back to [`from_legacy_env_vars`] and
    /// logs a deprecation warning.
    ///
    /// [`from_file`]: PipelineRegistry::from_file
    /// [`from_legacy_env_vars`]: PipelineRegistry::from_legacy_env_vars
    pub fn from_env() -> Result<Self, PipelineRegistryError> {
        if let Ok(path) = std::env::var("PIPELINE_REGISTRY_FILE") {
            Self::from_file(&path)
        } else {
            Self::from_legacy_env_vars()
        }
    }

    /// Load and validate the registry from a YAML file on disk.
    ///
    /// Three failure modes, each returning a distinct error variant:
    /// the file is unreadable (`IoError`), the YAML is malformed
    /// (`ParseError`), or validation rejects the parsed content
    /// (`Config`). Validation failures name the specific problem so
    /// the operator can fix the YAML without reading source code.
    pub fn from_file(path: &str) -> Result<Self, PipelineRegistryError> {
        let content =
            std::fs::read_to_string(path).map_err(|source| PipelineRegistryError::IoError {
                path: path.to_string(),
                source,
            })?;
        let registry: Self =
            serde_yaml::from_str(&content).map_err(|source| PipelineRegistryError::ParseError {
                path: path.to_string(),
                source,
            })?;
        registry.validate()?;
        Ok(registry)
    }

    /// Backward-compatibility fallback: build a registry from the four
    /// legacy env vars by scanning the profile directory for `*.yaml`
    /// files.
    ///
    /// One entry is created per profile file. The entry's `name` is
    /// the YAML's `name:` field (so `select_profile_for_document_type`
    /// callers that pass the filename stem still resolve under the
    /// new API). A file named `default.yaml` is marked `is_default`.
    ///
    /// Returns `Err(Config(...))` if any of the four env vars is unset
    /// — explicitly NOT supplying a default. The legacy path is
    /// strictly the existing behavior shifted under the registry
    /// abstraction; production deployments should set
    /// `PIPELINE_REGISTRY_FILE` and skip this branch entirely.
    pub fn from_legacy_env_vars() -> Result<Self, PipelineRegistryError> {
        let profiles = read_required_env_var("PROCESSING_PROFILE_DIR")?;
        let schemas = read_required_env_var("EXTRACTION_SCHEMA_DIR")?;
        let templates = read_required_env_var("EXTRACTION_TEMPLATE_DIR")?;
        let system_prompts = read_required_env_var("SYSTEM_PROMPT_DIR")?;

        tracing::warn!(
            "PIPELINE_REGISTRY_FILE not set — falling back to legacy env vars \
             (PROCESSING_PROFILE_DIR, EXTRACTION_SCHEMA_DIR, EXTRACTION_TEMPLATE_DIR, \
             SYSTEM_PROMPT_DIR). This fallback is deprecated; set PIPELINE_REGISTRY_FILE \
             to point at a pipeline_registry.yaml file."
        );

        let directories = PipelineDirectories {
            profiles,
            schemas,
            templates,
            system_prompts,
        };

        let document_types = scan_legacy_profile_dir(&directories.profiles)?;

        let registry = Self {
            directories,
            document_types,
        };
        registry.validate()?;
        Ok(registry)
    }

    /// Reject anything that would make the runtime misbehave.
    ///
    /// Every check returns a distinct error message that names the
    /// failing entity (the directory, the document type, the duplicate
    /// name). An operator reading the log on a failed start should be
    /// able to fix the YAML without consulting source.
    ///
    /// ## Rust Learning: `HashSet` for duplicate detection
    ///
    /// `HashSet::insert(value)` returns `false` if the value was
    /// already present. The duplicate-name check walks entries once,
    /// inserting each `name` and returning an error on the first
    /// `false` — O(n) instead of O(n²) for a nested-loop check.
    pub fn validate(&self) -> Result<(), PipelineRegistryError> {
        validate_directory("profiles", &self.directories.profiles)?;
        validate_directory("schemas", &self.directories.schemas)?;
        validate_directory("templates", &self.directories.templates)?;
        validate_directory("system_prompts", &self.directories.system_prompts)?;

        let mut seen: HashSet<&str> = HashSet::new();
        for entry in &self.document_types {
            if entry.name.is_empty() {
                return Err(PipelineRegistryError::Config(
                    "Document type entry has empty name".to_string(),
                ));
            }
            if entry.profile_file.is_empty() {
                return Err(PipelineRegistryError::Config(format!(
                    "Document type '{}' has empty profile_file",
                    entry.name
                )));
            }
            if !seen.insert(entry.name.as_str()) {
                return Err(PipelineRegistryError::Config(format!(
                    "Duplicate document type name in registry: '{}'",
                    entry.name
                )));
            }
            let path = Path::new(&self.directories.profiles).join(&entry.profile_file);
            if !path.exists() {
                return Err(PipelineRegistryError::Config(format!(
                    "Profile file not found for document type '{}': {}",
                    entry.name,
                    path.display()
                )));
            }
        }

        let default_count = self.document_types.iter().filter(|e| e.is_default).count();
        if default_count != 1 {
            return Err(PipelineRegistryError::Config(format!(
                "Registry must have exactly one default document type, found {default_count}"
            )));
        }

        Ok(())
    }

    /// Look up a document-type entry by registry key.
    ///
    /// The key is the upload-time `document_type` value (e.g.
    /// `"complaint"`), NOT the profile YAML's `name:` field. Returns
    /// `None` if the registry has no entry for `name` — callers
    /// typically chain `.or_else(|| registry.default_document_type())`.
    pub fn document_type(&self, name: &str) -> Option<&DocumentTypeEntry> {
        self.document_types.iter().find(|e| e.name == name)
    }

    /// Return the fallback entry (the one with `is_default: true`).
    ///
    /// Validation guarantees exactly one such entry exists in a valid
    /// registry, so callers can `.unwrap()` here only inside test code.
    /// Production code should `?`-propagate on `None` to keep the
    /// "registry was somehow invalidated post-startup" case observable.
    pub fn default_document_type(&self) -> Option<&DocumentTypeEntry> {
        self.document_types.iter().find(|e| e.is_default)
    }

    /// Full filesystem path to a profile YAML.
    pub fn profile_path(&self, profile_file: &str) -> String {
        join_dir(&self.directories.profiles, profile_file)
    }

    /// Full filesystem path to a schema YAML.
    pub fn schema_path(&self, schema_file: &str) -> String {
        join_dir(&self.directories.schemas, schema_file)
    }

    /// Full filesystem path to a prompt template.
    pub fn template_path(&self, template_file: &str) -> String {
        join_dir(&self.directories.templates, template_file)
    }

    /// Full filesystem path to a system-prompt file.
    pub fn system_prompt_path(&self, prompt_file: &str) -> String {
        join_dir(&self.directories.system_prompts, prompt_file)
    }

    /// Profile directory — for callers that need the directory itself
    /// (e.g. `tokio::fs::read_dir` for listing endpoints).
    pub fn profile_dir(&self) -> &str {
        &self.directories.profiles
    }

    /// Schema directory.
    pub fn schema_dir(&self) -> &str {
        &self.directories.schemas
    }

    /// Template directory.
    pub fn template_dir(&self) -> &str {
        &self.directories.templates
    }

    /// System-prompt directory.
    pub fn system_prompt_dir(&self) -> &str {
        &self.directories.system_prompts
    }

    /// Construct a minimal registry for tests that don't exercise the
    /// registry's runtime methods but still need an `Arc<PipelineRegistry>`
    /// to populate `AppState` / `AppContext`.
    ///
    /// **Not** validated — calling [`validate`] on the returned value
    /// would fail (the placeholder `/tmp` directories satisfy the
    /// existence check but no document_types entries exist, so the
    /// "exactly one default" rule rejects it). Tests that DO exercise
    /// registry methods should build a real registry with a tempfile
    /// layout instead, as the registry's own tests do.
    ///
    /// `#[doc(hidden)]` because this is implementation-detail-of-tests,
    /// not part of the supported API; production code that constructs
    /// a registry by hand is a code smell that the legacy fallback or
    /// the YAML loader should cover.
    ///
    /// [`validate`]: PipelineRegistry::validate
    #[doc(hidden)]
    pub fn stub_for_tests() -> Self {
        Self {
            directories: PipelineDirectories {
                profiles: "/tmp".to_string(),
                schemas: "/tmp".to_string(),
                templates: "/tmp".to_string(),
                system_prompts: "/tmp".to_string(),
            },
            document_types: Vec::new(),
        }
    }

    /// Document types sorted by `sort_order`, excluding the default
    /// entry.
    ///
    /// The default entry is omitted because UIs render it implicitly
    /// (as "Other" or via auto-detection); including it would let the
    /// operator pick "default" explicitly from a dropdown, which is
    /// confusing.
    pub fn document_types_sorted(&self) -> Vec<&DocumentTypeEntry> {
        let mut sorted: Vec<&DocumentTypeEntry> = self
            .document_types
            .iter()
            .filter(|e| !e.is_default)
            .collect();
        sorted.sort_by_key(|e| e.sort_order);
        sorted
    }
}

// ── Helpers ────────────────────────────────────────────────────

fn read_required_env_var(name: &str) -> Result<String, PipelineRegistryError> {
    std::env::var(name).map_err(|_| {
        PipelineRegistryError::Config(format!(
            "Neither PIPELINE_REGISTRY_FILE nor legacy env var '{name}' is set"
        ))
    })
}

fn validate_directory(label: &str, path: &str) -> Result<(), PipelineRegistryError> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(PipelineRegistryError::Config(format!(
            "Registry {label} directory does not exist: {path}"
        )));
    }
    if !p.is_dir() {
        return Err(PipelineRegistryError::Config(format!(
            "Registry {label} path is not a directory: {path}"
        )));
    }
    Ok(())
}

fn join_dir(dir: &str, file: &str) -> String {
    Path::new(dir).join(file).to_string_lossy().into_owned()
}

/// Build [`DocumentTypeEntry`] values by scanning a legacy profile
/// directory.
///
/// Parses each `*.yaml` file as a [`ProcessingProfile`] to read its
/// display fields. A file named `default.yaml` becomes the default
/// entry. Parse failures abort with a `Config` error — Standing Rule 1
/// forbids silently dropping a profile during startup.
fn scan_legacy_profile_dir(
    profile_dir: &str,
) -> Result<Vec<DocumentTypeEntry>, PipelineRegistryError> {
    let entries =
        std::fs::read_dir(profile_dir).map_err(|source| PipelineRegistryError::IoError {
            path: profile_dir.to_string(),
            source,
        })?;

    let mut document_types = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| PipelineRegistryError::IoError {
            path: profile_dir.to_string(),
            source,
        })?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("yaml") {
            continue;
        }
        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                PipelineRegistryError::Config(format!(
                    "Profile directory contains a non-UTF-8 filename: {}",
                    path.display()
                ))
            })?;

        let profile =
            crate::pipeline::config::ProcessingProfile::from_file(&path).map_err(|error_msg| {
                PipelineRegistryError::Config(format!(
                    "Failed to parse profile YAML '{filename}' during legacy scan: {error_msg}"
                ))
            })?;

        let is_default = filename == "default.yaml";
        document_types.push(DocumentTypeEntry {
            name: profile.name.clone(),
            display_name: if profile.display_name.is_empty() {
                profile.name.clone()
            } else {
                profile.display_name.clone()
            },
            profile_file: filename,
            description: profile.description.clone(),
            is_default,
            sort_order: 0,
        });
    }

    Ok(document_types)
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
