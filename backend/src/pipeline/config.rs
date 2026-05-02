//! Pipeline configuration types — profile loading and config resolution.
//!
//! Processing profiles are YAML files on mounted storage (not database rows).
//! This module loads them, validates references, and resolves the three-level
//! configuration hierarchy: system defaults → profile → per-document overrides.
//!
//! Design: DOC_PROCESSING_CONFIG_DESIGN_v2.md Sections 3.1, 3.2.2, 3.7.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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

    /// SHA-256 of the YAML body as it was on disk at load time.
    ///
    /// Lowercase hex, populated by [`ProcessingProfile::from_file`] /
    /// [`ProcessingProfile::from_yaml_str`] *after* reading the body and
    /// *before* deserialization. Two runs against the same `name` but
    /// different YAML content are distinguishable in the audit log via
    /// this hash (see [`ResolvedConfig::profile_hash`] and
    /// AUDIT_PIPELINE_CONFIG_GAPS.md Gap 4).
    ///
    /// `#[serde(default)]` so YAML on disk that doesn't carry the field
    /// (every shipped profile, since this is a runtime-derived value)
    /// still parses cleanly. The default is the empty string; the loader
    /// then *overwrites* it with the real hash. A reader who somehow
    /// receives a serialized `ProcessingProfile` without going through
    /// the loader gets the empty string — operationally equivalent to
    /// "unknown source" and an obvious sentinel in any audit query.
    #[serde(default)]
    pub profile_hash: String,
}

