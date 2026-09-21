//! The tool contract: what a caller implements to let the model fetch things.
//!
//! ## Rust Learning: `Arc<dyn ChatTool>` — a registry of trait objects
//!
//! A turn offers a *list* of tools of different concrete types (one reads a
//! document, one reads answer history, …). A `Vec` needs one element type, so each
//! tool is stored as `dyn ChatTool` — a trait object, dispatched through a vtable
//! at runtime. `Arc` makes it cheaply shareable across the concurrent tool calls
//! of one round (each call gets a clone of the pointer, not of the tool), and the
//! `Send + Sync` bounds are what let those calls run on tokio's worker threads.

use serde_json::Value;

use crate::request::ToolSpec;

/// A tool the model may call.
#[async_trait::async_trait]
pub trait ChatTool: Send + Sync {
    /// The name the model calls it by. Must match [`ChatTool::spec`]'s name.
    fn name(&self) -> &str;

    /// The tool's declaration, sent in the request.
    fn spec(&self) -> ToolSpec;

    /// Run the tool on the model's input.
    ///
    /// # Errors
    /// A message for the MODEL, sent back as an `is_error` tool result. The
    /// implementor logs the operator-facing detail itself.
    async fn run(&self, input: Value) -> Result<String, String>;
}
