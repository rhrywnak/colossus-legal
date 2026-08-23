//! Who is allowed to sign a change, from the login.
//!
//! ## What used to be here, and why it is not
//!
//! This module also composed one NOTE for every panel that rendered one — the
//! day it was written, and the "struck Tue 19 Aug" line under a withdrawn one.
//! Roman's ruling of 2026-08-23 retired notes from the interface entirely:
//! Chuck does not write them, he reviews Marie's answers with her at their
//! weekly meeting. The composing functions went with the panels that called
//! them.
//!
//! ## ⚑ The `practice_notes` TABLE IS UNTOUCHED
//!
//! Not one row was deleted, and none will be. The UI stops reading them; the
//! table keeps what it has. (Measured 2026-08-23: it holds ZERO rows on DEV —
//! every scenario, every level — so nothing is in fact being hidden.)
//!
//! `attribution` stays because the deck EDITOR signs its changes with it, and
//! the editor survives this task.

use crate::auth::AuthUser;

/// Who a write is attributed to: the stable id, and the name a screen prints.
///
/// ## Why there are no selectors any more
///
/// There used to be two — "Editing as" on the deck editor and an author picker
/// on every note — and behind them a stored allow-list of display names. The
/// premise was that this build has one shared login and therefore cannot know
/// who is acting. That premise was WRONG: Chuck and Marie have had logins since
/// March, and every request already arrives with an authenticated user. The
/// selectors were asking a question the server could already answer — and then,
/// worse, silently refusing to work until somebody answered it.
///
/// ## Rust Learning: returning a tuple struct's worth of data without the struct
///
/// Two `String`s that always travel together would normally earn a struct. They
/// do not here because every caller destructures them immediately into two
/// columns (`author` and `author_id`), and a struct would add a name to import
/// at eight call sites to save nothing at any of them. The ORDER is the risk —
/// both are strings — so the return is `(id, name)` in the same order the
/// columns are declared everywhere, and every call site binds them by name.
pub fn attribution(user: &AuthUser) -> (String, String) {
    // `display_name` is what the screen prints beside a note; `username` is what
    // identifies the person when somebody is renamed in Authentik. A display
    // name that is blank — possible, if Authentik has no name for an account —
    // falls back to the username rather than rendering an empty author.
    let name = if user.display_name.trim().is_empty() {
        user.username.clone()
    } else {
        user.display_name.clone()
    };
    (user.username.clone(), name)
}

#[cfg(test)]
#[path = "practice_notes_tests.rs"]
mod tests;
