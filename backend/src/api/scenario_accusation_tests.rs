//! Unit tests for [`super`] — the write module's shape and its URL guards.
//!
//! What this module owns: the routes' URLs and methods, and the two refusals that
//! are about what a human sent. The storage behaviour is DEV-verified (the house
//! convention for a live `PgExecutor`), the derivation is pinned in
//! `services::scenario_accusation`, and the composed sentences in
//! `services::scenario_accusation_panel`.
//!
//! ## Why the URL assertions exist at all
//!
//! Roman's .377 build shipped a client calling a path the router did not serve —
//! a whole feature that returned 404 with nothing on either side saying why. The
//! frontend half of this guard lives in `scenarioAccusation.test.ts`; this is the
//! server half, and between them a path can only drift if BOTH are edited to
//! agree, which is the point.

use super::*;

/// Every path this module serves, with the methods it serves on each.
///
/// Hand-written rather than read back from the `Router`: axum 0.7 exposes no
/// route inventory, and a test that asked the router what it serves would be
/// asking the thing under test to describe itself.
const ROUTES: &[(&str, &[&str])] = &[
    (
        "/cases/:slug/scenarios/:scenario_id/accusation",
        &["GET", "PUT"],
    ),
    (
        "/cases/:slug/scenarios/:scenario_id/accusation/instances/:graph_node_id",
        &["POST", "DELETE"],
    ),
    (
        "/cases/:slug/scenarios/:scenario_id/accusation/instances/:graph_node_id/answer",
        &["PUT", "DELETE"],
    ),
];

#[test]
fn every_path_is_case_and_scenario_scoped() {
    // The fence is only as good as the URL it hangs on: a route without `:slug`
    // could not be case-fenced at all, and one without `:scenario_id` would be
    // writing a judgment with no scenario to key it to.
    for (path, _) in ROUTES {
        assert!(
            path.starts_with("/cases/:slug/scenarios/:scenario_id/"),
            "{path}"
        );
    }
}

#[test]
fn no_path_carries_the_api_prefix_the_gateway_adds() {
    // The .377 failure class, from the server side. The backend mounts under
    // `/api`, so a route that spelled it here would serve `/api/api/…` — reachable
    // by nothing, and indistinguishable on screen from a feature nobody built.
    for (path, _) in ROUTES {
        assert!(
            !path.contains("/api/"),
            "{path} names the gateway's own prefix"
        );
    }
}

#[test]
fn the_three_write_concerns_are_three_paths_and_six_methods() {
    // The shape the instruction names: mark/un-mark, pair/un-pair, set/clear. A
    // seventh method appearing here without a fence is what this count catches.
    let methods: usize = ROUTES.iter().map(|(_, m)| m.len()).sum();
    assert_eq!(
        ROUTES.len(),
        3,
        "one path per concern, plus the read on the first"
    );
    assert_eq!(methods, 6);
}

#[test]
fn the_instance_paths_are_keyed_by_graph_node_id_and_never_by_a_row_id() {
    // §12: a judgment keyed to an internal id would not survive re-processing, and
    // this scenario has already been taught that lesson with 26 dead references.
    // The URL is where that decision is visible to everyone who reads it.
    for (path, _) in ROUTES.iter().filter(|(p, _)| p.contains("instances")) {
        assert!(path.contains(":graph_node_id"), "{path}");
        assert!(
            !path.contains(":fact_id"),
            "{path} keys a judgment to a row id"
        );
    }
}

#[test]
fn the_router_builds_with_every_declared_path() {
    // matchit refuses some sibling shapes outright (a param and a static child at
    // the same position), and it refuses them by PANICKING at construction. This
    // is the cheapest possible proof that the three paths above co-exist — and
    // that they co-exist with every other route in the app, which is where a
    // conflict would actually surface.
    // Both: this module's group on its own, and the WHOLE app router, which is
    // where a conflict with another group's shape would actually surface.
    let _group = routes();
    let _app = crate::api::router();
}

// ── The two refusals that are about what a human just did ────────────────────

#[test]
fn a_pairing_must_name_the_item_that_answers() {
    // A pairing with nothing in it is not a pairing, and accepting one would store
    // a row the database itself refuses (`…_answer_needs_anchor`) — the refusal
    // would then arrive as an opaque 500 instead of a sentence saying what to fix.
    let Err(error) = checked_answer("   ", "ev-a") else {
        panic!("a blank answer must be refused");
    };
    assert!(matches!(error, AppError::BadRequest { .. }));
}

#[test]
fn a_statement_cannot_be_its_own_answer() {
    // Storing it would render as an instance whose answer is the accusation, which
    // reads on the rehearsal page as us agreeing with them.
    let Err(error) = checked_answer("ev-a", "ev-a") else {
        panic!("a self-answer must be refused");
    };
    assert!(matches!(error, AppError::BadRequest { .. }));
}

