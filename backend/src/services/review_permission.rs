//! Who may press "Done reviewing".
//!
//! ## Why this is its own module (CC_TASK_REVIEW_PERMISSION_v1)
//!
//! `practice_reviewer_usernames` did two unrelated jobs: it said who may press
//! the button, and it said whose names the war room prints. Roman took himself
//! off the list so the dashboard would stop naming him, and lost the ability to
//! review with it (2026-09-22). **Permission must never depend on a display
//! list**, so the rule lives here, apart from both.
//!
//! ## Reusability checkpoint
//!
//! No case name, no group name, no settings key. The caller passes
//! `user.is_admin()` — the group vocabulary (`GROUP_ADMIN`) stays in
//! `colossus-auth`, and the shown list arrives as a plain slice. `colossus-ai`
//! can take this file unchanged.

/// Whether this signed-in user may mark a deck reviewed.
///
/// Two ways in, and they are deliberately different in kind:
/// - **listed** — their login is one of the `shown_reviewers` (the settings
///   pair, which since v1 of this task is a DISPLAY list);
/// - **admin** — they hold the deployment's admin group, whether or not any
///   screen names them.
///
/// ## Domain note: what a blank id means here
///
/// A blank `user_id` is refused before anything else, even for an admin. The
/// bench reader drops blank entries so the list cannot hold one, but `"" == ""`
/// is true, and this is the comparison where being wrong hands a write control
/// to somebody the auth layer could not name.
///
/// ## ⚑ `AuthMode::Optional` makes the local anonymous user an ADMIN
///
/// `colossus_auth::AuthUser::anonymous()` carries the admin group, and the
/// extractor returns it when no `X-authentik-*` headers arrive AND
/// `AUTH_MODE=optional`. On a laptop with no proxy, then, everybody may review.
/// That is the intended local-development posture, and it is safe in the
/// deployments because both DEV and PROD run `AUTH_MODE=required`
/// (`colossus-ansible/roles/colossus-legal/templates/colossus-legal-backend.env.j2`,
/// verified on both hosts 2026-09-22); an unset or unknown value also reads as
/// `Required`, which is the crate's safe default.
///
/// ## Rust Learning: `iter().any()` over a slice
///
/// `any` short-circuits on the first match and returns a plain `bool` — no
/// allocation and no `HashSet` built per request for a list of two. `r == user_id`
/// compares a `&String` with a `&str` through `PartialEq<str>`, so neither side
/// needs `.as_str()`.
pub fn may_review(user_id: &str, is_admin: bool, shown_reviewers: &[String]) -> bool {
    if user_id.trim().is_empty() {
        return false;
    }
    is_admin || shown_reviewers.iter().any(|r| r == user_id)
}

#[cfg(test)]
#[path = "review_permission_tests.rs"]
mod tests;
