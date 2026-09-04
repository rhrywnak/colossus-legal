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
//!
//! ## ⚑ Two failures this module exists to prevent
//!
//! Both are the same shape: an input that produces a plausible-looking vector
//! which is not a vector of what the caller thought they embedded.
//!
//! 1. **Silent truncation.** fastembed installs `TruncationParams` on the
//!    tokenizer, so an over-length input is quietly shortened — no error, no
//!    log, no change in the return type. A query truncated at the end loses
//!    the allegations at the end, which is precisely the evidence a gather is
//!    reaching for, and every later measurement would be measuring the wrong
//!    thing while looking correct. It is still allowed (truncation may one day
//!    be the right behaviour) but it is no longer invisible.
//! 2. **The empty input.** An empty string embeds to a degenerate vector that
//!    matches arbitrarily. A pool filled from one looks like a working search
//!    and is noise. That one is rejected outright rather than warned about,
//!    because there is no reading of it that is correct.
//!
//! ## ⚑ The cap you ask for is not always the cap you get
//!
//! fastembed's `load_tokenizer` does
//! `let max_length = max_length.min(model_max_length as usize)` — it CLAMPS the
//! requested limit to the `model_max_length` in the model's own
//! `tokenizer_config.json`, silently. Asking for 8192 against a model whose
//! config says 512 leaves you at 512 with no indication.
//!
//! So this module never trusts what it asked for. It reads back what the
//! tokenizer actually enforces and warns at startup if the two differ. Measured
//! on DEV: nomic-embed-text-v1.5 ships `model_max_length: 8192`, so the request
//! below lands exactly and is not clamped — but that is a fact about the model
//! that a future model swap could quietly change.

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::path::PathBuf;

// STRUCTURAL: the input limit of the MODEL, not of the deployment.
//
// `nomic-embed-text-v1.5` supports 8192 tokens natively; fastembed's own
// `DEFAULT_MAX_LENGTH` of 512 is the library's default, not the model's
// ceiling, and it silently truncated anything longer. This constant is bound to
// the `EmbeddingModel::NomicEmbedTextV15` chosen three lines below it — change
// one and you must change the other — so it is not environment-variable and
// does not belong in config. No deployment wants a different number here.
//
// Raising it re-embeds nothing: the longest text this corpus has ever embedded
// is ~364 tokens, far below the old 512, so no existing vector was ever
// truncated and none of them change. `a_short_text_embeds_identically_under_
// the_raised_cap` proves that rather than asserting it.
const MAX_INPUT_TOKENS: usize = 8192;

// STRUCTURAL: the two special tokens ([CLS] and [SEP]) a BERT WordPiece
// tokenizer adds around every input. Used only to make the cheap pre-filter
// below conservative; it is a property of the tokenizer, not a setting.
const SPECIAL_TOKENS_ADDED: usize = 2;

/// How much of an over-length input the warning quotes back.
///
/// CONST: a log-message ergonomics value, not a tunable — the same shape as
/// `anthropic_transport::ERROR_BODY_PREVIEW_CHARS` and
/// `anthropic_stream::MALFORMED_PREVIEW_CHARS`. Long enough to identify which
/// text was truncated, short enough that the very input this warning fires on
/// — an over-length one, by definition — cannot flood the log with the
/// megabytes that triggered it. Nothing downstream reads it and no deployment
/// behaves differently for a different value.
const WARN_EXCERPT_CHARS: usize = 120;

/// Wraps fastembed's TextEmbedding model for in-process vector generation.
pub struct EmbeddingService {
    model: TextEmbedding,
    /// The limit the tokenizer ACTUALLY enforces, read back after construction
    /// rather than assumed from [`MAX_INPUT_TOKENS`] — see the module note on
    /// fastembed's silent clamp. `None` would mean no truncation is configured
    /// at all, in which case nothing can be truncated and nothing is warned.
    effective_max_tokens: Option<usize>,
}

impl EmbeddingService {
    /// Create the embedding service.
    ///
    /// On first run this downloads the model weights (~270 MB from HuggingFace)
    /// into `cache_path`. Subsequent calls load from disk.
    pub fn new(cache_path: &str) -> Result<Self, EmbeddingError> {
        let options = InitOptions::new(EmbeddingModel::NomicEmbedTextV15)
            .with_max_length(MAX_INPUT_TOKENS)
            .with_cache_dir(PathBuf::from(cache_path))
            .with_show_download_progress(true);

        let model = TextEmbedding::try_new(options)?;
        let effective_max_tokens = model.tokenizer.get_truncation().map(|t| t.max_length);
        report_effective_cap(effective_max_tokens);
        Ok(Self {
            model,
            effective_max_tokens,
        })
    }

