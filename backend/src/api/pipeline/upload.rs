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
use crate::pipeline::config::{PipelineConfigOverrides, ProcessingProfile};
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

/// Profile filename used when no profile in the directory declares the
/// requested `document_type`. Loaded via [`ProcessingProfile::load`] which
/// already appends `.yaml`, so this is the stem only.
const FALLBACK_PROFILE_STEM: &str = "default";

/// Errors emitted by [`select_profile_for_document_type`].
///
/// These represent configuration errors discovered at selection time —
/// the kind that should fail an upload loudly so the operator fixes the
/// profile YAMLs immediately, not silently degrade to a "best guess".
///
/// ## Rust Learning: `Debug` on a public error enum
///
/// `#[derive(Debug)]` is the minimum a returnable error needs so
/// callers can `?`-propagate it and use `{:?}` formatting in tests
/// (see the `panic!` arms below). For human-facing messages we
/// implement [`std::fmt::Display`] separately — different audiences,
/// different formatting.
#[derive(Debug)]
pub enum ProfileSelectionError {
    /// More than one profile claims `is_default: true` for the same
    /// `document_type`. Exactly one default per document_type is the
    /// invariant the operator must restore by editing the YAMLs.
    MultipleDefaults {
        document_type: String,
        profile_names: Vec<String>,
    },
    /// One or more profiles match the `document_type` but none have
    /// `is_default: true`. Without a default, selection is undefined —
    /// the operator must mark exactly one profile as the default.
    NoDefaultAmongMatches {
        document_type: String,
        profile_names: Vec<String>,
    },
    /// The profiles directory itself could not be read.
    DirectoryReadError {
        path: String,
        source: std::io::Error,
    },
    /// A specific YAML file in the profiles directory failed to parse.
    ProfileParseError { filename: String, error: String },
}

impl std::fmt::Display for ProfileSelectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MultipleDefaults {
                document_type,
                profile_names,
            } => write!(
                f,
                "Multiple profiles set is_default: true for document_type='{document_type}': [{}]. Exactly one profile per document_type must be the default — edit the profile YAMLs to fix.",
                profile_names.join(", ")
            ),
            Self::NoDefaultAmongMatches {
                document_type,
                profile_names,
            } => write!(
                f,
                "Profiles match document_type='{document_type}' but none have is_default: true: [{}]. Mark exactly one as default.",
                profile_names.join(", ")
            ),
            Self::DirectoryReadError { path, source } => {
                write!(f, "Failed to read profiles directory '{path}': {source}")
            }
            Self::ProfileParseError { filename, error } => {
                write!(f, "Failed to parse profile '{filename}': {error}")
            }
        }
    }
}

impl std::error::Error for ProfileSelectionError {}

/// Lift a [`ProfileSelectionError`] into an HTTP-500 [`AppError`].
///
/// All four variants represent operator-visible misconfiguration the
/// upload handler should surface immediately. The Display impl carries
/// the human-readable detail; the JSON 500 body shows that string.
impl From<ProfileSelectionError> for AppError {
    fn from(err: ProfileSelectionError) -> Self {
        AppError::Internal {
            message: err.to_string(),
        }
    }
}

