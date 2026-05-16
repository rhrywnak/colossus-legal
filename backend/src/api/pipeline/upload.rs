//! POST /api/admin/pipeline/documents — Upload a PDF with metadata.
//!
//! ## Rust Learning: Two-phase multipart parsing
//!
//! Unlike a JSON body which is parsed all at once, multipart/form-data arrives
//! as a stream of fields. We loop through all fields, collecting metadata strings
//! and the file bytes separately, then validate that all required fields were
//! present after the loop completes.

use std::path::Path;

use axum::{
    extract::{Multipart, State},
    Json,
};
use sha2::{Digest, Sha256};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::pipeline::config::{
    PipelineConfigOverrides, ProcessingProfile, ProcessingProfileLoadError,
};
use crate::pipeline::registry::PipelineRegistry;
use crate::pipeline::validation::validate_profile;
use crate::repositories::audit_repository::log_admin_action;
use crate::repositories::pipeline_repository::{self, steps, PipelineConfigInput};
use crate::state::AppState;

use super::{field_text, require_field, UploadDocumentResponse, MAX_FILE_SIZE};

/// Schema filename used when a document's processing profile cannot be loaded.
///
/// Profile YAMLs are the source of truth for `schema_file`. This constant is
/// only consulted on a profile-load failure (missing file, parse error) so
/// the upload doesn't 500 — the operator still sees a `tracing::warn!` and
/// can fix the profile.
pub const FALLBACK_SCHEMA_FILE: &str = "complaint_v4.yaml";

/// Resolve and load the processing profile for an upload via the registry.
///
/// Two paths:
///
/// 1. **Explicit version override.** When `profile_version` is supplied,
///    treats the value as a per-upload override and loads
///    `<document_type>_<version>.yaml` directly. Bypasses the registry's
///    document-type mapping — the operator has explicitly asked for that
///    specific file, the registry's "is_default for this type" doesn't
///    apply. Mirrors the pre-registry behaviour of the same UI knob.
///
/// 2. **Registry lookup.** Looks up `document_type` in the registry. If
///    no entry matches, falls back to the registry's default entry.
///    Loads the profile YAML named by the entry's `profile_file`.
///
/// Errors when (a) the explicit-version YAML cannot be loaded, (b) the
/// registry contains neither a matching entry nor a default (a degraded
/// registry — validate() would have caught this at startup, so this is
/// belt-and-suspenders), or (c) the registry-named profile YAML cannot
/// be loaded.
///
/// ## Why a free function instead of a method on PipelineRegistry?
///
/// The function takes `&PipelineRegistry` rather than living on the
/// registry so its logic is unit-testable without spinning up an `Arc`
/// or `AppState`. The upload handler is its only caller today; if more
/// emerge, inlining as a registry method is a trivial refactor.
///
/// ## Rust Learning: returning `AppError` from a non-handler helper
///
/// The function returns the same `AppError` the handler `?`-propagates
/// later, which keeps error mapping in one place: profile-load failures
/// surface as 500 via the existing `From<ProcessingProfileLoadError>`
/// impl in `error.rs`; "no registry entry" surfaces as a clean
/// `AppError::Internal` message.
pub fn resolve_upload_profile(
    registry: &PipelineRegistry,
    document_type: &str,
    profile_version: Option<&str>,
) -> Result<ProcessingProfile, AppError> {
    if let Some(v) = profile_version.filter(|s| !s.is_empty()) {
        let name = format!("{document_type}_{v}");
        tracing::info!(
            document_type,
            profile_version = v,
            resolved_profile = %name,
            "Upload specified profile_version — using explicit override"
        );
        return ProcessingProfile::load(registry.profile_dir(), &name).map_err(AppError::from);
    }

    let entry = registry
        .document_type(document_type)
        .or_else(|| registry.default_document_type())
        .ok_or_else(|| AppError::Internal {
            message: format!(
                "No profile configured for document type '{document_type}' and no \
                 default profile exists in the pipeline registry. Check \
                 pipeline_registry.yaml."
            ),
        })?;

    tracing::info!(
        document_type,
        registry_entry = %entry.name,
        profile_file = %entry.profile_file,
        "Upload resolved profile via registry"
    );

    let profile_path = registry.profile_path(&entry.profile_file);
    ProcessingProfile::from_file(Path::new(&profile_path)).map_err(|e| AppError::Internal {
        message: format!(
            "Failed to load profile '{}' (registry entry '{}'): {e}",
            entry.profile_file, entry.name
        ),
    })
}

