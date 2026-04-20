//! Shared DTOs, constants, and helpers used across the config_endpoints
//! submodules (templates, schemas, system_prompts, models, profiles).
//!
//! Types here are intentionally generic across "one file in a directory"
//! resource shapes so the individual endpoint modules only differ in the
//! directory they point at and the file extension they enforce.

use serde::{Deserialize, Serialize};

use crate::error::AppError;

// ── File-resource DTOs ──────────────────────────────────────────

/// Response body for GET `/<resource>/:filename` — the whole file content.
///
/// Admin UIs render this in a code-editor widget. `size_bytes` lets the
/// client warn on oversized files before attempting to display them.
#[derive(Debug, Serialize)]
pub struct FileContentResponse {
    pub filename: String,
    pub content: String,
    pub size_bytes: u64,
}

/// Request body for POST `/<resource>` — create a new file.
#[derive(Debug, Deserialize)]
pub struct CreateFileInput {
    pub filename: String,
    pub content: String,
}

/// Request body for PUT `/<resource>/:filename` — overwrite existing content.
#[derive(Debug, Deserialize)]
pub struct UpdateFileInput {
    pub content: String,
}

// ── Filename validation ─────────────────────────────────────────

/// Whitelist-validate a filename before using it in a filesystem path.
///
/// Accepts `[A-Za-z0-9_.-]+` (letters, digits, `_`, `-`, `.`). Rejects
/// empty strings, slashes, backslashes, and any character outside the
/// set. This lets filenames like `pass1_complaint.md` through while
/// blocking path-traversal attempts like `../etc/passwd`.
///
/// Profile names use a stricter whitelist (no `.`) — see
/// `profiles::validate_profile_name`.
pub fn validate_filename(filename: &str) -> Result<(), AppError> {
    if filename.is_empty() {
        return Err(AppError::BadRequest {
            message: "Filename must not be empty".into(),
            details: serde_json::json!({"field": "filename"}),
        });
    }
    let ok = filename
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.');
    if !ok {
        return Err(AppError::BadRequest {
            message: format!(
                "Invalid filename '{filename}' — only letters, digits, '_', '-', '.' are allowed"
            ),
            details: serde_json::json!({"field": "filename"}),
        });
    }
    // Defensive: reject `..` as a whole-filename token so "..", "..yaml"-like
    // traversal patterns never reach `Path::join` even if they pass the
    // char whitelist via the literal '.' characters.
    if filename == "." || filename == ".." {
        return Err(AppError::BadRequest {
            message: format!("Invalid filename '{filename}'"),
            details: serde_json::json!({"field": "filename"}),
        });
    }
    Ok(())
}

/// Assert a filename ends with a required extension (e.g. `".md"`, `".yaml"`).
///
/// Returns `BadRequest` with a clear message if the extension is missing.
/// The extension string must include the leading dot.
pub fn require_extension(filename: &str, required_ext: &str) -> Result<(), AppError> {
    if !filename.ends_with(required_ext) {
        return Err(AppError::BadRequest {
            message: format!("Filename '{filename}' must end with '{required_ext}'"),
            details: serde_json::json!({"field": "filename", "expected_ext": required_ext}),
        });
    }
    Ok(())
}

// ── Profile-reference scanner ───────────────────────────────────

/// Return profile filenames under `profile_dir` whose YAML content
/// contains `needle` as a substring.
///
/// Used to block destructive operations on resources referenced by a
/// profile — deleting a model that a profile uses, deleting a schema or
/// template that a profile points at, etc. The substring match is
/// intentionally loose; false positives simply preserve a referenced
/// resource, which is the safer bias.
///
/// If `profile_dir` does not exist, returns an empty `Vec` (not an error):
/// the reference check succeeds vacuously. Unreadable individual files
/// are logged and skipped.
pub async fn profiles_referencing(
    profile_dir: &str,
    needle: &str,
) -> Result<Vec<String>, std::io::Error> {
    let mut matches = Vec::new();

    let mut entries = match tokio::fs::read_dir(profile_dir).await {
        Ok(r) => r,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(matches),
        Err(e) => return Err(e),
    };

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        let content = match tokio::fs::read_to_string(&path).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(
                    file = %path.display(),
                    error = %e,
                    "Failed to read profile file while scanning for references (skipping)"
                );
                continue;
            }
        };
        if content.contains(needle) {
            matches.push(entry.file_name().to_string_lossy().to_string());
        }
    }

    matches.sort();
    Ok(matches)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_filename_accepts_dotted_names() {
        assert!(validate_filename("pass1_complaint.md").is_ok());
        assert!(validate_filename("complaint_v2.yaml").is_ok());
        assert!(validate_filename("file-name_01.txt").is_ok());
    }

    #[test]
    fn validate_filename_rejects_traversal() {
        assert!(validate_filename("../etc/passwd").is_err());
        assert!(validate_filename("foo/bar").is_err());
        assert!(validate_filename("foo\\bar").is_err());
        assert!(validate_filename("").is_err());
        assert!(validate_filename(".").is_err());
        assert!(validate_filename("..").is_err());
    }

    #[test]
    fn require_extension_checks_suffix() {
        assert!(require_extension("foo.md", ".md").is_ok());
        assert!(require_extension("foo.yaml", ".md").is_err());
        assert!(require_extension("foo", ".md").is_err());
    }
}
