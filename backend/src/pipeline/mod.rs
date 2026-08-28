//! Pipeline module: application-level constants and (future) step logic.

/// The Anthropic streaming adapter: `ExtractionEngine` over the Messages API
/// with `"stream": true`. Replaced the Rig-backed `rig_provider` on 2026-08-28
/// after a non-streaming call hit Anthropic's ~10-minute wall mid-generation.
pub mod anthropic_engine;
/// Pure SSE framing and message accumulation — no I/O, so the whole streaming
/// protocol is assertable from a transcript without a paid API call.
pub mod anthropic_stream;
/// The socket half of the streaming call, including the inter-event idle
/// timeout that replaces the whole-request timeout.
pub mod anthropic_transport;
pub mod chunking_strategies;
pub mod config;
pub mod constants;
pub mod context;
/// The pre-ingest edge bar (2026-08-25 rulings) — pure verdicts over pass-2
/// relationship output. See the module docs for the three rules.
pub mod edge_bar;
/// Configuration and reporting for the edge bar — kept out of `edge_bar` so
/// that module stays pure and directly assertable.
pub mod edge_bar_report;
pub mod extraction_engine;
pub mod providers;
pub mod registry;
pub mod rig_llm_bridge;
pub mod step_progress;
pub mod step_recorder;
pub mod steps;
pub mod task;
/// Truncation detection at the provider boundary — census R-3's fix.
pub mod truncation;
pub mod validation;
pub mod workflow;
pub mod workflow_admin;
pub mod workflow_steps;
