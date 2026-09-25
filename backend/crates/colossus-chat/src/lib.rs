//! # colossus-chat — a case-blind chat engine over the Anthropic Messages API
//!
//! This crate knows how to hold a conversation with a model: stream it, cache
//! the expensive prefix, hand it documents it must quote from, let it call
//! tools, and let the provider compact a long history. It knows NOTHING about
//! any particular project, person, or subject matter. Everything specific — the
//! system prompt, the documents, the tools, the model id — arrives from the
//! caller as configuration and data.
//!
//! ## Reusability checkpoint (Standing Rule 11)
//!
//! "Could colossus-ai use this with zero code changes?" — yes, by construction:
//! no string in this crate names a person, a document type, or a domain term,
//! and a test in the consuming repo scans for exactly that.
//!
//! ## Modules
//!
//! - [`sse`] — `text/event-stream` framing (shared with the extraction engine).
//! - [`transport`] — the idle-timeout stream driver (shared with the extraction engine).
//! - [`request`] — the caller's request and the Messages body built from it.
//! - [`accumulate`] — full-fidelity reassembly of a streamed assistant message.
//! - [`usage`] — token accounting including cache reads and writes.
//! - [`citation`] — every quotation checked against the caller's stored text.
//! - [`tools`] — the tool contract.
//! - [`backend`] — one streamed call over HTTP, behind a trait.
//! - [`engine`] — the bounded tool loop that runs one conversational turn.
//! - [`keepwarm`] — when to re-send a cached prefix so the provider keeps it.

pub mod accumulate;
pub mod backend;
mod blocks;
pub mod citation;
pub mod engine;
pub mod keepwarm;
pub mod request;
pub mod sse;
pub mod tools;
pub mod transport;
pub mod usage;

pub use accumulate::{AssistantMessage, ChatStreamError};
pub use backend::{AnthropicBackend, ChatBackend, ChatTransportError, EngineConfig};
pub use citation::{RejectedCitation, VerifiedCitation};
pub use engine::{run_turn, ChatError, ChatEvent, ChatOutcome};
pub use request::{
    build_prewarm_body, CacheTtl, ChatDocument, ChatRequest, Message, RequestError, Role, ToolSpec,
};
pub use tools::ChatTool;
pub use usage::Usage;