/// Selects the profile to use for a given `document_type` by scanning
/// the profiles directory and respecting the YAML `is_default` flag.
///
/// Algorithm:
/// 1. Read every `*.yaml` file in `profile_dir` and parse it as a
///    [`ProcessingProfile`].
/// 2. Filter to profiles whose `document_type == Some(document_type)`.
/// 3. Among matches, find the one(s) with `is_default == true`.
/// 4. Exactly one match with `is_default: true` → return its filename
///    stem (e.g., `"complaint_profile_v5_1"`).
/// 5. Zero matches at all (no profile claims this document_type) →
///    return `"default"` so the caller loads `default.yaml` via
///    [`ProcessingProfile::load`].
/// 6. Multiple `is_default: true` for the same document_type →
///    [`ProfileSelectionError::MultipleDefaults`].
/// 7. Matches exist but none are `is_default: true` →
///    [`ProfileSelectionError::NoDefaultAmongMatches`].
///
/// ## Why the filename stem (not the YAML `name:` field)?
///
/// The returned string is passed to [`ProcessingProfile::load`] which
/// loads `{stem}.yaml`. Returning the stem keeps the round trip cheap
/// and lets a profile's display `name:` (e.g. `"complaint_v5_1"`) stay
/// independent of its on-disk filename (e.g. `complaint_profile_v5_1.yaml`).
///
/// ## Why no caching?
///
/// Profile selection runs once per upload — single-digit times per day
/// in production. Re-reading a small directory of YAML files on each call
/// is in the noise compared to the LLM extraction that follows. Skipping
/// a cache also means an operator can edit a profile YAML and have the
/// next upload pick up the change without restarting the backend, which
/// matches the existing [`ProcessingProfile::load`] behavior.
///
/// ## Rust Learning: `Result<String, ProfileSelectionError>` as the
/// return type
///
/// Owned `String` (not `&'static str` like the old hardcoded match)
/// lets the selector return values it computed at runtime — filename
/// stems — without lifetime gymnastics. The cost is one allocation per
/// successful selection; negligible at upload frequency.
pub fn select_profile_for_document_type(
    profile_dir: &Path,
    document_type: &str,
) -> Result<String, ProfileSelectionError> {
    let entries =
        std::fs::read_dir(profile_dir).map_err(|e| ProfileSelectionError::DirectoryReadError {
            path: profile_dir.display().to_string(),
            source: e,
        })?;

    // (filename_stem, is_default) for every profile whose document_type
    // matches the request. Vec rather than HashMap because order is only
    // for the error variant's reproducibility — we need to report the
    // conflicting filenames, not dedupe by name.
    let mut matches: Vec<(String, bool)> = Vec::new();

    for entry in entries {
        // A directory read error on a single entry shouldn't abort
        // selection — it could be a transient filesystem hiccup. Skip
        // and continue; if no candidates are found we'll either fall
        // back to "default" or surface a NoDefault error elsewhere.
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("yaml") {
            continue;
        }

        let stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };

        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("<unknown>")
            .to_string();

        // A profile that fails to parse is a configuration error the
        // operator must fix. Surface it loudly instead of skipping;
        // a malformed YAML hiding among valid ones could otherwise
        // cause silent selection-of-a-stale-profile.
        let profile = ProcessingProfile::from_file(&path).map_err(|e| {
            ProfileSelectionError::ProfileParseError {
                filename: filename.clone(),
                error: e,
            }
        })?;

        if profile.document_type.as_deref() == Some(document_type) {
            matches.push((stem, profile.is_default));
        }
    }

    let defaults: Vec<String> = matches
        .iter()
        .filter(|(_, is_default)| *is_default)
        .map(|(stem, _)| stem.clone())
        .collect();

    match defaults.len() {
        // No profile is marked default. Either zero matches at all
        // (legitimate "unknown document_type" → fall back to default.yaml)
        // or one+ matches but none defaulted (operator forgot to set
        // is_default: true after splitting profiles).
        0 => {
            if matches.is_empty() {
                Ok(FALLBACK_PROFILE_STEM.to_string())
            } else {
                Err(ProfileSelectionError::NoDefaultAmongMatches {
                    document_type: document_type.to_string(),
                    profile_names: matches.into_iter().map(|(s, _)| s).collect(),
                })
            }
        }
        1 => Ok(defaults
            .into_iter()
            .next()
            .expect("len == 1 verified above")),
        _ => Err(ProfileSelectionError::MultipleDefaults {
            document_type: document_type.to_string(),
            profile_names: defaults,
        }),
    }
}

