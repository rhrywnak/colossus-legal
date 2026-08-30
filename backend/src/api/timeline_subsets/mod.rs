//! Timeline subsets — the endpoints (T1.3).
//!
//! ```text
//! GET    /api/timeline/subsets                         the home section's list
//! POST   /api/timeline/subsets                         create, with its events
//! GET    /api/timeline/subsets/:id                     the subset, events joined
//! PUT    /api/timeline/subsets/:id                     name and/or description
//! PUT    /api/timeline/subsets/:id/events              REPLACE the ordered set
//! DELETE /api/timeline/subsets/:id                     SOFT delete
//! POST   /api/timeline/subsets/:id/undelete            the Undo
//!
//! GET    /api/cases/:slug/scenarios/:sid/subsets       the button's data
//! POST   /api/cases/:slug/scenarios/:sid/subsets       attach
//! DELETE /api/cases/:slug/scenarios/:sid/subsets/:ssid detach
//! ```
//!
//! ## ⚑ THE READ/WRITE LINE
//!
//! The three `GET`s take `Option<AuthUser>` and are open — looking at a story is
//! not privileged, exactly as looking at the chronology is not (chronology Phase
//! A). Every other handler takes `AuthUser`, so an anonymous request is a 401
//! before the body runs, and `writes::tests` scans this directory and fails if
//! one is ever declared with the optional extractor.
//!
//! ## ⚑ ONE WRITE PATH
//!
//! No handler here opens a transaction and no handler here calls the
//! repository's write module. Every mutation is exactly one call into
//! `services::chronology_subset_write`, which owns its transaction and ends at
//! the seal that writes the history row.
//!
//! ## CRITICAL — the pipeline pool
//!
//! Every table this directory touches lives in `colossus_legal_v2`, so every
//! query uses `&state.pipeline_pool`, NOT `state.pg_pool`.
//!
//! ## The four modules
//!
//! - [`reads`] serves the three open reads.
//! - [`writes`] is the subset's own five mutations.
//! - [`scenario_links`] is the attach/detach pair and the button's read, under
//!   the scenario path where the case fence lives.
//! - [`support`] is what they share: the refusal-to-status table, the id parse,
//!   and the response composition every write answers with.

pub mod reads;
pub mod scenario_links;
pub(super) mod support;
pub mod writes;
