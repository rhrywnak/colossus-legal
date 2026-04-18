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

/// Shared subprocess helper: render one PDF page with pdftoppm, then OCR with tesseract.
///
/// Both [`ocr_page`] (hardcoded default config, used by the HTTP handler) and
/// [`ocr_page_with_config`] (configurable, used by the pipeline step) delegate
/// here so there is exactly one place that shells out to the external tools.
///
/// `.kill_on_drop(true)` is set on both child processes. When the pipeline
/// executor cancels a step via `tokio::select!`, the step future is dropped
/// mid-await; without `kill_on_drop` an in-flight tesseract keeps running
/// to completion as a zombie. The HTTP path never gets cancelled, so this
/// is primarily a pipeline-cancel fix — but both callers benefit.
async fn run_ocr_subprocesses(
    pdf_path: &str,
    page_number: u32,
    dpi: u32,
    lang: &str,
    oem: u32,
) -> Result<String, OcrError> {
    let tmp = tempfile::TempDir::new()?;
    let prefix = tmp.path().join("page");

    let render = Command::new("pdftoppm")
        .kill_on_drop(true)
        .arg("-png")
        .arg("-f")
        .arg(page_number.to_string())
        .arg("-l")
        .arg(page_number.to_string())
        .arg("-r")
        .arg(dpi.to_string())
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

    let png_path = find_png_in_dir(tmp.path())?;
    let output_base = tmp.path().join("output");

    let tess = Command::new("tesseract")
        .kill_on_drop(true)
        .arg(&png_path)
        .arg(&output_base)
        .arg("--oem")
        .arg(oem.to_string())
        .arg("-l")
        .arg(lang)
        .output()
        .await?;

    if !tess.status.success() {
        let stderr = String::from_utf8_lossy(&tess.stderr);
        return Err(OcrError::OcrFailed(format!(
            "tesseract exit {}: {stderr}",
            tess.status
        )));
    }

    let txt_path = tmp.path().join("output.txt");
    let text = tokio::fs::read_to_string(&txt_path).await?;
    Ok(text)
    // `tmp: TempDir` drops here — RAII cleanup of the PNG and the .txt file.
}

/// OCR a single page of a PDF using pdftoppm (render) + tesseract (recognise).
///
/// Uses the compiled-in defaults: [`OCR_DPI`] for rendering, `"eng"` for the
/// tesseract language, `1` for the OCR engine mode. For the configurable
/// pipeline-step variant, see [`ocr_page_with_config`].
///
/// ## Arguments
/// - `pdf_path` — full filesystem path to the PDF
/// - `page_number` — 1-indexed page to OCR
/// - `_total_pages` — total page count (unused; we glob for the output file
///   instead of constructing the zero-padded filename)
pub async fn ocr_page(
    pdf_path: &str,
    page_number: u32,
    _total_pages: u32,
) -> Result<String, OcrError> {
    run_ocr_subprocesses(pdf_path, page_number, OCR_DPI, "eng", 1).await
}

/// OCR a single page of a PDF with caller-supplied configuration.
///
/// Used by the pipeline `ExtractText` step, which resolves its
/// [`crate::pipeline::steps::extract_text::OcrConfig`] from
/// `pipeline_config.step_config` JSONB → `PIPELINE_OCR_*` env vars → compiled
/// defaults. The HTTP handler keeps calling [`ocr_page`] with hardcoded
/// defaults.
pub async fn ocr_page_with_config(
    pdf_path: &str,
    page_number: u32,
    cfg: &crate::pipeline::steps::extract_text::OcrConfig,
) -> Result<String, OcrError> {
    run_ocr_subprocesses(pdf_path, page_number, cfg.dpi, &cfg.lang, cfg.oem).await
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