/// Resolve the schema filename for a document type via the registry.
///
/// Used by extract_text (after auto-detection) to keep
/// `pipeline_config.schema_file` in sync with `documents.document_type`
/// once Pass-0 has determined the real type. Looks up the document type
/// in the registry, falls back to the registry's default entry, and
/// loads the resulting profile to read its `schema_file`.
///
/// ## Error policy — fall back on missing, propagate on corruption
///
/// - **No registry entry AND no default** → log a `tracing::warn!` and
///   return [`FALLBACK_SCHEMA_FILE`]. The registry guarantees a default
///   exists at startup (`validate()` rejects registries without one),
///   so reaching this arm means the registry was somehow degraded
///   post-startup — falling back keeps the extraction running and
///   surfaces the issue in the log.
/// - **Profile YAML missing or unreadable** → log a `tracing::warn!`
///   and return [`FALLBACK_SCHEMA_FILE`]. Preserves the silent-fallback
///   contract `extract_text` had before the registry; documents
///   uploaded before the profile YAMLs were renamed/moved still work.
/// - **Profile YAML parses but is malformed** → propagate as
///   `ProcessingProfileLoadError::ParseError`. A malformed YAML is a
///   configuration error the operator must fix, not silently mask.
pub fn schema_file_for_document_type(
    registry: &PipelineRegistry,
    document_type: &str,
) -> Result<String, ProcessingProfileLoadError> {
    let entry = match registry
        .document_type(document_type)
        .or_else(|| registry.default_document_type())
    {
        Some(e) => e,
        None => {
            tracing::warn!(
                document_type,
                fallback = FALLBACK_SCHEMA_FILE,
                "No registry entry and no default — using fallback schema"
            );
            return Ok(FALLBACK_SCHEMA_FILE.to_string());
        }
    };

    let profile_path = registry.profile_path(&entry.profile_file);
    match ProcessingProfile::from_file(Path::new(&profile_path)) {
        Ok(p) => Ok(p.schema_file),
        Err(e) => {
            tracing::warn!(
                document_type,
                profile_file = %entry.profile_file,
                error = %e,
                fallback = FALLBACK_SCHEMA_FILE,
                "Failed to load registry profile — using fallback schema"
            );
            Ok(FALLBACK_SCHEMA_FILE.to_string())
        }
    }
}

/// Build the initial per-document override map from a matched profile.
///
/// Mirrors the field-by-field shape upload writes to `pipeline_config`
/// after the base row is inserted. Every column on the override path is
/// listed here so a reader can confirm the profile-to-row mapping in one
/// place — the Configuration Panel reads exactly these columns when it
/// renders the post-upload state.
///
/// `chunking_config` and `context_config` are deliberately left as
/// `None`: seeding them with the profile's current values would freeze
/// a snapshot at upload time, which would *prevent* future profile YAML
/// edits from flowing through to this document. Leaving the override
/// NULL means the resolver picks the profile's live map at every
/// resolve. Operators who want a per-document tweak set them through
/// PATCH /config.
///
/// Factored out of `upload_document` so the column coverage is
/// unit-testable without spinning up an Axum runtime.
pub fn overrides_for_upload(profile: &ProcessingProfile) -> PipelineConfigOverrides {
    PipelineConfigOverrides {
        profile_name: Some(profile.name.clone()),
        extraction_model: Some(profile.extraction_model.clone()),
        pass2_extraction_model: profile.pass2_extraction_model.clone(),
        pass2_template_file: profile.pass2_template_file.clone(),
        template_file: Some(profile.template_file.clone()),
        system_prompt_file: profile.system_prompt_file.clone(),
        chunking_mode: Some(profile.chunking_mode.clone()),
        chunk_size: profile.chunk_size,
        chunk_overlap: profile.chunk_overlap,
        max_tokens: Some(profile.max_tokens),
        temperature: Some(profile.temperature),
        run_pass2: Some(profile.run_pass2),
        // Bug #8 fix — per-document overridable. Seeded from profile so
        // the Configuration Panel shows the profile's defaults instead
        // of compiled fallbacks; operators change via PATCH /config.
        auto_approve_grounded: Some(profile.auto_approve_grounded),
        global_rules_file: profile.global_rules_file.clone(),
        chunking_config: None,
        context_config: None,
    }
}

