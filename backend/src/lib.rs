// Library entry point for shared modules (services, models, etc.)
pub mod api;
pub mod auth;
pub mod bias;
pub mod canonical_elements;
pub mod cli;
pub mod config;
pub mod database;
pub mod domain;
pub mod dto;
pub mod error;
pub(crate) mod llm_retry;
pub mod models;
pub mod neo4j;
pub mod oneshot;
pub mod partymerge;
pub mod partyresolve;
pub mod pipeline;
pub mod practice;
pub mod prompt_loader;
pub mod rekey;
pub mod remap;
pub mod repositories;
pub mod restate_endpoint;
pub mod services;
pub mod state;
pub mod twinmerge;

/// Crate-wide SQL invariants, asserted by scanning the source tree.
///
/// Declared at the crate root deliberately: the scan inside covers EVERY `.rs`
/// file under `src/`, not one module's queries, and a reader debugging a bind
/// bug anywhere should be able to find the test that polices it from here. It
/// lived briefly inside `repositories::pipeline_repository` and that was the
/// wrong signal — it implied a scope narrower than the check actually has.
#[cfg(test)]
mod sql_invariants;

/// Crate-wide disk invariants over the extraction profiles, templates and
/// schemas, asserted by scanning them.
///
/// Declared at the crate root for the same reason as [`sql_invariants`]: the
/// scan inside covers EVERY profile YAML, not one module's, and a reader chasing
/// a template/schema disagreement should be able to find the test that polices it
/// from here.
#[cfg(test)]
mod template_invariants;
