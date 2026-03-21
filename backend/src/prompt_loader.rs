//! Loads LLM prompt templates from disk at startup.
//!
//! Prompts are text files stored in a configurable directory (default:
//! `/data/documents/prompts/`). If a file is missing or unreadable,
//! the loader returns `None` and the caller falls back to its
//! hardcoded default.
//!
//! ## Why load at startup, not per-request?
//!
//! Prompts change rarely (manual edit + container restart). Loading once
//! avoids per-request disk I/O and keeps the hot path allocation-free.
//! To pick up changes, restart the container.
//!
//! ## Rust Learning: std::fs::read_to_string
//!
//! `std::fs::read_to_string(path)` reads the entire file into a `String`.
//! It returns `io::Result<String>` — `Ok(content)` if successful,
//! `Err(e)` if the file doesn't exist or can't be read.
//! We use `.ok()` to convert the `Result` to `Option`, discarding the
//! error details (we log them first).

use std::path::Path;

// ---------------------------------------------------------------------------
// Default synthesis prompt (moved from main.rs to co-locate with loader)
// ---------------------------------------------------------------------------

/// Default synthesis system prompt used when no external file is found.
///
/// This is the exact prompt previously hardcoded in `main.rs` as `SYSTEM_PROMPT`.
/// It includes 7 RULES for evidence-based answers plus FORMATTING instructions
/// for markdown output.
pub const DEFAULT_SYNTHESIS_PROMPT: &str = r#"You are a legal research assistant analyzing case evidence.

You have been given evidence from a case knowledge graph, including verbatim quotes from sworn testimony, court filings, and documentary evidence. Each piece of evidence includes its source document and page number where available.

RULES:
1. Answer using ONLY the provided evidence. Do not infer facts not present in the evidence.
2. For every factual claim in your answer, cite the specific evidence ID in parentheses, e.g., (evidence-phillips-q73).
3. When evidence items contradict each other, note the contradiction explicitly and identify which party made each statement.
4. If the provided evidence does not contain enough information to answer the question, say so clearly. Do not speculate.
5. Use plain language accessible to a non-lawyer, but maintain legal precision for citations.
6. When describing patterns (e.g., "Phillips repeatedly..."), list each specific instance with its citation.
7. When citing evidence, ALWAYS include the source document title and page number in parentheses after the quote. Format: '[verbatim quote]' (Document Title, p.XX). If page number is not available, include only the document title. Use inline quotes, NOT markdown blockquote syntax (do not use > for quotes). Keep all citations flowing naturally within paragraphs.

FORMATTING:
- Use markdown formatting in your response.
- Use **bold** for key names, dates, and legal terms on first mention.
- Use ## headers to organize multi-part answers into clear sections.
- Use > blockquotes for verbatim quotes from evidence.
- Use numbered or bulleted lists when presenting multiple items.
- Keep paragraphs focused — one main point per paragraph.
- Do NOT use # (h1) headers — start with ## (h2) at the highest level.
- Do NOT over-format. If the answer is a single paragraph, just write the paragraph without headers or lists."#;

// ---------------------------------------------------------------------------
// Prompt loader
// ---------------------------------------------------------------------------

/// Loaded prompt templates. Fields are `Option` — `None` means
/// "file not found, use hardcoded default."
pub struct LoadedPrompts {
    /// Synthesis system prompt (for LegalAssembler).
    pub synthesis: Option<String>,
    /// Decomposition prompt template (for LlmDecomposer).
    /// Contains `{docs}`, `{persons}`, `{question}`, `{strategy}` placeholders.
    pub decomposition: Option<String>,
}

/// Load all prompt files from the given directory.
///
/// Expected files:
///
/// - `synthesis.md` — synthesis system prompt
/// - `decomposition.md` — decomposition prompt template
///
/// Missing files are logged at `info` level (not a warning — this is
/// expected on first deploy before prompt files are created).
pub fn load_prompts(dir: &Path) -> LoadedPrompts {
    tracing::info!(dir = %dir.display(), "Loading prompt templates from disk");

    let synthesis = load_file(&dir.join("synthesis.md"), "synthesis");
    let decomposition = load_file(&dir.join("decomposition.md"), "decomposition");

    let loaded_count = [&synthesis, &decomposition]
        .iter()
        .filter(|p| p.is_some())
        .count();

    tracing::info!(
        loaded = loaded_count,
        total = 2,
        "Prompt loading complete (missing prompts will use compiled defaults)"
    );

    LoadedPrompts {
        synthesis,
        decomposition,
    }
}

/// Load a single prompt file. Returns `None` if the file doesn't exist
/// or can't be read.
fn load_file(path: &Path, name: &str) -> Option<String> {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            let trimmed = content.trim().to_string();
            if trimmed.is_empty() {
                tracing::info!(
                    prompt = name,
                    path = %path.display(),
                    "Prompt file exists but is empty — using compiled default"
                );
                None
            } else {
                tracing::info!(
                    prompt = name,
                    path = %path.display(),
                    chars = trimmed.len(),
                    "Loaded prompt from file"
                );
                Some(trimmed)
            }
        }
        Err(e) => {
            tracing::info!(
                prompt = name,
                path = %path.display(),
                error = %e,
                "Prompt file not found — using compiled default"
            );
            None
        }
    }
}
