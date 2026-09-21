//! # colossus-chat — a case-blind chat engine over the Anthropic Messages API
//!
//! This crate knows how to hold a conversation with a model: stream it, cache
//! the expensive prefix, hand it documents it must quote from, let it call
//! tools, and let the provider compact a long history. It knows NOTHING about
//! any particular case, witness, or legal matter. Everything specific — the
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

pub mod sse;
pub mod transport;