    /// Embed a batch of texts. Returns one `Vec<f32>` per input text.
    /// Each vector has exactly 768 dimensions.
    ///
    /// # Errors
    /// [`EmbeddingError::EmptyInput`] if any text is empty or whitespace-only —
    /// see the module note. Over-length texts are warned about, not rejected.
    pub fn embed_batch(&mut self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        reject_empty(&refs)?;
        self.warn_on_truncation(&refs);
        let embeddings = self.model.embed(refs, None)?;
        Ok(embeddings)
    }

    /// Embed a single text. Returns a 768-dimensional vector.
    ///
    /// # Errors
    /// [`EmbeddingError::EmptyInput`] if the text is empty or whitespace-only.
    pub fn embed_one(&mut self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        reject_empty(&[text])?;
        self.warn_on_truncation(&[text]);
        let embeddings = self.model.embed(vec![text], None)?;
        embeddings
            .into_iter()
            .next()
            .ok_or(EmbeddingError::EmptyResult)
    }

    /// Say so, loudly, when an input is about to be silently shortened.
    ///
    /// ## Why this does not tokenize everything twice
    ///
    /// `embed()` tokenizes internally, so counting tokens here would be a
    /// second pass over every input. It is avoided with a pre-filter that is
    /// cheap and SOUND rather than approximate: a WordPiece token consumes at
    /// least one byte of input, so `bytes + 2 specials` is an upper bound on
    /// the token count. Any input whose byte length clears the cap therefore
    /// provably cannot truncate, and is skipped with an O(1) `len()`.
    ///
    /// Only the inputs that fail that bound are tokenized — currently none in
    /// this corpus, whose longest text is ~1639 bytes against an 8192 cap — so
    /// the steady-state cost of this check is one integer comparison per text.
    fn warn_on_truncation(&self, texts: &[&str]) {
        let Some(limit) = self.effective_max_tokens else {
            return; // No truncation configured: nothing can be silently dropped.
        };
        for text in texts {
            if cannot_truncate(text, limit) {
                continue;
            }
            match self.count_tokens(text) {
                Ok(count) => {
                    if let Some(w) = truncation_warning(count, limit, text) {
                        tracing::warn!(
                            tokens = w.tokens,
                            limit = w.limit,
                            dropped = w.dropped,
                            excerpt = %w.excerpt,
                            "input exceeds the embedding model's token limit and WILL be \
                             truncated at the end; the resulting vector is of a shortened text"
                        );
                    }
                }
                // The count is diagnostic. Failing the embed because the warning
                // could not be computed would be worse than embedding — but the
                // failure to count is itself logged rather than swallowed.
                Err(e) => tracing::warn!(
                    error = %e,
                    excerpt = %excerpt(text),
                    "could not count tokens for an over-long input; it may be truncated \
                     silently"
                ),
            }
        }
    }

    /// The token count BEFORE truncation.
    ///
    /// ## Rust Learning: recovering what a library threw away
    ///
    /// The model's tokenizer has truncation installed, so its encoding is
    /// already shortened — asking it for a length would return the limit, not
    /// the truth. But `tokenizers` keeps the discarded windows in
    /// `get_overflowing()`, and with fastembed's stride of 0 those windows do
    /// not overlap. So the original length is the kept ids plus every
    /// overflowing window's ids, exactly. No second tokenizer, no clone, and
    /// no `tokenizers` dependency of our own.
    fn count_tokens(&self, text: &str) -> Result<usize, EmbeddingError> {
        let encoding = self
            .model
            .tokenizer
            .encode(text, true)
            .map_err(|e| EmbeddingError::TokenCount(e.to_string()))?;
        let overflow: usize = encoding
            .get_overflowing()
            .iter()
            .map(|window| window.get_ids().len())
            .sum();
        Ok(encoding.get_ids().len() + overflow)
    }
}

