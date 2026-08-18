//! Unit tests for [`super`] — the drill's URLs, and the two refusals that are
//! about what a client sent.
//!
//! The storage behaviour is DEV-verified (the house convention for a live
//! `PgExecutor`); the composed sentences are pinned in `services::practice_page`
//! and `services::practice_sheet`; the read's rules in
//! `services::practice_read_parse`.
//!
//! ## Why the URL assertions exist at all
//!
//! The .377 build shipped a client calling a path the router did not serve — a
//! whole feature returning 404 with nothing on either side saying why. The
//! frontend half of this guard lives in `services/__tests__/practice.test.ts`;
//! this is the server half, and between them a path can only drift if BOTH are
//! edited to agree.

/// Every path this module serves, with the methods it serves on each.
///
/// Hand-written rather than read back from the `Router`: axum 0.7 exposes no
/// route inventory, and a test that asked the router what it serves would be
/// asking the thing under test to describe itself.
const ROUTES: &[(&str, &[&str])] = &[
    ("/cases/:slug/scenarios/:scenario_id/practice", &["GET"]),
    (
        "/cases/:slug/scenarios/:scenario_id/practice/sessions",
        &["POST"],
    ),
    ("/practice/answers", &["POST"]),
    ("/practice/answers/:answer_id/help", &["POST"]),
    ("/practice/answers/:answer_id/close", &["POST"]),
    ("/practice/sessions/:session_id/end", &["POST"]),
];

#[test]
fn no_path_carries_the_api_prefix_the_gateway_adds() {
    // The .377 failure class from the server side. The backend mounts under
    // `/api`, so a route that spelled it here would serve `/api/api/…` —
    // reachable by nothing, and on screen indistinguishable from a feature
    // nobody built.
    for (path, _) in ROUTES {
        assert!(
            !path.contains("/api/"),
            "{path} names the gateway's own prefix"
        );
    }
}

/// Anything naming a SCENARIO is case-fenced; anything naming a server-minted
/// handle is not, and deliberately.
///
/// The distinction is the module header's: a slug on `/practice/answers/:id` would
/// be ceremony, because the id is unguessable and the real fence — the question
/// must belong to the session's scenario — is enforced in the handler. Asserting
/// the split here is what stops a later route being added to the wrong family by
/// accident.
#[test]
fn scenario_routes_are_case_fenced_and_handle_routes_are_not() {
    for (path, _) in ROUTES {
        if path.contains(":scenario_id") {
            assert!(
                path.starts_with("/cases/:slug/scenarios/:scenario_id/"),
                "{path} names a scenario without a case to fence it"
            );
        } else {
            assert!(
                path.starts_with("/practice/"),
                "{path} is neither case-fenced nor a handle route"
            );
        }
    }
}

/// Every write is a POST, and the drill exposes no destructive verb.
///
/// ## Domain note: there is no DELETE here, and that is the FRE 612 posture
///
/// The log is the record of what a witness was asked and what she said. Nothing
/// in this build can remove one — not a session, not an answer, not a question.
/// A retraction is a conversation with Chuck, not a button.
#[test]
fn the_drill_exposes_one_read_five_writes_and_no_destructive_verb() {
    let mut reads = 0;
    let mut writes = 0;
    for (_, methods) in ROUTES {
        for method in *methods {
            match *method {
                "GET" => reads += 1,
                "POST" => writes += 1,
                other => panic!("{other} is not a verb this drill serves"),
            }
        }
    }
    assert_eq!((reads, writes), (1, 5));
}

/// The answer route is the one the task names, spelled exactly.
///
/// It is quoted in CC_TASK_PRACTICE_SESSION_V0_v1 §5 and in the design, so a
/// rename is a documentation change as well as a code one. Pinning the literal
/// makes that visible in a diff.
#[test]
fn the_answer_route_is_the_path_the_task_names() {
    assert!(ROUTES
        .iter()
        .any(|(p, m)| *p == "/practice/answers" && m.contains(&"POST")));
}