/// POST /api/admin/pipeline/documents
///
/// Accepts a multipart form with a PDF file and metadata fields.
/// Saves the PDF to disk, computes SHA-256, and creates document +
/// pipeline_config records in the pipeline database.
pub async fn upload_document(
    user: AuthUser,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<(axum::http::StatusCode, Json<UploadDocumentResponse>), AppError> {
    require_admin(&user)?;
    let start = std::time::Instant::now();
    tracing::info!(user = %user.username, "POST /api/admin/pipeline/documents");

    // Collect all multipart fields into local variables.
    // Fields can arrive in any order, so we parse them all first.
    //
    // Bug #6 fix: the multipart `pass1_model` and `pass2_model` fields
    // are gone. They were upload-time operator overrides that wrote to
    // dead pipeline_config columns (`pass1_model`/`pass2_model`) the
    // resolver did not read. Per-upload model selection now flows via
    // the matched profile; per-document tweaks happen post-upload via
    // PATCH /config (which writes `extraction_model` — the column the
    // resolver actually reads).
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut doc_id: Option<String> = None;
    let mut title: Option<String> = None;
    let mut document_type: Option<String> = None;
    let mut admin_instructions: Option<String> = None;
    let mut profile_version: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest {
            message: format!("Failed to read multipart field: {e}"),
            details: serde_json::json!({}),
        })?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                file_name = field.file_name().map(|s| s.to_string());
                let data = field.bytes().await.map_err(|e| AppError::BadRequest {
                    message: format!("Failed to read file data: {e}"),
                    details: serde_json::json!({}),
                })?;
                file_data = Some(data.to_vec());
            }
            "id" => doc_id = Some(field_text(&name, field).await?),
            "title" => title = Some(field_text(&name, field).await?),
            "document_type" => document_type = Some(field_text(&name, field).await?),
            "admin_instructions" => admin_instructions = Some(field_text(&name, field).await?),
            "profile_version" => profile_version = Some(field_text(&name, field).await?),
            _ => { /* ignore unknown fields */ }
        }
    }

    // Validate required fields
    let doc_id = require_field("id", doc_id)?;
    let title = require_field("title", title)?;
    let document_type = document_type.unwrap_or_else(|| "auto".to_string());

    // Resolve and load the processing profile via the registry. The
    // explicit-version override path runs `ProcessingProfile::load`
    // against `<document_type>_<version>.yaml`; the default path
    // consults `state.registry` to map the upload's `document_type`
    // to a profile_file, falling back to the registry's default entry
    // when no document-type-specific entry exists.
    //
    // The loaded profile is reused below to pre-populate per-document
    // override columns, avoiding a duplicate disk read.
    let profile =
        resolve_upload_profile(&state.registry, &document_type, profile_version.as_deref())?;

    // Bug #1/#4 fix: validate the profile's cross-references before
    // writing anything to disk or DB. An invalid `extraction_model`
    // (model ID not in `llm_models`) used to slip through here and
    // surface as a `ModelNotFound` at extract time — after the PDF was
    // already on disk and the row inserted. Validation up front turns
    // the same failure into a clean 400 before any side effect.
    validate_profile(&state.pipeline_pool, &state.registry, &profile).await?;

    // Schema file — derived exclusively from the processing profile.
    // The profile load above succeeded (we propagated on error), so we
    // can take the schema_file unconditionally; the previous "no profile
    // loaded — fallback" branch is dead code after defect #2.1's fix.
    let schema_file = profile.schema_file.clone();
    let file_data = file_data.ok_or_else(|| AppError::BadRequest {
        message: "No 'file' field in multipart upload".to_string(),
        details: serde_json::json!({}),
    })?;

    // Complaint-first enforcement: the first document must be a complaint.
    // The complaint establishes parties, claims, and legal context that all
    // other documents reference.
    let has_complaint =
        pipeline_repository::documents::has_document_of_type(&state.pipeline_pool, "complaint")
            .await
            .map_err(|e| AppError::Internal {
                message: format!("DB error: {e}"),
            })?;

    if !has_complaint && document_type != "complaint" && document_type != "auto" {
        return Err(AppError::BadRequest {
            message: "A Complaint document must be uploaded first. The Complaint establishes the parties, claims, and legal context that all other documents reference.".to_string(),
            details: serde_json::json!({ "document_type": document_type }),
        });
    }

    // Validate file
    if file_data.len() > MAX_FILE_SIZE {
        return Err(AppError::BadRequest {
            message: format!(
                "File too large: {} bytes (max {MAX_FILE_SIZE})",
                file_data.len()
            ),
            details: serde_json::json!({ "size_bytes": file_data.len(), "max_bytes": MAX_FILE_SIZE }),
        });
    }

    let original_name = file_name.unwrap_or_else(|| format!("{doc_id}.pdf"));

    // Validate file extension — accept PDF, Word, and plain text.
    //
    // ## Rust Learning: `matches!` macro for multi-pattern boolean checks
    //
    // `matches!(value, pattern1 | pattern2 | pattern3)` is syntactic sugar
    // for a match expression that returns bool. Much cleaner than chaining
    // `|| lower.ends_with(".pdf") || lower.ends_with(".docx")` etc.
    // The `rsplit('.').next()` idiom extracts the extension by splitting
    // from the right — correctly handles names like "document.v2.pdf".
    let lower_name = original_name.to_lowercase();
    let valid_extension = matches!(
        lower_name.rsplit('.').next(),
        Some("pdf") | Some("docx") | Some("txt")
    );
    if !valid_extension {
        return Err(AppError::BadRequest {
            message: "Unsupported file type. Accepted formats: .pdf, .docx, .txt".to_string(),
            details: serde_json::json!({ "filename": original_name }),
        });
    }

    // Compute SHA-256 hash
    let file_hash = format!("{:x}", Sha256::digest(&file_data));

    // Preserve the original file extension in the stored filename so
    // that ExtractText can detect the format from the file on disk.
    //
    // ## Rust Learning: why not just store the format in the DB?
    //
    // We DO store it in the DB (original_format column). But the
    // filename extension serves as a secondary signal — and
    // `detect_format` reads magic bytes from the actual file, so
    // even a wrong extension won't break extraction. Belt and suspenders.
    let extension = lower_name.rsplit('.').next().unwrap_or("pdf");
    let storage_filename = format!("{doc_id}.{extension}");

    // Prevent path traversal
    if doc_id.contains("..") || doc_id.contains('/') || doc_id.contains('\\') {
        return Err(AppError::BadRequest {
            message: "Invalid document ID — must not contain path separators".to_string(),
            details: serde_json::json!({}),
        });
    }

    let dest_path = format!(
        "{}/{}",
        state.config.document_storage_path.trim_end_matches('/'),
        storage_filename
    );

    // Check for existing document in the database (409 if already exists)
    if pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .is_some()
    {
        return Err(AppError::Conflict {
            message: format!("Document '{doc_id}' already exists"),
            details: serde_json::json!({ "document_id": doc_id }),
        });
    }

    // Write file to disk
    tokio::fs::write(&dest_path, &file_data)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to write file to disk: {e}"),
        })?;

    // Detect the document's actual format from file content (magic bytes).
    // This is more reliable than trusting the file extension — a misnamed
    // file is detected correctly because mimetype-detector reads the file
    // header, not the name.
    //
    // ## Rust Learning: `colossus_pdf::detect_format`
    //
    // Returns a `DocumentFormat` enum (Pdf, Docx, PlainText). Internally
    // uses the `mimetype-detector` crate which reads the first few bytes
    // of the file (magic bytes / file signature). A .docx file is actually
    // a ZIP archive containing XML — the detector reads the ZIP header and
    // OOXML content types to identify it correctly.
    let (detected_mime, detected_format) =
        match colossus_pdf::detect_format(std::path::Path::new(&dest_path)) {
            Ok(format) => {
                // ## Rust Learning: matching on an enum to derive two related values
                //
                // We need both the MIME type string (for the DB column) and the
                // short format key (for ExtractText routing). A single match on
                // DocumentFormat gives us both without repeating the branching logic.
                let (mime, fmt) = match format {
                    colossus_pdf::DocumentFormat::Pdf => ("application/pdf", "pdf"),
                    colossus_pdf::DocumentFormat::Docx => (
                        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                        "docx",
                    ),
                    colossus_pdf::DocumentFormat::PlainText => ("text/plain", "txt"),
                };
                (mime.to_string(), fmt.to_string())
            }
            Err(e) => {
                // Fall back to extension-based detection if magic byte detection
                // fails (e.g., empty file, unrecognized header). Log the error
                // so operators can investigate, but don't fail the upload.
                tracing::warn!(
                    doc_id = %doc_id,
                    error = %e,
                    extension = %extension,
                    "Format detection failed — falling back to file extension"
                );
                let (mime, fmt) = match extension {
                    "docx" => (
                        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                        "docx",
                    ),
                    "txt" => ("text/plain", "txt"),
                    _ => ("application/pdf", "pdf"),
                };
                (mime.to_string(), fmt.to_string())
            }
        };

    tracing::info!(
        doc_id = %doc_id,
        mime_type = %detected_mime,
        format = %detected_format,
        "Detected document format"
    );

    // Insert document record with detected format metadata.
    pipeline_repository::insert_document(
        &state.pipeline_pool,
        &doc_id,
        &title,
        &storage_filename,
        &file_hash,
        &document_type,
        Some(&detected_mime),
        Some(&detected_format),
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Failed to insert document: {e}"),
    })?;

    // PDF content classification — only meaningful for PDF files.
    // DOCX and TXT files don't have scanned vs text-based pages.
    if detected_format == "pdf" {
        // Classify PDF content — this populates text/scanned/mixed, per-page
        // OCR flags, and character counts. Failures are logged but MUST NOT
        // block the upload: the default `content_type = 'unknown'` stays on
        // the row and ExtractText handles discovery the hard way at processing
        // time. Design: PDF_CONTENT_CLASSIFICATION_DESIGN_v2.md Phase B.
        let classification = match colossus_pdf::PdfTextExtractor::open(&dest_path) {
            Ok(mut extractor) => match extractor.classify() {
                Ok(c) => Some(c),
                Err(e) => {
                    tracing::warn!(
                        doc_id = %doc_id, error = %e,
                        "PDF classification failed, defaulting to unknown"
                    );
                    None
                }
            },
            Err(e) => {
                tracing::warn!(
                    doc_id = %doc_id, error = %e,
                    "Failed to open PDF for classification"
                );
                None
            }
        };

        if let Some(ref c) = classification {
            let pages_ocr: Vec<i32> = c.pages_needing_ocr.iter().map(|&p| p as i32).collect();
            if let Err(e) = sqlx::query(
                "UPDATE documents SET content_type = $1, page_count = $2, \
                 text_pages = $3, scanned_pages = $4, pages_needing_ocr = $5, \
                 total_chars = $6 WHERE id = $7",
            )
            .bind(c.content_type.as_str())
            .bind(c.page_count as i32)
            .bind(c.text_pages as i32)
            .bind(c.scanned_pages as i32)
            .bind(&pages_ocr)
            .bind(c.total_chars as i32)
            .bind(&doc_id)
            .execute(&state.pipeline_pool)
            .await
            {
                tracing::warn!(
                    doc_id = %doc_id, error = %e,
                    "Failed to store PDF classification — upload succeeds, content_type stays 'unknown'"
                );
            }
        }
    }

    // Insert pipeline config (base row — no model columns; the matched
    // profile's model & params are written immediately below via the
    // override path).
    let config_input = PipelineConfigInput {
        schema_file,
        admin_instructions,
        prior_context_doc_ids: None,
    };
    pipeline_repository::insert_pipeline_config(
        &state.pipeline_pool,
        &doc_id,
        &config_input,
        &user.username,
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Failed to insert pipeline config: {e}"),
    })?;

    // Problem 4: pre-populate the per-document override columns from the
    // matched processing profile. The Configuration Panel reads those
    // columns — without this, it shows compiled defaults until the user
    // manually overrides each dropdown. The profile load above
    // succeeded (we propagated on error), so this is unconditional now;
    // the previous `if let Some(...)` guard against the silent-fallback
    // None case is dead code after defect #2.1's fix.
    {
        let overrides = overrides_for_upload(&profile);
        if let Err(e) = pipeline_repository::patch_pipeline_config_overrides(
            &state.pipeline_pool,
            &doc_id,
            &overrides,
        )
        .await
        {
            tracing::warn!(
                doc_id = %doc_id, error = %e,
                "Failed to persist profile overrides at upload (non-fatal) — \
                 Configuration Panel may show compiled defaults until user edits"
            );
        }
    }

    tracing::info!(user = %user.username, doc_id = %doc_id, size = file_data.len(), "Pipeline document uploaded");

    // Record step (after document exists in DB so FK is satisfied)
    if let Ok(step_id) = steps::record_step_start(
        &state.pipeline_pool,
        &doc_id,
        "upload",
        &user.username,
        &serde_json::json!({"filename": original_name, "document_type": document_type}),
    )
    .await
    {
        if let Err(e) = steps::record_step_complete(
            &state.pipeline_pool,
            step_id,
            start.elapsed().as_secs_f64(),
            &serde_json::json!({
                "file_name": original_name,
                "file_size_bytes": file_data.len(),
                "file_hash": file_hash,
                "document_type": document_type,
            }),
        )
        .await
        {
            tracing::error!(
                document_id = %doc_id,
                step_id = step_id,
                error = %e,
                "Failed to record upload step completion — audit trail gap"
            );
        }
    }

    log_admin_action(
        &state.audit_repo,
        &user.username,
        "pipeline.document.upload",
        Some("document"),
        Some(&doc_id),
        Some(serde_json::json!({
            "title": title,
            "document_type": document_type,
            "size_bytes": file_data.len(),
            "file_hash": file_hash,
        })),
    )
    .await;

    // Fetch the inserted record to return it
    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::Internal {
            message: "Document was inserted but not found on re-read".to_string(),
        })?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(UploadDocumentResponse { document }),
    ))
}

