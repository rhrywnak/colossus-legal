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
#[allow(dead_code)] // tesseract path preserved pending removal task
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

    #[error("Surya OCR service error: {0}")]
    SuryaError(String),
}

// ── Surya OCR response types ────────────────────────────────────

/// Per-page result from the Surya OCR service.
///
/// `line_count` and `confidence` are deserialized but not currently consumed
/// — they're part of the wire contract and useful for future diagnostics.
#[allow(dead_code)]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SuryaPageResult {
    pub page_number: u32,
    pub text: String,
    pub line_count: usize,
    pub confidence: f64,
}

/// Full response from the Surya OCR service.
///
/// `filename` and `total_pages` echo what the server received; kept for
/// completeness of the wire contract even though Rust doesn't read them.
#[allow(dead_code)]
#[derive(Debug, serde::Deserialize)]
pub struct SuryaOcrResponse {
    pub filename: String,
    pub total_pages: u32,
    pub pages_processed: u32,
    pub elapsed_secs: f64,
    pub pages: Vec<SuryaPageResult>,
}

// ── Tool availability check ─────────────────────────────────────

/// Verify that `pdftoppm` and `tesseract` are installed and callable.
///
/// Call once before processing pages. Returns `Err(OcrError::ToolNotFound)`
/// with a descriptive message if either tool is missing.
#[allow(dead_code)] // tesseract path preserved pending removal task
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
#[allow(dead_code)] // tesseract path preserved pending removal task
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
#[allow(dead_code)] // tesseract path preserved pending removal task
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
#[allow(dead_code)] // tesseract path preserved pending removal task
pub async fn ocr_page_with_config(
    pdf_path: &str,
    page_number: u32,
    cfg: &crate::pipeline::steps::extract_text::OcrConfig,
) -> Result<String, OcrError> {
    run_ocr_subprocesses(pdf_path, page_number, cfg.dpi, &cfg.lang, cfg.oem).await
}

// ── Surya OCR (GPU service) ─────────────────────────────────────

/// OCR a PDF by sending it to the Surya GPU service.
///
/// Sends the entire PDF file as a multipart upload to the Surya service.
/// Returns per-page text for all pages, or only for specific pages if
/// `page_numbers` is provided.
///
/// The Surya service URL is read from the `SURYA_OCR_URL` env var.
///
/// ## Why a shared `reqwest::Client`
///
/// Callers pass the shared `http_client` from `AppState` / `AppContext`.
/// The shared client pools connections and has a `connect_timeout` baked in
/// at startup — we override the per-request `timeout` to 300s here because
/// OCR on large scanned PDFs can legitimately take minutes.
///
/// ## Why whole-document instead of per-page
///
/// Unlike tesseract (which processes one page at a time via subprocess),
/// Surya loads the PDF once and batches all pages through the GPU model.
/// Sending pages individually would re-upload the PDF N times and lose
/// batch efficiency. One call, all pages.
pub async fn ocr_full_document_surya(
    http_client: &reqwest::Client,
    pdf_path: &str,
    page_numbers: Option<&[u32]>,
) -> Result<SuryaOcrResponse, OcrError> {
    let surya_url = std::env::var("SURYA_OCR_URL").map_err(|_| {
        OcrError::SuryaError(
            "SURYA_OCR_URL env var not set — cannot reach Surya OCR service".to_string(),
        )
    })?;

    let url = format!("{}/ocr", surya_url.trim_end_matches('/'));

    // Read the PDF file
    let pdf_bytes = tokio::fs::read(pdf_path)
        .await
        .map_err(|e| OcrError::SuryaError(format!("Failed to read PDF '{pdf_path}': {e}")))?;

    // Extract filename for the multipart form
    let filename = std::path::Path::new(pdf_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document.pdf")
        .to_string();

    // Build multipart form
    let file_part = reqwest::multipart::Part::bytes(pdf_bytes)
        .file_name(filename)
        .mime_str("application/pdf")
        .map_err(|e| OcrError::SuryaError(format!("Multipart build failed: {e}")))?;

    let mut form = reqwest::multipart::Form::new().part("file", file_part);

    // Add page filter if specific pages requested
    if let Some(pages) = page_numbers {
        let pages_str = pages
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(",");
        form = form.text("pages", pages_str);
    }

    let response = http_client
        .post(&url)
        .multipart(form)
        .timeout(std::time::Duration::from_secs(300))
        .send()
        .await
        .map_err(|e| {
            OcrError::SuryaError(format!(
                "Surya OCR request to {url} failed: {e}. \
                 Verify the service is running and can accept PDF uploads."
            ))
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(OcrError::SuryaError(format!(
            "Surya returned {status}: {body}"
        )));
    }

    let result: SuryaOcrResponse = response
        .json()
        .await
        .map_err(|e| OcrError::SuryaError(format!("Failed to parse Surya response: {e}")))?;

    tracing::info!(
        pdf = %pdf_path,
        pages = result.pages_processed,
        elapsed = result.elapsed_secs,
        "Surya OCR complete"
    );

    Ok(result)
}

/// Check if the Surya OCR service is reachable and models are loaded.
///
/// Uses a short 5-second timeout so a dead service fails fast and the
/// caller can degrade gracefully (log warning, skip OCR for scanned pages).
pub async fn check_surya_available(http_client: &reqwest::Client) -> Result<(), OcrError> {
    let surya_url = std::env::var("SURYA_OCR_URL").map_err(|_| {
        OcrError::SuryaError(
            "SURYA_OCR_URL env var not set. Set it to the Surya OCR service URL \
             (e.g. http://192.168.1.100:8340) in the container environment."
                .to_string(),
        )
    })?;

    let url = format!("{}/health", surya_url.trim_end_matches('/'));
    let response = http_client
        .get(&url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| {
            OcrError::SuryaError(format!(
                "Surya OCR service unreachable at {url}. Error: {e}. \
                 Verify the service is running and the URL is correct."
            ))
        })?;

    if !response.status().is_success() {
        return Err(OcrError::SuryaError(format!(
            "Surya health returned {}",
            response.status()
        )));
    }

    Ok(())
}

/// Find exactly one `page-*.png` file in a directory.
#[allow(dead_code)] // tesseract path preserved pending removal task
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
