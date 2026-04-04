//! OCR fallback for scanned PDF pages.
//!
//! Uses external CLI tools (`pdftoppm` + `tesseract`) to convert a single PDF
//! page into text. Called by extract_text when a page has too few characters
//! from native PDF text extraction.
//!
//! ## Rust Learning: RAII with `tempfile::TempDir`
//!
//! `TempDir` automatically deletes the directory and all files inside it when
//! the value is dropped. This means we don't need explicit cleanup code — even
//! if the function returns early due to an error, the temp files are removed.

use std::path::PathBuf;
use tokio::process::Command;

/// Pages with fewer non-whitespace characters than this are considered scanned.
pub const OCR_CHAR_THRESHOLD: usize = 50;

/// Render resolution for pdftoppm (dots per inch).
const OCR_DPI: u32 = 300;

// ── Error type ──────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum OcrError {
    #[error("OCR tool not found: {0}")]
    ToolNotFound(String),

    #[error("PDF render failed: {0}")]
    RenderFailed(String),

    #[error("OCR failed: {0}")]
    OcrFailed(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

// ── Tool availability check ─────────────────────────────────────

/// Verify that `pdftoppm` and `tesseract` are installed and callable.
///
/// Call once before processing pages. Returns `Err(OcrError::ToolNotFound)`
/// with a descriptive message if either tool is missing.
pub async fn check_ocr_tools_available() -> Result<(), OcrError> {
    let pdftoppm = Command::new("pdftoppm").arg("-v").output().await;
    if pdftoppm.is_err() {
        return Err(OcrError::ToolNotFound(
            "pdftoppm not found — install poppler-utils".to_string(),
        ));
    }

    let tesseract = Command::new("tesseract").arg("--version").output().await;
    if tesseract.is_err() {
        return Err(OcrError::ToolNotFound(
            "tesseract not found — install tesseract-ocr".to_string(),
        ));
    }

    Ok(())
}

// ── Per-page OCR ────────────────────────────────────────────────

/// OCR a single page of a PDF using pdftoppm (render) + tesseract (recognise).
///
/// ## Arguments
/// - `pdf_path` — full filesystem path to the PDF
/// - `page_number` — 1-indexed page to OCR
/// - `_total_pages` — total page count (unused; we glob for the output file
///   instead of constructing the zero-padded filename)
///
/// ## Flow
/// 1. Create temp directory (auto-cleaned on drop)
/// 2. `pdftoppm -png -f N -l N -r 300 <pdf> <prefix>` → renders one PNG
/// 3. Glob temp dir for the single `page-*.png` file
/// 4. `tesseract <png> <output_base> --oem 1 -l eng` → writes `output.txt`
/// 5. Read and return the text
pub async fn ocr_page(
    pdf_path: &str,
    page_number: u32,
    _total_pages: u32,
) -> Result<String, OcrError> {
    let tmp = tempfile::TempDir::new()?;
    let prefix = tmp.path().join("page");

    // Step 1: Render the page to PNG with pdftoppm.
    let render = Command::new("pdftoppm")
        .arg("-png")
        .arg("-f")
        .arg(page_number.to_string())
        .arg("-l")
        .arg(page_number.to_string())
        .arg("-r")
        .arg(OCR_DPI.to_string())
        .arg(pdf_path)
        .arg(&prefix)
        .output()
        .await?;

    if !render.status.success() {
        let stderr = String::from_utf8_lossy(&render.stderr);
        return Err(OcrError::RenderFailed(format!(
            "pdftoppm exit {}: {stderr}",
            render.status
        )));
    }

    // Step 2: Find the output PNG.
    // pdftoppm zero-pads based on total pages, so we glob rather than guess.
    let png_path = find_png_in_dir(tmp.path())?;

    // Step 3: Run tesseract on the PNG.
    let output_base = tmp.path().join("output");
    let tess = Command::new("tesseract")
        .arg(&png_path)
        .arg(&output_base)
        .arg("--oem")
        .arg("1")
        .arg("-l")
        .arg("eng")
        .output()
        .await?;

    if !tess.status.success() {
        let stderr = String::from_utf8_lossy(&tess.stderr);
        return Err(OcrError::OcrFailed(format!(
            "tesseract exit {}: {stderr}",
            tess.status
        )));
    }

    // Step 4: Read the output text (tesseract appends .txt automatically).
    let txt_path = tmp.path().join("output.txt");
    let text = tokio::fs::read_to_string(&txt_path).await?;

    Ok(text)
}

/// Find exactly one `page-*.png` file in a directory.
fn find_png_in_dir(dir: &std::path::Path) -> Result<PathBuf, OcrError> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "png" {
                return Ok(path);
            }
        }
    }
    Err(OcrError::RenderFailed(
        "pdftoppm produced no PNG output".to_string(),
    ))
}