#[cfg(test)]
mod tests {
    //! Upload-handler profile-resolution tests.
    //!
    //! `resolve_upload_profile` is the pure function the multipart-aware
    //! handler delegates to. Testing it directly avoids spinning up a
    //! database / Axum runtime — every assertion below builds a registry
    //! pointing at a tempdir of profile YAMLs and invokes the function
    //! exactly as the handler does.

    use super::*;

    use std::fs;
    use tempfile::TempDir;

    use crate::pipeline::registry::{DocumentTypeEntry, PipelineDirectories, PipelineRegistry};

    /// Build a minimal valid profile YAML body. Only the fields the
    /// `ProcessingProfile` deserializer demands (`name`, `display_name`,
    /// `schema_file`, `template_file`, `extraction_model`) are emitted —
    /// every other field uses the `#[serde(default)]` defaults. Keeps
    /// test YAMLs small and the assertions focused on resolution
    /// behaviour, not deserialization shape.
    fn make_profile_yaml(name: &str, schema_file: &str, template_file: &str) -> String {
        format!(
            "name: {name}\n\
             display_name: \"{name} display\"\n\
             schema_file: {schema_file}\n\
             template_file: {template_file}\n\
             extraction_model: claude-sonnet-4-6\n"
        )
    }

    /// Set up a profiles directory and a matching minimal registry.
    fn registry_with_profiles(
        profiles: &[(&str, &str, &str, &str)],
    ) -> (TempDir, PipelineRegistry) {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let dir_paths =
            ["profiles", "schemas", "templates", "system_prompts"].map(|name| root.join(name));
        for d in &dir_paths {
            fs::create_dir_all(d).unwrap();
        }
        let mut entries = Vec::new();
        for (registry_name, profile_file, schema_file, template_file) in profiles {
            let body = make_profile_yaml(registry_name, schema_file, template_file);
            fs::write(dir_paths[0].join(profile_file), body).unwrap();
            entries.push(DocumentTypeEntry {
                name: registry_name.to_string(),
                display_name: format!("{registry_name} display"),
                profile_file: profile_file.to_string(),
                description: String::new(),
                is_default: registry_name == &"default",
                sort_order: 0,
            });
        }
        let registry = PipelineRegistry {
            directories: PipelineDirectories {
                profiles: dir_paths[0].to_string_lossy().into_owned(),
                schemas: dir_paths[1].to_string_lossy().into_owned(),
                templates: dir_paths[2].to_string_lossy().into_owned(),
                system_prompts: dir_paths[3].to_string_lossy().into_owned(),
            },
            document_types: entries,
        };
        (tmp, registry)
    }