/// What a truncation warning says: the count, the limit, and what is lost.
///
/// Split out from the `tracing::warn!` call so the DECISION to warn and the
/// numbers it reports are testable without a tracing subscriber. The macro
/// below renders these fields and adds nothing of its own.
#[derive(Debug, PartialEq, Eq)]
struct TruncationWarning {
    tokens: usize,
    limit: usize,
    dropped: usize,
    excerpt: String,
}

/// `Some` when this input will actually lose tokens, `None` when it fits.
///
/// `dropped` is what the tokenizer discards: everything past the limit. It is
/// the number that tells an operator whether one sentence went missing or half
/// the query did.
fn truncation_warning(count: usize, limit: usize, text: &str) -> Option<TruncationWarning> {
    let dropped = count.checked_sub(limit).filter(|d| *d > 0)?;
    Some(TruncationWarning {
        tokens: count,
        limit,
        dropped,
        excerpt: excerpt(text),
    })
}

/// Whether an input provably cannot be truncated, using only its byte length.
///
/// ## Rust Learning: a sound bound beats an accurate estimate
///
/// A WordPiece token consumes at least one byte of its input, so the token
/// count can never exceed the byte count, plus the two special tokens the
/// tokenizer wraps around every input. That makes `bytes + 2 <= limit` a
/// one-sided guarantee rather than a guess: it is allowed to say "might
/// truncate" about a text that does not, but it can never say "safe" about one
/// that does. A chars/4 estimate would be closer on average and wrong in
/// exactly the case that matters.
fn cannot_truncate(text: &str, limit: usize) -> bool {
    text.len() + SPECIAL_TOKENS_ADDED <= limit
}

/// Say at startup what limit is really in force.
///
/// Rule 1: the requested cap and the enforced cap are operationally different
/// states, and before this they were indistinguishable — a clamp looked exactly
/// like a success.
fn report_effective_cap(effective: Option<usize>) {
    match effective {
        Some(limit) if limit < MAX_INPUT_TOKENS => tracing::warn!(
            requested = MAX_INPUT_TOKENS,
            enforced = limit,
            "the embedding model clamped the requested token limit to its own \
             model_max_length; inputs longer than the enforced limit will be truncated"
        ),
        Some(limit) => tracing::info!(limit, "embedding token limit in force"),
        None => tracing::warn!(
            "the embedding tokenizer reports no truncation limit; over-length inputs \
             cannot be detected"
        ),
    }
}

/// The guard: no empty or whitespace-only text ever reaches the model.
///
/// ## Why this is the guard, and why it cannot be bypassed
///
/// The embedding service owns the only `TextEmbedding` in the process, and
/// `model` is a private field, so `embed_one` and `embed_batch` are the only
/// doors to a vector. A caller who never read a doc comment still gets a named
/// error rather than a degenerate vector. Encoding the rule in a type on the
/// gather side instead would have been bypassable — a caller can always reach
/// past a newtype to the `String` behind it — and would have protected only
/// the one caller that used it.
///
/// Note this is deliberately NOT a check of `query_basis`: a `no_content`
/// query always has empty text, but a `theme_and_allegations` query whose
/// theme is blank and whose allegations all extracted empty has empty text
/// too. Emptiness and basis are different predicates, and it is emptiness that
/// makes the vector meaningless.
fn reject_empty(texts: &[&str]) -> Result<(), EmbeddingError> {
    match texts.iter().position(|t| t.trim().is_empty()) {
        Some(index) => Err(EmbeddingError::EmptyInput { index }),
        None => Ok(()),
    }
}

/// The first [`WARN_EXCERPT_CHARS`] characters, for identifying a text in a log.
///
/// Counts by `char`, not by byte, so a multi-byte character is never split —
/// slicing a `String` mid-character panics.
fn excerpt(text: &str) -> String {
    let head: String = text.chars().take(WARN_EXCERPT_CHARS).collect();
    if text.chars().count() > WARN_EXCERPT_CHARS {
        format!("{head}…")
    } else {
        head
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

    #[error(
        "refusing to embed an empty text at index {index}: an empty string embeds to a \
         degenerate vector that matches arbitrarily, so the pool it fills would look like \
         a working search and be noise"
    )]
    EmptyInput { index: usize },

    #[error("could not count tokens for an input: {0}")]
    TokenCount(String),
}

#[cfg(test)]
#[path = "embedding_service_tests.rs"]
mod tests;