/// Resolve the schema filename for a document type by consulting the
/// document's processing profile YAML.
///
/// Used by extract_text (after auto-detection) to keep `pipeline_config.schema_file`
/// in sync with `documents.document_type` once Pass-0 has determined the
/// real type. Profile-selection or profile-load failure is non-fatal:
/// returns [`FALLBACK_SCHEMA_FILE`] with a `tracing::warn!` so the
/// pipeline step doesn't crash on a misconfigured corpus — the operator
/// still sees the problem and can fix the profile.
///
/// The upload handler does NOT use this function on the fast path; it
/// calls [`select_profile_for_document_type`] directly and propagates
/// errors as 500. Errors at extract time should not propagate the same
/// way — the document is already in the system and the step needs to
/// keep going (with the fallback schema) rather than poison the pipeline.
pub fn schema_file_for_document_type(profile_dir: &str, document_type: &str) -> String {
    let profile_name = match select_profile_for_document_type(Path::new(profile_dir), document_type)
    {
        Ok(name) => name,
        Err(e) => {
            tracing::warn!(
                document_type,
                fallback = FALLBACK_SCHEMA_FILE,
                error = %e,
                "Profile selection failed at extract time — using fallback schema"
            );
            return FALLBACK_SCHEMA_FILE.to_string();
        }
    };
    match ProcessingProfile::load(profile_dir, &profile_name) {
        Ok(p) => p.schema_file,
        Err(e) => {
            tracing::warn!(
                document_type,
                profile = %profile_name,
                fallback = FALLBACK_SCHEMA_FILE,
                error = %e,
                "Failed to load profile to derive schema_file — using fallback"
            );
            FALLBACK_SCHEMA_FILE.to_string()
        }
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
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut doc_id: Option<String> = None;
    let mut title: Option<String> = None;
    let mut document_type: Option<String> = None;
    let mut pass1_model: Option<String> = None;
    let mut pass2_model: Option<String> = None;
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
            "pass1_model" => pass1_model = Some(field_text(&name, field).await?),
            "pass2_model" => pass2_model = Some(field_text(&name, field).await?),
            "admin_instructions" => admin_instructions = Some(field_text(&name, field).await?),
            "profile_version" => profile_version = Some(field_text(&name, field).await?),
            _ => { /* ignore unknown fields */ }
        }
    }

    // Validate required fields
    let doc_id = require_field("id", doc_id)?;
    let title = require_field("title", title)?;
    let document_type = document_type.unwrap_or_else(|| "auto".to_string());

    // Resolve the processing profile early so we can read `schema_file`
    // from the profile YAML (the single source of truth) and reuse the
    // loaded profile below to pre-populate the per-document override
    // columns, avoiding a duplicate disk read.
    //
    // Selection rules:
    //   - If `profile_version` was specified, treat it as an explicit
    //     override and load `<document_type>_<version>.yaml` directly.
    //     This is the user's "force a specific version" path; it does
    //     not consult is_default.
    //   - Otherwise scan the profiles directory, filter by document_type,
    //     and pick the one with `is_default: true`. Loud error on
    //     configuration ambiguity (multiple or zero defaults among
    //     matches) — surfaces as a 500 via the `?` propagation.
    let profile_name: String = if let Some(v) = profile_version.as_deref().filter(|s| !s.is_empty())
    {
        let name = format!("{document_type}_{v}");
        tracing::info!(
            document_type = %document_type,
            profile_version = %v,
            resolved_profile = %name,
            "Upload specified profile_version — using explicit override"
        );
        name
    } else {
        select_profile_for_document_type(
            Path::new(&state.config.processing_profile_dir),
            &document_type,
        )?
    };

    let profile = match ProcessingProfile::load(&state.config.processing_profile_dir, &profile_name)
    {
        Ok(p) => Some(p),
        Err(e) => {
            tracing::warn!(
                document_type = %document_type,
                profile = %profile_name,
                error = %e,
                "Profile load failed at upload — falling back for schema_file and skipping override pre-population"
            );
            None
        }
    };

    // Schema file — derived exclusively from the processing profile.
    // The frontend no longer sends this field; the profile YAML is the
    // single source of truth for which schema a document_type uses.
    let schema_file = profile
        .as_ref()
        .map(|p| p.schema_file.clone())
        .unwrap_or_else(|| {
            tracing::warn!(
                document_type = %document_type,
                fallback = FALLBACK_SCHEMA_FILE,
                "No profile loaded — using fallback schema"
            );
            FALLBACK_SCHEMA_FILE.to_string()
        });
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

    // Insert pipeline config
    let config_input = PipelineConfigInput {
        pass1_model,
        pass2_model,
        pass1_max_tokens: None,
        pass2_max_tokens: None,
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
    // manually overrides each dropdown. Reuse the profile loaded above;
    // a None here means the load failed and we already warned.
    if let Some(profile) = profile.as_ref() {
        let overrides = PipelineConfigOverrides {
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
            // Leave `chunking_config` and `context_config` as None at
            // upload time so the document inherits live from the
            // profile at resolve time. Seeding with the profile's
            // current values would freeze a snapshot here, which
            // would *prevent* future profile YAML edits from
            // affecting this document — confusing semantics.
            // The PATCH /config endpoint is the per-document
            // override path operators use to deviate from the profile.
            chunking_config: None,
            context_config: None,
        };
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
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // ── select_profile_for_document_type tests ──────────────────────
    //
    // The selector replaces the previous hardcoded match statement.
    // It scans the profiles directory, filters by `document_type`, and
    // picks the one with `is_default: true`. These tests pin the four
    // behaviors the upload handler relies on: pick-the-default,
    // zero-matches-falls-back, multiple-defaults-errors,
    // matches-but-no-default-errors.

    /// Build a minimal valid profile YAML body. Only the fields the
    /// selector cares about (`document_type`, `is_default`) and the
    /// required fields the deserializer demands (`name`, `display_name`,
    /// `schema_file`, `template_file`, `extraction_model`) are emitted —
    /// every other field uses the `#[serde(default)]` defaults in
    /// `ProcessingProfile`. Keeps test YAMLs small and the assertions
    /// focused on selection behavior, not deserialization shape.
    fn make_profile_yaml(name: &str, document_type: Option<&str>, is_default: bool) -> String {
        let dt_line = match document_type {
            Some(dt) => format!("document_type: {dt}\n"),
            None => String::new(),
        };
        format!(
            "name: {name}\n\
             display_name: Test\n\
             {dt_line}\
             schema_file: x.yaml\n\
             template_file: x.md\n\
             extraction_model: claude-sonnet-4-6\n\
             is_default: {is_default}\n"
        )
    }

    fn write_profile(dir: &Path, filename: &str, content: &str) {
        fs::write(dir.join(filename), content).expect("write profile YAML to tmp dir");
    }

    /// Headline test for this fix.
    ///
    /// Two complaint profiles co-exist: an older v5 (`is_default: false`)
    /// and a newer v5.1 (`is_default: true`). The selector must pick
    /// v5.1 — the one marked default — not v5.
    ///
    /// This test FAILS against the previous code (no
    /// `select_profile_for_document_type` function existed); it PASSES
    /// after this commit. Regressing the algorithm to filename-driven
    /// selection would break this test.
    #[test]
    fn test_select_default_when_multiple_profiles_same_document_type() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        write_profile(
            dir,
            "complaint_v5.yaml",
            &make_profile_yaml("complaint_v5", Some("complaint"), false),
        );
        write_profile(
            dir,
            "complaint_v5_1.yaml",
            &make_profile_yaml("complaint_v5_1", Some("complaint"), true),
        );

        let result = select_profile_for_document_type(dir, "complaint")
            .expect("selector should succeed when exactly one profile is the default");
        assert_eq!(
            result, "complaint_v5_1",
            "is_default: true profile must win over is_default: false"
        );
    }

    /// When no profile in the directory declares the requested
    /// `document_type`, the selector returns `"default"` so the caller
    /// loads `default.yaml`. This is the legitimate "unknown document
    /// type" fallback path.
    #[test]
    fn test_select_zero_matches_returns_default() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        write_profile(
            dir,
            "motion.yaml",
            &make_profile_yaml("motion", Some("motion"), true),
        );
        // default.yaml deliberately omits document_type — that's its role.
        write_profile(
            dir,
            "default.yaml",
            &make_profile_yaml("default", None, true),
        );

        let result = select_profile_for_document_type(dir, "complaint")
            .expect("zero matches should return Ok(\"default\")");
        assert_eq!(result, FALLBACK_PROFILE_STEM);
    }

    /// When two profiles claim `is_default: true` for the same
    /// `document_type`, the selector refuses to guess and errors loudly.
    /// The error carries both filenames so the operator can fix exactly
    /// the right files.
    #[test]
    fn test_select_multiple_defaults_errors() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        write_profile(
            dir,
            "complaint_a.yaml",
            &make_profile_yaml("complaint_a", Some("complaint"), true),
        );
        write_profile(
            dir,
            "complaint_b.yaml",
            &make_profile_yaml("complaint_b", Some("complaint"), true),
        );

        let result = select_profile_for_document_type(dir, "complaint");
        match result {
            Err(ProfileSelectionError::MultipleDefaults {
                document_type,
                profile_names,
            }) => {
                assert_eq!(document_type, "complaint");
                assert_eq!(profile_names.len(), 2);
                let mut names = profile_names;
                names.sort();
                assert_eq!(names, vec!["complaint_a", "complaint_b"]);
            }
            other => panic!("expected MultipleDefaults, got {other:?}"),
        }
    }

    /// When profiles match the document_type but none is marked
    /// `is_default: true`, the selector errors loudly rather than
    /// silently picking one. This catches the regression where adding
    /// a new variant profile but forgetting to flip the old default
    /// leaves the system without a defined choice.
    #[test]
    fn test_select_matches_but_no_default_errors() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        write_profile(
            dir,
            "complaint_a.yaml",
            &make_profile_yaml("complaint_a", Some("complaint"), false),
        );
        write_profile(
            dir,
            "complaint_b.yaml",
            &make_profile_yaml("complaint_b", Some("complaint"), false),
        );

        let result = select_profile_for_document_type(dir, "complaint");
        match result {
            Err(ProfileSelectionError::NoDefaultAmongMatches {
                document_type,
                profile_names,
            }) => {
                assert_eq!(document_type, "complaint");
                assert_eq!(profile_names.len(), 2);
            }
            other => panic!("expected NoDefaultAmongMatches, got {other:?}"),
        }
    }
}