    /// An upload tagged `document_type=discovery_response` must load
    /// the discovery_response profile — not the default. This is the
    /// headline bug the registry fixes: the previous on-disk profile
    /// scanner depended on a `document_type:` field that the
    /// discovery_response YAML was missing, so uploads were silently
    /// mapping to the default profile.
    #[test]
    fn test_upload_maps_discovery_response_to_correct_profile() {
        let (_tmp, registry) = registry_with_profiles(&[
            (
                "discovery_response",
                "discovery_response.yaml",
                "discovery_schema.yaml",
                "discovery_template.md",
            ),
            (
                "default",
                "default.yaml",
                "default_schema.yaml",
                "default_template.md",
            ),
        ]);

        let profile = resolve_upload_profile(&registry, "discovery_response", None)
            .expect("resolution must succeed for a registered document type");

        assert_eq!(profile.schema_file, "discovery_schema.yaml");
        assert_eq!(profile.template_file, "discovery_template.md");
    }

    /// An upload tagged with a document_type that the registry doesn't
    /// know must fall back to the registry's default entry. The handler
    /// uses this both for the `document_type="auto"` legacy value and
    /// for genuinely unknown types coming from the multipart form.
    #[test]
    fn test_upload_unknown_type_falls_back_to_default() {
        let (_tmp, registry) = registry_with_profiles(&[
            (
                "complaint",
                "complaint.yaml",
                "complaint_schema.yaml",
                "complaint_template.md",
            ),
            (
                "default",
                "default.yaml",
                "default_schema.yaml",
                "default_template.md",
            ),
        ]);

        let profile = resolve_upload_profile(&registry, "totally_unknown", None)
            .expect("unknown type must resolve via the default entry");

        assert_eq!(profile.schema_file, "default_schema.yaml");
        assert_eq!(profile.template_file, "default_template.md");
    }