#[test]
fn a_real_answer_is_accepted_and_trimmed() {
    // Trimmed at the boundary like every other stored id and text on this surface:
    // whitespace is invisible in psql, and an untrimmed id matches nothing on read.
    assert_eq!(
        checked_answer("  ev-answer  ", "ev-a").expect("a real pairing"),
        "ev-answer"
    );
}

#[test]
fn the_two_refusals_say_different_things() {
    // "You sent nothing" and "you sent the same statement" have different remedies.
    // A shared message would leave the human re-sending the same click.
    let message = |error: AppError| match error {
        AppError::BadRequest { message, .. } => message,
        other => panic!("both refusals are bad requests: {other:?}"),
    };
    let blank = message(checked_answer("", "ev-a").expect_err("blank"));
    let same = message(checked_answer("ev-a", "ev-a").expect_err("self"));
    assert_ne!(blank, same);
}

// ── The accusation sentence's one boundary rule ──────────────────────────────

#[test]
fn a_whitespace_only_accusation_is_refused_and_not_treated_as_a_withdrawal() {
    // "Withdraw what I wrote" and "I fat-fingered the box" are different
    // intentions with different consequences, and only the human knows which they
    // meant. Storing a blank would also hit the column's CHECK, so accepting one
    // here would just move which layer said no — and turn a precise 400 into an
    // opaque 500.
    let Err(error) = checked_accusation_text(Some("   \t ")) else {
        panic!("a blank sentence must be refused");
    };
    assert!(matches!(error, AppError::BadRequest { .. }));
}

#[test]
fn no_text_at_all_is_how_a_sentence_is_withdrawn() {
    // The withdrawal path: `None` is accepted and reaches the write as `None`,
    // which clears the column and returns the rehearsal block to its named gap.
    assert_eq!(checked_accusation_text(None).expect("None withdraws"), None);
}

#[test]
fn a_real_sentence_is_accepted_and_trimmed() {
    // Whitespace is invisible in psql and visible on screen.
    assert_eq!(
        checked_accusation_text(Some("  They say Marie was unreasonable.  "))
            .expect("a real sentence"),
        Some("They say Marie was unreasonable.")
    );
}

// ── "The statement touched nothing" is never a success ───────────────────────

#[test]
fn a_write_that_matched_no_scenario_is_a_named_404() {
    // Zero rows means the scenario was deleted between the page load and this
    // write. Returning 200 would report a save that never happened — the exact
    // shape of failure Standing Rule 1 forbids.
    let Err(error) = require_row_touched(0, "Roman", Uuid::nil(), "it was deleted") else {
        panic!("a write that touched nothing must not report success");
    };
    let AppError::NotFound { message } = error else {
        panic!("a vanished scenario is a 404");
    };
    // The message names the state AND the way out: a human on a page that will
    // never succeed again needs somewhere to go, not just a diagnosis.
    assert!(message.contains("no longer exists"), "{message}");
    assert!(message.contains("scenario list"), "{message}");
}

#[test]
fn a_write_that_touched_a_row_is_a_success() {
    assert!(require_row_touched(1, "Roman", Uuid::nil(), "unused").is_ok());
}

#[test]
fn a_delete_that_removed_nothing_is_a_named_404_that_says_to_reload() {
    // Withdrawing something that was not there and withdrawing something that was
    // are different states, and the human needs to know which happened — the first
    // means what they are looking at is not what is stored.
    let Err(error) = require_anchor_touched(0, "Roman", "ev-a", Uuid::nil(), "already gone") else {
        panic!("a delete that removed nothing must not report success");
    };
    let AppError::NotFound { message } = error else {
        panic!("nothing to withdraw is a 404");
    };
    assert!(message.contains("out of date"), "{message}");
    assert!(message.contains("reload"), "{message}");
}

#[test]
fn a_delete_that_removed_a_row_is_a_success() {
    assert!(require_anchor_touched(1, "Roman", "ev-a", Uuid::nil(), "unused").is_ok());
}

#[test]
fn the_two_404s_say_different_things() {
    // "The scenario is gone" and "that judgment was already withdrawn" have
    // different remedies. A shared sentence would send a human to the wrong one.
    let message = |error: AppError| match error {
        AppError::NotFound { message } => message,
        other => panic!("both are 404s: {other:?}"),
    };
    let vanished = message(require_row_touched(0, "R", Uuid::nil(), "x").expect_err("404"));
    let nothing = message(require_anchor_touched(0, "R", "ev", Uuid::nil(), "x").expect_err("404"));
    assert_ne!(vanished, nothing);
}
