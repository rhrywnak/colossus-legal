//! Library interface for the Colossus-Legal document processor CLI.

use anyhow::Result;

pub mod config;
pub mod paths;
pub mod prompt;
pub mod llm;
pub mod claims;
pub mod logging;
pub mod chunking;
pub mod dates;

/// Placeholder library entrypoint (not used by the current binary).
pub async fn run_document_processor_from_args() -> Result<()> {
    Ok(())
}
