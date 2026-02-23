//! In-process embedding service using fastembed-rs (ONNX runtime).
//!
//! ## Pattern: Builder pattern
//! fastembed uses `InitOptions::new(model).with_cache_dir(path)` — a builder
//! pattern where each `.with_*()` method returns `Self`, letting you chain
//! optional configuration in a readable way. The final `try_new(options)`
//! consumes the builder and produces the configured object.
//!
//! ## Pattern: sync-in-async with spawn_blocking
//! ONNX inference (`self.model.embed(...)`) is CPU-bound and synchronous.
//! Calling it directly inside an async function would block the tokio runtime,
//! starving other tasks. Instead, the pipeline wraps calls in
//! `tokio::task::spawn_blocking`, which moves the work to a dedicated thread
//! pool designed for blocking operations.
//!
//! ## CRITICAL: TextEmbedding is NOT Send
//! This means it cannot be stored in AppState (which must be Send + Sync
//! for Axum). Instead, we create one EmbeddingService per pipeline run
//! inside the spawn_blocking closure, use it, then drop it.

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::path::PathBuf;

/// Wraps fastembed's TextEmbedding model for in-process vector generation.
pub struct EmbeddingService {
    model: TextEmbedding,
}

impl EmbeddingService {
    /// Create the embedding service.
    ///
    /// On first run this downloads the model weights (~270 MB from HuggingFace)
    /// into `cache_path`. Subsequent calls load from disk.
    pub fn new(cache_path: &str) -> Result<Self, EmbeddingError> {
        let options = InitOptions::new(EmbeddingModel::NomicEmbedTextV15)
            .with_cache_dir(PathBuf::from(cache_path))
            .with_show_download_progress(true);

        let model = TextEmbedding::try_new(options)?;
        Ok(Self { model })
    }

    /// Embed a batch of texts. Returns one `Vec<f32>` per input text.
    /// Each vector has exactly 768 dimensions.
    pub fn embed_batch(&mut self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        let embeddings = self.model.embed(refs, None)?;
        Ok(embeddings)
    }

    /// Embed a single text. Returns a 768-dimensional vector.
    pub fn embed_one(&mut self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let embeddings = self.model.embed(vec![text], None)?;
        embeddings
            .into_iter()
            .next()
            .ok_or(EmbeddingError::EmptyResult)
    }
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during embedding operations.
#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("fastembed error: {0}")]
    Fastembed(#[from] anyhow::Error),

    #[error("embedding returned no results")]
    EmptyResult,
}
