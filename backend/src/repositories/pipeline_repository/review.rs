//! Review-panel repository surface (re-export hub).
//!
//! Pre-existing callers reach review functions through
//! `super::review::*` (e.g. `crate::repositories::pipeline_repository::review::approve_item`).
//! Splitting this module into focused siblings would normally break
//! that path; instead, every sibling's public surface flows through
//! this re-export hub so call sites need no edits.
//!
//! - [`review_grounding`](super::review_grounding) — `GROUNDED_STATUSES`
//!   whitelist + the three counts that bind it into SQL.
//! - [`review_items`](super::review_items) — `ReviewItemRow`,
//!   `ReviewActionResult`, `ItemTypeInfo` row types + read-only
//!   queries.
//! - [`review_actions`](super::review_actions) — mutating actions
//!   (approve / reject / edit / bulk approve / undo).
//! - [`review_edit_history`](super::review_edit_history) — append-only
//!   field-change audit trail.
//!
//! ## Adding new review functions
//!
//! Place new code in the sibling whose responsibility matches the
//! function, then add a corresponding `pub use` line below so existing
//! call sites using `super::review::*` continue to resolve. Do NOT add
//! function definitions to this file — its only job is the re-export
//! manifest.

pub use super::review_actions::{
    approve_item, bulk_approve, edit_item, reject_item, unapprove_item, unreject_item,
};
pub use super::review_edit_history::{get_edit_history, insert_edit_history, EditHistoryRecord};
pub use super::review_grounding::{
    count_flagged_items_for_document, count_pending, count_ungrounded_pending,
};
pub use super::review_items::{
    count_items, count_relationships_for_item, get_item_by_id, get_item_type_info, list_items,
    ItemTypeInfo, ReviewActionResult, ReviewItemRow,
};