    /// Bug audit follow-ups: every column on `pipeline_config` that
    /// the upload populates must come from the matched profile. Pin
    /// down the mapping so a future field addition (or removal) that
    /// drops the column from the override map is caught here, not in
    /// production when the Configuration Panel suddenly shows
    /// compiled fallbacks.
    #[test]
    fn test_pipeline_config_populated_from_profile() {
        let yaml = r#"
name: full_profile
display_name: Full
schema_file: full.yaml
template_file: full_pass1.md
pass2_template_file: full_pass2.md
system_prompt_file: full_sys.md
global_rules_file: rules.md
extraction_model: claude-sonnet-4-6
pass2_extraction_model: claude-opus-4-6
chunking_mode: structured
chunk_size: 7000
chunk_overlap: 400
max_tokens: 31000
temperature: 0.5
auto_approve_grounded: false
run_pass2: true
version: "5.1"
pipeline_type: case_structuring
"#;
        let profile = ProcessingProfile::from_yaml_str(yaml).expect("parses");
        let o = super::overrides_for_upload(&profile);

        assert_eq!(o.profile_name.as_deref(), Some("full_profile"));
        assert_eq!(o.extraction_model.as_deref(), Some("claude-sonnet-4-6"));
        assert_eq!(o.pass2_extraction_model.as_deref(), Some("claude-opus-4-6"));
        assert_eq!(o.template_file.as_deref(), Some("full_pass1.md"));
        assert_eq!(o.pass2_template_file.as_deref(), Some("full_pass2.md"));
        assert_eq!(o.system_prompt_file.as_deref(), Some("full_sys.md"));
        assert_eq!(o.global_rules_file.as_deref(), Some("rules.md"));
        assert_eq!(o.chunking_mode.as_deref(), Some("structured"));
        assert_eq!(o.chunk_size, Some(7000));
        assert_eq!(o.chunk_overlap, Some(400));
        assert_eq!(o.max_tokens, Some(31000));
        assert_eq!(o.temperature, Some(0.5));
        assert_eq!(o.run_pass2, Some(true));
        assert_eq!(o.auto_approve_grounded, Some(false));
        // chunking_config/context_config intentionally None so the
        // document inherits live from the profile at resolve time.
        assert!(o.chunking_config.is_none());
        assert!(o.context_config.is_none());
    }

