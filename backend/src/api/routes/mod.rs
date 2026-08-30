//! The API's route TABLE, one module per group.
//!
//! ## Why this directory exists (R-3, ruled 2026-08-26)
//!
//! `api/mod.rs` had grown to 299 non-comment lines: seventy `pub mod`
//! declarations, `router()`, and eleven route-group functions that between them
//! were two thirds of the file. The next endpoint group anywhere would have
//! pushed it past Rule 17's 300-line limit, so the ruling was that the groups
//! move out BEFORE that group is written — which is what this directory is.
//!
//! `api/mod.rs` keeps exactly three things: the module declarations, `router()`
//! as a table of contents, and the `/health` handler that is deliberately not
//! part of the API router. Everything that says WHICH PATH GOES WHERE lives
//! here.
//!
//! ## ⚑ Nothing moved but the code
//!
//! The split promised one property and nothing else: the same methods on the
//! same paths, before and after. `api::route_table_tests` walks the built
//! router and pins the whole `(method, path)` table, so a path that lost a
//! segment in the move is a failed test rather than a 404 somebody finds on DEV.
//!
//! ## Why groups, and not one module per handler
//!
//! Several handler modules already carry their own `routes()` — `case_health`,
//! `scenario_facts`, `rehearsal`, `practice`, `settings`, `timeline` — and those
//! are untouched: a module that owns a surface owns its paths. The eleven here
//! are the ones that never had a single owning module. `entity_routes` spans
//! seven handler modules and `admin_ops_routes` five; giving each of those a
//! home beside one of its handlers would have picked a winner arbitrarily. They
//! are groups because they are groups.

pub mod admin_document;
pub mod admin_ops;
pub mod case;
pub mod claim;
pub mod decomposition;
pub mod document;
pub mod entity;
pub mod interaction;
pub mod query;
pub mod scenario;
pub mod session;