/// SHA-256 hex digest of a UTF-8 string, lowercase.
///
/// Local to `config.rs` so the profile-loading code doesn't have to pull
/// in the `pub(crate)` helper from `pipeline::steps::llm_extract`. Keeps
/// the dependency direction one-way: steps depend on config, config does
/// not depend on steps. Both helpers compute the same value (verified by
/// the round-trip test below).
fn sha2_hex_yaml(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
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
    ///
    /// The SHA-256 of the **raw YAML body** (before deserialization) is
    /// computed and stored on the returned struct's `profile_hash` field.
    /// Audit reproducibility: two runs against the same profile filename
    /// but a content-edited YAML are distinguishable from the audit log
    /// alone — Gap 4 in AUDIT_PIPELINE_CONFIG_GAPS.md.
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read profile '{}': {e}", path.display()))?;
        Self::from_yaml_str(&content)
            .map_err(|e| format!("Failed to parse profile '{}': {e}", path.display()))
    }

    /// Parse a profile from an in-memory YAML string and compute its hash.
    ///
    /// Same semantics as [`from_file`] for the hash — the input string
    /// IS the source of truth, so its SHA-256 is the profile's
    /// fingerprint. Useful for tests that don't want to touch the
    /// filesystem and for any future call site that materialises a
    /// profile from a non-disk source (e.g. an admin-API PUT body).
    ///
    /// Returns a `String` error so the caller can format the path /
    /// origin into a useful message; this function only knows about the
    /// YAML body.
    pub fn from_yaml_str(yaml: &str) -> Result<Self, String> {
        let mut profile: Self = serde_yaml::from_str(yaml)
            .map_err(|e| format!("invalid YAML: {e}"))?;
        profile.profile_hash = sha2_hex_yaml(yaml);
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

// ── Cross-document context record ───────────────────────────────

/// One cross-document entity that contributed to a Pass-2 prompt.
///
/// Pass-2 prompts mix the document's own Pass-1 entities with entities
/// from previously PUBLISHED documents (the "cross-doc context"). This
/// record captures the minimum identity for one such cross-doc entity so
/// the audit trail can prove exactly which other-document entities
/// informed a Pass-2 run. Without it, a replay against a now-larger pool
/// of PUBLISHED documents would silently use a different context, and an
/// auditor would have no way to detect the divergence (Gap 3 in
/// AUDIT_PIPELINE_CONFIG_GAPS.md).
///
/// Three fields chosen to be the smallest set that lets a reader uniquely
/// re-locate the source entity:
///
/// * `document_id` — `documents.id` of the source document.
/// * `prefixed_id` — the `ctx:`-prefixed id used in the Pass-2 prompt's
///   `entities_json` (so an auditor can string-match against the
///   `assembled_prompt` column).
/// * `item_id` — `extraction_items.id` for direct DB join.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrossDocContextRecord {
    pub document_id: String,
    pub prefixed_id: String,
    pub item_id: i32,
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
    /// SHA-256 of the YAML body the resolver saw at load time. Copied
    /// from [`ProcessingProfile::profile_hash`] in `resolve_config`.
    /// Two runs against the same `profile_name` but a content-edited
    /// YAML are distinguishable from the database via this hash —
    /// AUDIT_PIPELINE_CONFIG_GAPS.md Gap 4.
    pub profile_hash: String,
    /// 1 or 2 — which extraction pass this snapshot describes. Set at
    /// snapshot-write time, not at resolve time. A reader can
    /// `SELECT processing_config->>'effective_pass'` to disambiguate
    /// without joining on the row's `pass_number` column. Default
    /// `1` from the resolver; Pass-2 overwrites in
    /// `write_processing_config_snapshot`.
    pub effective_pass: u8,
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
    /// SHA-256 of the resolved global-rules file content at runtime.
    ///
    /// Populated by the extraction step *after* loading the file (the
    /// resolver returns `None` here; the runtime overwrites it before
    /// serialising the snapshot, mirroring `template_hash` and
    /// `system_prompt_hash`).
    ///
    /// Three states matter for audit:
    ///
    /// * `None` — `global_rules_file` was `None` (no rules configured).
    /// * `Some(sha256(""))` — the file existed but was empty (deliberately
    ///   neutralised or mid-edit). An empty-rules run is operationally
    ///   different from a no-rules run; keeping the hash distinguishes
    ///   them in the JSONB audit trail.
    /// * `Some(sha256(content))` — normal case.
    pub global_rules_hash: Option<String>,
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

    /// Pass-2 only. List of `(document_id, prefixed_id, item_id)`
    /// triples for every cross-document entity that ended up in the
    /// Pass-2 prompt's `entities_json`. Empty on Pass-1 rows. The same
    /// data is also written to `extraction_runs.prior_context` as a
    /// JSON string for full reproducibility — duplicated deliberately
    /// so a JSONB-only audit query does not need to parse the TEXT
    /// column.
    ///
    /// `#[serde(default)]` so old Pass-1 snapshots that predate this
    /// field deserialize cleanly (default = empty vec, which is also
    /// the correct semantic for any non-Pass-2 row).
    #[serde(default)]
    pub pass2_cross_doc_entities: Vec<CrossDocContextRecord>,

    /// Pass-2 only. Sorted unique list of `document_id`s that
    /// contributed at least one entity to the Pass-2 prompt. Empty on
    /// Pass-1 rows. Useful for cheap "which prior runs informed this
    /// Pass-2?" queries without parsing the full entity list above.
    #[serde(default)]
    pub pass2_source_document_ids: Vec<String>,
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

/// Merge a per-document map override onto the profile's map, with
/// **per-key** audit-trail granularity.
///
/// The two map-shaped overrides on `pipeline_config` (`chunking_config`,
/// `context_config`) share this exact merge contract:
///
/// 1. **No override (`None`)** → return the profile's map verbatim.
///    Nothing pushed to `applied`.
/// 2. **Override (`Some(map)`)** → return `profile_map.extend(override_map)`
///    so override KEYS replace profile keys at the *key level* (an
///    operator overriding `units_per_chunk` doesn't have to re-state
///    every other key in the map).
///
/// Audit trail granularity: for each key in the override map whose value
/// **differs** from the profile's value (or that the profile doesn't
/// have at all), push `"{audit_label}.{key}"` to `applied`. Keys whose
/// override value matches the profile's value are NOT pushed — they
/// are no-ops at the resolved-config level, and the audit log should
/// reflect what *actually changed*. This is finer than the previous
/// behaviour, which pushed the bare label `"chunking_config"` whenever
/// any override was present (ambiguous about which sub-key changed).
///
/// ## Rust Learning: `&serde_json::Value`'s `==` is structural
///
/// Two `Value`s compare equal under `==` when their JSON shapes are
/// identical (recursively). That is the right notion of "different
/// from the profile" for audit purposes — `5` overridden with `5` is
/// not really a change, but `5` overridden with `5.0` IS a change
/// because the JSON types differ (and a downstream reader could behave
/// differently). Number-type-coercion is the JSON spec's problem, not
/// ours.
fn merge_map_override(
    profile_map: &HashMap<String, serde_json::Value>,
    override_map: Option<&HashMap<String, serde_json::Value>>,
    audit_label: &str,
    applied: &mut Vec<String>,
) -> HashMap<String, serde_json::Value> {
    match override_map {
        None => profile_map.clone(),
        Some(over) => {
            for (k, v) in over.iter() {
                let differs = match profile_map.get(k) {
                    None => true,            // key absent in profile — override adds it
                    Some(profile_v) => profile_v != v,
                };
                if differs {
                    applied.push(format!("{audit_label}.{k}"));
                }
            }
            let mut merged = profile_map.clone();
            merged.extend(over.iter().map(|(k, v)| (k.clone(), v.clone())));
            merged
        }
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
    let chunking_config = merge_map_override(
        &profile.chunking_config,
        overrides.chunking_config.as_ref(),
        "chunking_config",
        &mut applied,
    );

    let context_config = merge_map_override(
        &profile.context_config,
        overrides.context_config.as_ref(),
        "context_config",
        &mut applied,
    );

    ResolvedConfig {
        profile_name: overrides.profile_name.clone()
            .unwrap_or_else(|| profile.name.clone()),
        // The profile YAML body's hash flows straight through. Per-document
        // overrides cannot change it — the YAML body is the YAML body.
        profile_hash: profile.profile_hash.clone(),
        // Default to Pass-1. The Pass-2 step's
        // `write_processing_config_snapshot` overwrites to `2` at snapshot
        // time. Storing the value in the snapshot (rather than relying on
        // the row's `pass_number` column) makes JSONB self-describing.
        effective_pass: 1,
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
        // Filled at runtime by the extraction step after loading the rules
        // file (parallel to `template_hash` and `system_prompt_hash`).
        global_rules_hash: None,
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
        // Empty by default. Pass-2 fills these in via SnapshotRuntimeFields
        // before the snapshot is written; Pass-1 leaves them empty.
        pass2_cross_doc_entities: Vec::new(),
        pass2_source_document_ids: Vec::new(),
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
            profile_hash: String::new(),
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
            profile_hash: String::new(),
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
            profile_hash: String::new(),
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
            profile_hash: String::new(),
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
            profile_hash: String::new(),
            effective_pass: 1,
            model: "claude-sonnet-4-6".into(),
            pass2_model: None,
            template_file: "pass1_complaint.md".into(),
            template_hash: Some("abc123".into()),
            pass2_template_file: None,
            system_prompt_file: None,
            system_prompt_hash: None,
            global_rules_file: None,
            global_rules_hash: None,
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
            pass2_cross_doc_entities: Vec::new(),
            pass2_source_document_ids: Vec::new(),
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
            profile_hash: String::new(),
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
            profile_hash: String::new(),
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
        // Tracked as overridden — per-key granularity (Gap 1's audit
        // refinement). The bare `"chunking_config"` whole-map label is
        // gone; the audit now records exactly which sub-key changed so
        // a downstream reader of `processing_config.overrides_applied`
        // sees `["chunking_config.units_per_chunk"]` instead of an
        // ambiguous `["chunking_config"]`.
        assert!(
            resolved
                .overrides_applied
                .contains(&"chunking_config.units_per_chunk".to_string()),
            "expected per-key entry; got: {:?}",
            resolved.overrides_applied
        );
        assert!(
            !resolved
                .overrides_applied
                .contains(&"chunking_config".to_string()),
            "the bare whole-map label must NOT appear; got: {:?}",
            resolved.overrides_applied
        );
        // mode and strategy were not in the override map → no entry for them.
        assert!(!resolved
            .overrides_applied
            .contains(&"chunking_config.mode".to_string()));
        assert!(!resolved
            .overrides_applied
            .contains(&"chunking_config.strategy".to_string()));
    }

    #[test]
    fn resolved_config_with_chunking_config_serializes() {
        let mut chunking_config = HashMap::new();
        chunking_config.insert("mode".to_string(), serde_json::json!("structured"));
        chunking_config.insert("strategy".to_string(), serde_json::json!("qa_pair"));

        let config = ResolvedConfig {
            profile_name: "discovery".into(),
            profile_hash: String::new(),
            effective_pass: 1,
            model: "claude-sonnet-4-6".into(),
            pass2_model: None,
            template_file: "disc.md".into(),
            template_hash: None,
            pass2_template_file: None,
            system_prompt_file: None,
            system_prompt_hash: None,
            global_rules_file: None,
            global_rules_hash: None,
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
            pass2_cross_doc_entities: Vec::new(),
            pass2_source_document_ids: Vec::new(),
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

    // ── profile_hash (Gap 4) ─────────────────────────────────────────
    //
    // The hash is computed over the YAML body the loader saw, not over
    // the deserialised struct. That is the property an audit needs:
    // editing a comment, a whitespace, or a value all change the hash.

    #[test]
    fn from_yaml_str_populates_a_non_empty_profile_hash() {
        let yaml = r#"
name: complaint
display_name: Complaint
schema_file: complaint_v4.yaml
template_file: pass1_complaint_v4.md
extraction_model: claude-sonnet-4-6
"#;
        let p = ProcessingProfile::from_yaml_str(yaml).unwrap();
        assert!(!p.profile_hash.is_empty(), "hash must be populated");
        // 64 hex chars from sha256 → matches the contract.
        assert_eq!(p.profile_hash.len(), 64);
        assert!(
            p.profile_hash.chars().all(|c| c.is_ascii_hexdigit()),
            "hash must be lowercase hex digits"
        );
    }

    #[test]
    fn identical_yaml_bodies_produce_identical_profile_hashes() {
        let yaml = r#"
name: complaint
display_name: Complaint
schema_file: complaint_v4.yaml
template_file: pass1_complaint_v4.md
extraction_model: claude-sonnet-4-6
"#;
        let a = ProcessingProfile::from_yaml_str(yaml).unwrap();
        let b = ProcessingProfile::from_yaml_str(yaml).unwrap();
        assert_eq!(a.profile_hash, b.profile_hash);
    }

    #[test]
    fn editing_yaml_changes_the_profile_hash() {
        // Two YAMLs that produce the SAME deserialised struct values but
        // differ in their raw bytes (here: a comment-only edit). The
        // hash must differ — the audit needs to detect any change to
        // the body, including ones that don't materially affect runtime
        // behaviour. This is the "operationally meaningless" edit the
        // hash protects against being silently lost in the audit log.
        let original = r#"# original comment
name: complaint
display_name: Complaint
schema_file: complaint_v4.yaml
template_file: pass1_complaint_v4.md
extraction_model: claude-sonnet-4-6
"#;
        let edited = r#"# edited comment — different bytes, same struct values
name: complaint
display_name: Complaint
schema_file: complaint_v4.yaml
template_file: pass1_complaint_v4.md
extraction_model: claude-sonnet-4-6
"#;
        let a = ProcessingProfile::from_yaml_str(original).unwrap();
        let b = ProcessingProfile::from_yaml_str(edited).unwrap();
        // Same struct values (only the comment changed)…
        assert_eq!(a.name, b.name);
        assert_eq!(a.extraction_model, b.extraction_model);
        // …but the hashes must diverge so the audit can detect the edit.
        assert_ne!(
            a.profile_hash, b.profile_hash,
            "a comment-only edit to the YAML body must change the hash"
        );
    }

    #[test]
    fn resolve_config_copies_profile_hash_into_resolved() {
        let yaml = r#"
name: complaint
display_name: Complaint
schema_file: complaint_v4.yaml
template_file: pass1_complaint_v4.md
extraction_model: claude-sonnet-4-6
"#;
        let p = ProcessingProfile::from_yaml_str(yaml).unwrap();
        let r = resolve_config(&p, &PipelineConfigOverrides::default());
        assert_eq!(r.profile_hash, p.profile_hash);
        assert_eq!(r.effective_pass, 1, "resolver default is pass 1");
        assert!(r.pass2_cross_doc_entities.is_empty());
        assert!(r.pass2_source_document_ids.is_empty());
    }

    // ── synthesis_model deletion (Gap 10) ────────────────────────────

    #[test]
    fn yaml_with_legacy_synthesis_model_still_parses() {
        // Backward compat: an operator running an older YAML on disk
        // (one that still has `synthesis_model:`) must keep working.
        // The line is silently ignored because ProcessingProfile does
        // NOT use `#[serde(deny_unknown_fields)]`.
        let yaml = r#"
name: complaint
display_name: Complaint
schema_file: complaint_v4.yaml
template_file: pass1_complaint_v4.md
extraction_model: claude-sonnet-4-6
synthesis_model: claude-opus-4-7
"#;
        let p = ProcessingProfile::from_yaml_str(yaml)
            .expect("YAML with legacy synthesis_model must still parse");
        assert_eq!(p.name, "complaint");
        // synthesis_model is gone from the struct — there's no field to
        // assert. The success of from_yaml_str is the test.
        let _ = p;
    }

    #[test]
    fn no_profile_yaml_on_disk_carries_synthesis_model() {
        // Read every YAML in backend/profiles/ and assert none of them
        // contains the literal substring "synthesis_model:". Catches a
        // future operator who copy-pastes an old profile and forgets
        // to drop the line; without the assertion, the line would parse
        // silently (forward-compat by design) but we want disk and code
        // to stay aligned.
        let entries = std::fs::read_dir(
            // Cargo runs tests from the package root (backend/).
            "profiles",
        )
        .expect("backend/profiles/ must exist");
        let mut yaml_count = 0;
        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("yaml") {
                continue;
            }
            yaml_count += 1;
            let body = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            assert!(
                !body.contains("synthesis_model:"),
                "{} still carries `synthesis_model:` — drop the line",
                path.display()
            );
        }
        assert!(
            yaml_count >= 7,
            "expected at least 7 profile YAMLs, found {yaml_count}"
        );
    }

    // ── chunking_config / context_config merge semantics (Gap 1) ────
    //
    // The merge contract — `extend()` over the profile's map with
    // override keys winning — is exercised by
    // `resolve_config_merges_chunking_config_overrides` (above).
    // The tests below pin down the boundary cases the spec calls out.

    /// Helper: a minimal profile with a known chunking_config so the
    /// merge tests don't have to repeat 20 lines of fixture each time.
    fn profile_with_chunking_config(
        chunking_config: HashMap<String, serde_json::Value>,
    ) -> ProcessingProfile {
        ProcessingProfile {
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
            chunking_mode: "full".into(),
            chunk_size: None,
            chunk_overlap: None,
            chunking_config,
            context_config: HashMap::new(),
            max_tokens: 32000,
            temperature: 0.0,
            auto_approve_grounded: true,
            run_pass2: false,
            is_default: false,
            profile_hash: String::new(),
        }
    }

    // Test #5: profile {} + override {strategy: qa_pair} → {strategy: qa_pair}
    #[test]
    fn merge_empty_profile_with_override_yields_override_only() {
        let mut over = HashMap::new();
        over.insert("strategy".to_string(), serde_json::json!("qa_pair"));
        let profile = profile_with_chunking_config(HashMap::new());
        let overrides = PipelineConfigOverrides {
            chunking_config: Some(over),
            ..Default::default()
        };
        let resolved = resolve_config(&profile, &overrides);
        assert_eq!(resolved.chunking_config.len(), 1);
        assert_eq!(
            resolved.chunking_config.get("strategy").and_then(|v| v.as_str()),
            Some("qa_pair")
        );
        // Per-key audit: strategy was added → recorded.
        assert!(resolved
            .overrides_applied
            .contains(&"chunking_config.strategy".to_string()));
    }

    // Test #6: profile {fields} + override None → resolved == profile
    #[test]
    fn merge_with_no_override_yields_profile_map_verbatim() {
        let mut p = HashMap::new();
        p.insert("strategy".to_string(), serde_json::json!("section_heading"));
        p.insert("units_per_chunk".to_string(), serde_json::json!(5));
        let profile = profile_with_chunking_config(p.clone());
        let resolved = resolve_config(&profile, &PipelineConfigOverrides::default());
        assert_eq!(resolved.chunking_config, p);
        // No override → no `chunking_config.*` audit entries.
        assert!(!resolved
            .overrides_applied
            .iter()
            .any(|s| s.starts_with("chunking_config")));
    }

    // Test #7: profile {fields} + override Some({}) → resolved == profile,
    // and the empty-but-present override does NOT count as a change.
    #[test]
    fn merge_with_empty_override_map_is_a_noop() {
        let mut p = HashMap::new();
        p.insert("strategy".to_string(), serde_json::json!("section_heading"));
        p.insert("units_per_chunk".to_string(), serde_json::json!(5));
        let profile = profile_with_chunking_config(p.clone());
        let overrides = PipelineConfigOverrides {
            chunking_config: Some(HashMap::new()),
            ..Default::default()
        };
        let resolved = resolve_config(&profile, &overrides);
        assert_eq!(
            resolved.chunking_config, p,
            "empty override map adds nothing to the profile's map"
        );
        // Empty override has no keys → no per-key entries.
        assert!(!resolved
            .overrides_applied
            .iter()
            .any(|s| s.starts_with("chunking_config")));
    }

    // Test #8: same shape for context_config (sanity-checks the helper
    // is parametric in the audit_label, not chunking_config-specific).
    #[test]
    fn merge_works_for_context_config_with_distinct_audit_label() {
        let mut p = HashMap::new();
        p.insert("traversal_depth".to_string(), serde_json::json!(2));
        let profile = ProcessingProfile {
            context_config: p,
            ..profile_with_chunking_config(HashMap::new())
        };
        let mut over = HashMap::new();
        over.insert("traversal_depth".to_string(), serde_json::json!(5));
        let overrides = PipelineConfigOverrides {
            context_config: Some(over),
            ..Default::default()
        };
        let resolved = resolve_config(&profile, &overrides);
        assert_eq!(
            resolved.context_config.get("traversal_depth").and_then(|v| v.as_i64()),
            Some(5)
        );
        // Audit uses the `context_config.` prefix, not `chunking_config.`.
        assert!(resolved
            .overrides_applied
            .contains(&"context_config.traversal_depth".to_string()));
        assert!(!resolved
            .overrides_applied
            .iter()
            .any(|s| s.starts_with("chunking_config")));
    }

    // Test #11: override that matches the profile's value is a no-op
    // for the audit trail (only differing keys count).
    #[test]
    fn override_matching_profile_value_is_not_recorded() {
        let mut p = HashMap::new();
        p.insert("units_per_chunk".to_string(), serde_json::json!(5));
        let profile = profile_with_chunking_config(p);
        let mut over = HashMap::new();
        // Same value as the profile — operationally a no-op.
        over.insert("units_per_chunk".to_string(), serde_json::json!(5));
        let overrides = PipelineConfigOverrides {
            chunking_config: Some(over),
            ..Default::default()
        };
        let resolved = resolve_config(&profile, &overrides);
        assert!(
            !resolved
                .overrides_applied
                .iter()
                .any(|s| s.starts_with("chunking_config")),
            "matching-value override must not be recorded; got: {:?}",
            resolved.overrides_applied
        );
    }
}
