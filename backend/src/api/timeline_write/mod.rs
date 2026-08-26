//! The case chronology's WRITE endpoints (Phase C, §C1).
//!
//! ```text
//! POST   /api/timeline/events                      create
//! PUT    /api/timeline/events/:id                  edit the same fields
//! DELETE /api/timeline/events/:id                  SOFT delete (design R10)
//! POST   /api/timeline/events/:id/undelete         the Undo
//! POST   /api/timeline/events/:id/links            link a target
//! DELETE /api/timeline/events/:id/links?…          unlink, by the natural key
//! POST   /api/timeline/events/:id/notes            add an attributed note
//! DELETE /api/timeline/events/:id/notes/:note_id   the author retires their own
//! GET    /api/timeline/documents?q=…               the picker's search (a READ)
//! ```
//!
//! ## Why a directory and not three files beside `timeline.rs`
//!
//! They were three siblings for most of Phase C, and `api::mod` — a pure table
//! of contents — went from 298 non-comment lines to 301 against Rule 17's 300
//! for the three `pub mod` lines that declared them. A directory declares ONE
//! module there instead. The grouping is honest rather than arithmetical: these
//! three files are one surface, they share `support`, and none of them is
//! reachable except through the routes `api::timeline` merges.
//!
//! ## The three modules
//!
//! - [`events`] changes the dated fact itself: create, edit, delete, undo.
//! - [`links`] hangs things off an event that already exists — links, notes —
//!   and serves the document picker's search.
//! - [`support`] is what both share: the refusal-to-status table, the target
//!   resolution the READ handler also uses, and the response composition.
//!
//! ## ⚑ The guard, in one sentence
//!
//! Every mutating handler in this directory takes `user: AuthUser` (a 401 for
//! anonymous before the body runs), stamps the acting user through
//! `services::chronology_guard::open_write`, and ends at `seal_and_commit` —
//! the only committer, so no write can land without its history row. See
//! `events_tests` for the scan that proves it, and `chronology_guard` for why
//! the seal consumes the transaction.

pub mod events;
pub mod links;
pub(super) mod support;