    /// Bug-fix anchor: after upload, the resolved config the runtime
    /// will use matches the profile's values for every field that's
    /// not specifically overridden. Round-trip:
    ///   profile → upload-time overrides → resolve_config → resolved
    /// must equal the profile's intent.
    #[test]
    fn test_resolved_config_matches_pipeline_config() {
        use crate::pipeline::config::resolve_config;
        let yaml = r#"
name: roundtrip
display_name: Roundtrip
schema_file: rt.yaml
template_file: rt_pass1.md
pass2_template_file: rt_pass2.md
system_prompt_file: rt_sys.md
global_rules_file: rt_rules.md
extraction_model: claude-sonnet-4-6
pass2_extraction_model: claude-opus-4-6
chunking_mode: full
chunk_size: 8000
chunk_overlap: 500
max_tokens: 32000
temperature: 0.0
auto_approve_grounded: true
run_pass2: true
version: "1.0"
pipeline_type: evidence_anchoring
"#;
        let profile = ProcessingProfile::from_yaml_str(yaml).expect("parses");
        let overrides = super::overrides_for_upload(&profile);
        let resolved = resolve_config(&profile, &overrides);

        assert_eq!(resolved.profile_name, profile.name);
        assert_eq!(resolved.model, profile.extraction_model);
        assert_eq!(resolved.pass2_model, profile.pass2_extraction_model);
        assert_eq!(resolved.template_file, profile.template_file);
        assert_eq!(resolved.pass2_template_file, profile.pass2_template_file);
        assert_eq!(resolved.system_prompt_file, profile.system_prompt_file);
        assert_eq!(resolved.global_rules_file, profile.global_rules_file);
        assert_eq!(resolved.schema_file, profile.schema_file);
        assert_eq!(resolved.chunking_mode, profile.chunking_mode);
        assert_eq!(resolved.chunk_size, profile.chunk_size);
        assert_eq!(resolved.chunk_overlap, profile.chunk_overlap);
        assert_eq!(resolved.max_tokens, profile.max_tokens);
        assert_eq!(resolved.temperature, profile.temperature);
        assert_eq!(resolved.run_pass2, profile.run_pass2);
        assert_eq!(
            resolved.auto_approve_grounded,
            profile.auto_approve_grounded
        );
        assert_eq!(resolved.version, profile.version);
        assert_eq!(resolved.pipeline_type, profile.pipeline_type);
    }

