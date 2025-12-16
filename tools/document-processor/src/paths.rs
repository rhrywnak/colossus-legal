//! Path resolution utilities for the document processor.
//!
//! Responsible for:
//! - Validating directories
//! - Resolving input file paths
//! - Deriving document names
//! - Resolving output file paths

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

/// Ensure a path exists and is a directory.
pub fn validate_directory(path: &str, name: &str) -> Result<()> {
    let dir_path = Path::new(path);

    if !dir_path.exists() {
        bail!(
            "{} does not exist: {}\nPlease create the directory",
            name,
            path
        );
    }

    if !dir_path.is_dir() {
        bail!("{} is not a directory: {}", name, path);
    }

    Ok(())
}

/// Resolve the input file path based on filename and an input directory.
///
/// If `filename` is absolute or contains path separators, it is used as-is.
/// Otherwise, it is resolved relative to `input_dir`.
pub fn resolve_input_path(filename: &str, input_dir: &str) -> Result<PathBuf> {
    let path = Path::new(filename);

    if path.is_absolute() || filename.contains('/') || filename.contains('\\') {
        if !path.exists() {
            bail!("Input file does not exist: {}", filename);
        }
        Ok(path.to_path_buf())
    } else {
        let full_path = Path::new(input_dir).join(filename);
        if !full_path.exists() {
            bail!(
                "Input file not found: {}\nLooked in: {}",
                filename,
                full_path.display()
            );
        }
        Ok(full_path)
    }
}

/// Extract the document "name" (stem) from a path, without extension.
pub fn extract_document_name(path: &Path) -> Result<String> {
    let filename = path
        .file_stem()
        .and_then(|n| n.to_str())
        .context("Invalid input filename")?;

    Ok(filename.to_string())
}

/// Resolve the output file path.
///
/// If `explicit_output` is provided, it is used as-is.
/// Otherwise, we derive a name from the input filename and append `suffix`,
/// placing the result in `output_dir`.
pub fn resolve_output_path(
    input_path: &Path,
    explicit_output: Option<&str>,
    output_dir: &str,
    suffix: &str,
) -> Result<PathBuf> {
    if let Some(output) = explicit_output {
        Ok(PathBuf::from(output))
    } else {
        let input_filename = input_path
            .file_name()
            .and_then(|n| n.to_str())
            .context("Invalid input filename")?;

        let output_filename = format!("{}{}", input_filename, suffix);
        Ok(Path::new(output_dir).join(output_filename))
    }
}