    /// If the registry has no default entry AND the document_type
    /// isn't registered, resolution must error — NOT silently pick a
    /// profile. A real registry can't reach this state because
    /// `validate()` rejects "no default" at startup, so the error is
    /// belt-and-suspenders for a registry mutated post-startup.
    #[test]
    fn test_upload_no_default_profile_returns_error() {
        // Registry with one entry that is NOT the default — i.e. no
        // default exists at all. Skip registry_with_profiles() because
        // it auto-marks the "default"-named entry as is_default.
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let profiles_dir = root.join("profiles");
        fs::create_dir_all(&profiles_dir).unwrap();
        fs::write(
            profiles_dir.join("complaint.yaml"),
            make_profile_yaml("complaint", "s.yaml", "t.md"),
        )
        .unwrap();
        let registry = PipelineRegistry {
            directories: PipelineDirectories {
                profiles: profiles_dir.to_string_lossy().into_owned(),
                schemas: profiles_dir.to_string_lossy().into_owned(),
                templates: profiles_dir.to_string_lossy().into_owned(),
                system_prompts: profiles_dir.to_string_lossy().into_owned(),
            },
            document_types: vec![DocumentTypeEntry {
                name: "complaint".to_string(),
                display_name: "Complaint".to_string(),
                profile_file: "complaint.yaml".to_string(),
                description: String::new(),
                is_default: false,
                sort_order: 1,
            }],
        };

        let err = resolve_upload_profile(&registry, "totally_unknown", None)
            .expect_err("missing entry AND missing default must error, not silently default");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("totally_unknown") && msg.contains("default"),
            "error must name both the missing document_type and the missing-default cause; got: {msg}"
        );
    }
}
